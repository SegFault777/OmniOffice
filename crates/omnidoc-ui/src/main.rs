// SPDX-License-Identifier: GPL-3.0-only
//! OmniDoc desktop UI. OS-native window (winit) + native dialogs/clipboard/IME, drawn with Skia.
//! Layout = Word 365 (title-bar search, tabbed ribbon, style gallery) + LibreOffice/Word Navigation pane
//! + WPS-style familiarity + Google Docs/VS Code (command palette, find bar) + Word "Focus" mode.
use omnidoc_core::{cmd, edit::Editor, formats, model::*};
use skia_safe::{surfaces, textlayout::*, Color, Font, FontMgr, FontStyle, Paint, Rect};
use omni_common::config;
use omni_shell::icon;
use std::{num::NonZeroU32, path::PathBuf, sync::Arc, time::{Duration, Instant}};
use winit::{
    dpi::{LogicalPosition, LogicalSize}, event::*, event_loop::{ControlFlow, EventLoop}, keyboard::{Key, ModifiersState, NamedKey}, window::Window,
};

struct Theme { bg: u32, paper: u32, ink: u32, bar: u32, bar2: u32, accent: u32, line: u32 }
const LIGHT: Theme = Theme { bg: 0xE6E8EC, paper: 0xFFFFFF, ink: 0x1F2328, bar: 0xF5F6F8, bar2: 0xFFFFFF, accent: 0x2B579A, line: 0xD0D3D9 };
const DARK: Theme = Theme { bg: 0x1B1C1F, paper: 0x2A2B2F, ink: 0xE8E9EB, bar: 0x2B2D31, bar2: 0x232428, accent: 0x6B9BF2, line: 0x3C3E44 };
const FAMS: &[&str] = &["Noto Sans CJK KR", "Noto Sans KR", "Malgun Gothic", "Apple SD Gothic Neo", "Segoe UI", "Helvetica Neue", "Arial", "Noto Sans", "sans-serif"];
const MONO: &[&str] = &["Consolas", "Menlo", "DejaVu Sans Mono", "Noto Sans Mono CJK KR", "monospace"];
fn col(c: u32) -> Color { Color::from_argb(255, (c >> 16) as u8, (c >> 8) as u8, c as u8) }
fn u16len(s: &str) -> usize { s.chars().map(char::len_utf16).sum() }
fn u16of(s: &str, chars: usize) -> usize { s.chars().take(chars).map(char::len_utf16).sum() }
fn charof(s: &str, u: usize) -> usize { let (mut n, mut k) = (0, 0); for c in s.chars() { if n >= u { break; } n += c.len_utf16(); k += 1; } k }
fn inside(r: &Rect, p: (f32, f32)) -> bool { p.0 >= r.left && p.0 <= r.right && p.1 >= r.top && p.1 <= r.bottom }

