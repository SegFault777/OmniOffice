// SPDX-License-Identifier: GPL-3.0-only
//! OmniCel core: sparse sheet, formula evaluator (+ - * / comparisons, SUM AVERAGE MIN MAX COUNT IF ABS ROUND),
//! CSV import/export and the OMC container (stored ZIP: mimetype, manifest.json, sheet.csv).
use omni_common::zipio;
use std::collections::{BTreeMap, HashSet};

#[derive(Clone, Debug, PartialEq)]
pub enum Value { Num(f64), Text(String), Err(&'static str) }
impl Value {
    pub fn show(&self) -> String { match self { Value::Num(n) if n.fract() == 0.0 && n.abs() < 1e15 => format!("{}", *n as i64), Value::Num(n) => format!("{n}"), Value::Text(t) => t.clone(), Value::Err(e) => e.to_string() } }
}
pub fn col_name(mut c: u32) -> String { let mut s = String::new(); loop { s.insert(0, (b'A' + (c % 26) as u8) as char); if c < 26 { break; } c = c / 26 - 1; } s }
/// "B12" -> (row 11, col 1)
pub fn parse_ref(s: &str) -> Option<(u32, u32)> {
    let l = s.chars().take_while(|c| c.is_ascii_alphabetic()).count(); if l == 0 || l == s.len() { return None; }
    let col = s[..l].to_ascii_uppercase().bytes().fold(0u32, |a, b| a * 26 + (b - b'A') as u32 + 1) - 1;
    let row: u32 = s[l..].parse().ok()?; if row == 0 { None } else { Some((row - 1, col)) }
}
#[derive(Default, Clone, Debug)]
pub struct Sheet { cells: BTreeMap<(u32, u32), String> }
impl Sheet {
    pub fn set(&mut self, r: u32, c: u32, raw: &str) { if raw.is_empty() { self.cells.remove(&(r, c)); } else { self.cells.insert((r, c), raw.to_string()); } }
    pub fn raw(&self, r: u32, c: u32) -> &str { self.cells.get(&(r, c)).map_or("", |s| s.as_str()) }
    pub fn dims(&self) -> (u32, u32) { self.cells.keys().fold((0, 0), |a, k| (a.0.max(k.0 + 1), a.1.max(k.1 + 1))) }
    pub fn eval(&self, r: u32, c: u32) -> Value { self.eval_in(r, c, &mut HashSet::new()) }
    fn eval_in(&self, r: u32, c: u32, seen: &mut HashSet<(u32, u32)>) -> Value {
        let raw = self.raw(r, c);
        let Some(f) = raw.strip_prefix('=') else { return raw.trim().parse::<f64>().map(Value::Num).unwrap_or_else(|_| Value::Text(raw.to_string())) };
        if !seen.insert((r, c)) { return Value::Err("#CYCLE!"); }
        let mut p = P { s: f.chars().collect(), i: 0, sh: self, seen };
        let v = p.cmp(); p.ws(); let v = if p.i < p.s.len() { Value::Err("#PARSE!") } else { v };
        p.seen.remove(&(r, c)); v
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
                (',', false) => row.push(std::mem::take(&mut f)),
                ('\r', false) => {}
                ('\n', false) => { row.push(std::mem::take(&mut f)); for v in row.drain(..) { sh.set(r, c, &v); c += 1; } r += 1; c = 0; }
                (x, _) => f.push(x),
            }
            i += 1;
        }
        if !f.is_empty() || !row.is_empty() { row.push(f); for v in row { sh.set(r, c, &v); c += 1; } }
        sh
    }
    pub fn to_omc(&self) -> Vec<u8> {
        let mut z = zipio::Writer::new(); z.add("mimetype", b"application/vnd.omnioffice.omc+zip");
        z.add("manifest.json", format!("{{\"omc\":1,\"generator\":\"OmniCel {}\"}}", omni_common::BUILD_ID).as_bytes()); z.add("sheet.csv", self.to_csv().as_bytes()); z.finish()
    }
    pub fn from_omc(d: &[u8]) -> Result<Sheet, String> { let c = zipio::read(d, "sheet.csv")?.ok_or("OMC: sheet.csv missing")?; Ok(Sheet::from_csv(&String::from_utf8_lossy(&c))) }
}
struct P<'a> { s: Vec<char>, i: usize, sh: &'a Sheet, seen: &'a mut HashSet<(u32, u32)> }
fn num(v: &Value) -> Result<f64, Value> { match v { Value::Num(n) => Ok(*n), Value::Err(_) => Err(v.clone()), Value::Text(t) => if t.is_empty() { Ok(0.) } else { Err(Value::Err("#VALUE!")) } } }
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
        let st = self.i; while self.s.get(self.i).map_or(false, |c| c.is_ascii_alphanumeric() || *c == '.' || *c == '_') { self.i += 1; }
        let w: String = self.s[st..self.i].iter().collect(); if w.is_empty() { return Value::Err("#PARSE!"); }
        if let Ok(n) = w.parse::<f64>() { return Value::Num(n); }
        if self.eat("(") { return self.call(&w.to_ascii_uppercase()); }
        match parse_ref(&w) { Some((r, c)) => self.sh.eval_in(r, c, self.seen), None => Value::Err("#NAME?") }
    }
    /// Parse a call's arguments: each argument is a range (expanded) or an expression.
    fn call(&mut self, f: &str) -> Value {
        let mut args: Vec<Value> = vec![]; let mut single: Vec<Value> = vec![];
        if !self.eat(")") { loop {
            self.ws(); let st = self.i;
            let mut j = st; while self.s.get(j).map_or(false, |c| c.is_ascii_alphanumeric()) { j += 1; }
            let a: String = self.s[st..j].iter().collect();
            if self.s.get(j) == Some(&':') && parse_ref(&a).is_some() {
                let mut k = j + 1; while self.s.get(k).map_or(false, |c| c.is_ascii_alphanumeric()) { k += 1; }
                let b: String = self.s[j + 1..k].iter().collect();
                if let (Some((r1, c1)), Some((r2, c2))) = (parse_ref(&a), parse_ref(&b)) { self.i = k; for r in r1.min(r2)..=r1.max(r2) { for c in c1.min(c2)..=c1.max(c2) { args.push(self.sh.eval_in(r, c, self.seen)); } } }
            } else { let v = self.cmp(); single.push(v.clone()); args.push(v); }
            if self.eat(")") { break; } if !self.eat(",") { return Value::Err("#PARSE!"); } } }
        if f == "IF" { let c = single.first().map(num); return match (c, single.get(1), single.get(2)) { (Some(Ok(c)), Some(a), b) => if c != 0. { a.clone() } else { b.cloned().unwrap_or(Value::Num(0.)) }, (Some(Err(e)), ..) => e, _ => Value::Err("#VALUE!") }; }
        if let Some(e) = args.iter().find(|v| matches!(v, Value::Err(_))) { return e.clone(); }
        let nums: Vec<f64> = args.iter().filter_map(|v| if let Value::Num(n) = v { Some(*n) } else { None }).collect();
        match f {
            "SUM" => Value::Num(nums.iter().sum()), "COUNT" => Value::Num(nums.len() as f64),
            "AVERAGE" => if nums.is_empty() { Value::Err("#DIV/0!") } else { Value::Num(nums.iter().sum::<f64>() / nums.len() as f64) },
            "MIN" => Value::Num(nums.iter().cloned().fold(f64::INFINITY, f64::min)), "MAX" => Value::Num(nums.iter().cloned().fold(f64::NEG_INFINITY, f64::max)),
            "ABS" => nums.first().map_or(Value::Err("#VALUE!"), |x| Value::Num(x.abs())),
            "ROUND" => match (nums.first(), nums.get(1)) { (Some(x), Some(d)) => { let m = 10f64.powi(*d as i32); Value::Num((x * m).round() / m) } (Some(x), None) => Value::Num(x.round()), _ => Value::Err("#VALUE!") },
            _ => Value::Err("#NAME?"),
        }
    }
}
#[cfg(test)]
mod tests { use super::*;
    fn sh() -> Sheet { let mut s = Sheet::default(); for (r, c, v) in [(0, 0, "10"), (1, 0, "20"), (2, 0, "=SUM(A1:A2)*2"), (0, 1, "=A3/(A1-10)"), (1, 1, "=B2"), (2, 1, "=IF(A1<A2,\"yes\",\"no\")"), (0, 2, "he said \"hi\", ok")] { s.set(r, c, v); } s }
    #[test] fn formulas() { let s = sh(); assert_eq!(s.eval(2, 0), Value::Num(60.)); assert_eq!(s.eval(0, 1), Value::Err("#DIV/0!")); assert_eq!(s.eval(1, 1), Value::Err("#CYCLE!")); assert_eq!(s.eval(2, 1), Value::Text("yes".into()));
        let mut t = sh(); t.set(3, 0, "=AVERAGE(A1:A3)+MAX(A1,A2)-ROUND(1.26,1)"); assert_eq!(t.eval(3, 0).show(), "48.7"); assert_eq!(col_name(27), "AB"); assert_eq!(parse_ref("AB3"), Some((2, 27))); }
    #[test] fn files() { let s = sh(); assert_eq!(Sheet::from_csv(&s.to_csv()).to_csv(), s.to_csv()); let back = Sheet::from_omc(&s.to_omc()).unwrap(); assert_eq!(back.raw(0, 2), "he said \"hi\", ok"); assert_eq!(back.eval(2, 0), Value::Num(60.)); }
}
