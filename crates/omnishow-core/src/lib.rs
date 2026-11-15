// SPDX-License-Identifier: GPL-3.0-only
//! OmniShow core: slides with title, bullets, notes, layout, transition and simple shapes, plus a colour theme per deck.
//! OMS container: stored ZIP with mimetype, manifest.json and deck.md. deck.md is Markdown: slides separated by `---`,
//! `# Title`, `- bullet`, `> speaker note`, and `<!-- key: value -->` directives (theme, layout, transition, shape).
use omni_common::zipio;

macro_rules! named_enum { ($name:ident { $($v:ident = $s:expr),+ }) => {
    #[derive(Clone, Copy, Debug, Default, PartialEq, Eq)] pub enum $name { #[default] $($v),+ }
    impl $name { pub const ALL: &'static [$name] = &[$($name::$v),+];
        pub fn name(self) -> &'static str { match self { $($name::$v => $s),+ } }
        pub fn parse(s: &str) -> Option<$name> { match s { $($s => Some($name::$v),)+ _ => None } } }
} }
named_enum!(Layout { Content = "content", Title = "title", Section = "section", Blank = "blank" });
named_enum!(Transition { None = "none", Fade = "fade", Push = "push", Wipe = "wipe" });
named_enum!(Theme { Office = "office", Ocean = "ocean", Forest = "forest", Sunset = "sunset", Mono = "mono" });
named_enum!(ShapeKind { Rect = "rect", Ellipse = "ellipse", TextBox = "textbox", Line = "line" });