struct App {
    ed: Editor, path: Option<PathBuf>, dark: bool, tab: &'static str, ribbon: bool, nav: bool, focus: bool, zoom: f32, scroll: f32, follow: bool,
    palette: Option<(String, usize)>, find: Option<(String, String, bool, bool)>, preedit: String, mods: ModifiersState, mouse: (f32, f32), drag: bool,
    toast: String, fc: FontCollection, hot: Vec<(Rect, String)>, lay: Vec<(Paragraph, f32)>, page_x: f32, view_h: f32, sf: f32, ime_pos: (f32, f32),
    recent: Vec<String>, ctx: Option<(f32, f32)>, marks: bool, stats: bool, ribbon_scroll: f32, ribbon_w: f32, last_click: (Instant, (f32, f32), u8), last_autosave: Instant,
}
impl App {
    fn new() -> Self {
        let mut fc = FontCollection::new(); fc.set_default_font_manager(FontMgr::default(), None);
        App { ed: Editor::new(Doc::default()), path: None, dark: false, tab: "Home", ribbon: true, nav: true, focus: false, zoom: 1.0, scroll: 0.0, follow: false,
            palette: None, find: None, preedit: String::new(), mods: ModifiersState::empty(), mouse: (0., 0.), drag: false, toast: String::new(), fc, hot: vec![], lay: vec![],
            page_x: 0., view_h: 0., sf: 1.0, ime_pos: (0., 0.),
            recent: config::recent_load("omnidoc"), ctx: None, marks: false, stats: false, ribbon_scroll: 0., ribbon_w: 0., last_click: (Instant::now(), (0., 0.), 0), last_autosave: Instant::now() }
    }
    fn th(&self) -> &'static Theme { if self.dark { &DARK } else { &LIGHT } }
    fn ui_font(&self, size: f32, bold: bool) -> Font {
        let m = FontMgr::default(); let st = if bold { FontStyle::bold() } else { FontStyle::normal() };
        let tf = FAMS.iter().find_map(|f| m.match_family_style(f, st)).or_else(|| m.legacy_make_typeface(None, st)).expect("no system fonts");
        Font::new(tf, size)
    }
    fn build(&self, p: &Para, w: f32, pre: Option<usize>) -> Paragraph {
        let (size, bold) = match p.style { Style::Title => (30., true), Style::H1 => (24., true), Style::H2 => (19., true), Style::H3 => (16., true), Style::Code => (13., false), _ => (14.5, false) };
        let mut ps = ParagraphStyle::new();
        ps.set_text_align(match p.align { Align::Left => TextAlign::Left, Align::Center => TextAlign::Center, Align::Right => TextAlign::Right, Align::Justify => TextAlign::Justify });
        let mut b = ParagraphBuilder::new(&ps, &self.fc);
        let ink = self.th().ink;
        let mut put = |b: &mut ParagraphBuilder, r: &Run, ul: bool| {
            let mut t = TextStyle::new(); t.set_font_size(size * self.zoom * 1.333); t.set_color(col(ink));
            t.set_font_families(if p.style == Style::Code { MONO } else { FAMS });
            t.set_font_style(match (bold || r.bold, r.italic) { (true, true) => FontStyle::bold_italic(), (true, false) => FontStyle::bold(), (false, true) => FontStyle::italic(), _ => FontStyle::normal() });
            if r.underline || ul { t.set_decoration_type(TextDecoration::UNDERLINE); t.set_decoration_color(col(ink)); }
            b.push_style(&t); b.add_text(&r.text); b.pop();
        };
        let (a, rest) = match pre { Some(o) if !self.preedit.is_empty() => { let (a, r) = p.split(o); (a, Some(r)) } _ => (p.runs.clone(), None) };
        for r in &a { put(&mut b, r, false); }
        if let Some(rest) = rest { put(&mut b, &Run { text: self.preedit.clone(), ..self.ed.fmt_at(self.ed.cur) }, true); for r in &rest { put(&mut b, r, false); } }
        if p.is_empty() && self.preedit.is_empty() { put(&mut b, &Run::plain(" "), false); }
        let mut para = b.build(); para.layout(w); para
    }
    fn locate(&self, pos: (usize, usize)) -> (f32, f32, f32) {
        let (para, top) = &self.lay[pos.0]; let t = self.ed.doc.paras[pos.0].text();
        let mut u = u16of(&t, pos.1); if pos.0 == self.ed.cur.0 { u += u16len(&self.preedit); }
        if t.is_empty() && self.preedit.is_empty() { return (0., *top, para.height()); }
        let r = if u == 0 { para.get_rects_for_range(0..1, RectHeightStyle::Max, RectWidthStyle::Tight).first().map(|b| (b.rect.left, b.rect)) }
                else { para.get_rects_for_range(u - 1..u, RectHeightStyle::Max, RectWidthStyle::Tight).first().map(|b| (b.rect.right, b.rect)) };
        r.map_or((0., *top, para.height()), |(x, r)| (x, top + r.top, r.height()))
    }
    fn hit(&self, x: f32, y: f32) -> (usize, usize) {
        for (i, (p, top)) in self.lay.iter().enumerate() {
            if y < top + p.height() || i + 1 == self.lay.len() {
                let t = self.ed.doc.paras[i].text(); let u = p.get_glyph_position_at_coordinate((x, y - top)).position.max(0) as usize;
                return (i, charof(&t, u).min(t.chars().count()));
            }
        }
        (0, 0)
    }
    fn open(&mut self) {
        let exts: Vec<&str> = formats::FORMATS.iter().filter(|f| f.2).map(|f| f.0).collect();
        if let Some(p) = rfd::FileDialog::new().add_filter("Documents", &exts).pick_file() { self.open_path(p); }
    }
    fn open_path(&mut self, p: PathBuf) {
        let ext = p.extension().and_then(|e| e.to_str()).unwrap_or("").to_string();
        match std::fs::read(&p).map_err(|e| e.to_string()).and_then(|d| formats::load(&ext, &d)) {
            Ok(d) => { self.ed = Editor::new(d); self.add_recent(&p); self.path = Some(p); self.scroll = 0.; self.toast = "Opened".into(); }
            Err(e) => self.toast = e,
        }
    }
    fn add_recent(&mut self, p: &std::path::Path) { config::recent_push(&mut self.recent, &p.to_string_lossy(), 8); config::recent_save("omnidoc", &self.recent); }
    /// Write an autosave copy every 30 s while there are unsaved changes.
    fn autosave(&mut self) {
        if self.ed.dirty && self.last_autosave.elapsed() > Duration::from_secs(30) {
            let p = config::autosave_path("omnidoc", "omd"); if let Some(d) = p.parent() { let _ = std::fs::create_dir_all(d); }
            if let Ok(b) = formats::save("omd", &self.ed.doc) { let _ = std::fs::write(p, b); }
            self.last_autosave = Instant::now();
        }
    }
    fn save(&mut self, as_new: bool) {
        if self.path.is_none() || as_new {
            let exts: Vec<&str> = formats::FORMATS.iter().filter(|f| f.3).map(|f| f.0).collect();
            match rfd::FileDialog::new().add_filter("Documents", &exts).set_file_name("document.omd").save_file() { Some(p) => self.path = Some(p), None => return }
        }
        let p = self.path.clone().unwrap(); let ext = p.extension().and_then(|e| e.to_str()).unwrap_or("omd").to_string();
        match formats::save(&ext, &self.ed.doc).and_then(|d| std::fs::write(&p, d).map_err(|e| e.to_string())) {
            Ok(()) => { self.ed.dirty = false; self.add_recent(&p); let _ = std::fs::remove_file(config::autosave_path("omnidoc", "omd")); self.toast = "Saved".into() } Err(e) => self.toast = e,
        }
    }
    fn run(&mut self, id: &str) {
        if let Some(t) = id.strip_prefix("tab:") { self.tab = ["File", "Home", "Review", "View"].into_iter().find(|x| *x == t).unwrap_or("Home"); return; }
        if let Some(i) = id.strip_prefix("recent:").and_then(|x| x.parse::<usize>().ok()) { if let Some(p) = self.recent.get(i).cloned() { self.open_path(PathBuf::from(p)); } return; }
        match id {
            "file.new" => { self.ed = Editor::new(Doc::default()); self.path = None; }
            "file.open" => self.open(), "file.save" => self.save(false), "file.saveas" => self.save(true),
            "view.outline" => self.nav = !self.nav, "view.focus" => self.focus = !self.focus, "view.dark" => self.dark = !self.dark,
            "view.ribbon" => self.ribbon = !self.ribbon, "view.zoomin" => self.zoom = (self.zoom + 0.1).min(3.), "view.zoomout" => self.zoom = (self.zoom - 0.1).max(0.5),
            "find.open" => self.find = Some((self.ed.selected_text(), String::new(), false, false)),
            "find.regex" => match self.find.as_mut() { Some(f) => f.3 = !f.3, None => self.find = Some((String::new(), String::new(), false, true)) },
            "view.marks" => self.marks = !self.marks, "tools.stats" => self.stats = !self.stats, "palette.open" => self.palette = Some((String::new(), 0)),
            "edit.copy" | "edit.cut" => { let t = self.ed.selected_text(); if !t.is_empty() { if let Ok(mut c) = arboard::Clipboard::new() { let _ = c.set_text(t); } if id == "edit.cut" { self.ed.delete_selection(); } } }
            "edit.paste" => if let Ok(t) = arboard::Clipboard::new().and_then(|mut c| c.get_text()) { self.ed.insert(&t); },
            _ => { cmd::exec(&mut self.ed, id); }
        }
        self.follow = true;
    }
    fn key(&mut self, e: &KeyEvent) {
        if e.state != ElementState::Pressed { return; }
        let shift = self.mods.shift_key();
        if self.stats && matches!(e.logical_key, Key::Named(NamedKey::Escape)) { self.stats = false; return; }
        if let Some((q, sel)) = self.palette.as_mut() {
            match &e.logical_key {
                Key::Named(NamedKey::Escape) => self.palette = None,
                Key::Named(NamedKey::ArrowDown) => *sel += 1, Key::Named(NamedKey::ArrowUp) => *sel = sel.saturating_sub(1),
                Key::Named(NamedKey::Backspace) => { q.pop(); *sel = 0; }
                Key::Named(NamedKey::Enter) => { let id = cmd::search(q).get(*sel).map(|c| c.id); self.palette = None; if let Some(id) = id { self.run(id); } }
                _ => if let Some(t) = &e.text { if !self.mods.control_key() { q.push_str(t); *sel = 0; } },
            }
            return;
        }
        if let Some((q, r, f2, rx)) = self.find.as_mut() {
            let field = |q: &mut String, r: &mut String, f2: bool| if f2 { r as *mut String } else { q as *mut String };
            let rx = *rx;
            match &e.logical_key {
                Key::Named(NamedKey::Escape) => { self.find = None; return }
                Key::Named(NamedKey::Tab) => { *f2 = !*f2; return }
                Key::Named(NamedKey::Enter) => { let (qq, rr) = (q.clone(), r.clone());
                    if self.mods.control_key() { match if rx { self.ed.replace_all_regex(&qq, &rr, false) } else { Ok(self.ed.replace_all(&qq, &rr, false)) } { Ok(n) => self.toast = format!("Replaced {n}"), Err(e) => self.toast = e } }
                    else { match if rx { self.ed.find_next_regex(&qq, false) } else { Ok(self.ed.find_next(&qq, false)) } { Ok(true) => {} Ok(false) => self.toast = "Not found".into(), Err(e) => self.toast = e } }
                    self.follow = true; return }
                Key::Named(NamedKey::Backspace) => { unsafe { (*field(q, r, *f2)).pop(); } return }
                _ => if !self.mods.control_key() && !self.mods.alt_key() { if let Some(t) = &e.text { unsafe { (*field(q, r, *f2)).push_str(t); } return } },
            }
        }
        let name = match &e.logical_key { Key::Character(c) => c.to_uppercase(), Key::Named(NamedKey::F1) => "F1".into(), Key::Named(NamedKey::F9) => "F9".into(), Key::Named(NamedKey::F11) => "F11".into(), _ => String::new() };
        if !name.is_empty() {
            let mut s = String::new(); if self.mods.control_key() || self.mods.super_key() { s += "Ctrl+" } if self.mods.alt_key() { s += "Alt+" } if shift { s += "Shift+" } s += &name;
            if let Some(c) = cmd::COMMANDS.iter().find(|c| c.keys == s) { let id = c.id; self.run(id); return; }
            if self.mods.control_key() || self.mods.super_key() { return; }
        }
        let ed = &mut self.ed; self.follow = true;
        match &e.logical_key {
            Key::Named(NamedKey::Enter) => { if !ed.accept_completion() { ed.autocorrect(); ed.insert("\n"); } } Key::Named(NamedKey::Backspace) => ed.backspace(), Key::Named(NamedKey::Delete) => ed.delete(),
            Key::Named(NamedKey::Tab) => ed.insert("    "),
            Key::Named(NamedKey::ArrowLeft) => { let p = ed.left(); ed.mv(p, shift) } Key::Named(NamedKey::ArrowRight) => { let p = ed.right(); ed.mv(p, shift) }
            Key::Named(NamedKey::ArrowUp) | Key::Named(NamedKey::ArrowDown) if !self.lay.is_empty() => {
                let up = matches!(e.logical_key, Key::Named(NamedKey::ArrowUp)); let (x, y, h) = self.locate(self.ed.cur);
                let p = self.hit(x, if up { y - h * 0.5 } else { y + h * 1.5 }); self.ed.mv(p, shift);
            }
            Key::Named(NamedKey::Home) => { let c = ed.cur; ed.mv((c.0, 0), shift) } Key::Named(NamedKey::End) => { let c = ed.cur; let l = ed.doc.paras[c.0].len(); ed.mv((c.0, l), shift) }
            Key::Named(NamedKey::PageDown) => self.scroll += self.view_h * 0.9, Key::Named(NamedKey::PageUp) => self.scroll = (self.scroll - self.view_h * 0.9).max(0.),
            _ => if let Some(t) = &e.text { if !t.chars().any(char::is_control) && !self.mods.alt_key() { if t.as_str() == " " { ed.autocorrect(); } ed.insert(t); } },
        }
    }
    fn click(&mut self) {
        let m = self.mouse;
        if self.stats { self.stats = false; return; }
        if self.ctx.take().is_some() {
            if let Some(id) = self.hot.iter().rev().find(|(r, _)| inside(r, m)).and_then(|x| x.1.strip_prefix("ctx:").map(String::from)) { self.run(&id); }
            return;
        }
        if let Some(id) = self.hot.iter().rev().find(|(r, _)| inside(r, m)).map(|x| x.1.clone()) {
            if let Some(i) = id.strip_prefix("nav:").and_then(|x| x.parse().ok()) { self.ed.mv((i, 0), false); self.follow = true; } else { self.run(&id); }
            return;
        }
        if !self.lay.is_empty() && m.0 >= self.page_x { let p = self.page_hit(); self.ed.mv(p, false); self.ed.anchor = Some(p);
            let n = if self.last_click.0.elapsed() < Duration::from_millis(400) && (self.last_click.1 .0 - m.0).abs() < 5. && (self.last_click.1 .1 - m.1).abs() < 5. { self.last_click.2 % 3 + 1 } else { 1 };
            self.last_click = (Instant::now(), m, n);
            match n { 2 => self.ed.select_word(), 3 => self.ed.select_para(), _ => self.drag = true }
        }
    }
    fn right_click(&mut self) {
        if !self.lay.is_empty() && self.mouse.0 >= self.page_x && self.mouse.1 > self.body_top() && self.ed.range().is_none() { let p = self.page_hit(); self.ed.mv(p, false); }
        self.ctx = Some(self.mouse);
    }
    fn page_hit(&self) -> (usize, usize) { self.hit(self.mouse.0 - self.page_x - 72. * self.zoom, self.mouse.1 + self.scroll - self.body_top() - 40.) }
    fn body_top(&self) -> f32 { if self.focus { 0. } else { 36. + 28. + if self.ribbon { 84. } else { 0. } + if self.find.is_some() { 34. } else { 0. } } }
}

