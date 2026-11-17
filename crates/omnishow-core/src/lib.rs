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
named_enum!(ShapeKind { Rect = "rect", Ellipse = "ellipse", TextBox = "textbox", Line = "line", Image = "image" });

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
pub struct Shape { pub kind: ShapeKind, pub x: f32, pub y: f32, pub w: f32, pub h: f32, pub text: String, pub color: Option<u32>, pub image: Option<std::sync::Arc<Vec<u8>>> }
impl Shape {
    pub fn new(kind: ShapeKind) -> Shape { Shape { kind, x: 0.3, y: 0.35, w: 0.4, h: 0.25, text: String::new(), color: None, image: None } }
    /// A picture shape sized to fit inside half of the slide while keeping the aspect ratio of a 16:9 slide.
    pub fn picture(data: Vec<u8>) -> Option<Shape> {
        let (pw, ph) = omni_common::image::size(&data)?; let aspect = pw as f32 / ph.max(1) as f32 * 9. / 16.; // width fraction / height fraction
        let (mut w, mut h) = (0.5f32, 0.5 / aspect); if h > 0.7 { h = 0.7; w = h * aspect; }
        Some(Shape { kind: ShapeKind::Image, x: (1. - w) / 2., y: (1. - h) / 2., w, h, text: String::new(), color: None, image: Some(std::sync::Arc::new(data)) })
    }
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Slide { pub title: String, pub bullets: Vec<String>, pub notes: String, pub layout: Layout, pub transition: Transition, pub shapes: Vec<Shape> }
#[derive(Clone, Debug, PartialEq)]
pub struct Deck { pub slides: Vec<Slide>, pub theme: Theme, pub numbers: bool }
impl Slide {
    /// Move shape `i` to the front (top of the stack) or the back; returns its new index.
    pub fn restack(&mut self, i: usize, front: bool) -> usize { let s = self.shapes.remove(i); if front { self.shapes.push(s); self.shapes.len() - 1 } else { self.shapes.insert(0, s); 0 } }
}
impl Default for Deck { fn default() -> Self { Deck { slides: vec![Slide { title: "Untitled".into(), ..Default::default() }], theme: Theme::Office, numbers: false } } }
impl Deck {
    pub fn to_markdown(&self) -> String { self.to_markdown_media().0 }
    /// Markdown plus the media files (name, bytes) referenced by image shapes.
    pub fn to_markdown_media(&self) -> (String, Vec<(String, std::sync::Arc<Vec<u8>>)>) {
        let (mut o, mut media) = (String::new(), vec![]);
        if self.theme != Theme::Office { o.push_str(&format!("<!-- theme: {} -->\n", self.theme.name())); }
        if self.numbers { o.push_str("<!-- slide-numbers: on -->\n"); }
        for (i, s) in self.slides.iter().enumerate() {
            if i > 0 { o.push_str("---\n"); }
            o.push_str(&format!("# {}\n", s.title));
            if s.layout != Layout::Content { o.push_str(&format!("<!-- layout: {} -->\n", s.layout.name())); }
            if s.transition != Transition::None { o.push_str(&format!("<!-- transition: {} -->\n", s.transition.name())); }
            for sh in &s.shapes {
                let mut text = sh.text.replace('\n', " ");
                if let (ShapeKind::Image, Some(d)) = (sh.kind, &sh.image) { text = format!("media/img{}.{}", media.len() + 1, omni_common::image::ext(d).unwrap_or("bin")); media.push((text.clone(), d.clone())); }
                o.push_str(&format!("<!-- shape: {} {:.4} {:.4} {:.4} {:.4} {}{} -->\n", sh.kind.name(), sh.x, sh.y, sh.w, sh.h, sh.color.map_or(String::new(), |c| format!("#{c:06X} ")), text));
            }
            for b in &s.bullets { o.push_str(&format!("- {b}\n")); } for n in s.notes.lines() { o.push_str(&format!("> {n}\n")); }
        }
        (o, media)
    }
    pub fn from_markdown(s: &str) -> Deck { Deck::from_markdown_media(s, None) }
    /// Parse deck markdown; `media` resolves `media/...` names of image shapes to bytes.
    pub fn from_markdown_media(s: &str, media: Option<&dyn Fn(&str) -> Option<Vec<u8>>>) -> Deck {
        let (mut slides, mut cur, mut theme, mut numbers) = (vec![], None::<Slide>, Theme::Office, false);
        let flush = |c: &mut Option<Slide>, slides: &mut Vec<Slide>| if let Some(x) = c.take() { slides.push(x); };
        for l in s.replace("\r\n", "\n").lines() {
            if l.trim() == "---" { flush(&mut cur, &mut slides); continue; }
            if let Some(t) = l.strip_prefix("# ") { flush(&mut cur, &mut slides); cur = Some(Slide { title: t.to_string(), ..Default::default() }); continue; }
            if let Some(d) = l.trim().strip_prefix("<!--").and_then(|x| x.strip_suffix("-->")) {
                let (k, v) = d.split_once(':').map_or((d.trim(), ""), |(k, v)| (k.trim(), v.trim()));
                match k {
                    "theme" => theme = Theme::parse(v).unwrap_or_default(), "slide-numbers" => numbers = v == "on",
                    "layout" => if let Some(x) = Layout::parse(v) { cur.get_or_insert_with(Slide::default).layout = x; },
                    "transition" => if let Some(x) = Transition::parse(v) { cur.get_or_insert_with(Slide::default).transition = x; },
                    "shape" => { let t: Vec<&str> = v.splitn(6, ' ').collect(); let f = |i: usize| t.get(i).and_then(|x| x.parse::<f32>().ok());
                        if let (Some(k), Some(x), Some(y), Some(w), Some(h)) = (t.first().and_then(|k| ShapeKind::parse(k)), f(1), f(2), f(3), f(4)) { cur.get_or_insert_with(Slide::default).shapes.push({
                            let rest = t.get(5).unwrap_or(&"").to_string();
                            let (color, text) = match rest.strip_prefix('#').and_then(|r| r.get(..6).map(|c| (c, &r[6..]))) { Some((c, tail)) if u32::from_str_radix(c, 16).is_ok() => (u32::from_str_radix(c, 16).ok(), tail.trim_start().to_string()), _ => (None, rest) };
                            let image = if k == ShapeKind::Image { media.and_then(|m| m(&text)).map(std::sync::Arc::new) } else { None };
                            Shape { kind: k, x, y, w, h, text: if k == ShapeKind::Image { String::new() } else { text }, color, image } }); } }
                    _ => {} }
                continue;
            }
            let c = cur.get_or_insert_with(Slide::default);
            if let Some(b) = l.strip_prefix("- ").or_else(|| l.strip_prefix("* ")) { c.bullets.push(b.to_string()); }
            else if let Some(n) = l.strip_prefix("> ") { if !c.notes.is_empty() { c.notes.push('\n'); } c.notes.push_str(n); }
            else if !l.trim().is_empty() { c.bullets.push(l.trim().to_string()); }
        }
        flush(&mut cur, &mut slides);
        if slides.is_empty() { Deck { theme, numbers, ..Deck::default() } } else { Deck { slides, theme, numbers } }
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
        z.add("manifest.json", format!("{{\"oms\":3,\"generator\":\"OmniShow {}\"}}", omni_common::BUILD_ID).as_bytes());
        let (md, media) = self.to_markdown_media(); z.add("deck.md", md.as_bytes()); for (n, b) in media { z.add(&n, &b); } z.finish()
    }
    pub fn from_oms(d: &[u8]) -> Result<Deck, String> { let c = zipio::read(d, "deck.md")?.ok_or("OMS: deck.md missing")?; let media = |n: &str| zipio::read(d, n).ok().flatten(); Ok(Deck::from_markdown_media(&String::from_utf8_lossy(&c), Some(&media))) }
}
#[cfg(test)]
mod tests { use super::*;
    #[test] fn roundtrip_directives_and_outline() {
        let png = { let mut p = vec![0x89, b'P', b'N', b'G', 13, 10, 26, 10, 0, 0, 0, 13, b'I', b'H', b'D', b'R']; p.extend(200u32.to_be_bytes()); p.extend(100u32.to_be_bytes()); p.extend([8, 6, 0, 0, 0]); p };
        let d = Deck { theme: Theme::Ocean, numbers: true, slides: vec![
            Slide { title: "Hi 한글".into(), bullets: vec!["a".into(), "b".into()], notes: "say\nmore".into(), layout: Layout::Title, transition: Transition::Fade, shapes: vec![Shape { text: "box 안".into(), color: Some(0x2E7D32), ..Shape::new(ShapeKind::Rect) }, Shape { x: 0.0625, ..Shape::new(ShapeKind::Line) }, Shape { x: 0.25, y: 0.25, w: 0.5, h: 0.5, ..Shape::picture(png.clone()).unwrap() }] },
            Slide { title: "Two".into(), ..Default::default() }] };
        assert_eq!(Deck::from_oms(&d.to_oms()).unwrap(), d);
        assert_eq!(Deck::from_markdown("# A\n- x\n"), Deck { slides: vec![Slide { title: "A".into(), bullets: vec!["x".into()], ..Default::default() }], theme: Theme::Office, numbers: false }); // v1 files still load
        let o = Deck::from_outline(&[(1, "A".into()), (2, "a1".into()), (1, "B".into())]); assert_eq!((o.slides.len(), o.slides[0].bullets.clone()), (2, vec!["a1".to_string()]));
    }
    #[test] fn pictures_and_stacking() {
        let mut p = vec![0x89, b'P', b'N', b'G', 13, 10, 26, 10, 0, 0, 0, 13, b'I', b'H', b'D', b'R']; p.extend(200u32.to_be_bytes()); p.extend(100u32.to_be_bytes()); p.extend([8, 6, 0, 0, 0]);
        let s = Shape::picture(p).unwrap(); assert!((s.w / s.h * 16. / 9. - 2.0).abs() < 0.01 && s.x > 0.0, "keeps the 2:1 aspect: {s:?}"); assert!(Shape::picture(b"x".to_vec()).is_none());
        let mut sl = Slide { shapes: vec![Shape::new(ShapeKind::Rect), Shape { text: "b".into(), ..Shape::new(ShapeKind::Ellipse) }, Shape { text: "c".into(), ..Shape::new(ShapeKind::Line) }], ..Default::default() };
        assert_eq!(sl.restack(0, true), 2); assert_eq!(sl.shapes[2].kind, ShapeKind::Rect); assert_eq!(sl.restack(2, false), 0); assert_eq!(sl.shapes[0].kind, ShapeKind::Rect);
    }
    #[test] fn move_and_themes() { let mut d = Deck { slides: vec![Slide { title: "1".into(), ..Default::default() }, Slide { title: "2".into(), ..Default::default() }], ..Deck::default() };
        assert_eq!(d.move_slide(0, false), 1); assert_eq!(d.slides[0].title, "2"); assert_eq!(d.move_slide(0, true), 0); assert!(Theme::ALL.len() == 5 && Theme::Mono.colors().bg == 0x1E1F22); }
}
