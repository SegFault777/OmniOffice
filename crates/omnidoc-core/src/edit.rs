// SPDX-License-Identifier: GPL-3.0-only
//! Editing engine: cursor, selection, formatting, undo/redo, find/replace, outline.
use crate::model::*;
pub type Pos = (usize, usize); // (paragraph, char offset)
#[derive(Clone, Copy)]
pub enum Fmt { Bold, Italic, Underline }
fn get(r: &Run, f: Fmt) -> bool { match f { Fmt::Bold => r.bold, Fmt::Italic => r.italic, Fmt::Underline => r.underline } }
fn set(r: &mut Run, f: Fmt, v: bool) { match f { Fmt::Bold => r.bold = v, Fmt::Italic => r.italic = v, Fmt::Underline => r.underline = v } }
fn fold(s: &str, cs: bool) -> Vec<char> { s.chars().map(|c| if cs { c } else { c.to_lowercase().next().unwrap_or(c) }).collect() }

pub struct Editor {
    pub doc: Doc, pub cur: Pos, pub anchor: Option<Pos>, pub pend: Option<Run>,
    undo: Vec<(Doc, Pos)>, redo: Vec<(Doc, Pos)>, typing: bool, pub dirty: bool,
}
impl Editor {
    pub fn new(doc: Doc) -> Self { Editor { doc, cur: (0, 0), anchor: None, pend: None, undo: vec![], redo: vec![], typing: false, dirty: false } }
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
        let mut np = Para { style: st, align: old.align, runs: b }; np.normalize();
        self.doc.paras[r].runs = a; self.doc.paras[r].normalize();
        self.doc.paras.insert(r + 1, np); self.cur = (r + 1, 0);
    }
    pub fn insert(&mut self, s: &str) {
        match self.range() { Some((a, b)) => { self.snap(false); self.cut(a, b); } None => self.snap(!s.contains('\n')) }
        for (i, line) in s.replace("\r\n", "\n").split('\n').enumerate() { if i > 0 { self.split_para(); } self.ins_text(line); }
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
    /// Headings for the navigation pane: (level, paragraph index, title).
    pub fn outline(&self) -> Vec<(u8, usize, String)> {
        self.doc.paras.iter().enumerate().filter(|(_, p)| p.style.level() > 0 || p.style == Style::Title)
            .map(|(i, p)| (p.style.level().max(1), i, p.text())).collect()
    }
}