fn rrect(c: &skia_safe::Canvas, r: Rect, rad: f32, color: u32) { let mut p = Paint::default(); p.set_anti_alias(true); p.set_color(col(color)); c.draw_round_rect(r, rad, rad, &p); }
fn text(c: &skia_safe::Canvas, f: &Font, s: &str, x: f32, y: f32, color: u32) { let mut p = Paint::default(); p.set_anti_alias(true); p.set_color(col(color)); c.draw_str(s, (x, y), f, &p); }

impl App {
    fn draw(&mut self, c: &skia_safe::Canvas, w: f32, h: f32) {
        let t = self.th(); self.hot.clear();
        c.clear(col(t.bg));
        let (f, fb) = (self.ui_font(12.5, false), self.ui_font(12.5, true));
        let top = self.body_top(); let nav_w = if self.nav && !self.focus { 230. } else { 0. }; let status_h = if self.focus { 0. } else { 24. };
        // ---- document ----
        let page_w = (794. * self.zoom).min(w - nav_w - 24.).max(200.); self.page_x = nav_w + ((w - nav_w - page_w) / 2.).max(0.);
        let inner = page_w - 144. * self.zoom;
        let pre = self.ed.cur.0;
        self.lay.clear(); let mut y = 0.;
        for (i, p) in self.ed.doc.paras.iter().enumerate() { let pa = self.build(p, inner, (i == pre).then_some(self.ed.cur.1)); let hh = pa.height() + 8. * self.zoom; self.lay.push((pa, y)); y += hh; }
        let total = y + 80.; self.view_h = h - top - status_h;
        if self.follow { let (_, cy, ch) = self.locate(self.ed.cur); let cy = cy + 40.; if cy < self.scroll { self.scroll = cy - 20. } else if cy + ch > self.scroll + self.view_h { self.scroll = cy + ch - self.view_h + 40. } self.follow = false; }
        self.scroll = self.scroll.clamp(0., (total + 80. - self.view_h).max(0.));
        c.save(); c.clip_rect(Rect::new(nav_w, top, w, h - status_h), None, false);
        let page = Rect::new(self.page_x, top + 16. - self.scroll, self.page_x + page_w, top + 16. - self.scroll + total + 60.);
        rrect(c, page.with_outset((1., 1.)), 2., t.line); rrect(c, page, 2., t.paper);
        let (ox, oy) = (self.page_x + 72. * self.zoom, top - self.scroll + 40.);
        if let Some((a, b)) = self.ed.range() {
            for i in a.0..=b.0 {
                let tx = self.ed.doc.paras[i].text(); let (s, e) = (if i == a.0 { a.1 } else { 0 }, if i == b.0 { b.1 } else { tx.chars().count() });
                for bx in self.lay[i].0.get_rects_for_range(u16of(&tx, s)..u16of(&tx, e).max(u16of(&tx, s) + 1), RectHeightStyle::Max, RectWidthStyle::Tight) {
                    let r = bx.rect.with_offset((ox, oy + self.lay[i].1)); let mut p = Paint::default(); p.set_color(Color::from_argb(90, (t.accent >> 16) as u8, (t.accent >> 8) as u8, t.accent as u8)); c.draw_rect(r, &p);
                }
            }
        }
        for (p, ty) in &self.lay { p.paint(c, (ox, oy + ty)); }
        let (cx, cy, ch) = self.locate(self.ed.cur); let mut p = Paint::default(); p.set_color(col(t.ink)); c.draw_rect(Rect::new(ox + cx, oy + cy, ox + cx + 1.5, oy + cy + ch), &p);
        self.ime_pos = (ox + cx, oy + cy + ch);
        if self.marks { let mf = self.ui_font(12., false); for i in 0..self.lay.len() { let (x, y, h) = self.locate((i, self.ed.doc.paras[i].len())); text(c, &mf, "¶", ox + x + 4., oy + y + h * 0.75, 0x8A9099); } }
        if self.preedit.is_empty() { if let Some(g) = self.ed.completion() { let gf = self.ui_font(14.5 * 1.333 * self.zoom, false); text(c, &gf, &g, ox + cx + 1., oy + cy + ch * 0.78, 0x8A9099); } }
        c.restore();
        if self.focus { return; }
        // ---- navigation pane (headings) ----
        if nav_w > 0. {
            let r = Rect::new(0., top, nav_w, h - status_h); let mut p = Paint::default(); p.set_color(col(t.bar2)); c.draw_rect(r, &p);
            icon(c, "nav_pane", 10., top + 5., 20., t.ink); text(c, &fb, "Navigation  -  Headings", 36., top + 20., t.ink);
            for (k, (lvl, i, title)) in self.ed.outline().into_iter().enumerate().take(60) {
                let rr = Rect::new(4., top + 32. + k as f32 * 24., nav_w - 4., top + 54. + k as f32 * 24.);
                if inside(&rr, self.mouse) { rrect(c, rr, 4., t.bar); }
                let label: String = title.chars().take(26).collect(); text(c, &f, &label, 12. + lvl as f32 * 10., rr.bottom - 7., t.ink); self.hot.push((rr, format!("nav:{i}")));
            }
        }
        // ---- find bar ----
        if let Some((q, r, f2, rx)) = self.find.clone() {
            let by = 36. + 28. + if self.ribbon { 84. } else { 0. };
            let mut p = Paint::default(); p.set_color(col(t.bar)); c.draw_rect(Rect::new(0., by, w, by + 34.), &p);
            for (k, (lab, val)) in [("Find", q), ("Replace", r)].into_iter().enumerate() { let x = 12. + k as f32 * 280.; let rr = Rect::new(x + 58., by + 5., x + 270., by + 29.); text(c, &f, lab, x, by + 22., t.ink); rrect(c, rr, 4., if (k == 1) == f2 { t.accent } else { t.line }); rrect(c, rr.with_inset((1.5, 1.5)), 3., t.paper); text(c, &f, &val, rr.left + 6., by + 22., t.ink); }
            text(c, &f, &format!("[{}] Regex (Alt+R)   Enter: next   Ctrl+Enter: replace all   Tab: switch   Esc: close", if rx { "x" } else { " " }), 580., by + 22., t.ink);
        }
        // ---- title bar + search box ----
        let mut p = Paint::default(); p.set_color(col(t.accent)); c.draw_rect(Rect::new(0., 0., w, 36.), &p);
        let name = self.path.as_ref().and_then(|p| p.file_name()).and_then(|n| n.to_str()).unwrap_or("Untitled").to_string();
        text(c, &fb, &format!("OmniDoc  |  {}  {}", name, if self.ed.dirty { "- Unsaved" } else { "- Saved" }), 12., 23., 0xFFFFFF);
        let sb = Rect::new(w / 2. - 180., 6., w / 2. + 180., 30.); rrect(c, sb, 12., 0xFFFFFF); icon(c, "search", sb.left + 12., 9., 18., 0x555B66); text(c, &f, "Search commands  (Ctrl+Shift+P)", sb.left + 38., 23., 0x555B66); self.hot.push((sb, "palette.open".into()));
        // ---- tabs ----
        let mut p = Paint::default(); p.set_color(col(t.bar)); c.draw_rect(Rect::new(0., 36., w, 36. + 28. + if self.ribbon { 84. } else { 0. }), &p);
        let mut x = 10.;
        for tb in ["File", "Home", "Review", "View"] {
            let tw = f.measure_str(tb, None).0 + 24.; let r = Rect::new(x, 38., x + tw, 64.);
            text(c, if tb == self.tab { &fb } else { &f }, tb, x + 12., 56., t.ink);
            if tb == self.tab { let mut p = Paint::default(); p.set_color(col(t.accent)); c.draw_rect(Rect::new(x + 8., 62., x + tw - 8., 64.), &p); }
            self.hot.push((r, format!("tab:{tb}"))); x += tw;
        }
        // ---- ribbon ----
        if self.ribbon {
            let (mut x, mut group, mut gstart) = (10. - self.ribbon_scroll, "", 10. - self.ribbon_scroll); let ry = 68.;
            let mut end_group = |c: &skia_safe::Canvas, g: &str, x0: f32, x1: f32| { if !g.is_empty() { let w = f.measure_str(g, None).0; text(c, &f, g, x0 + (x1 - x0 - w) / 2., ry + 76., 0x707680); let mut p = Paint::default(); p.set_color(col(t.line)); c.draw_rect(Rect::new(x1 + 4., ry + 6., x1 + 5., ry + 72.), &p); } };
            let (fa, fi, fu) = { let c = self.ed.pend.clone().unwrap_or_else(|| self.ed.fmt_at(self.ed.cur)); (c.bold, c.italic, c.underline) };
            const ORDER: &[&str] = &["File", "Undo", "Clipboard", "Font", "Paragraph", "Styles", "Editing", "Case", "Proofing", "Find", "Show", "Zoom"];
            let mut cs: Vec<&cmd::Command> = cmd::COMMANDS.iter().filter(|c| c.tab == self.tab).collect();
            cs.sort_by_key(|c| ORDER.iter().position(|g| *g == c.group).unwrap_or(99));
            for cm in cs {
                if cm.group != group { end_group(c, group, gstart, x - 6.); group = cm.group; x += 10.; gstart = x; }
                let big = match cm.id { "style.h1" => 17., "style.h2" => 15., "style.title" => 18., _ => 12.5 };
                let bf = self.ui_font(big, matches!(cm.id, "fmt.bold" | "style.h1" | "style.h2" | "style.title"));
                let ic = cmd::icon(cm.id); let lf = self.ui_font(11.5, false);
                let bw = if ic.is_empty() { (bf.measure_str(cm.title, None).0 + 22.).max(46.) } else { (lf.measure_str(cm.title, None).0 + 16.).max(52.) }; let r = Rect::new(x, ry + 6., x + bw, ry + 62.);
                let on = (cm.id == "fmt.bold" && fa) || (cm.id == "fmt.italic" && fi) || (cm.id == "fmt.underline" && fu) || (cm.id == "view.dark" && self.dark) || (cm.id == "view.outline" && self.nav) || (cm.id == "fmt.painter" && self.ed.painter.is_some()) || (cm.id == "view.marks" && self.marks) || (cm.id == "tools.autocorrect" && self.ed.autocorrect_on) || (cm.id == "tools.complete" && self.ed.completion_on);
                if on || inside(&r, self.mouse) { rrect(c, r, 6., if on { t.line } else { t.bar2 }); }
                if ic.is_empty() {
                    text(c, &bf, cm.title, x + (bw - bf.measure_str(cm.title, None).0) / 2., ry + 30., t.ink);
                    if !cm.keys.is_empty() { let kf = self.ui_font(9.5, false); let kw = kf.measure_str(cm.keys, None).0; text(c, &kf, cm.keys, x + (bw - kw) / 2., ry + 52., 0x8A9099); }
                } else {
                    icon(c, ic, x + (bw - 26.) / 2., ry + 9., 26., if on { t.accent } else { t.ink });
                    text(c, &lf, cm.title, x + (bw - lf.measure_str(cm.title, None).0) / 2., ry + 52., t.ink);
                }
                self.hot.push((r, cm.id.into())); x += bw + 4.;
            }
            if self.tab == "File" {
                end_group(c, group, gstart, x - 6.); group = "Recent"; x += 10.; gstart = x;
                for (i, p) in self.recent.iter().take(6).enumerate() {
                    let nm = std::path::Path::new(p).file_name().and_then(|n| n.to_str()).unwrap_or(p); let nm: String = nm.chars().take(18).collect();
                    let bw = (f.measure_str(&nm, None).0 + 22.).max(46.); let r = Rect::new(x, ry + 6., x + bw, ry + 62.);
                    if inside(&r, self.mouse) { rrect(c, r, 6., t.bar2); } icon(c, "doc", x + (bw - 26.) / 2., ry + 9., 26., t.ink); text(c, &f, &nm, x + 11., ry + 52., t.ink); self.hot.push((r, format!("recent:{i}"))); x += bw + 4.;
                }
            }
            end_group(c, group, gstart, x); self.ribbon_w = x + self.ribbon_scroll;
        }
        // ---- status bar ----
        let mut p = Paint::default(); p.set_color(col(t.bar)); c.draw_rect(Rect::new(0., h - status_h, w, h), &p);
        let fmt = self.path.as_ref().and_then(|p| p.extension()).and_then(|e| e.to_str()).unwrap_or("omd").to_uppercase();
        text(c, &f, &format!("Words: {}    {}    {}", self.ed.doc.words(), fmt, self.toast), 10., h - 8., t.ink);
        for (lab, id, dx) in [("-", "view.zoomout", 110.), ("+", "view.zoomin", 24.)] { let r = Rect::new(w - dx - 40., h - 22., w - dx - 16., h - 2.); let _ = lab; icon(c, if id == "view.zoomin" { "zoom_in" } else { "zoom_out" }, r.left + 1., r.top + 1., 22., t.ink); self.hot.push((r, id.into())); }
        text(c, &f, &format!("{}%", (self.zoom * 100.) as i32), w - 90., h - 8., t.ink);
        // ---- context menu ----
        if let Some((mx, my)) = self.ctx {
            let items = cmd::context_menu(self.ed.range().is_some());
            let hgt: f32 = items.iter().map(|i| if *i == "-" { 9. } else { 26. }).sum::<f32>() + 8.; let (px, py) = (mx.min(w - 260.), my.min(h - hgt - 4.));
            rrect(c, Rect::new(px - 1., py - 1., px + 251., py + hgt + 1.), 8., t.line); rrect(c, Rect::new(px, py, px + 250., py + hgt), 7., t.bar2);
            let mut y = py + 4.;
            for id in items {
                if id == "-" { let mut p = Paint::default(); p.set_color(col(t.line)); c.draw_rect(Rect::new(px + 8., y + 4., px + 242., y + 5.), &p); y += 9.; continue; }
                let cm = cmd::COMMANDS.iter().find(|c| c.id == id); let rr = Rect::new(px + 4., y, px + 246., y + 26.);
                if inside(&rr, self.mouse) { rrect(c, rr, 5., t.accent); }
                let hi = inside(&rr, self.mouse); if let Some(cm) = cm { icon(c, cmd::icon(id), px + 12., y + 4., 18., if hi { 0xFFFFFF } else { t.ink }); text(c, &f, cm.title, px + 38., y + 18., if hi { 0xFFFFFF } else { t.ink }); text(c, &f, cm.keys, px + 150., y + 18., if hi { 0xFFFFFF } else { 0x8A9099 }); }
                self.hot.push((rr, format!("ctx:{id}"))); y += 26.;
            }
        }
        // ---- statistics dialog ----
        if self.stats {
            let s = self.ed.stats(); let (x0, y0) = ((w - 320.) / 2., 120.);
            rrect(c, Rect::new(x0, y0, x0 + 320., y0 + 214.), 10., t.bar2); text(c, &fb, "Document Statistics", x0 + 18., y0 + 30., t.ink);
            for (k, (l, v)) in [("Words", s.words), ("Characters", s.chars), ("Characters (no spaces)", s.chars_no_spaces), ("Sentences", s.sentences), ("Paragraphs", s.paras), ("Pages (estimate)", s.pages)].into_iter().enumerate() {
                text(c, &f, l, x0 + 18., y0 + 58. + k as f32 * 22., t.ink); text(c, &fb, &v.to_string(), x0 + 250., y0 + 58. + k as f32 * 22., t.ink);
            }
            text(c, &f, "Click or press Esc to close", x0 + 18., y0 + 200., 0x8A9099);
        }
        // ---- command palette ----
        if let Some((q, sel)) = self.palette.clone() {
            let res = cmd::search(&q); let n = res.len().min(9); let pw = 520.; let x0 = (w - pw) / 2.;
            let mut p = Paint::default(); p.set_color(Color::from_argb(80, 0, 0, 0)); c.draw_rect(Rect::new(0., 0., w, h), &p);
            rrect(c, Rect::new(x0, 70., x0 + pw, 70. + 44. + n as f32 * 30. + 8.), 10., t.bar2);
            text(c, &fb, &format!("> {q}"), x0 + 16., 98., t.ink);
            for (k, cm) in res.iter().take(9).enumerate() {
                let y0 = 118. + k as f32 * 30.; if k == sel.min(n.saturating_sub(1)) { rrect(c, Rect::new(x0 + 6., y0, x0 + pw - 6., y0 + 28.), 6., t.accent); }
                let hi = k == sel.min(n.saturating_sub(1)); icon(c, cmd::icon(cm.id), x0 + 16., y0 + 5., 18., if hi { 0xFFFFFF } else { t.ink }); text(c, &f, cm.title, x0 + 44., y0 + 19., if hi { 0xFFFFFF } else { t.ink }); text(c, &f, cm.keys, x0 + pw - 140., y0 + 19., if hi { 0xFFFFFF } else { 0x8A9099 });
            }
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let el = EventLoop::new()?;
    #[allow(deprecated)]
    let window = Arc::new(el.create_window(Window::default_attributes().with_title("OmniDoc").with_inner_size(LogicalSize::new(1280., 820.)))?);
    window.set_ime_allowed(true); // Korean/Japanese/Chinese input
    let ctx = softbuffer::Context::new(window.clone())?;
    let mut sb = softbuffer::Surface::new(&ctx, window.clone())?;
    let mut app = App::new();
    if let Some(p) = std::env::args().nth(1) {
        let ext = std::path::Path::new(&p).extension().and_then(|e| e.to_str()).unwrap_or("").to_string();
        match std::fs::read(&p).map_err(|e| e.to_string()).and_then(|d| formats::load(&ext, &d)) { Ok(d) => { app.ed = Editor::new(d); app.path = Some(PathBuf::from(p)); } Err(e) => app.toast = e }
    }
    if std::env::args().nth(1).is_none() {
        if let Ok(b) = std::fs::read(config::autosave_path("omnidoc", "omd")) { if let Ok(d) = formats::load("omd", &b) { app.ed = Editor::new(d); app.ed.dirty = true; app.toast = "Recovered unsaved work (autosave)".into(); } }
    }
    #[allow(deprecated)]
    el.run(move |event, elwt| {
        elwt.set_control_flow(ControlFlow::WaitUntil(Instant::now() + Duration::from_secs(30)));
        app.autosave();
        let Event::WindowEvent { event, .. } = event else { return };
        match event {
            WindowEvent::CloseRequested => elwt.exit(),
            WindowEvent::ModifiersChanged(m) => app.mods = m.state(),
            WindowEvent::KeyboardInput { event, .. } => { app.key(&event); window.request_redraw() }
            WindowEvent::Ime(Ime::Commit(s)) => { app.preedit.clear(); app.ed.insert(&s); app.follow = true; window.request_redraw() }
            WindowEvent::Ime(Ime::Preedit(s, _)) => { app.preedit = s; window.request_redraw() }
            WindowEvent::CursorMoved { position, .. } => {
                let sf = window.scale_factor(); app.mouse = ((position.x / sf) as f32, (position.y / sf) as f32);
                if app.drag && !app.lay.is_empty() { let p = app.page_hit(); app.ed.cur = p; }
                window.request_redraw()
            }
            WindowEvent::MouseInput { state, button: MouseButton::Left, .. } => { if state == ElementState::Pressed { app.click() } else { app.drag = false } window.request_redraw() }
            WindowEvent::MouseInput { state: ElementState::Pressed, button: MouseButton::Right, .. } => { app.right_click(); window.request_redraw() }
            WindowEvent::MouseWheel { delta, .. } if app.mouse.1 > 36. && app.mouse.1 < 36. + 28. + 84. && app.ribbon => {
                let dy = match delta { MouseScrollDelta::LineDelta(x, y) => (x + y) * 48., MouseScrollDelta::PixelDelta(p) => (p.x + p.y) as f32 };
                let max = (app.ribbon_w - window.inner_size().width as f32 / window.scale_factor() as f32 + 20.).max(0.); app.ribbon_scroll = (app.ribbon_scroll - dy).clamp(0., max); window.request_redraw()
            }
            WindowEvent::MouseWheel { delta, .. } => {
                app.scroll -= match delta { MouseScrollDelta::LineDelta(_, y) => y * 48., MouseScrollDelta::PixelDelta(p) => p.y as f32 / window.scale_factor() as f32 };
                window.request_redraw()
            }
            WindowEvent::Resized(_) | WindowEvent::ScaleFactorChanged { .. } => window.request_redraw(),
            WindowEvent::RedrawRequested => {
                let (sz, sf) = (window.inner_size(), window.scale_factor() as f32);
                let (Some(w), Some(h)) = (NonZeroU32::new(sz.width), NonZeroU32::new(sz.height)) else { return };
                let Some(mut surf) = surfaces::raster_n32_premul((w.get() as i32, h.get() as i32)) else { return };
                { let c = surf.canvas(); c.scale((sf, sf)); app.draw(c, sz.width as f32 / sf, sz.height as f32 / sf); }
                let info = surf.image_info(); let mut px = vec![0u8; (w.get() * h.get() * 4) as usize];
                if !surf.read_pixels(&info, &mut px, (w.get() * 4) as usize, (0, 0)) { return; }
                if sb.resize(w, h).is_err() { return; }
                if let Ok(mut buf) = sb.buffer_mut() { for (i, p) in px.chunks_exact(4).enumerate() { buf[i] = u32::from_le_bytes([p[0], p[1], p[2], p[3]]); } let _ = buf.present(); }
                window.set_ime_cursor_area(LogicalPosition::new(app.ime_pos.0, app.ime_pos.1), LogicalSize::new(8, 20));
                let name = app.path.as_ref().and_then(|p| p.file_name()).and_then(|n| n.to_str()).unwrap_or("Untitled");
                window.set_title(&format!("{}{} - OmniDoc {}", name, if app.ed.dirty { " *" } else { "" }, omnidoc_core::VERSION));
            }
            _ => {}
        }
    })?;
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn every_command_icon_exists() { for c in cmd::COMMANDS { let i = cmd::icon(c.id); assert!(i.is_empty() || omni_shell::has_icon(i), "{} -> {i}", c.id); } }
}
