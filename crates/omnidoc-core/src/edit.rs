// SPDX-License-Identifier: GPL-3.0-only
//! Editing engine: cursor, selection, formatting, undo/redo, find/replace, outline.
use crate::model::*;
use std::collections::HashMap;
pub type Pos = (usize, usize); // (paragraph, char offset)
#[derive(Clone, Copy)]
pub enum Fmt { Bold, Italic, Underline, Strike }
fn get(r: &Run, f: Fmt) -> bool { match f { Fmt::Bold => r.bold, Fmt::Italic => r.italic, Fmt::Underline => r.underline, Fmt::Strike => r.strike } }
fn set(r: &mut Run, f: Fmt, v: bool) { match f { Fmt::Bold => r.bold = v, Fmt::Italic => r.italic = v, Fmt::Underline => r.underline = v, Fmt::Strike => r.strike = v } }
fn fold(s: &str, cs: bool) -> Vec<char> { s.chars().map(|c| if cs { c } else { c.to_lowercase().next().unwrap_or(c) }).collect() }

/// Captured formatting for the Format Painter.
#[derive(Clone, Debug)]
pub struct Painter { pub run: Run, pub style: Style, pub align: Align }
#[derive(Clone, Copy, Debug)]
pub enum Case { Upper, Lower, Title, Sentence }
#[derive(Debug, PartialEq, Eq)]
pub struct Stats { pub words: usize, pub chars: usize, pub chars_no_spaces: usize, pub sentences: usize, pub paras: usize, pub pages: usize }
const AUTOCORRECT: &[(&str, &str)] = &[("(c)", "©"), ("(r)", "®"), ("(tm)", "™"), ("--", "–"), ("...", "…"), ("->", "→"), ("<-", "←"), ("1/2", "½"), ("1/4", "¼"), ("3/4", "¾"),
    ("teh", "the"), ("adn", "and"), ("taht", "that"), ("recieve", "receive"), ("dont", "don't"), ("cant", "can't"), ("i", "I")];

