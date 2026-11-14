// SPDX-License-Identifier: GPL-3.0-only
//! OmniShow: presentation UI (slide sorter, 16:9 canvas, speaker notes, slideshow mode).
use omni_common::config;
use omni_shell::*;
use omnishow_core::{Deck, Slide};
use std::path::PathBuf;

#[derive(Clone, Copy, PartialEq)]
enum Field { Title, Bullet(usize), Notes }
struct Show { deck: Deck, path: Option<PathBuf>, cur: usize, focus: Field, present: bool, dirty: bool, toast: String, hot: Vec<(Rect, Field)>, thumbs: Vec<Rect>, size: (f32, f32), quit: bool, ic: Vec<(Rect, &'static str, &'static str)>, hover: (f32, f32) }
fn pop(s: &mut String) { s.pop(); }
impl Show {
    fn field(&mut self) -> &mut String {
        let f = self.focus; let s = &mut self.deck.slides[self.cur];
        match f { Field::Title => &mut s.title, Field::Notes => &mut s.notes, Field::Bullet(i) => { while s.bullets.len() <= i { s.bullets.push(String::new()); } &mut s.bullets[i] } }
    }
    fn open(&mut self) {
        let Some(p) = rfd::FileDialog::new().add_filter("OmniShow / Markdown", &["oms", "md"]).pick_file() else { return };
        let d = std::fs::read(&p).unwrap_or_default();
        let r = if p.extension().map_or(false, |e| e == "oms") { Deck::from_oms(&d) } else { Ok(Deck::from_markdown(&String::from_utf8_lossy(&d))) };
        match r { Ok(d) => { self.deck = d; self.path = Some(p); self.cur = 0; self.dirty = false; self.toast = "Opened".into() } Err(e) => self.toast = e }
    }
    fn save(&mut self) {
        if self.path.is_none() { self.path = rfd::FileDialog::new().add_filter("OmniShow", &["oms"]).add_filter("Markdown outline", &["md"]).set_file_name("deck.oms").save_file(); }
        let Some(p) = self.path.clone() else { return };
        let b = if p.extension().map_or(false, |e| e == "md") { self.deck.to_markdown().into_bytes() } else { self.deck.to_oms() };
        match std::fs::write(&p, b) { Ok(()) => { self.dirty = false; self.toast = "Saved".into(); let mut r = config::recent_load("omnishow"); config::recent_push(&mut r, &p.to_string_lossy(), 8); config::recent_save("omnishow", &r); } Err(e) => self.toast = e.to_string() }
    }
    fn act(&mut self, id: &str) {
        match id {
            "new" => { self.deck.slides.insert(self.cur + 1, Slide::default()); self.cur += 1; self.focus = Field::Title; self.dirty = true; }
            "dup" => { let s = self.deck.slides[self.cur].clone(); self.deck.slides.insert(self.cur + 1, s); self.cur += 1; self.dirty = true; }
            "del" => if self.deck.slides.len() > 1 { self.deck.slides.remove(self.cur); self.cur = self.cur.min(self.deck.slides.len() - 1); self.dirty = true; },
            "import" => self.import_outline(), "present" => self.present = true, "open" => self.open(), _ => self.save(),
        }
    }
    /// Import the heading outline of any document OmniDoc can read (Ctrl+I).
    fn import_outline(&mut self) {
        let Some(p) = rfd::FileDialog::new().pick_file() else { return };
        let ext = p.extension().and_then(|e| e.to_str()).unwrap_or("").to_string();
        match std::fs::read(&p).map_err(|e| e.to_string()).and_then(|d| omnidoc_core::formats::load(&ext, &d)) {
            Ok(doc) => { let o: Vec<(u8, String)> = omnidoc_core::edit::Editor::new(doc).outline().into_iter().map(|x| (x.0, x.2)).collect(); self.deck = Deck::from_outline(&o); self.cur = 0; self.dirty = true; self.toast = format!("{} slides from outline", self.deck.slides.len()); }
            Err(e) => self.toast = e,
        }
    }
    fn slide(&self, c: &Canvas, s: &Slide, r: Rect, edit: bool) {
        let k = r.width() / 960.; fill(c, r, 0xFFFFFF); stroke(c, r, 0xC9CED6, 1.);
        let focus = |f: Field| edit && self.focus == f; let caret = |on: bool| if on { "|" } else { "" };
        if s.bullets.is_empty() && !focus(Field::Bullet(0)) {
            let t = format!("{}{}", s.title, caret(focus(Field::Title))); let tw = measure(&t, 56. * k, true); text(c, &t, r.left + (r.width() - tw) / 2., r.top + r.height() * 0.55, 56. * k, true, 0x1F3864);
        } else {
            fill(c, Rect::new(r.left, r.top, r.left + 14. * k, r.bottom), 0xC55A11);
            text(c, &format!("{}{}", s.title, caret(focus(Field::Title))), r.left + 60. * k, r.top + 110. * k, 44. * k, true, 0x1F3864);
            for (i, b) in s.bullets.iter().enumerate() { text(c, &format!("\u{2022}  {}{}", b, caret(focus(Field::Bullet(i)))), r.left + 70. * k, r.top + (210. + i as f32 * 62.) * k, 30. * k, false, 0x222222); }
        }
    }
}
impl App for Show {
    fn title(&self) -> String { format!("{}{} - OmniShow {}", self.path.as_ref().and_then(|p| p.file_name()).and_then(|n| n.to_str()).unwrap_or("Untitled"), if self.dirty { " *" } else { "" }, omni_common::BUILD_ID) }
    fn wants_quit(&self) -> bool { self.quit }
    fn draw(&mut self, c: &Canvas, w: f32, h: f32) {
        self.size = (w, h);
        if self.present { fill(c, Rect::new(0., 0., w, h), 0x000000); let (sw, sh) = if w / h > 16. / 9. { (h * 16. / 9., h) } else { (w, w * 9. / 16.) }; self.slide(c, &self.deck.slides[self.cur].clone(), Rect::new((w - sw) / 2., (h - sh) / 2., (w + sw) / 2., (h + sh) / 2.), false); return; }
        fill(c, Rect::new(0., 0., w, h), 0xE9ECF1); fill(c, Rect::new(0., 0., w, 36.), 0xC55A11);
        text(c, &format!("OmniShow  |  {}{}", self.path.as_ref().and_then(|p| p.file_name()).and_then(|n| n.to_str()).unwrap_or("Untitled"), if self.dirty { "  - Unsaved" } else { "" }), 12., 23., 13., true, 0xFFFFFF);
        self.ic.clear(); let mut hov = "";
        for (i, (id, ic, tip)) in [("new", "add_slide", "New slide (Ctrl+M)"), ("dup", "content_copy", "Duplicate (Ctrl+D)"), ("del", "delete", "Delete (Ctrl+Del)"), ("import", "import", "Import outline (Ctrl+I)"), ("present", "play", "Present (F5)"), ("open", "folder_open", "Open (Ctrl+O)"), ("save", "save", "Save (Ctrl+S)")].into_iter().enumerate() {
            let r = Rect::new(400. + i as f32 * 36., 5., 400. + i as f32 * 36. + 30., 31.); if inside(&r, self.hover) { rrect(c, r, 6., 0xD9702E); hov = tip; }
            icon(c, ic, r.left + 3., r.top + 1., 24., 0xFFFFFF); self.ic.push((r, id, tip));
        }
        text(c, hov, 660., 23., 12., false, 0xFBE3D3);
        fill(c, Rect::new(0., 36., 210., h), 0xF6F7F9); self.thumbs.clear();
        for (i, s) in self.deck.slides.iter().enumerate() { let r = Rect::new(18., 50. + i as f32 * 118., 192., 50. + i as f32 * 118. + 99.); if r.bottom > h { break; }
            self.slide(c, s, r, false); text(c, &(i + 1).to_string(), 4., r.top + 14., 12., true, 0x555555); if i == self.cur { stroke(c, r.with_outset((3., 3.)), 0xC55A11, 3.); } self.thumbs.push(r); }
        let area = Rect::new(230., 50., w - 20., h - 150.); let sw = area.width().min(area.height() * 16. / 9.); let sh = sw * 9. / 16.;
        let r = Rect::new(area.left + (area.width() - sw) / 2., area.top + (area.height() - sh) / 2., area.left + (area.width() + sw) / 2., area.top + (area.height() + sh) / 2.);
        let s = self.deck.slides[self.cur].clone(); self.slide(c, &s, r, true);
        let k = r.width() / 960.; self.hot.clear(); self.hot.push((Rect::new(r.left, r.top + 50. * k, r.right, r.top + 130. * k), Field::Title));
        for i in 0..=s.bullets.len() { self.hot.push((Rect::new(r.left, r.top + (170. + i as f32 * 62.) * k, r.right, r.top + (225. + i as f32 * 62.) * k), Field::Bullet(i))); }
        let nr = Rect::new(230., h - 140., w - 20., h - 10.); fill(c, nr, 0xFFFFFF); stroke(c, nr, if self.focus == Field::Notes { 0xC55A11 } else { 0xC9CED6 }, if self.focus == Field::Notes { 2. } else { 1. });
        text(c, "Speaker notes", nr.left + 10., nr.top + 20., 12., true, 0x888888);
        for (i, l) in format!("{}{}", s.notes, if self.focus == Field::Notes { "|" } else { "" }).lines().enumerate().take(5) { text(c, l, nr.left + 10., nr.top + 44. + i as f32 * 20., 13., false, 0x222222); }
        self.hot.push((nr, Field::Notes)); text(c, &format!("Slide {} of {}   {}", self.cur + 1, self.deck.slides.len(), self.toast), 250., 23., 12., false, 0xFFFFFF);
    }
    fn key(&mut self, e: &KeyEvent, m: ModifiersState) {
        self.toast.clear();
        if self.present {
            match &e.logical_key { Key::Named(NamedKey::Escape) | Key::Named(NamedKey::F5) => self.present = false,
                Key::Named(NamedKey::ArrowRight) | Key::Named(NamedKey::ArrowDown) | Key::Named(NamedKey::PageDown) | Key::Named(NamedKey::Space) | Key::Named(NamedKey::Enter) => self.cur = (self.cur + 1).min(self.deck.slides.len() - 1),
                Key::Named(NamedKey::ArrowLeft) | Key::Named(NamedKey::ArrowUp) | Key::Named(NamedKey::PageUp) | Key::Named(NamedKey::Backspace) => self.cur = self.cur.saturating_sub(1), _ => {} }
            return;
        }
        if m.control_key() || m.super_key() {
            match &e.logical_key {
                Key::Character(k) => match k.to_lowercase().as_str() {
                    "s" => self.save(), "o" => self.open(), "i" => self.import_outline(),
                    "m" => { self.deck.slides.insert(self.cur + 1, Slide::default()); self.cur += 1; self.focus = Field::Title; self.dirty = true; }
                    "d" => { let s = self.deck.slides[self.cur].clone(); self.deck.slides.insert(self.cur + 1, s); self.cur += 1; self.dirty = true; } _ => {} },
                Key::Named(NamedKey::Delete) if self.deck.slides.len() > 1 => { self.deck.slides.remove(self.cur); self.cur = self.cur.min(self.deck.slides.len() - 1); self.dirty = true; }
                _ => {} }
            return;
        }
        let nb = self.deck.slides[self.cur].bullets.len();
        match &e.logical_key {
            Key::Named(NamedKey::F5) => self.present = true,
            Key::Named(NamedKey::PageDown) => self.cur = (self.cur + 1).min(self.deck.slides.len() - 1), Key::Named(NamedKey::PageUp) => self.cur = self.cur.saturating_sub(1),
            Key::Named(NamedKey::ArrowDown) => self.focus = match self.focus { Field::Title if nb > 0 => Field::Bullet(0), Field::Bullet(i) if i + 1 < nb => Field::Bullet(i + 1), f => f },
            Key::Named(NamedKey::ArrowUp) => self.focus = match self.focus { Field::Bullet(0) => Field::Title, Field::Bullet(i) => Field::Bullet(i - 1), f => f },
            Key::Named(NamedKey::Enter) => match self.focus {
                Field::Title => { if nb == 0 { self.deck.slides[self.cur].bullets.push(String::new()); } self.focus = Field::Bullet(0); }
                Field::Bullet(i) => { self.deck.slides[self.cur].bullets.insert(i + 1, String::new()); self.focus = Field::Bullet(i + 1); self.dirty = true; }
                Field::Notes => { self.field().push('\n'); self.dirty = true; } },
            Key::Named(NamedKey::Backspace) => { if let Field::Bullet(i) = self.focus { if self.deck.slides[self.cur].bullets.get(i).map_or(false, |b| b.is_empty()) { if i < nb { self.deck.slides[self.cur].bullets.remove(i); } self.focus = if i == 0 { Field::Title } else { Field::Bullet(i - 1) }; return; } } pop(self.field()); self.dirty = true; }
            _ => if let Some(t) = &e.text { if !t.chars().any(char::is_control) { self.field().push_str(t); self.dirty = true; } },
        }
    }
    fn text(&mut self, s: &str) { if !self.present { self.field().push_str(s); self.dirty = true; } }
    fn moved(&mut self, x: f32, y: f32) { self.hover = (x, y); }
    fn mouse(&mut self, x: f32, y: f32, _r: bool) {
        if !self.present { if let Some(id) = self.ic.iter().find(|h| inside(&h.0, (x, y))).map(|h| h.1) { self.act(id); return; } }
        if self.present { self.cur = (self.cur + 1).min(self.deck.slides.len() - 1); return; }
        if let Some(i) = self.thumbs.iter().position(|r| inside(r, (x, y))) { self.cur = i; return; }
        if let Some((_, f)) = self.hot.iter().find(|(r, _)| inside(r, (x, y))).cloned() { self.focus = f; }
    }
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut s = Show { deck: Deck::default(), path: None, cur: 0, focus: Field::Title, present: false, dirty: false, toast: String::new(), hot: vec![], thumbs: vec![], size: (1180., 760.), quit: false, ic: vec![], hover: (0., 0.) };
    if let Some(p) = std::env::args().nth(1) { let p = PathBuf::from(p); let d = std::fs::read(&p).unwrap_or_default(); let r = if p.extension().map_or(false, |e| e == "oms") { Deck::from_oms(&d) } else { Ok(Deck::from_markdown(&String::from_utf8_lossy(&d))) }; if let Ok(d) = r { s.deck = d; s.path = Some(p); } }
    run(s, "OmniShow")
}