pub struct ThemeColors { pub bg: u32, pub title: u32, pub text: u32, pub accent: u32 }
impl Theme {
    pub fn colors(self) -> ThemeColors {
        let (bg, title, text, accent) = match self { Theme::Office => (0xFFFFFF, 0x1F3864, 0x222222, 0xC55A11), Theme::Ocean => (0xF2F8FC, 0x0B3C5D, 0x1B2B34, 0x1D70A2),
            Theme::Forest => (0xF3F8F4, 0x1E4D2B, 0x1F2A22, 0x4C9F70), Theme::Sunset => (0xFFF7F0, 0x7A2E1D, 0x33231E, 0xE4572E), Theme::Mono => (0x1E1F22, 0xFFFFFF, 0xD8DAE0, 0xFFB400) };
        ThemeColors { bg, title, text, accent }
    }
}
/// Position and size are fractions of the slide (0..1).
#[derive(Clone, Debug, PartialEq)]
pub struct Shape { pub kind: ShapeKind, pub x: f32, pub y: f32, pub w: f32, pub h: f32, pub text: String }
impl Shape { pub fn new(kind: ShapeKind) -> Shape { Shape { kind, x: 0.3, y: 0.35, w: 0.4, h: 0.25, text: String::new() } } }
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Slide { pub title: String, pub bullets: Vec<String>, pub notes: String, pub layout: Layout, pub transition: Transition, pub shapes: Vec<Shape> }
#[derive(Clone, Debug, PartialEq)]
pub struct Deck { pub slides: Vec<Slide>, pub theme: Theme }
impl Default for Deck { fn default() -> Self { Deck { slides: vec![Slide { title: "Untitled".into(), ..Default::default() }], theme: Theme::Office } } }
impl Deck {
    pub fn to_markdown(&self) -> String {
        let mut o = String::new();
        if self.theme != Theme::Office { o.push_str(&format!("<!-- theme: {} -->\n", self.theme.name())); }
        for (i, s) in self.slides.iter().enumerate() {
            if i > 0 { o.push_str("---\n"); }
            o.push_str(&format!("# {}\n", s.title));
            if s.layout != Layout::Content { o.push_str(&format!("<!-- layout: {} -->\n", s.layout.name())); }
            if s.transition != Transition::None { o.push_str(&format!("<!-- transition: {} -->\n", s.transition.name())); }
            for sh in &s.shapes { o.push_str(&format!("<!-- shape: {} {:.4} {:.4} {:.4} {:.4} {} -->\n", sh.kind.name(), sh.x, sh.y, sh.w, sh.h, sh.text.replace('\n', " "))); }
            for b in &s.bullets { o.push_str(&format!("- {b}\n")); } for n in s.notes.lines() { o.push_str(&format!("> {n}\n")); }
        }
        o
    }
    pub fn from_markdown(s: &str) -> Deck {
        let (mut slides, mut cur, mut theme) = (vec![], None::<Slide>, Theme::Office);
        let flush = |c: &mut Option<Slide>, slides: &mut Vec<Slide>| if let Some(x) = c.take() { slides.push(x); };
        for l in s.replace("\r\n", "\n").lines() {
            if l.trim() == "---" { flush(&mut cur, &mut slides); continue; }
            if let Some(t) = l.strip_prefix("# ") { flush(&mut cur, &mut slides); cur = Some(Slide { title: t.to_string(), ..Default::default() }); continue; }
            if let Some(d) = l.trim().strip_prefix("<!--").and_then(|x| x.strip_suffix("-->")) {
                let (k, v) = d.split_once(':').map_or((d.trim(), ""), |(k, v)| (k.trim(), v.trim()));
                match k {
                    "theme" => theme = Theme::parse(v).unwrap_or_default(),
                    "layout" => if let Some(x) = Layout::parse(v) { cur.get_or_insert_with(Slide::default).layout = x; },
                    "transition" => if let Some(x) = Transition::parse(v) { cur.get_or_insert_with(Slide::default).transition = x; },
                    "shape" => { let t: Vec<&str> = v.splitn(6, ' ').collect(); let f = |i: usize| t.get(i).and_then(|x| x.parse::<f32>().ok());
                        if let (Some(k), Some(x), Some(y), Some(w), Some(h)) = (t.first().and_then(|k| ShapeKind::parse(k)), f(1), f(2), f(3), f(4)) { cur.get_or_insert_with(Slide::default).shapes.push(Shape { kind: k, x, y, w, h, text: t.get(5).unwrap_or(&"").to_string() }); } }
                    _ => {} }
                continue;
            }
            let c = cur.get_or_insert_with(Slide::default);
            if let Some(b) = l.strip_prefix("- ").or_else(|| l.strip_prefix("* ")) { c.bullets.push(b.to_string()); }
            else if let Some(n) = l.strip_prefix("> ") { if !c.notes.is_empty() { c.notes.push('\n'); } c.notes.push_str(n); }
            else if !l.trim().is_empty() { c.bullets.push(l.trim().to_string()); }
        }
        flush(&mut cur, &mut slides);
        if slides.is_empty() { Deck { theme, ..Deck::default() } } else { Deck { slides, theme } }
    }
    /// Build a deck from a document outline: each top-level heading becomes a slide, deeper headings become bullets.
    pub fn from_outline(items: &[(u8, String)]) -> Deck {
        let (mut slides, top) = (Vec::<Slide>::new(), items.iter().map(|x| x.0).min().unwrap_or(1));
        for (l, t) in items { if *l == top || slides.is_empty() { slides.push(Slide { title: t.clone(), ..Default::default() }); } else { slides.last_mut().unwrap().bullets.push(t.clone()); } }
        if slides.is_empty() { Deck::default() } else { Deck { slides, ..Deck::default() } }
    }
    /// Move slide `i` up or down; returns its new index.
    pub fn move_slide(&mut self, i: usize, up: bool) -> usize {
        let j = if up { i.saturating_sub(1) } else { (i + 1).min(self.slides.len() - 1) }; self.slides.swap(i, j); j
    }
    pub fn to_oms(&self) -> Vec<u8> {
        let mut z = zipio::Writer::new(); z.add("mimetype", b"application/vnd.omnioffice.oms+zip");
        z.add("manifest.json", format!("{{\"oms\":2,\"generator\":\"OmniShow {}\"}}", omni_common::BUILD_ID).as_bytes()); z.add("deck.md", self.to_markdown().as_bytes()); z.finish()
    }
    pub fn from_oms(d: &[u8]) -> Result<Deck, String> { let c = zipio::read(d, "deck.md")?.ok_or("OMS: deck.md missing")?; Ok(Deck::from_markdown(&String::from_utf8_lossy(&c))) }
}
#[cfg(test)]
mod tests { use super::*;
    #[test] fn roundtrip_directives_and_outline() {
        let d = Deck { theme: Theme::Ocean, slides: vec![
            Slide { title: "Hi 한글".into(), bullets: vec!["a".into(), "b".into()], notes: "say\nmore".into(), layout: Layout::Title, transition: Transition::Fade, shapes: vec![Shape { text: "box 안".into(), ..Shape::new(ShapeKind::Rect) }, Shape { x: 0.0625, ..Shape::new(ShapeKind::Line) }] },
            Slide { title: "Two".into(), ..Default::default() }] };
        assert_eq!(Deck::from_oms(&d.to_oms()).unwrap(), d);
        assert_eq!(Deck::from_markdown("# A\n- x\n"), Deck { slides: vec![Slide { title: "A".into(), bullets: vec!["x".into()], ..Default::default() }], theme: Theme::Office }); // v1 files still load
        let o = Deck::from_outline(&[(1, "A".into()), (2, "a1".into()), (1, "B".into())]); assert_eq!((o.slides.len(), o.slides[0].bullets.clone()), (2, vec!["a1".to_string()]));
    }
    #[test] fn move_and_themes() { let mut d = Deck { slides: vec![Slide { title: "1".into(), ..Default::default() }, Slide { title: "2".into(), ..Default::default() }], ..Deck::default() };
        assert_eq!(d.move_slide(0, false), 1); assert_eq!(d.slides[0].title, "2"); assert_eq!(d.move_slide(0, true), 0); assert!(Theme::ALL.len() == 5 && Theme::Mono.colors().bg == 0x1E1F22); }
}
