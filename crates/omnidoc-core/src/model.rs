// SPDX-License-Identifier: GPL-3.0-only
//! Rich document model: paragraphs made of formatted runs.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Style { #[default] Normal, Title, H1, H2, H3, Quote, Code }
impl Style {
    pub fn level(self) -> u8 { match self { Style::H1 => 1, Style::H2 => 2, Style::H3 => 3, _ => 0 } }
    pub fn from_level(l: u32) -> Style { match l { 1 => Style::H1, 2 => Style::H2, 3 => Style::H3, _ => Style::Normal } }
}
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Align { #[default] Left, Center, Right, Justify }

#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct Run { pub text: String, pub bold: bool, pub italic: bool, pub underline: bool }
impl Run {
    pub fn fmt_eq(&self, o: &Run) -> bool { self.bold == o.bold && self.italic == o.italic && self.underline == o.underline }
    pub fn plain(t: &str) -> Run { Run { text: t.into(), ..Default::default() } }
}
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct Para { pub style: Style, pub align: Align, pub runs: Vec<Run> }
impl Para {
    pub fn plain(t: &str) -> Para { Para { runs: vec![Run::plain(t)], ..Default::default() } }
    pub fn text(&self) -> String { self.runs.iter().map(|r| r.text.as_str()).collect() }
    pub fn len(&self) -> usize { self.runs.iter().map(|r| r.text.chars().count()).sum() }
    pub fn is_empty(&self) -> bool { self.len() == 0 }
    /// Merge neighbouring runs with equal formatting and drop empty ones.
    pub fn normalize(&mut self) {
        let mut out: Vec<Run> = Vec::new();
        for r in self.runs.drain(..) {
            if r.text.is_empty() { continue; }
            match out.last_mut() { Some(l) if l.fmt_eq(&r) => l.text.push_str(&r.text), _ => out.push(r) }
        }
        self.runs = out;
    }
    /// Split the runs at a character offset.
    pub fn split(&self, off: usize) -> (Vec<Run>, Vec<Run>) {
        let (mut a, mut b, mut n) = (vec![], vec![], 0);
        for r in &self.runs {
            let l = r.text.chars().count();
            if n + l <= off { a.push(r.clone()) } else if n >= off { b.push(r.clone()) } else {
                let at = r.text.char_indices().nth(off - n).map(|x| x.0).unwrap_or(r.text.len());
                a.push(Run { text: r.text[..at].into(), ..r.clone() });
                b.push(Run { text: r.text[at..].into(), ..r.clone() });
            }
            n += l;
        }
        (a, b)
    }
}
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Doc { pub paras: Vec<Para> }
impl Default for Doc { fn default() -> Self { Doc { paras: vec![Para::default()] } } }
impl Doc {
    pub fn from_plain(s: &str) -> Doc {
        let mut p: Vec<Para> = s.replace("\r\n", "\n").split('\n').map(Para::plain).collect();
        if p.len() > 1 && p.last().map_or(false, |x| x.is_empty()) { p.pop(); }
        Doc { paras: p }
    }
    pub fn plain(&self) -> String { self.paras.iter().map(|p| p.text() + "\n").collect() }
    pub fn words(&self) -> usize { self.paras.iter().map(|p| p.text().split_whitespace().count()).sum() }
}