pub struct Editor {
    pub painter: Option<Painter>, pub autocorrect_on: bool, pub completion_on: bool,
    pub doc: Doc, pub cur: Pos, pub anchor: Option<Pos>, pub pend: Option<Run>,
    undo: Vec<(Doc, Pos)>, redo: Vec<(Doc, Pos)>, typing: bool, pub dirty: bool,
}
impl Editor {
    pub fn new(doc: Doc) -> Self { Editor { painter: None, autocorrect_on: true, completion_on: true, doc, cur: (0, 0), anchor: None, pend: None, undo: vec![], redo: vec![], typing: false, dirty: false } }
    fn snap(&mut self, coalesce: bool) {
        if !(coalesce && self.typing) {
            self.undo.push((self.doc.clone(), self.cur));
            if self.undo.len() > 500 { self.undo.remove(0); }
        }
        self.typing = coalesce; self.redo.clear(); self.dirty = true;
    }
    pub fn undo(&mut self) { if let Some((d, c)) = self.undo.pop() { self.redo.push((std::mem::replace(&mut self.doc, d), self.cur)); self.cur = c; self.anchor = None; self.typing = false; self.dirty = true; } }
    pub fn redo(&mut self) { if let Some((d, c)) = self.redo.pop() { self.undo.push((std::mem::replace(&mut self.doc, d), self.cur)); self.cur = c; self.anchor = None; self.typing = false; self.dirty = true; } }
    pub fn range(&self) -> Option<(Pos, Pos)> {
        let a = self.anchor?; if a == self.cur { return None; }
        Some(if a < self.cur { (a, self.cur) } else { (self.cur, a) })
    }
    pub fn fmt_at(&self, p: Pos) -> Run {
        let para = &self.doc.paras[p.0]; let (mut n, mut last) = (0, None);
        for r in &para.runs { let l = r.text.chars().count(); if last.is_none() { last = Some(r); } if n + l >= p.1 && p.1 > 0 { return Run { text: String::new(), ..r.clone() }; } n += l; }
        last.map(|r| Run { text: String::new(), ..r.clone() }).unwrap_or_default()
    }
    fn cut(&mut self, a: Pos, b: Pos) {
        let (h, _) = self.doc.paras[a.0].split(a.1); let (_, t) = self.doc.paras[b.0].split(b.1);
        let mut p = self.doc.paras[a.0].clone(); p.runs = h; p.runs.extend(t); p.normalize();
        self.doc.paras.splice(a.0..=b.0, [p]); self.cur = a; self.anchor = None;
    }
    fn ins_text(&mut self, s: &str) {
        if s.is_empty() { return; }
        let f = self.pend.clone().unwrap_or_else(|| self.fmt_at(self.cur));
        let (r, o) = self.cur; let (mut a, b) = self.doc.paras[r].split(o);
        a.push(Run { text: s.into(), ..f }); a.extend(b);
        self.doc.paras[r].runs = a; self.doc.paras[r].normalize(); self.cur.1 += s.chars().count();
    }
    fn split_para(&mut self) {
        let (r, o) = self.cur; let (a, b) = self.doc.paras[r].split(o);
        let old = &self.doc.paras[r];
        let st = if old.style.level() > 0 || old.style == Style::Title { Style::Normal } else { old.style };
        let mut np = Para { style: st, align: old.align, runs: b, ..Default::default() }; np.normalize();
        self.doc.paras[r].runs = a; self.doc.paras[r].normalize();
        self.doc.paras.insert(r + 1, np); self.cur = (r + 1, 0);
    }
    pub fn insert(&mut self, s: &str) {
        match self.range() { Some((a, b)) => { self.snap(false); self.cut(a, b); } None => self.snap(!s.contains('\n')) }
        for (i, line) in s.replace("\r\n", "\n").split('\n').enumerate() { if i > 0 { if !self.table_tab(false) { self.split_para(); } } self.ins_text(line); }
    }
    pub fn backspace(&mut self) {
        self.snap(false);
        if let Some((a, b)) = self.range() { return self.cut(a, b); }
        let (r, o) = self.cur;
        if o > 0 { self.cut((r, o - 1), (r, o)) } else if r > 0 { let l = self.doc.paras[r - 1].len(); self.cut((r - 1, l), (r, 0)) }
    }
    pub fn delete(&mut self) {
        self.snap(false);
        if let Some((a, b)) = self.range() { return self.cut(a, b); }
        let (r, o) = self.cur;
        if o < self.doc.paras[r].len() { self.cut((r, o), (r, o + 1)) } else if r + 1 < self.doc.paras.len() { self.cut((r, o), (r + 1, 0)) }
    }
    pub fn mv(&mut self, p: Pos, extend: bool) {
        if extend { if self.anchor.is_none() { self.anchor = Some(self.cur); } } else { self.anchor = None; }
        self.cur = p; self.pend = None; self.typing = false;
    }
    pub fn left(&self) -> Pos { let (r, o) = self.cur; if o > 0 { (r, o - 1) } else if r > 0 { (r - 1, self.doc.paras[r - 1].len()) } else { (0, 0) } }
    pub fn right(&self) -> Pos { let (r, o) = self.cur; if o < self.doc.paras[r].len() { (r, o + 1) } else if r + 1 < self.doc.paras.len() { (r + 1, 0) } else { (r, o) } }
    /// Plain text of the selection ("" when nothing is selected).
    pub fn selected_text(&self) -> String {
        let Some((a, b)) = self.range() else { return String::new() };
        let mut out = String::new();
        for i in a.0..=b.0 {
            let t: Vec<char> = self.doc.paras[i].text().chars().collect();
            let (s, e) = (if i == a.0 { a.1 } else { 0 }, if i == b.0 { b.1 } else { t.len() });
            out.extend(&t[s..e]); if i < b.0 { out.push('\n'); }
        }
        out
    }
    pub fn delete_selection(&mut self) { if let Some((a, b)) = self.range() { self.snap(false); self.cut(a, b); } }
    pub fn select_all(&mut self) { let l = self.doc.paras.len() - 1; self.anchor = Some((0, 0)); self.cur = (l, self.doc.paras[l].len()); }
    fn map_range<F: FnMut(&mut Run)>(&mut self, a: Pos, b: Pos, mut f: F) {
        for i in a.0..=b.0 {
            let p = &mut self.doc.paras[i];
            let (s, e) = (if i == a.0 { a.1 } else { 0 }, if i == b.0 { b.1 } else { p.len() });
            if s >= e { continue; }
            let (pre, rest) = p.split(s);
            let (mut mid, post) = Para { runs: rest, ..Default::default() }.split(e - s);
            mid.iter_mut().for_each(&mut f);
            p.runs = pre; p.runs.extend(mid); p.runs.extend(post); p.normalize();
        }
    }
    pub fn toggle(&mut self, f: Fmt) {
        if let Some((a, b)) = self.range() {
            let mut all = true; let mut probe = Editor::new(self.doc.clone());
            probe.map_range(a, b, |r| all &= get(r, f));
            self.snap(false); self.map_range(a, b, |r| set(r, f, !all));
        } else {
            let mut c = self.pend.clone().unwrap_or_else(|| self.fmt_at(self.cur)); let v = !get(&c, f); set(&mut c, f, v); self.pend = Some(c);
        }
    }
    fn para_span(&self) -> (usize, usize) { self.range().map_or((self.cur.0, self.cur.0), |(a, b)| (a.0, b.0)) }
    pub fn set_style(&mut self, s: Style) { self.snap(false); let (a, b) = self.para_span(); for p in &mut self.doc.paras[a..=b] { p.style = s; } }
    pub fn set_align(&mut self, al: Align) { self.snap(false); let (a, b) = self.para_span(); for p in &mut self.doc.paras[a..=b] { p.align = al; } }
    fn matches(p: &Para, q: &[char], cs: bool) -> Vec<usize> {
        let t = fold(&p.text(), cs); let mut v = vec![]; let mut i = 0;
        while !q.is_empty() && i + q.len() <= t.len() { if t[i..i + q.len()] == *q { v.push(i); i += q.len(); } else { i += 1; } }
        v
    }
    /// Select the next match after the cursor (wrapping). Returns false if none.
    pub fn find_next(&mut self, q: &str, cs: bool) -> bool {
        let qc = fold(q, cs); let n = self.doc.paras.len();
        let start = self.range().map_or(self.cur, |r| r.1);
        for k in 0..=n {
            let i = (start.0 + k) % n;
            for m in Self::matches(&self.doc.paras[i], &qc, cs) {
                if (k == 0 && m < start.1) || (k == n && m >= start.1) { continue; }
                self.anchor = Some((i, m)); self.cur = (i, m + qc.len()); return true;
            }
        }
        false
    }
    pub fn replace_all(&mut self, q: &str, with: &str, cs: bool) -> usize {
        let qc = fold(q, cs); let mut count = 0; self.snap(false);
        for i in 0..self.doc.paras.len() {
            for m in Self::matches(&self.doc.paras[i], &qc, cs).into_iter().rev() {
                self.cut((i, m), (i, m + qc.len())); self.ins_text(with); count += 1;
            }
        }
        self.cur = (0, 0); self.anchor = None; count
    }

