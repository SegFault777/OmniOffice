// SPDX-License-Identifier: GPL-3.0-only
//! OmniCel core: sparse sheet, formula evaluator, cell formats, charts, fill/sort helpers, CSV and the OMC container.
//! Formulas: + - * / comparisons, $absolute references, ranges and SUM AVERAGE MIN MAX COUNT COUNTA MEDIAN IF AND OR NOT ABS ROUND
//! SUMIF COUNTIF CONCAT LEN UPPER LOWER. OMC = stored ZIP: mimetype, manifest.json, sheet.csv, formats.csv, charts.csv.
use omni_common::zipio;
use std::collections::{BTreeMap, HashSet};

#[derive(Clone, Debug, PartialEq)]
pub enum Value { Num(f64), Text(String), Err(&'static str) }
impl Value {
    pub fn show(&self) -> String { match self { Value::Num(n) if n.fract() == 0.0 && n.abs() < 1e15 => format!("{}", *n as i64), Value::Num(n) => format!("{n}"), Value::Text(t) => t.clone(), Value::Err(e) => e.to_string() } }
}
pub fn col_name(mut c: u32) -> String { let mut s = String::new(); loop { s.insert(0, (b'A' + (c % 26) as u8) as char); if c < 26 { break; } c = c / 26 - 1; } s }
/// "B12" or "$B$12" -> (row 11, col 1)
pub fn parse_ref(s: &str) -> Option<(u32, u32)> {
    let s: String = s.chars().filter(|c| *c != '$').collect();
    let l = s.chars().take_while(|c| c.is_ascii_alphabetic()).count(); if l == 0 || l == s.len() { return None; }
    let col = s[..l].to_ascii_uppercase().bytes().fold(0u32, |a, b| a.saturating_mul(26) + (b - b'A') as u32 + 1) - 1;
    let row: u32 = s[l..].parse().ok()?; if row == 0 { None } else { Some((row - 1, col)) }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum NumFmt { #[default] General, Fixed2, Percent, Currency, Thousands }
impl NumFmt {
    pub fn code(self) -> u8 { self as u8 }
    pub fn from_code(c: u8) -> NumFmt { [NumFmt::General, NumFmt::Fixed2, NumFmt::Percent, NumFmt::Currency, NumFmt::Thousands].get(c as usize).copied().unwrap_or_default() }
}
fn group(n: f64, decimals: usize) -> String {
    let s = format!("{:.*}", decimals, n.abs()); let (i, f) = s.split_once('.').map_or((s.as_str(), ""), |(a, b)| (a, b));
    let mut o = String::new(); for (k, ch) in i.chars().enumerate() { if k > 0 && (i.len() - k) % 3 == 0 { o.push(','); } o.push(ch); }
    format!("{}{}{}", if n < 0.0 { "-" } else { "" }, o, if f.is_empty() { String::new() } else { format!(".{f}") })
}
pub fn format_num(n: f64, f: NumFmt) -> String {
    match f { NumFmt::General => Value::Num(n).show(), NumFmt::Fixed2 => format!("{n:.2}"), NumFmt::Percent => format!("{:.2}%", n * 100.0),
        NumFmt::Currency => if n < 0.0 { format!("-${}", group(-n, 2)) } else { format!("${}", group(n, 2)) }, NumFmt::Thousands => group(n, if n.fract() == 0.0 { 0 } else { 2 }) }
}
/// Cell formatting. `align`: 0 automatic (numbers right, text left), 1 left, 2 center, 3 right.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CellFmt { pub bold: bool, pub align: u8, pub num: NumFmt, pub fill: Option<u32>, pub color: Option<u32> }
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ChartKind { #[default] Bar, Line, Pie }
/// A chart over `range` (r1, c1, r2, c2) drawn with its top-left corner at cell `at`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Chart { pub kind: ChartKind, pub range: (u32, u32, u32, u32), pub at: (u32, u32), pub title: String }

#[derive(Default, Clone, Debug)]
pub struct Sheet { cells: BTreeMap<(u32, u32), String>, fmts: BTreeMap<(u32, u32), CellFmt>, pub charts: Vec<Chart> }
impl Sheet {
    pub fn set(&mut self, r: u32, c: u32, raw: &str) { if raw.is_empty() { self.cells.remove(&(r, c)); } else { self.cells.insert((r, c), raw.to_string()); } }
    pub fn raw(&self, r: u32, c: u32) -> &str { self.cells.get(&(r, c)).map_or("", |s| s.as_str()) }
    pub fn dims(&self) -> (u32, u32) { self.cells.keys().fold((0, 0), |a, k| (a.0.max(k.0 + 1), a.1.max(k.1 + 1))) }
    pub fn fmt(&self, r: u32, c: u32) -> CellFmt { self.fmts.get(&(r, c)).copied().unwrap_or_default() }
    pub fn update_fmt(&mut self, r1: u32, c1: u32, r2: u32, c2: u32, f: impl Fn(&mut CellFmt)) {
        for r in r1.min(r2)..=r1.max(r2) { for c in c1.min(c2)..=c1.max(c2) { let mut x = self.fmt(r, c); f(&mut x); if x == CellFmt::default() { self.fmts.remove(&(r, c)); } else { self.fmts.insert((r, c), x); } } }
    }
    pub fn eval(&self, r: u32, c: u32) -> Value { self.eval_in(r, c, &mut HashSet::new()) }
    /// Text shown in the grid: the value with the cell's number format applied.
    pub fn display(&self, r: u32, c: u32) -> String { match self.eval(r, c) { Value::Num(n) => format_num(n, self.fmt(r, c).num), v => v.show() } }
    fn eval_in(&self, r: u32, c: u32, seen: &mut HashSet<(u32, u32)>) -> Value {
        let raw = self.raw(r, c);
        let Some(f) = raw.strip_prefix('=') else { return raw.trim().parse::<f64>().map(Value::Num).unwrap_or_else(|_| Value::Text(raw.to_string())) };
        if !seen.insert((r, c)) { return Value::Err("#CYCLE!"); }
        let mut p = P { s: f.chars().collect(), i: 0, sh: self, seen };
        let v = p.cmp(); p.ws(); let v = if p.i < p.s.len() { Value::Err("#PARSE!") } else { v };
        p.seen.remove(&(r, c)); v
    }
    /// Raw contents of a range as tab-separated text (rows end with \n).
    pub fn range_tsv(&self, r1: u32, c1: u32, r2: u32, c2: u32) -> String {
        let (ra, rb, ca, cb) = (r1.min(r2), r1.max(r2), c1.min(c2), c1.max(c2)); let mut o = String::new();
        for r in ra..=rb { for c in ca..=cb { if c > ca { o.push('\t'); } o.push_str(self.raw(r, c).replace(['\t', '\n'], " ").as_str()); } o.push('\n'); }
        o
    }
    /// Paste tab-separated text with its top-left corner at (r, c). Returns (rows, columns) written.
    pub fn paste_tsv(&mut self, r: u32, c: u32, s: &str) -> (u32, u32) {
        let (mut rows, mut cols) = (0u32, 0u32);
        for (i, line) in s.replace("\r\n", "\n").trim_end_matches('\n').split('\n').enumerate() { for (j, v) in line.split('\t').enumerate() { self.set(r + i as u32, c + j as u32, v); cols = cols.max(j as u32 + 1); } rows = i as u32 + 1; }
        (rows, cols)
    }
    pub fn clear_range(&mut self, r1: u32, c1: u32, r2: u32, c2: u32) { for r in r1.min(r2)..=r1.max(r2) { for c in c1.min(c2)..=c1.max(c2) { self.set(r, c, ""); } } }
    /// Copy the first row of the range down; formulas are adjusted like Excel's fill handle (relative references shift).
    pub fn fill_down(&mut self, r1: u32, c1: u32, r2: u32, c2: u32) {
        let (ra, rb) = (r1.min(r2), r1.max(r2)); for c in c1.min(c2)..=c1.max(c2) { let src = self.raw(ra, c).to_string(); for r in ra + 1..=rb { self.set(r, c, &if src.starts_with('=') { shift_formula(&src, (r - ra) as i32, 0) } else { src.clone() }); } }
    }
    pub fn fill_right(&mut self, r1: u32, c1: u32, r2: u32, c2: u32) {
        let (ca, cb) = (c1.min(c2), c1.max(c2)); for r in r1.min(r2)..=r1.max(r2) { let src = self.raw(r, ca).to_string(); for c in ca + 1..=cb { self.set(r, c, &if src.starts_with('=') { shift_formula(&src, 0, (c - ca) as i32) } else { src.clone() }); } }
    }
    /// Sort the rows of a range by the values in column `key` (numbers before text). Formulas are moved as text, not adjusted.
    pub fn sort_rows(&mut self, r1: u32, c1: u32, r2: u32, c2: u32, key: u32, asc: bool) {
        let (ra, rb, ca, cb) = (r1.min(r2), r1.max(r2), c1.min(c2), c1.max(c2));
        let mut rows: Vec<(Value, Vec<String>)> = (ra..=rb).map(|r| (self.eval(r, key), (ca..=cb).map(|c| self.raw(r, c).to_string()).collect())).collect();
        let ord = |a: &Value, b: &Value| match (a, b) { (Value::Num(x), Value::Num(y)) => x.partial_cmp(y).unwrap_or(std::cmp::Ordering::Equal), (Value::Num(_), _) => std::cmp::Ordering::Less, (_, Value::Num(_)) => std::cmp::Ordering::Greater, _ => a.show().to_lowercase().cmp(&b.show().to_lowercase()) };
        rows.sort_by(|a, b| if asc { ord(&a.0, &b.0) } else { ord(&b.0, &a.0) });
        for (i, (_, cells)) in rows.into_iter().enumerate() { for (j, v) in cells.iter().enumerate() { self.set(ra + i as u32, ca + j as u32, v); } }
    }
    /// Put =SUM(...) in (r, c) over the numbers directly above (or, failing that, directly to the left).
    pub fn autosum(&mut self, r: u32, c: u32) -> bool {
        let num = |s: &Sheet, r: u32, c: u32| matches!(s.eval(r, c), Value::Num(_));
        let mut s = r; while s > 0 && num(self, s - 1, c) { s -= 1; }
        if s < r { self.set(r, c, &format!("=SUM({}{}:{}{})", col_name(c), s + 1, col_name(c), r)); return true; }
        let mut k = c; while k > 0 && num(self, r, k - 1) { k -= 1; }
        if k < c { self.set(r, c, &format!("=SUM({}{}:{}{})", col_name(k), r + 1, col_name(c - 1), r + 1)); return true; }
        false
    }
    /// Labels and numeric series of a chart: a text first row is the header, a text first column holds the labels.
    pub fn chart_data(&self, ch: &Chart) -> (Vec<String>, Vec<(String, Vec<f64>)>) {
        let (r1, c1, r2, c2) = (ch.range.0.min(ch.range.2), ch.range.1.min(ch.range.3), ch.range.0.max(ch.range.2), ch.range.1.max(ch.range.3));
        let text = |r: u32, c: u32| matches!(self.eval(r, c), Value::Text(ref t) if !t.is_empty());
        let header = r2 > r1 && (c1..=c2).all(|c| text(r1, c)); let r0 = if header { r1 + 1 } else { r1 };
        let labels_col = c2 > c1 && (r0..=r2).all(|r| text(r, c1)); let c0 = if labels_col { c1 + 1 } else { c1 };
        let labels = (r0..=r2).map(|r| if labels_col { self.eval(r, c1).show() } else { (r - r0 + 1).to_string() }).collect();
        let series = (c0..=c2).map(|c| (if header { self.eval(r1, c).show() } else { col_name(c) }, (r0..=r2).map(|r| if let Value::Num(n) = self.eval(r, c) { n } else { 0.0 }).collect())).collect();
        (labels, series)
    }
    pub fn to_csv(&self) -> String {
        let (rows, cols) = self.dims(); let mut o = String::new();
        for r in 0..rows { for c in 0..cols { if c > 0 { o.push(','); } let v = self.raw(r, c); if v.contains([',', '"', '\n']) { o.push('"'); o.push_str(&v.replace('"', "\"\"")); o.push('"'); } else { o.push_str(v); } } o.push('\n'); }
        o
    }
    pub fn from_csv(s: &str) -> Sheet {
        let (mut sh, mut row, mut f, mut q, mut r, mut c) = (Sheet::default(), vec![], String::new(), false, 0u32, 0u32);
        let ch: Vec<char> = s.chars().collect(); let mut i = 0;
        while i < ch.len() {
            match (ch[i], q) {
                ('"', true) if ch.get(i + 1) == Some(&'"') => { f.push('"'); i += 1 } ('"', _) => q = !q,
                (',', false) => row.push(std::mem::take(&mut f)), ('\r', false) => {}
                ('\n', false) => { row.push(std::mem::take(&mut f)); for v in row.drain(..) { sh.set(r, c, &v); c += 1; } r += 1; c = 0; }
                (x, _) => f.push(x),
            }
            i += 1;
        }
        if !f.is_empty() || !row.is_empty() { row.push(f); for v in row { sh.set(r, c, &v); c += 1; } }
        sh
    }
    pub fn to_omc(&self) -> Vec<u8> {
        let hex = |c: Option<u32>| c.map_or("-".to_string(), |c| format!("{c:06X}"));
        let fm: String = self.fmts.iter().map(|((r, c), f)| format!("{r},{c},{},{},{},{},{}\n", f.bold as u8, f.align, f.num.code(), hex(f.fill), hex(f.color))).collect();
        let cs: String = self.charts.iter().map(|k| format!("{},{},{},{},{},{},{},{}\n", k.kind as u8, k.range.0, k.range.1, k.range.2, k.range.3, k.at.0, k.at.1, k.title.replace('\n', " "))).collect();
        let mut z = zipio::Writer::new(); z.add("mimetype", b"application/vnd.omnioffice.omc+zip");
        z.add("manifest.json", format!("{{\"omc\":2,\"generator\":\"OmniCel {}\"}}", omni_common::BUILD_ID).as_bytes());
        z.add("sheet.csv", self.to_csv().as_bytes()); z.add("formats.csv", fm.as_bytes()); z.add("charts.csv", cs.as_bytes()); z.finish()
    }
    pub fn from_omc(d: &[u8]) -> Result<Sheet, String> {
        let c = zipio::read(d, "sheet.csv")?.ok_or("OMC: sheet.csv missing")?; let mut s = Sheet::from_csv(&String::from_utf8_lossy(&c));
        let hex = |t: &str| u32::from_str_radix(t, 16).ok();
        if let Some(f) = zipio::read(d, "formats.csv").ok().flatten() { for l in String::from_utf8_lossy(&f).lines() { let t: Vec<&str> = l.split(',').collect();
            if let (Some(r), Some(c)) = (t.first().and_then(|x| x.parse().ok()), t.get(1).and_then(|x| x.parse().ok())) { if t.len() >= 7 { s.fmts.insert((r, c), CellFmt { bold: t[2] == "1", align: t[3].parse().unwrap_or(0), num: NumFmt::from_code(t[4].parse().unwrap_or(0)), fill: hex(t[5]), color: hex(t[6]) }); } } } }
        if let Some(f) = zipio::read(d, "charts.csv").ok().flatten() { for l in String::from_utf8_lossy(&f).lines() { let t: Vec<&str> = l.splitn(8, ',').collect(); let n = |i: usize| t.get(i).and_then(|x| x.parse::<u32>().ok());
            if let (Some(k), Some(a), Some(b), Some(c), Some(d), Some(e), Some(f)) = (n(0), n(1), n(2), n(3), n(4), n(5), n(6)) { s.charts.push(Chart { kind: [ChartKind::Bar, ChartKind::Line, ChartKind::Pie].get(k as usize).copied().unwrap_or_default(), range: (a, b, c, d), at: (e, f), title: t.get(7).unwrap_or(&"").to_string() }); } } }
        Ok(s)
    }
}
/// Shift the relative references of a formula by (dr, dc) rows/columns; $A$1 style references stay put.
pub fn shift_formula(f: &str, dr: i32, dc: i32) -> String {
    let ch: Vec<char> = f.chars().collect(); let (mut o, mut i) = (String::new(), 0);
    while i < ch.len() {
        if ch[i] == '"' { let st = i; i += 1; while i < ch.len() && ch[i] != '"' { i += 1; } i = (i + 1).min(ch.len()); o.extend(&ch[st..i]); continue; }
        if ch[i].is_ascii_alphabetic() || ch[i] == '$' {
            let st = i; while i < ch.len() && (ch[i].is_ascii_alphanumeric() || ch[i] == '$' || ch[i] == '_') { i += 1; }
            let tok: String = ch[st..i].iter().collect();
            let (ac, rest) = tok.strip_prefix('$').map_or((false, tok.as_str()), |r| (true, r)); let l = rest.chars().take_while(|c| c.is_ascii_alphabetic()).count();
            let (cols, rest2) = rest.split_at(l); let (ar, digits) = rest2.strip_prefix('$').map_or((false, rest2), |r| (true, r));
            if l > 0 && !digits.is_empty() && digits.chars().all(|c| c.is_ascii_digit()) && ch.get(i) != Some(&'(') {
                let col = cols.to_ascii_uppercase().bytes().fold(0i64, |a, b| a * 26 + (b - b'A') as i64 + 1) - 1 + if ac { 0 } else { dc as i64 };
                let row: i64 = digits.parse::<i64>().unwrap_or(1) + if ar { 0 } else { dr as i64 };
                if col < 0 || row < 1 { o.push_str("#REF!"); } else { o.push_str(&format!("{}{}{}{}", if ac { "$" } else { "" }, col_name(col as u32), if ar { "$" } else { "" }, row)); }
            } else { o.push_str(&tok); }
            continue;
        }
        o.push(ch[i]); i += 1;
    }
    o
}
struct P<'a> { s: Vec<char>, i: usize, sh: &'a Sheet, seen: &'a mut HashSet<(u32, u32)> }
fn num(v: &Value) -> Result<f64, Value> { match v { Value::Num(n) => Ok(*n), Value::Err(_) => Err(v.clone()), Value::Text(t) => if t.is_empty() { Ok(0.) } else { Err(Value::Err("#VALUE!")) } } }
fn crit_match(v: &Value, crit: &Value) -> bool {
    let (op, rhs) = match crit { Value::Text(t) => { let t = t.trim(); let op = ["<=", ">=", "<>", "<", ">", "="].iter().find(|o| t.starts_with(**o)).copied().unwrap_or("="); (op, t.strip_prefix(op).unwrap_or(t).trim().to_string()) } o => ("=", o.show()) };
    if let (Value::Num(a), Ok(b)) = (v, rhs.parse::<f64>()) { return match op { "<=" => *a <= b, ">=" => *a >= b, "<>" => *a != b, "<" => *a < b, ">" => *a > b, _ => *a == b }; }
    let (a, b) = (v.show().to_lowercase(), rhs.to_lowercase()); if op == "<>" { a != b } else { op == "=" && a == b }
}
impl<'a> P<'a> {
    fn ws(&mut self) { while self.s.get(self.i).map_or(false, |c| c.is_whitespace()) { self.i += 1; } }
    fn eat(&mut self, t: &str) -> bool { self.ws(); let n = t.chars().count(); if self.s[self.i..].iter().take(n).collect::<String>() == t { self.i += n; true } else { false } }
    fn cmp(&mut self) -> Value {
        let a = self.add();
        for (op, f) in [("<=", (|a: f64, b: f64| a <= b) as fn(f64, f64) -> bool), (">=", |a, b| a >= b), ("<>", |a, b| a != b), ("<", |a, b| a < b), (">", |a, b| a > b), ("=", |a, b| a == b)] {
            if self.eat(op) { let b = self.add(); return match (num(&a), num(&b)) { (Ok(x), Ok(y)) => Value::Num(f(x, y) as u8 as f64), (Err(e), _) | (_, Err(e)) => e }; }
        }
        a
    }
    fn add(&mut self) -> Value { let mut a = self.mul(); loop { let op = if self.eat("+") { 1. } else if self.eat("-") { -1. } else { return a }; let b = self.mul(); a = match (num(&a), num(&b)) { (Ok(x), Ok(y)) => Value::Num(x + op * y), (Err(e), _) | (_, Err(e)) => e }; } }
    fn mul(&mut self) -> Value {
        let mut a = self.unary();
        loop { let div = if self.eat("*") { false } else if self.eat("/") { true } else { return a }; let b = self.unary();
            a = match (num(&a), num(&b)) { (Ok(x), Ok(y)) => if div && y == 0. { Value::Err("#DIV/0!") } else { Value::Num(if div { x / y } else { x * y }) }, (Err(e), _) | (_, Err(e)) => e }; }
    }
    fn unary(&mut self) -> Value { if self.eat("-") { return match num(&self.unary()) { Ok(x) => Value::Num(-x), Err(e) => e }; } self.atom() }
    fn atom(&mut self) -> Value {
        self.ws();
        if self.eat("(") { let v = self.cmp(); if !self.eat(")") { return Value::Err("#PARSE!"); } return v; }
        if self.s.get(self.i) == Some(&'"') { self.i += 1; let st = self.i; while self.i < self.s.len() && self.s[self.i] != '"' { self.i += 1; } let t: String = self.s[st..self.i].iter().collect(); self.i += 1; return Value::Text(t); }
        let st = self.i; while self.s.get(self.i).map_or(false, |c| c.is_ascii_alphanumeric() || *c == '.' || *c == '_' || *c == '$') { self.i += 1; }
        let w: String = self.s[st..self.i].iter().collect(); if w.is_empty() { return Value::Err("#PARSE!"); }
        if let Ok(n) = w.parse::<f64>() { return Value::Num(n); }
        if self.eat("(") { return self.call(&w.to_ascii_uppercase()); }
        match parse_ref(&w) { Some((r, c)) => self.sh.eval_in(r, c, self.seen), None => Value::Err("#NAME?") }
    }
    /// Parse a call's arguments (each is a range, expanded, or an expression) and evaluate the function.
    fn call(&mut self, f: &str) -> Value {
        let mut groups: Vec<Vec<Value>> = vec![];
        if !self.eat(")") { loop {
            self.ws(); let st = self.i; let mut j = st; while self.s.get(j).map_or(false, |c| c.is_ascii_alphanumeric() || *c == '$') { j += 1; }
            let a: String = self.s[st..j].iter().collect();
            if self.s.get(j) == Some(&':') && parse_ref(&a).is_some() {
                let mut k = j + 1; while self.s.get(k).map_or(false, |c| c.is_ascii_alphanumeric() || *c == '$') { k += 1; }
                let b: String = self.s[j + 1..k].iter().collect();
                if let (Some((r1, c1)), Some((r2, c2))) = (parse_ref(&a), parse_ref(&b)) { self.i = k; let mut g = vec![]; for r in r1.min(r2)..=r1.max(r2) { for c in c1.min(c2)..=c1.max(c2) { g.push(self.sh.eval_in(r, c, self.seen)); } } groups.push(g); }
            } else { let v = self.cmp(); groups.push(vec![v]); }
            if self.eat(")") { break; } if !self.eat(",") { return Value::Err("#PARSE!"); } } }
        let first = |i: usize| groups.get(i).and_then(|g| g.first()).cloned();
        match f {
            "IF" => return match first(0).map(|c| num(&c)) { Some(Ok(c)) => if c != 0. { first(1).unwrap_or(Value::Err("#VALUE!")) } else { first(2).unwrap_or(Value::Num(0.)) }, Some(Err(e)) => e, None => Value::Err("#VALUE!") },
            "SUMIF" | "COUNTIF" => {
                let (Some(range), Some(crit)) = (groups.first(), first(1)) else { return Value::Err("#VALUE!") }; let sums = groups.get(2).unwrap_or(range);
                let mut t = 0.; let mut n = 0.; for (i, v) in range.iter().enumerate() { if crit_match(v, &crit) { n += 1.; if let Some(Value::Num(x)) = sums.get(i) { t += x; } } }
                return Value::Num(if f == "SUMIF" { t } else { n });
            }
            _ => {}
        }
        let args: Vec<Value> = groups.into_iter().flatten().collect();
        if let Some(e) = args.iter().find(|v| matches!(v, Value::Err(_))) { return e.clone(); }
        let nums: Vec<f64> = args.iter().filter_map(|v| if let Value::Num(n) = v { Some(*n) } else { None }).collect();
        match f {
            "SUM" => Value::Num(nums.iter().sum()), "COUNT" => Value::Num(nums.len() as f64),
            "COUNTA" => Value::Num(args.iter().filter(|v| !matches!(v, Value::Text(t) if t.is_empty())).count() as f64),
            "AVERAGE" => if nums.is_empty() { Value::Err("#DIV/0!") } else { Value::Num(nums.iter().sum::<f64>() / nums.len() as f64) },
            "MEDIAN" => if nums.is_empty() { Value::Err("#NUM!") } else { let mut s = nums.clone(); s.sort_by(|a, b| a.partial_cmp(b).unwrap()); let m = s.len() / 2; Value::Num(if s.len() % 2 == 1 { s[m] } else { (s[m - 1] + s[m]) / 2. }) },
            "MIN" => Value::Num(nums.iter().cloned().fold(f64::INFINITY, f64::min)), "MAX" => Value::Num(nums.iter().cloned().fold(f64::NEG_INFINITY, f64::max)),
            "ABS" => nums.first().map_or(Value::Err("#VALUE!"), |x| Value::Num(x.abs())),
            "ROUND" => match (nums.first(), nums.get(1)) { (Some(x), Some(d)) => { let m = 10f64.powi(*d as i32); Value::Num((x * m).round() / m) } (Some(x), None) => Value::Num(x.round()), _ => Value::Err("#VALUE!") },
            "AND" => Value::Num(nums.iter().all(|n| *n != 0.) as u8 as f64), "OR" => Value::Num(nums.iter().any(|n| *n != 0.) as u8 as f64), "NOT" => nums.first().map_or(Value::Err("#VALUE!"), |x| Value::Num((*x == 0.) as u8 as f64)),
            "CONCAT" => Value::Text(args.iter().map(|v| v.show()).collect()), "LEN" => args.first().map_or(Value::Err("#VALUE!"), |v| Value::Num(v.show().chars().count() as f64)),
            "UPPER" => args.first().map_or(Value::Err("#VALUE!"), |v| Value::Text(v.show().to_uppercase())), "LOWER" => args.first().map_or(Value::Err("#VALUE!"), |v| Value::Text(v.show().to_lowercase())),
            _ => Value::Err("#NAME?"),
        }
    }
}
#[cfg(test)]
mod tests { use super::*;
    fn sh() -> Sheet { let mut s = Sheet::default(); for (r, c, v) in [(0, 0, "10"), (1, 0, "20"), (2, 0, "=SUM(A1:A2)*2"), (0, 1, "=A3/(A1-10)"), (1, 1, "=B2"), (2, 1, "=IF(A1<A2,\"yes\",\"no\")"), (0, 2, "he said \"hi\", ok")] { s.set(r, c, v); } s }
    #[test] fn formulas() { let s = sh(); assert_eq!(s.eval(2, 0), Value::Num(60.)); assert_eq!(s.eval(0, 1), Value::Err("#DIV/0!")); assert_eq!(s.eval(1, 1), Value::Err("#CYCLE!")); assert_eq!(s.eval(2, 1), Value::Text("yes".into()));
        let mut t = sh(); t.set(3, 0, "=AVERAGE(A1:A3)+MAX(A1,A2)-ROUND(1.26,1)"); assert_eq!(t.eval(3, 0).show(), "48.7"); assert_eq!(col_name(27), "AB"); assert_eq!(parse_ref("$AB$3"), Some((2, 27))); }
    #[test] fn more_functions() {
        let mut s = Sheet::default(); for (i, (n, v)) in [("a", "5"), ("b", "15"), ("a", "25"), ("c", "")].iter().enumerate() { s.set(i as u32, 0, n); s.set(i as u32, 1, v); }
        for (f, want) in [("=SUMIF(A1:A4,\"a\",B1:B4)", "30"), ("=COUNTIF(B1:B4,\">10\")", "2"), ("=COUNTA(A1:B4)", "7"), ("=MEDIAN(B1:B4)", "15"), ("=CONCAT(A1,\"-\",B1)", "a-5"), ("=LEN(\"한글!\")", "3"),
            ("=UPPER(A1)&", "#PARSE!"), ("=AND(B1,B2)", "1"), ("=OR(0,0)", "0"), ("=NOT(0)", "1"), ("=SUMIF(B1:B4,\"<=15\")", "20")] { s.set(9, 9, f); assert_eq!(s.eval(9, 9).show(), want, "{f}"); }
    }
    #[test] fn fill_shift_sort_autosum() {
        assert_eq!(shift_formula("=A1+$B$2*SUM(C3:D4)+\"A1\"", 2, 1), "=B3+$B$2*SUM(D5:E6)+\"A1\""); assert_eq!(shift_formula("=A1", -1, 0), "=#REF!");
        let mut s = Sheet::default(); s.set(0, 0, "1"); s.set(0, 1, "=A1*2"); s.set(1, 0, "5"); s.set(2, 0, "3");
        s.fill_down(0, 1, 2, 1); assert_eq!((s.raw(1, 1), s.raw(2, 1)), ("=A2*2", "=A3*2")); assert_eq!(s.eval(2, 1), Value::Num(6.));
        s.fill_right(0, 1, 0, 3); assert_eq!(s.raw(0, 3), "=C1*2");
        s.set(3, 0, ""); assert!(s.autosum(3, 0)); assert_eq!(s.raw(3, 0), "=SUM(A1:A3)"); assert_eq!(s.eval(3, 0), Value::Num(9.));
        let mut t = Sheet::default(); for (i, (a, b)) in [("pear", "3"), ("apple", "10"), ("fig", "2")].iter().enumerate() { t.set(i as u32, 0, a); t.set(i as u32, 1, b); }
        t.sort_rows(0, 0, 2, 1, 0, true); assert_eq!((t.raw(0, 0), t.raw(2, 1)), ("apple", "3")); t.sort_rows(0, 0, 2, 1, 1, false); assert_eq!(t.raw(0, 0), "apple");
    }
    #[test] fn formats_and_charts() {
        assert_eq!((format_num(1234.5, NumFmt::Currency), format_num(0.256, NumFmt::Percent), format_num(1234567., NumFmt::Thousands), format_num(2., NumFmt::Fixed2)), ("$1,234.50".to_string(), "25.60%".to_string(), "1,234,567".to_string(), "2.00".to_string()));
        let mut s = Sheet::default(); for (r, row) in [["Month", "Sales", "Cost"], ["Jan", "10", "4"], ["Feb", "20", "6"]].iter().enumerate() { for (c, v) in row.iter().enumerate() { s.set(r as u32, c as u32, v); } }
        s.update_fmt(1, 1, 2, 2, |f| { f.num = NumFmt::Currency; f.bold = true; f.fill = Some(0xFFF2CC); }); assert_eq!(s.display(1, 1), "$10.00");
        let ch = Chart { kind: ChartKind::Line, range: (0, 0, 2, 2), at: (4, 0), title: "Q1, results".into() }; s.charts.push(ch.clone());
        let (labels, series) = s.chart_data(&ch); assert_eq!(labels, ["Jan", "Feb"]); assert_eq!(series, [("Sales".to_string(), vec![10., 20.]), ("Cost".to_string(), vec![4., 6.])]);
        let back = Sheet::from_omc(&s.to_omc()).unwrap(); assert_eq!((back.fmt(2, 2), back.charts.clone(), back.display(2, 1)), (s.fmt(2, 2), s.charts.clone(), "$20.00".to_string()));
    }
    #[test] fn tsv_copy_paste() { let mut s = sh(); let t = s.range_tsv(0, 0, 2, 0); assert_eq!(t, "10\n20\n=SUM(A1:A2)*2\n"); let n = s.paste_tsv(5, 3, "x\ty\n1\t2\n"); assert_eq!(n, (2, 2)); assert_eq!((s.raw(5, 3), s.raw(6, 4)), ("x", "2")); s.clear_range(5, 3, 6, 4); assert_eq!(s.raw(6, 4), ""); }
    #[test] fn files() { let s = sh(); assert_eq!(Sheet::from_csv(&s.to_csv()).to_csv(), s.to_csv()); let back = Sheet::from_omc(&s.to_omc()).unwrap(); assert_eq!(back.raw(0, 2), "he said \"hi\", ok"); assert_eq!(back.eval(2, 0), Value::Num(60.)); }
}
