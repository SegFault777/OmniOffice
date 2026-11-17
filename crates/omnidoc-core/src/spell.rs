// SPDX-License-Identifier: GPL-3.0-only
//! Spell checking against a word list (one word per line). No dictionary is bundled: the UI loads the first list it finds
//! (user file in the config directory, then common system locations). Only plain ASCII words are checked, so Korean and other
//! scripts are never flagged.
use std::collections::HashSet;
#[derive(Default, Clone)]
pub struct Dict { words: HashSet<String> }
impl Dict {
    pub fn from_text(s: &str) -> Dict { Dict { words: s.lines().map(|l| l.trim().to_lowercase()).filter(|l| !l.is_empty()).collect() } }
    pub fn is_empty(&self) -> bool { self.words.is_empty() }
    pub fn len(&self) -> usize { self.words.len() }
    pub fn add(&mut self, w: &str) { self.words.insert(w.to_lowercase()); }
    /// Load the first available list: `extra` (e.g. the user's file) then system word lists.
    pub fn load_default(extra: &[std::path::PathBuf]) -> Option<Dict> {
        let sys = ["/usr/share/dict/words", "/usr/share/dict/american-english", "/usr/share/dict/british-english", "/usr/share/hunspell/en_US.dic"];
        let mut d = Dict::default();
        for p in extra.iter().cloned().chain(sys.iter().map(std::path::PathBuf::from)) {
            if let Ok(s) = std::fs::read_to_string(&p) { let s = if p.extension().map_or(false, |e| e == "dic") { s.lines().skip(1).map(|l| l.split('/').next().unwrap_or("")).collect::<Vec<_>>().join("\n") } else { s }; let n = Dict::from_text(&s); d.words.extend(n.words); }
        }
        (!d.is_empty()).then_some(d)
    }
    fn checkable(w: &str) -> bool { w.chars().count() >= 2 && w.chars().all(|c| c.is_ascii_alphabetic() || c == '\'') }
    pub fn check(&self, w: &str) -> bool {
        let w = w.trim_matches('\''); if !Self::checkable(w) { return true; }
        let l = w.to_lowercase(); self.words.contains(&l) || l.strip_suffix("'s").map_or(false, |b| self.words.contains(b))
    }
    /// Words at distance one (delete, transpose, replace, insert) that are in the list, shortest first.
    pub fn suggest(&self, w: &str, max: usize) -> Vec<String> {
        let l: Vec<char> = w.to_lowercase().chars().collect(); let mut c: HashSet<String> = HashSet::new(); let al = 'a'..='z';
        for i in 0..=l.len() {
            if i < l.len() { let mut v = l.clone(); v.remove(i); c.insert(v.iter().collect()); for x in al.clone() { let mut v = l.clone(); v[i] = x; c.insert(v.iter().collect()); } }
            if i + 1 < l.len() { let mut v = l.clone(); v.swap(i, i + 1); c.insert(v.iter().collect()); }
            for x in al.clone() { let mut v = l.clone(); v.insert(i, x); c.insert(v.iter().collect()); }
        }
        let mut r: Vec<String> = c.into_iter().filter(|x| x != &l.iter().collect::<String>() && self.words.contains(x)).collect();
        r.sort_by_key(|x| (x.chars().count() as i32 - l.len() as i32).abs() * 10 + 0); r.sort_by(|a, b| (a.len(), a).cmp(&(b.len(), b))); r.truncate(max);
        if w.chars().next().map_or(false, |c| c.is_uppercase()) { r = r.into_iter().map(|s| { let mut c = s.chars(); c.next().map(|f| f.to_uppercase().chain(c).collect()).unwrap_or_default() }).collect(); }
        r
    }
    /// Misspelled words in `text` as (start, end) character ranges.
    pub fn misspelled(&self, text: &str) -> Vec<(usize, usize)> {
        let (mut out, mut start, mut cur) = (vec![], 0usize, String::new()); let chars: Vec<char> = text.chars().collect();
        for (i, c) in chars.iter().chain(std::iter::once(&' ')).enumerate() {
            if c.is_alphabetic() || *c == '\'' { if cur.is_empty() { start = i; } cur.push(*c); }
            else { if !cur.is_empty() && !self.check(&cur) { out.push((start, i)); } cur.clear(); }
        }
        out
    }
}
#[cfg(test)]
mod tests { use super::*;
    #[test] fn spelling() {
        let d = Dict::from_text("hello\nworld\nthe\nquick\nbrown\nfox\nit\n"); assert!(d.check("Hello") && d.check("it's") && d.check("한글") && d.check("a1") && !d.check("wrold"));
        assert_eq!(d.suggest("wrold", 3), ["world"]); assert_eq!(d.suggest("Helo", 3), ["Hello"]);
        assert_eq!(d.misspelled("The quik fox, 한글 and hello"), [(4, 8), (17, 20)]);
    }
}