    // ---------- Word-style features ----------
    pub fn copy_format(&self) -> Painter { let p = &self.doc.paras[self.cur.0]; Painter { run: self.fmt_at(self.cur), style: p.style, align: p.align } }
    /// Apply captured formatting to the selection (or just the paragraph when nothing is selected).
    pub fn paint_format(&mut self, pt: &Painter) {
        self.snap(false);
        if let Some((a, b)) = self.range() { self.map_range(a, b, |r| { r.bold = pt.run.bold; r.italic = pt.run.italic; r.underline = pt.run.underline; r.strike = pt.run.strike; r.size = pt.run.size; r.color = pt.run.color; }); } else { self.pend = Some(pt.run.clone()); }
        let (a, b) = self.para_span(); for p in &mut self.doc.paras[a..=b] { p.style = pt.style; p.align = pt.align; }
    }
    pub fn word_bounds(&self, p: Pos) -> (Pos, Pos) {
        let t: Vec<char> = self.doc.paras[p.0].text().chars().collect(); let w = |c: &char| c.is_alphanumeric() || *c == '\'';
        let (mut s, mut e) = (p.1.min(t.len()), p.1.min(t.len()));
        while s > 0 && w(&t[s - 1]) { s -= 1 } while e < t.len() && w(&t[e]) { e += 1 }
        ((p.0, s), (p.0, e))
    }
    pub fn select_word(&mut self) { let (a, b) = self.word_bounds(self.cur); self.anchor = Some(a); self.cur = b; }
    pub fn select_para(&mut self) { let r = self.cur.0; self.anchor = Some((r, 0)); self.cur = (r, self.doc.paras[r].len()); }
    pub fn change_case(&mut self, c: Case) {
        let (a, b) = self.range().unwrap_or_else(|| self.word_bounds(self.cur)); if a == b { return; }
        self.snap(false); let (mut in_word, mut first) = (false, true);
        self.map_range(a, b, |r| r.text = r.text.chars().map(|ch| {
            let up = ch.to_uppercase().next().unwrap_or(ch); let lo = ch.to_lowercase().next().unwrap_or(ch);
            let o = match c { Case::Upper => up, Case::Lower => lo, Case::Title => if in_word { lo } else { up },
                Case::Sentence => if first && ch.is_alphabetic() { first = false; up } else { lo } };
            in_word = ch.is_alphanumeric() || ch == '\''; if matches!(ch, '.' | '!' | '?') { first = true; } o }).collect());
    }
    /// Sort the selected paragraphs (or the whole document) alphabetically.
    pub fn sort_paras(&mut self, desc: bool) {
        self.snap(false);
        let (a, b) = if self.range().is_some() { self.para_span() } else { (0, self.doc.paras.len() - 1) };
        let s = &mut self.doc.paras[a..=b]; s.sort_by_key(|p| p.text().to_lowercase()); if desc { s.reverse(); }
        self.cur = (a, 0); self.anchor = None;
    }
    // ---------- LibreOffice Writer-style features ----------
    /// AutoCorrect the word before the cursor. Call just before inserting a space or newline.
    pub fn autocorrect(&mut self) -> bool {
        let (r, o) = self.cur; if !self.autocorrect_on || self.range().is_some() || o == 0 { return false; }
        let t: Vec<char> = self.doc.paras[r].text().chars().collect(); let mut s = o; while s > 0 && !t[s - 1].is_whitespace() { s -= 1; }
        let tok: String = t[s..o].iter().collect(); if tok.is_empty() { return false; }
        let mut new = AUTOCORRECT.iter().find(|x| x.0 == tok).map(|x| x.1.to_string()).unwrap_or_else(|| tok.clone());
        let starts = s == 0 || (s >= 2 && matches!(t[s - 2], '.' | '!' | '?') && t[s - 1] == ' ');
        if starts && new.chars().all(|c| c.is_alphabetic() || c == '\'') { let mut c = new.chars(); if let Some(f) = c.next() { new = f.to_uppercase().chain(c).collect(); } }
        if new == tok { return false; }
        self.snap(false); self.cut((r, s), (r, o)); self.ins_text(&new); true
    }
    /// Word completion: the most frequent longer word in the document that starts with the typed prefix.
    pub fn completion(&self) -> Option<String> {
        let (r, o) = self.cur; if !self.completion_on || self.range().is_some() { return None; }
        let t: Vec<char> = self.doc.paras[r].text().chars().collect(); let mut s = o; while s > 0 && t[s - 1].is_alphabetic() { s -= 1; }
        if o - s < 3 || t.get(o).map_or(false, |c| c.is_alphabetic()) { return None; }
        let pre: String = t[s..o].iter().collect::<String>().to_lowercase(); let n = o - s; let mut freq: HashMap<String, usize> = HashMap::new();
        for p in &self.doc.paras { for w in p.text().split(|c: char| !c.is_alphabetic()) { let l = w.to_lowercase(); if l.chars().count() > n && l.starts_with(&pre) { *freq.entry(l).or_default() += 1; } } }
        freq.into_iter().min_by_key(|(w, f)| (std::cmp::Reverse(*f), w.chars().count(), w.clone())).map(|(w, _)| w.chars().skip(n).collect())
    }
    pub fn accept_completion(&mut self) -> bool { match self.completion() { Some(s) => { self.insert(&s); true } None => false } }
    fn re_matches(p: &Para, re: &regex::Regex, with: Option<&str>) -> Vec<(usize, usize, String)> {
        let t = p.text(); let idx: Vec<usize> = t.char_indices().map(|x| x.0).chain([t.len()]).collect();
        re.captures_iter(&t).filter_map(|c| {
            let m = c.get(0)?; if m.start() == m.end() { return None; }
            let mut rep = String::new(); if let Some(w) = with { c.expand(w, &mut rep); }
            Some((idx.binary_search(&m.start()).ok()?, idx.binary_search(&m.end()).ok()?, rep))
        }).collect()
    }
    fn regex(q: &str, cs: bool) -> Result<regex::Regex, String> { regex::RegexBuilder::new(q).case_insensitive(!cs).build().map_err(|e| e.to_string()) }
    pub fn find_next_regex(&mut self, q: &str, cs: bool) -> Result<bool, String> {
        let re = Self::regex(q, cs)?; let n = self.doc.paras.len(); let start = self.range().map_or(self.cur, |r| r.1);
        for k in 0..=n {
            let i = (start.0 + k) % n;
            for (s, e, _) in Self::re_matches(&self.doc.paras[i], &re, None) { if (k == 0 && s < start.1) || (k == n && s >= start.1) { continue; } self.anchor = Some((i, s)); self.cur = (i, e); return Ok(true); }
        }
        Ok(false)
    }
    pub fn replace_all_regex(&mut self, q: &str, with: &str, cs: bool) -> Result<usize, String> {
        let re = Self::regex(q, cs)?; self.snap(false); let mut count = 0;
        for i in 0..self.doc.paras.len() { for (s, e, rep) in Self::re_matches(&self.doc.paras[i], &re, Some(with)).into_iter().rev() { self.cut((i, s), (i, e)); self.ins_text(&rep); count += 1; } }
        self.cur = (0, 0); self.anchor = None; Ok(count)
    }
    pub fn stats(&self) -> Stats {
        let t = self.doc.plain(); let chars = t.chars().filter(|c| *c != '\n').count();
        Stats { words: self.doc.words(), chars, chars_no_spaces: t.chars().filter(|c| !c.is_whitespace()).count(),
            sentences: t.split(|c| matches!(c, '.' | '!' | '?')).filter(|x| !x.trim().is_empty()).count(), paras: self.doc.paras.len(), pages: chars / 2800 + 1 }
    }

