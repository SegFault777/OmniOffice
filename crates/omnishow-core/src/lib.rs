// SPDX-License-Identifier: GPL-3.0-only
//! OmniShow core: slides with a title, bullet points and speaker notes.
//! OMS container: stored ZIP with mimetype, manifest.json and deck.md (slides separated by `---`;
//! `# Title`, `- bullet`, `> speaker note`).
use omni_common::zipio;
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Slide { pub title: String, pub bullets: Vec<String>, pub notes: String }
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Deck { pub slides: Vec<Slide> }
impl Default for Deck { fn default() -> Self { Deck { slides: vec![Slide { title: "Untitled".into(), ..Default::default() }] } } }
impl Deck {
    pub fn to_markdown(&self) -> String {
        let mut o = String::new();
        for (i, s) in self.slides.iter().enumerate() {
            if i > 0 { o.push_str("---\n"); }
            o.push_str(&format!("# {}\n", s.title)); for b in &s.bullets { o.push_str(&format!("- {b}\n")); } for n in s.notes.lines() { o.push_str(&format!("> {n}\n")); }
        }
        o
    }
    pub fn from_markdown(s: &str) -> Deck {
        let mut slides = vec![]; let mut cur: Option<Slide> = None;
        let flush = |c: &mut Option<Slide>, slides: &mut Vec<Slide>| if let Some(x) = c.take() { slides.push(x); };
        for l in s.replace("\r\n", "\n").lines() {
            if l.trim() == "---" { flush(&mut cur, &mut slides); continue; }
            if let Some(t) = l.strip_prefix("# ") { flush(&mut cur, &mut slides); cur = Some(Slide { title: t.to_string(), ..Default::default() }); continue; }
            let c = cur.get_or_insert_with(Slide::default);
            if let Some(b) = l.strip_prefix("- ").or_else(|| l.strip_prefix("* ")) { c.bullets.push(b.to_string()); }
            else if let Some(n) = l.strip_prefix("> ") { if !c.notes.is_empty() { c.notes.push('\n'); } c.notes.push_str(n); }
            else if !l.trim().is_empty() { c.bullets.push(l.trim().to_string()); }
        }
        flush(&mut cur, &mut slides); if slides.is_empty() { Deck::default() } else { Deck { slides } }
    }
    /// Build a deck from a document outline: each (level, title) heading becomes a slide, deeper headings become bullets.
    pub fn from_outline(items: &[(u8, String)]) -> Deck {
        let mut slides: Vec<Slide> = vec![]; let top = items.iter().map(|x| x.0).min().unwrap_or(1);
        for (l, t) in items { if *l == top || slides.is_empty() { slides.push(Slide { title: t.clone(), ..Default::default() }); } else { slides.last_mut().unwrap().bullets.push(t.clone()); } }
        if slides.is_empty() { Deck::default() } else { Deck { slides } }
    }
    pub fn to_oms(&self) -> Vec<u8> {
        let mut z = zipio::Writer::new(); z.add("mimetype", b"application/vnd.omnioffice.oms+zip");
        z.add("manifest.json", format!("{{\"oms\":1,\"generator\":\"OmniShow {}\"}}", omni_common::BUILD_ID).as_bytes()); z.add("deck.md", self.to_markdown().as_bytes()); z.finish()
    }
    pub fn from_oms(d: &[u8]) -> Result<Deck, String> { let c = zipio::read(d, "deck.md")?.ok_or("OMS: deck.md missing")?; Ok(Deck::from_markdown(&String::from_utf8_lossy(&c))) }
}
#[cfg(test)]
mod tests { use super::*;
    #[test] fn roundtrip_and_outline() {
        let d = Deck { slides: vec![Slide { title: "Hi 한글".into(), bullets: vec!["a".into(), "b".into()], notes: "say\nmore".into() }, Slide { title: "Two".into(), ..Default::default() }] };
        assert_eq!(Deck::from_oms(&d.to_oms()).unwrap(), d);
        let o = Deck::from_outline(&[(1, "A".into()), (2, "a1".into()), (1, "B".into())]); assert_eq!((o.slides.len(), o.slides[0].bullets.clone()), (2, vec!["a1".to_string()]));
    }
}