    // ---------- run formatting: size, colour ----------
    fn with_run(&mut self, f: impl Fn(&mut Run)) {
        if let Some((a, b)) = self.range() { self.snap(false); self.map_range(a, b, |r| f(r)); }
        else { let mut c = self.pend.clone().unwrap_or_else(|| self.fmt_at(self.cur)); f(&mut c); self.pend = Some(c); }
    }
    /// Change the font size of the selection (or of the next typed text) by `delta` points (6..=96, default 12).
    pub fn change_size(&mut self, delta: i16) { self.with_run(|r| r.size = Some((r.size.unwrap_or(12) as i16 + delta).clamp(6, 96) as u8)); }
    pub fn set_color(&mut self, color: Option<u32>) { self.with_run(|r| r.color = color); }
    // ---------- lists and indentation (markers are plain text, like imported documents) ----------
    fn replace_head(p: &mut Para, at: usize, remove: usize, insert: &str) {
        let (a, b) = p.split(at); let rest = Para { runs: b, ..Default::default() }.split(remove).1;
        let mut runs = a; if !insert.is_empty() { runs.push(Run::plain(insert)); } runs.extend(rest); p.runs = runs; p.normalize();
    }
    /// (leading spaces, marker length in chars, is_bullet) of a paragraph text.
    fn marker(t: &str) -> (usize, usize, Option<bool>) {
        let lead = t.chars().take_while(|c| *c == ' ').count(); let rest: Vec<char> = t.chars().skip(lead).collect();
        if rest.starts_with(&['\u{2022}', ' ']) { return (lead, 2, Some(true)); }
        let d = rest.iter().take_while(|c| c.is_ascii_digit()).count();
        if d > 0 && rest.get(d) == Some(&'.') && rest.get(d + 1) == Some(&' ') { return (lead, d + 2, Some(false)); }
        (lead, 0, None)
    }
    /// Toggle bullets (`bullet = true`) or numbering on the selected paragraphs.
    pub fn toggle_list(&mut self, bullet: bool) {
        let sel = self.range().is_some(); let (a, b) = self.para_span(); let ps: Vec<usize> = (a..=b).filter(|i| !self.doc.paras[*i].table && self.doc.paras[*i].image.is_none()).collect();
        if ps.is_empty() { return; }
        self.snap(false); let all = ps.iter().all(|i| Self::marker(&self.doc.paras[*i].text()).2 == Some(bullet)); let mut n = 0;
        for i in ps { let (lead, len, _) = Self::marker(&self.doc.paras[i].text()); n += 1;
            let ins = if all { String::new() } else if bullet { "\u{2022} ".to_string() } else { format!("{n}. ") };
            Self::replace_head(&mut self.doc.paras[i], lead, len, &ins); }
        self.keep_span(sel, (a, b));
    }
    /// Keep a multi-paragraph selection after a paragraph-level edit (or put the caret at the end of the paragraph).
    fn keep_span(&mut self, had_selection: bool, span: (usize, usize)) {
        let last = self.doc.paras[span.1].len();
        if had_selection { self.anchor = Some((span.0, 0)); self.cur = (span.1, last); } else { self.anchor = None; self.cur = (span.1, last); }
    }
    /// Indent (delta > 0) or outdent (delta < 0) the selected paragraphs by 4 spaces per step.
    pub fn indent(&mut self, delta: i32) {
        let sel = self.range().is_some(); let (a, b) = self.para_span(); self.snap(false);
        for i in a..=b { if self.doc.paras[i].table || self.doc.paras[i].image.is_some() { continue; }
            let lead = self.doc.paras[i].text().chars().take_while(|c| *c == ' ').count();
            if delta > 0 { Self::replace_head(&mut self.doc.paras[i], 0, 0, "    "); } else { Self::replace_head(&mut self.doc.paras[i], 0, lead.min(4), ""); } }
        self.keep_span(sel, (a, b));
    }
    // ---------- tables: rows are paragraphs with `table = true`, cells separated by '\t' ----------
    pub fn table_block(&self, r: usize) -> Option<(usize, usize)> {
        if !self.doc.paras.get(r)?.table { return None; }
        let (mut s, mut e) = (r, r); while s > 0 && self.doc.paras[s - 1].table { s -= 1; } while e + 1 < self.doc.paras.len() && self.doc.paras[e + 1].table { e += 1; } Some((s, e))
    }
    pub fn insert_table(&mut self, rows: usize, cols: usize) {
        let (rows, cols) = (rows.clamp(1, 50), cols.clamp(1, 12)); self.snap(false);
        let row = || Para { table: true, runs: vec![Run::plain(&"\t".repeat(cols - 1))], ..Default::default() };
        let at = self.cur.0; let replace = self.doc.paras[at].is_empty() && !self.doc.paras[at].table && self.doc.paras[at].image.is_none();
        let new: Vec<Para> = (0..rows).map(|_| row()).collect(); let first = if replace { at } else { at + 1 };
        self.doc.paras.splice(first..first + replace as usize, new);
        let last = first + rows; if last >= self.doc.paras.len() || self.doc.paras[last].table { self.doc.paras.insert(last, Para::default()); }
        self.cur = (first, 0); self.anchor = None;
    }
    /// Move to the next (or previous) cell; the last cell appends a new row. False when not in a table.
    pub fn table_tab(&mut self, back: bool) -> bool {
        let (r, o) = self.cur; let Some((_, e)) = self.table_block(r) else { return false };
        let t: Vec<char> = self.doc.paras[r].text().chars().collect(); self.anchor = None; self.pend = None;
        if back {
            let starts: Vec<usize> = std::iter::once(0).chain(t.iter().enumerate().filter(|x| *x.1 == '\t').map(|x| x.0 + 1)).collect();
            if let Some(&p) = starts.iter().rev().find(|&&p| p < o) { self.cur = (r, p); } else if let Some((s, _)) = self.table_block(r).filter(|b| b.0 < r) { let pt: Vec<char> = self.doc.paras[r - 1].text().chars().collect(); let last = pt.iter().rposition(|c| *c == '\t').map_or(0, |p| p + 1); let _ = s; self.cur = (r - 1, last); }
        } else if let Some(p) = t.iter().enumerate().skip(o).find(|x| *x.1 == '\t').map(|x| x.0) { self.cur = (r, p + 1); }
        else if r < e { self.cur = (r + 1, 0); } else { self.table_add_row(); }
        true
    }
    pub fn table_add_row(&mut self) {
        let r = self.cur.0; let Some(_) = self.table_block(r) else { return }; self.snap(false);
        let tabs = self.doc.paras[r].text().chars().filter(|c| *c == '\t').count();
        self.doc.paras.insert(r + 1, Para { table: true, runs: vec![Run::plain(&"\t".repeat(tabs))], ..Default::default() }); self.cur = (r + 1, 0);
    }
    pub fn table_del_row(&mut self) {
        let r = self.cur.0; let Some((s, e)) = self.table_block(r) else { return }; self.snap(false);
        if s == e { self.doc.paras[r].table = false; } else { self.doc.paras.remove(r); self.cur = (r.min(e - 1), 0); }
    }
    pub fn table_add_col(&mut self) {
        let Some((s, e)) = self.table_block(self.cur.0) else { return }; self.snap(false);
        for i in s..=e { let p = &mut self.doc.paras[i]; p.runs.push(Run::plain("\t")); p.normalize(); }
    }
    pub fn table_del_col(&mut self) {
        let Some((s, e)) = self.table_block(self.cur.0) else { return }; self.snap(false);
        if self.doc.paras[s].text().contains('\t') { for i in s..=e { let n = self.doc.paras[i].len(); let t: Vec<char> = self.doc.paras[i].text().chars().collect(); if let Some(p) = t.iter().rposition(|c| *c == '\t') { Self::replace_head(&mut self.doc.paras[i], p, n - p, ""); } } }
        self.cur = (self.cur.0, self.cur.1.min(self.doc.paras[self.cur.0].len()));
    }
    // ---------- images ----------
    /// Insert a picture paragraph after the current paragraph. Errors when the data is not PNG/JPEG/GIF.
    pub fn insert_image(&mut self, data: Vec<u8>, alt: &str) -> Result<(), String> {
        let (w, h) = omni_common::image::size(&data).ok_or("Unsupported image (PNG, JPEG and GIF are supported)")?; self.snap(false);
        let at = self.cur.0 + 1; self.doc.paras.insert(at, Para { image: Some(Image { data: std::sync::Arc::new(data), w, h, alt: alt.to_string() }), ..Default::default() });
        if at + 1 >= self.doc.paras.len() { self.doc.paras.push(Para::default()); } self.cur = (at + 1, 0); self.anchor = None; Ok(())
    }
    /// Headings for the navigation pane: (level, paragraph index, title).
    pub fn outline(&self) -> Vec<(u8, usize, String)> {
        self.doc.paras.iter().enumerate().filter(|(_, p)| p.style.level() > 0 || p.style == Style::Title)
            .map(|(i, p)| (p.style.level().max(1), i, p.text())).collect()
    }
}
