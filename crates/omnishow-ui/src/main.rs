// SPDX-License-Identifier: GPL-3.0-only
//! OmniShow: presentation UI in the style of PowerPoint (ribbon, quick access toolbar, slide panel, notes, status bar).
use omni_common::config;
use omni_shell::ribbon::{self, Btn, Colors, Group, Tab};
use omni_shell::*;
use omnishow_core::{Deck, Layout, Shape, ShapeKind, Slide, Theme, Transition};
use std::{path::PathBuf, time::{Duration, Instant}};

const ORANGE: u32 = 0xC55A11;
struct Cmd { id: &'static str, label: &'static str, icon: &'static str, tab: &'static str, group: &'static str }
const fn c(id: &'static str, label: &'static str, icon: &'static str, tab: &'static str, group: &'static str) -> Cmd { Cmd { id, label, icon, tab, group } }
const TABS: [&str; 7] = ["File", "Home", "Insert", "Design", "Transitions", "Slide Show", "View"];
const SHAPE_TAB: &str = "Shape Format";
const CMDS: &[Cmd] = &[
    c("file.new", "New", "note_add", "File", "File"), c("file.open", "Open", "folder_open", "File", "File"), c("file.save", "Save", "save", "File", "File"),
    c("file.saveas", "Save As", "save_as", "File", "File"), c("file.import", "Import Outline", "import", "File", "Import"), c("file.exportpng", "Export PNG", "export_png", "File", "Export"),
    c("edit.undo", "Undo", "undo", "Home", "Undo"), c("edit.redo", "Redo", "redo", "Home", "Undo"),
    c("slide.new", "New Slide", "add_slide", "Home", "Slides"), c("slide.dup", "Duplicate", "content_copy", "Home", "Slides"), c("slide.del", "Delete", "delete", "Home", "Slides"),
    c("slide.up", "Move Up", "arrow_up", "Home", "Slides"), c("slide.down", "Move Down", "arrow_down", "Home", "Slides"),
    c("layout.title", "Title", "layout_title", "Home", "Layout"), c("layout.content", "Content", "layout_content", "Home", "Layout"),
    c("layout.section", "Section", "layout_section", "Home", "Layout"), c("layout.blank", "Blank", "layout_blank", "Home", "Layout"),
    c("shape.textbox", "Text Box", "textbox", "Insert", "Shapes"), c("shape.rect", "Rectangle", "rect", "Insert", "Shapes"),
    c("shape.ellipse", "Ellipse", "ellipse", "Insert", "Shapes"), c("shape.line", "Line", "line", "Insert", "Shapes"),
    c("shape.image", "Picture", "image", "Insert", "Illustrations"), c("insert.numbers", "Slide Number", "slide_no", "Insert", "Header & Footer"),
    c("sf.theme", "Theme", "fill", SHAPE_TAB, "Fill"), c("sf.red", "Red", "fill", SHAPE_TAB, "Fill"), c("sf.blue", "Blue", "fill", SHAPE_TAB, "Fill"), c("sf.green", "Green", "fill", SHAPE_TAB, "Fill"), c("sf.gold", "Gold", "fill", SHAPE_TAB, "Fill"), c("sf.gray", "Gray", "fill", SHAPE_TAB, "Fill"),
    c("sf.front", "Bring Forward", "front", SHAPE_TAB, "Arrange"), c("sf.back", "Send Backward", "back", SHAPE_TAB, "Arrange"), c("sf.dup", "Duplicate", "content_copy", SHAPE_TAB, "Arrange"), c("sf.del", "Delete", "delete", SHAPE_TAB, "Arrange"),
    c("sf.left", "Left", "align_l", SHAPE_TAB, "Align"), c("sf.center", "Center", "align_c", SHAPE_TAB, "Align"), c("sf.right", "Right", "align_r", SHAPE_TAB, "Align"),
    c("theme.office", "Office", "palette", "Design", "Themes"), c("theme.ocean", "Ocean", "palette", "Design", "Themes"), c("theme.forest", "Forest", "palette", "Design", "Themes"),
    c("theme.sunset", "Sunset", "palette", "Design", "Themes"), c("theme.mono", "Mono Dark", "palette", "Design", "Themes"),
    c("tr.none", "None", "none", "Transitions", "Transition to This Slide"), c("tr.fade", "Fade", "fade", "Transitions", "Transition to This Slide"),
    c("tr.push", "Push", "push", "Transitions", "Transition to This Slide"), c("tr.wipe", "Wipe", "wipe", "Transitions", "Transition to This Slide"), c("tr.all", "Apply To All", "copy_all", "Transitions", "Timing"),
    c("show.begin", "From Beginning", "present_all", "Slide Show", "Start Slide Show"), c("show.current", "From Current Slide", "from_current", "Slide Show", "Start Slide Show"), c("show.presenter", "Presenter View", "presenter", "Slide Show", "Start Slide Show"),
    c("view.normal", "Normal", "slides", "View", "Presentation Views"), c("view.sorter", "Slide Sorter", "grid", "View", "Presentation Views"),
    c("view.notes", "Notes", "notes", "View", "Show"), c("view.ribbon", "Collapse Ribbon", "collapse", "View", "Show"),
    c("zoom.in", "Zoom In", "zoom_in", "View", "Zoom"), c("zoom.out", "Zoom Out", "zoom_out", "View", "Zoom"), c("zoom.fit", "Fit", "fit", "View", "Zoom"),
];
const QAT_OPTIONS: [&str; 7] = ["file.new", "file.open", "file.save", "edit.undo", "edit.redo", "show.begin", "slide.new"];
fn cmd(id: &str) -> Option<&'static Cmd> { CMDS.iter().find(|c| c.id == id) }

#[derive(Clone, Copy, PartialEq)]
enum Field { Title, Bullet(usize), Notes, Shape(usize) }
#[derive(Clone, Copy, PartialEq)]
enum Mode { Normal, Sorter, Present }
struct Show {
    deck: Deck, path: Option<PathBuf>, cur: usize, focus: Field, mode: Mode, dirty: bool, toast: String, quit: bool, hover: (f32, f32), size: (f32, f32),
    hot: Vec<(Rect, String)>, fields: Vec<(Rect, Field)>, thumbs: Vec<Rect>, canvas: Rect, tab: &'static str, collapsed: bool, rscroll: f32, rwidth: f32, notes: bool, zoom: f32, tscroll: f32,
    undo: Vec<Deck>, redo: Vec<Deck>, typing: Option<(usize, Field)>, qat: Vec<String>, qat_menu: Option<f32>, ctx: Option<(f32, f32, Option<String>)>, palette: Option<(String, usize)>,
    sel: Option<usize>, drag: Option<(usize, f32, f32)>, anim: Option<(Instant, usize)>, last_click: (Instant, usize), presenter: bool, started: Instant, prev_sel: Option<usize>, drag_pending: bool,
}
impl Show {
    fn new() -> Self {
        let mut qat: Vec<String> = std::fs::read_to_string(config::config_dir().join("omnishow-qat.txt")).map(|s| s.lines().filter(|l| cmd(l).is_some()).map(String::from).collect()).unwrap_or_default();
        if qat.is_empty() { qat = ["file.save", "edit.undo", "edit.redo", "show.begin"].map(String::from).to_vec(); }
        Show { deck: Deck::default(), path: None, cur: 0, focus: Field::Title, mode: Mode::Normal, dirty: false, toast: String::new(), quit: false, hover: (0., 0.), size: (1180., 760.), hot: vec![], fields: vec![], thumbs: vec![],
            canvas: Rect::new(0., 0., 1., 1.), tab: "Home", collapsed: false, rscroll: 0., rwidth: 0., notes: true, zoom: 1.0, tscroll: 0., undo: vec![], redo: vec![], typing: None, qat, qat_menu: None, ctx: None, palette: None,
            sel: None, drag: None, anim: None, last_click: (Instant::now(), usize::MAX), presenter: false, started: Instant::now(), prev_sel: None, drag_pending: false }
    }
    fn snap(&mut self) { self.undo.push(self.deck.clone()); if self.undo.len() > 200 { self.undo.remove(0); } self.redo.clear(); self.dirty = true; self.typing = None; }
    fn name(&self) -> String { self.path.as_ref().and_then(|p| p.file_name()).and_then(|n| n.to_str()).unwrap_or("Untitled").to_string() }
    fn slide(&mut self) -> &mut Slide { let i = self.cur.min(self.deck.slides.len() - 1); &mut self.deck.slides[i] }
    fn field(&mut self) -> Option<&mut String> {
        let f = self.focus; let s = self.slide();
        match f { Field::Title => Some(&mut s.title), Field::Notes => Some(&mut s.notes),
            Field::Bullet(i) => { while s.bullets.len() <= i { s.bullets.push(String::new()); } Some(&mut s.bullets[i]) }
            Field::Shape(i) => s.shapes.get_mut(i).map(|x| &mut x.text) }
    }
    fn type_text(&mut self, t: &str) {
        if self.typing != Some((self.cur, self.focus)) { self.snap(); self.typing = Some((self.cur, self.focus)); }
        if let Some(f) = self.field() { f.push_str(t); } self.dirty = true;
    }
    fn open_file(&mut self) {
        let Some(p) = rfd::FileDialog::new().add_filter("OmniShow / Markdown", &["oms", "md"]).pick_file() else { return };
        let d = std::fs::read(&p).unwrap_or_default();
        match if p.extension().map_or(false, |e| e == "oms") { Deck::from_oms(&d) } else { Ok(Deck::from_markdown(&String::from_utf8_lossy(&d))) } {
            Ok(d) => { self.deck = d; self.path = Some(p); self.cur = 0; self.dirty = false; self.undo.clear(); self.redo.clear(); self.sel = None; self.toast = "Opened".into() } Err(e) => self.toast = e }
    }
    fn save(&mut self, as_new: bool) {
        if self.path.is_none() || as_new { self.path = rfd::FileDialog::new().add_filter("OmniShow", &["oms"]).add_filter("Markdown outline", &["md"]).set_file_name("deck.oms").save_file().or(self.path.take()); }
        let Some(p) = self.path.clone() else { return };
        let b = if p.extension().map_or(false, |e| e == "md") { self.deck.to_markdown().into_bytes() } else { self.deck.to_oms() };
        match std::fs::write(&p, b) { Ok(()) => { self.dirty = false; self.toast = "Saved".into(); let mut r = config::recent_load("omnishow"); config::recent_push(&mut r, &p.to_string_lossy(), 8); config::recent_save("omnishow", &r); } Err(e) => self.toast = e.to_string() }
    }
    fn import_outline(&mut self) {
        let Some(p) = rfd::FileDialog::new().pick_file() else { return };
        let ext = p.extension().and_then(|e| e.to_str()).unwrap_or("").to_string();
        match std::fs::read(&p).map_err(|e| e.to_string()).and_then(|d| omnidoc_core::formats::load(&ext, &d)) {
            Ok(doc) => { let o: Vec<(u8, String)> = omnidoc_core::edit::Editor::new(doc).outline().into_iter().map(|x| (x.0, x.2)).collect(); self.snap(); self.deck = Deck::from_outline(&o); self.cur = 0; self.toast = format!("{} slides from outline", self.deck.slides.len()); }
            Err(e) => self.toast = e }
    }
    fn go(&mut self, d: i32) {
        let n = (self.cur as i32 + d).clamp(0, self.deck.slides.len() as i32 - 1) as usize;
        if n != self.cur { self.anim = (self.deck.slides[n].transition != Transition::None).then(|| (Instant::now(), self.cur)); self.cur = n; }
    }
    fn add_shape(&mut self, k: ShapeKind) { self.snap(); let mut s = Shape::new(k); if k == ShapeKind::Line { s.h = 0.; s.w = 0.3; } let sl = self.slide(); sl.shapes.push(s); let i = sl.shapes.len() - 1; self.sel = Some(i); self.focus = Field::Shape(i); }
    fn run(&mut self, id: &str) {
        if let Some(t) = id.strip_prefix("tab:") { if let Some(t) = TABS.iter().chain([SHAPE_TAB].iter()).find(|x| **x == t) { self.tab = t; } return; }
        let n = self.deck.slides.len();
        match id {
            "file.new" => { self.deck = Deck::default(); self.path = None; self.cur = 0; self.dirty = false; self.undo.clear(); self.redo.clear(); }
            "file.open" => self.open_file(), "file.save" => self.save(false), "file.saveas" => self.save(true), "file.import" => self.import_outline(),
            "edit.undo" => if let Some(d) = self.undo.pop() { self.redo.push(std::mem::replace(&mut self.deck, d)); self.cur = self.cur.min(self.deck.slides.len() - 1); self.dirty = true; self.typing = None; },
            "edit.redo" => if let Some(d) = self.redo.pop() { self.undo.push(std::mem::replace(&mut self.deck, d)); self.cur = self.cur.min(self.deck.slides.len() - 1); self.dirty = true; self.typing = None; },
            "slide.new" => { self.snap(); self.deck.slides.insert(self.cur + 1, Slide::default()); self.cur += 1; self.focus = Field::Title; }
            "slide.dup" => { self.snap(); let s = self.deck.slides[self.cur].clone(); self.deck.slides.insert(self.cur + 1, s); self.cur += 1; }
            "slide.del" => if n > 1 { self.snap(); self.deck.slides.remove(self.cur); self.cur = self.cur.min(self.deck.slides.len() - 1); self.sel = None; },
            "slide.up" => if self.cur > 0 { self.snap(); self.cur = self.deck.move_slide(self.cur, true); }, "slide.down" => if self.cur + 1 < n { self.snap(); self.cur = self.deck.move_slide(self.cur, false); },
            "shape.textbox" => self.add_shape(ShapeKind::TextBox), "shape.rect" => self.add_shape(ShapeKind::Rect), "shape.ellipse" => self.add_shape(ShapeKind::Ellipse), "shape.line" => self.add_shape(ShapeKind::Line),
            "tr.all" => { self.snap(); let t = self.deck.slides[self.cur].transition; for s in &mut self.deck.slides { s.transition = t; } }
            "show.begin" => { self.cur = 0; self.anim = None; self.presenter = false; self.mode = Mode::Present; } "show.current" => { self.anim = None; self.presenter = false; self.mode = Mode::Present; }
            "show.presenter" => { self.anim = None; self.presenter = true; self.started = Instant::now(); self.mode = Mode::Present; }
            "file.exportpng" => self.export_png(), "shape.image" => self.add_picture(), "insert.numbers" => { self.snap(); self.deck.numbers = !self.deck.numbers; }
            "view.normal" => self.mode = Mode::Normal, "view.sorter" => self.mode = Mode::Sorter, "view.notes" => self.notes = !self.notes, "view.ribbon" => self.collapsed = !self.collapsed,
            "zoom.in" => self.zoom = (self.zoom + 0.1).min(1.0), "zoom.out" => self.zoom = (self.zoom - 0.1).max(0.4), "zoom.fit" => self.zoom = 1.0,
            "palette.open" => self.palette = Some((String::new(), 0)), "qat.menu" => self.qat_menu = Some(self.hover.0),
            _ => {
                if let Some(x) = id.strip_prefix("sf.") { self.shape_cmd(x); }
                else if let Some(l) = id.strip_prefix("layout.").and_then(Layout::parse) { self.snap(); self.slide().layout = l; }
                else if let Some(t) = id.strip_prefix("theme.").and_then(Theme::parse) { self.snap(); self.deck.theme = t; }
                else if let Some(t) = id.strip_prefix("tr.").and_then(Transition::parse) { self.snap(); self.slide().transition = t; }
            }
        }
    }
    fn add_picture(&mut self) {
        let Some(p) = rfd::FileDialog::new().add_filter("Pictures", &["png", "jpg", "jpeg", "gif"]).pick_file() else { return };
        match std::fs::read(&p).ok().and_then(Shape::picture) {
            Some(sh) => { self.snap(); let sl = self.slide(); sl.shapes.push(sh); let i = sl.shapes.len() - 1; self.sel = Some(i); self.focus = Field::Shape(i); }
            None => self.toast = "Unsupported image (PNG, JPEG and GIF are supported)".into(),
        }
    }
    fn export_png(&mut self) {
        let Some(dir) = rfd::FileDialog::new().pick_folder() else { return };
        let mut n = 0; for i in 0..self.deck.slides.len() { if let Some(b) = render_png(&self.deck, i, 1920) { if std::fs::write(dir.join(format!("slide-{:02}.png", i + 1)), b).is_ok() { n += 1; } } }
        self.toast = format!("Exported {n} slide images");
    }
    /// Commands of the contextual "Shape Format" tab, applied to the selected shape.
    fn shape_cmd(&mut self, x: &str) {
        let Some(i) = self.sel else { return }; if i >= self.slide().shapes.len() { return; } self.snap();
        match x {
            "theme" | "red" | "blue" | "green" | "gold" | "gray" => { self.slide().shapes[i].color = match x { "red" => Some(0xC0392B), "blue" => Some(0x1F6FB2), "green" => Some(0x2E7D32), "gold" => Some(0xE0A800), "gray" => Some(0x6B7280), _ => None }; }
            "front" | "back" => { let n = self.slide().restack(i, x == "front"); self.sel = Some(n); self.focus = Field::Shape(n); }
            "dup" => { let mut sh = self.slide().shapes[i].clone(); sh.x = (sh.x + 0.03).min(1. - sh.w); sh.y = (sh.y + 0.03).min(1. - sh.h); let sl = self.slide(); sl.shapes.push(sh); let n = sl.shapes.len() - 1; self.sel = Some(n); self.focus = Field::Shape(n); }
            "del" => { self.slide().shapes.remove(i); self.sel = None; self.focus = Field::Title; }
            "left" | "center" | "right" => { let sh = &mut self.slide().shapes[i]; sh.x = match x { "left" => 0.02, "center" => (1. - sh.w) / 2., _ => 1. - sh.w - 0.02 }; }
            _ => {}
        }
    }
    fn set_qat(&mut self, id: &str, on: bool) {
        self.qat.retain(|x| x != id); if on { self.qat.push(id.to_string()); }
        let d = config::config_dir(); let _ = std::fs::create_dir_all(&d); let _ = std::fs::write(d.join("omnishow-qat.txt"), self.qat.join("\n"));
    }
    fn enabled(&self, id: &str) -> bool {
        let n = self.deck.slides.len();
        match id { "edit.undo" => !self.undo.is_empty(), "edit.redo" => !self.redo.is_empty(), "slide.del" => n > 1, "slide.up" => self.cur > 0, "slide.down" => self.cur + 1 < n, _ => true }
    }
    fn is_on(&self, id: &str) -> bool {
        let s = &self.deck.slides[self.cur.min(self.deck.slides.len() - 1)];
        if let Some(t) = id.strip_prefix("theme.") { return Theme::parse(t) == Some(self.deck.theme); }
        if let Some(t) = id.strip_prefix("tr.") { return Transition::parse(t) == Some(s.transition) && id != "tr.all"; }
        if let Some(t) = id.strip_prefix("layout.") { return Layout::parse(t) == Some(s.layout); }
        match id { "view.normal" => self.mode == Mode::Normal, "view.sorter" => self.mode == Mode::Sorter, "view.notes" => self.notes, "view.ribbon" => self.collapsed, "insert.numbers" => self.deck.numbers, _ => false }
    }
    fn tabs(&self) -> Vec<Tab> {
        let mut names: Vec<&'static str> = TABS.to_vec(); if self.sel.is_some() { names.push(SHAPE_TAB); }
        names.iter().map(|t| { let mut groups: Vec<Group> = vec![];
            for cm in CMDS.iter().filter(|c| c.tab == *t) {
                if groups.last().map_or(true, |g| g.name != cm.group) { groups.push(Group { name: cm.group, btns: vec![] }); }
                groups.last_mut().unwrap().btns.push(Btn { id: cm.id.into(), label: cm.label.into(), icon: cm.icon, on: self.is_on(cm.id), enabled: self.enabled(cm.id) });
            }
            Tab { name: t, groups } }).collect()
    }
    // ---------- drawing ----------
    fn draw_slide(&self, c: &Canvas, s: &Slide, r: Rect, idx: usize, edit: bool) {
        paint_slide(c, &View { deck: &self.deck, idx, edit, focus: self.focus, sel: self.sel, border: edit || self.mode == Mode::Sorter }, s, r);
    }
    fn draw_present(&self, c: &Canvas, w: f32, h: f32) -> bool {
        fill(c, Rect::new(0., 0., w, h), 0x000000);
        let (sw, sh) = if w / h > 16. / 9. { (h * 16. / 9., h) } else { (w, w * 9. / 16.) };
        let r = Rect::new((w - sw) / 2., (h - sh) / 2., (w + sw) / 2., (h + sh) / 2.); let cur = &self.deck.slides[self.cur];
        let Some((t0, pi)) = self.anim else { self.draw_slide(c, cur, r, self.cur, false); return false };
        let t = (t0.elapsed().as_secs_f32() / 0.45).min(1.0); let pi = pi.min(self.deck.slides.len() - 1); let prev = &self.deck.slides[pi];
        match cur.transition {
            Transition::Fade => { self.draw_slide(c, prev, r, pi, false); c.save_layer_alpha_f(r, t); self.draw_slide(c, cur, r, self.cur, false); c.restore(); }
            Transition::Push => { for (s, i, dx) in [(prev, pi, -t * sw), (cur, self.cur, sw - t * sw)] { c.save(); c.clip_rect(r, None, false); c.translate((dx, 0.)); self.draw_slide(c, s, r, i, false); c.restore(); } }
            Transition::Wipe => { self.draw_slide(c, prev, r, pi, false); c.save(); c.clip_rect(Rect::new(r.left, r.top, r.left + t * sw, r.bottom), None, false); self.draw_slide(c, cur, r, self.cur, false); c.restore(); }
            Transition::None => self.draw_slide(c, cur, r, self.cur, false),
        }
        t < 1.0
    }
    /// Presenter view: current slide, next slide, notes and a timer.
    fn draw_presenter(&self, c: &Canvas, w: f32, h: f32) {
        fill(c, Rect::new(0., 0., w, h), 0x1B1C1F); let n = self.deck.slides.len(); let lw = w * 0.62 - 30.;
        let cr = Rect::new(20., 40., 20. + lw, 40. + lw * 9. / 16.); self.draw_slide(c, &self.deck.slides[self.cur], cr, self.cur, false);
        text(c, &format!("Slide {} of {}", self.cur + 1, n), 20., 28., 14., true, 0xFFFFFF);
        let el = self.started.elapsed().as_secs(); text(c, &format!("{:02}:{:02}:{:02}", el / 3600, el / 60 % 60, el % 60), w - 150., 36., 24., true, 0xFFB400);
        let rx = w * 0.62 + 10.; let rw = w - rx - 20.; text(c, "Next", rx, 28., 14., true, 0xAAAAAA);
        if self.cur + 1 < n { let nr = Rect::new(rx, 40., rx + rw, 40. + rw * 9. / 16.); self.draw_slide(c, &self.deck.slides[self.cur + 1], nr, self.cur + 1, false); } else { text(c, "End of presentation", rx, 70., 16., false, 0xCCCCCC); }
        let ny = 60. + rw * 9. / 16.; text(c, "Notes", rx, ny + 10., 14., true, 0xAAAAAA);
        for (i, l) in self.deck.slides[self.cur].notes.lines().take(12).enumerate() { text(c, l, rx, ny + 36. + i as f32 * 24., 16., false, 0xEEEEEE); }
        text(c, "Right/Space: next   Left: previous   Esc: exit", 20., h - 16., 12., false, 0x888888);
    }
}
struct View<'a> { deck: &'a Deck, idx: usize, edit: bool, focus: Field, sel: Option<usize>, border: bool }
/// Paint one slide into `r` (used for the editor, thumbnails, slideshow and PNG export).
fn paint_slide(c: &Canvas, v: &View, s: &Slide, r: Rect) {
    let tc = v.deck.theme.colors(); let k = r.width() / 960.; let (w, h) = (r.width(), r.height());
    let caret = |f: Field| if v.edit && v.focus == f { "|" } else { "" };
    fill(c, r, if s.layout == Layout::Section { tc.accent } else { tc.bg }); if v.border { stroke(c, r, 0xC9CED6, 1.); }
    c.save(); c.clip_rect(r, None, false);
    match s.layout {
        Layout::Content => {
            fill(c, Rect::new(r.left, r.top, r.left + 14. * k, r.bottom), tc.accent);
            text(c, &format!("{}{}", s.title, caret(Field::Title)), r.left + 60. * k, r.top + 110. * k, 44. * k, true, tc.title);
            for (i, b) in s.bullets.iter().enumerate() { text(c, &format!("\u{2022}  {}{}", b, caret(Field::Bullet(i))), r.left + 70. * k, r.top + (210. + i as f32 * 62.) * k, 30. * k, false, tc.text); }
        }
        Layout::Title => {
            let t = format!("{}{}", s.title, caret(Field::Title)); text(c, &t, r.left + (w - measure(&t, 56. * k, true)) / 2., r.top + h * 0.48, 56. * k, true, tc.title);
            fill(c, Rect::new(r.left + w * 0.35, r.top + h * 0.53, r.left + w * 0.65, r.top + h * 0.53 + 4. * k), tc.accent);
            for (i, b) in s.bullets.iter().enumerate() { let t = format!("{}{}", b, caret(Field::Bullet(i))); text(c, &t, r.left + (w - measure(&t, 28. * k, false)) / 2., r.top + h * 0.53 + (50. + i as f32 * 44.) * k, 28. * k, false, tc.text); }
        }
        Layout::Section => {
            text(c, &format!("{}{}", s.title, caret(Field::Title)), r.left + 70. * k, r.top + h * 0.55, 60. * k, true, 0xFFFFFF);
            for (i, b) in s.bullets.iter().enumerate() { text(c, &format!("{}{}", b, caret(Field::Bullet(i))), r.left + 74. * k, r.top + h * 0.55 + (50. + i as f32 * 40.) * k, 26. * k, false, 0xFFE9D6); }
        }
        Layout::Blank => {}
    }
    for (i, sh) in s.shapes.iter().enumerate() {
        let sr = Rect::new(r.left + sh.x * w, r.top + sh.y * h, r.left + (sh.x + sh.w) * w, r.top + (sh.y + sh.h) * h);
        let fillc = sh.color.unwrap_or(tc.accent); let on_fill = if sh.color.is_none() && v.deck.theme == Theme::Mono { 0x1E1F22 } else { 0xFFFFFF };
        let t = format!("{}{}", sh.text, caret(Field::Shape(i)));
        match sh.kind {
            ShapeKind::Rect => { rrect(c, sr, 6. * k, fillc); text(c, &t, sr.left + (sr.width() - measure(&t, 24. * k, false)) / 2., sr.top + sr.height() / 2. + 8. * k, 24. * k, false, on_fill); }
            ShapeKind::Ellipse => { oval(c, sr, fillc); text(c, &t, sr.left + (sr.width() - measure(&t, 24. * k, false)) / 2., sr.top + sr.height() / 2. + 8. * k, 24. * k, false, on_fill); }
            ShapeKind::TextBox => text(c, &t, sr.left + 8. * k, sr.top + 30. * k, 28. * k, false, sh.color.unwrap_or(tc.text)),
            ShapeKind::Line => line(c, (sr.left, sr.top), (sr.right, sr.bottom), fillc, (4. * k).max(1.5)),
            ShapeKind::Image => if let Some(d) = &sh.image { draw_image(c, d, sr); } else { stroke(c, sr, 0x999999, 1.); },
        }
        if v.edit && v.sel == Some(i) { stroke(c, sr.with_outset((3., 3.)), 0x2B7FFF, 2.); for (hx, hy) in [(sr.left, sr.top), (sr.right, sr.top), (sr.left, sr.bottom), (sr.right, sr.bottom)] { fill(c, Rect::new(hx - 4., hy - 4., hx + 4., hy + 4.), 0x2B7FFF); } }
    }
    if v.deck.numbers { let n = (v.idx + 1).to_string(); text(c, &n, r.right - 40. * k - measure(&n, 20. * k, false), r.bottom - 24. * k, 20. * k, false, if s.layout == Layout::Section { 0xFFFFFF } else { tc.text }); }
    c.restore();
}
/// Render slide `idx` of the deck as a PNG of the given width (16:9).
fn render_png(deck: &Deck, idx: usize, width: i32) -> Option<Vec<u8>> {
    let (w, h) = (width, width * 9 / 16); let mut surf = skia_safe::surfaces::raster_n32_premul((w, h))?;
    paint_slide(surf.canvas(), &View { deck, idx, edit: false, focus: Field::Title, sel: None, border: false }, deck.slides.get(idx)?, Rect::new(0., 0., w as f32, h as f32));
    surf.image_snapshot().encode(None, skia_safe::EncodedImageFormat::PNG, None).map(|d| d.as_bytes().to_vec())
}
impl App for Show {
    fn title(&self) -> String { format!("{}{} - OmniShow {}", self.name(), if self.dirty { " *" } else { "" }, omni_common::BUILD_ID) }
    fn wants_quit(&self) -> bool { self.quit }
    fn tick(&mut self) -> bool { self.mode == Mode::Present && self.presenter }
    fn tick_interval(&self) -> Duration { if self.mode == Mode::Present && self.presenter { Duration::from_secs(1) } else { Duration::from_secs(30) } }
    fn animating(&self) -> bool { self.mode == Mode::Present && self.anim.map_or(false, |a| a.0.elapsed() < Duration::from_millis(450)) }
    fn draw(&mut self, c: &Canvas, w: f32, h: f32) {
        self.size = (w, h); self.hot.clear(); self.fields.clear(); self.thumbs.clear();
        if self.sel.is_none() && self.tab == SHAPE_TAB { self.tab = "Home"; }
        if self.sel.is_some() && self.prev_sel.is_none() { self.tab = SHAPE_TAB; } self.prev_sel = self.sel;
        if self.mode == Mode::Present { if self.presenter { self.draw_presenter(c, w, h); } else if !self.draw_present(c, w, h) { self.anim = None; } return; }
        fill(c, Rect::new(0., 0., w, h), 0xE9ECF1);
        // title bar: logo, quick access toolbar, file name, search
        fill(c, Rect::new(0., 0., w, 36.), ORANGE); text(c, "OmniShow", 12., 23., 13., true, 0xFFFFFF);
        let mut x = 100.;
        for id in self.qat.clone() { let Some(cm) = cmd(&id) else { continue }; let r = Rect::new(x, 5., x + 28., 31.); let en = self.enabled(&id);
            if en && inside(&r, self.hover) { rrect(c, r, 6., 0xD9702E); } icon(c, cm.icon, x + 3., 7., 22., if en { 0xFFFFFF } else { 0xE8B590 }); if en { self.hot.push((r, id)); } x += 30.; }
        let dd = Rect::new(x, 5., x + 22., 31.); icon(c, "dropdown", x, 8., 22., 0xFFFFFF); self.hot.push((dd, "qat.menu".into())); x += 34.;
        text(c, &format!("{}  \u{2022}  {}", self.name(), if self.dirty { "Unsaved" } else { "Saved" }), x, 23., 12., false, 0xFFFFFF);
        let sb = Rect::new(w - 360., 6., w - 16., 30.); rrect(c, sb, 12., 0xFFFFFF); icon(c, "search", sb.left + 10., 9., 18., 0x666666); text(c, "Search commands", sb.left + 36., 23., 12.5, false, 0x666666); self.hot.push((sb, "palette.open".into()));
        // ribbon
        let tabs = self.tabs(); let col = Colors { accent: ORANGE, bar: 0xF6F7F9, bar2: 0xFFFFFF, line: 0xE0E3E8, ink: 0x222222 };
        let (top, cw) = ribbon::draw(c, w, 36., &tabs, self.tab, self.collapsed, self.rscroll, self.hover, &col, &mut self.hot); self.rwidth = cw;
        let status_h = 24.; let notes_h = if self.notes && self.mode == Mode::Normal { 110. } else { 0. };
        // body
        match self.mode {
            Mode::Normal => {
                fill(c, Rect::new(0., top, 210., h - status_h), 0xF6F7F9);
                c.save(); c.clip_rect(Rect::new(0., top, 210., h - status_h), None, false);
                for (i, s) in self.deck.slides.clone().iter().enumerate() {
                    let y = top + 12. - self.tscroll + i as f32 * 118.; let r = Rect::new(26., y, 196., y + 95.6);
                    if y > h || y + 100. < top { self.thumbs.push(Rect::new(0., 0., 0., 0.)); continue; }
                    self.draw_slide(c, s, r, i, false); text(c, &(i + 1).to_string(), 6., y + 14., 12., true, 0x555555);
                    if i == self.cur { stroke(c, r.with_outset((3., 3.)), ORANGE, 3.); } self.thumbs.push(r);
                }
                c.restore();
                let area = Rect::new(230., top + 14., w - 20., h - status_h - notes_h - 14.);
                let sw = (area.width().min(area.height() * 16. / 9.) * self.zoom).max(80.); let sh = sw * 9. / 16.;
                self.canvas = Rect::new(area.left + (area.width() - sw) / 2., area.top + (area.height() - sh) / 2., area.left + (area.width() + sw) / 2., area.top + (area.height() + sh) / 2.);
                let s = self.deck.slides[self.cur].clone(); self.draw_slide(c, &s, self.canvas, self.cur, true);
                let (r, k) = (self.canvas, self.canvas.width() / 960.);
                self.fields.push((Rect::new(r.left, r.top + 50. * k, r.right, r.top + 130. * k), Field::Title));
                for i in 0..=s.bullets.len() { self.fields.push((Rect::new(r.left, r.top + (170. + i as f32 * 62.) * k, r.right, r.top + (225. + i as f32 * 62.) * k), Field::Bullet(i))); }
                if notes_h > 0. {
                    let nr = Rect::new(230., h - status_h - notes_h + 4., w - 20., h - status_h - 6.); fill(c, nr, 0xFFFFFF); stroke(c, nr, if self.focus == Field::Notes { ORANGE } else { 0xC9CED6 }, if self.focus == Field::Notes { 2. } else { 1. });
                    text(c, "Click to add notes", nr.left + 10., nr.top + 18., 11., true, 0x999999);
                    for (i, l) in format!("{}{}", s.notes, if self.focus == Field::Notes { "|" } else { "" }).lines().enumerate().take(4) { text(c, l, nr.left + 10., nr.top + 40. + i as f32 * 20., 13., false, 0x222222); }
                    self.fields.push((nr, Field::Notes));
                }
            }
            _ => { // slide sorter
                let cols = (((w - 40.) / 230.) as usize).max(1);
                for (i, s) in self.deck.slides.clone().iter().enumerate() {
                    let (cx, cy) = ((i % cols) as f32, (i / cols) as f32); let r = Rect::new(30. + cx * 230., top + 24. + cy * 150. - self.tscroll, 30. + cx * 230. + 200., top + 24. + cy * 150. - self.tscroll + 112.5);
                    self.draw_slide(c, s, r, i, false); text(c, &(i + 1).to_string(), r.left, r.bottom + 16., 12., true, 0x555555); if i == self.cur { stroke(c, r.with_outset((3., 3.)), ORANGE, 3.); } self.thumbs.push(r);
                }
            }
        }
        // status bar (slide count, notes, views, zoom)
        fill(c, Rect::new(0., h - status_h, w, h), 0xF6F7F9);
        text(c, &format!("Slide {} of {}", self.cur + 1, self.deck.slides.len()), 12., h - 8., 12., false, 0x333333); text(c, &self.toast, 150., h - 8., 12., false, 0x777777);
        let mut x = w - 480.;
        for (id, ic, lab) in [("view.notes", "notes", "Notes"), ("view.normal", "slides", ""), ("view.sorter", "grid", ""), ("show.current", "present_all", "")] {
            let bw = if lab.is_empty() { 26. } else { 70. }; let r = Rect::new(x, h - 23., x + bw, h - 1.); if self.is_on(id) { rrect(c, r, 4., 0xDDE1E8); }
            icon(c, ic, x + 3., h - 21., 18., 0x333333); if !lab.is_empty() { text(c, lab, x + 25., h - 8., 12., false, 0x333333); } self.hot.push((r, id.into())); x += bw + 4.;
        }
        x += 14.; icon(c, "zoom_out", x, h - 21., 18., 0x333333); self.hot.push((Rect::new(x, h - 23., x + 20., h - 1.), "zoom.out".into()));
        let track = Rect::new(x + 26., h - 14., x + 146., h - 10.); fill(c, track, 0xB5BAC2); let kx = track.left + (self.zoom - 0.4) / 0.6 * track.width(); rrect(c, Rect::new(kx - 4., h - 20., kx + 4., h - 4.), 3., ORANGE);
        self.hot.push((Rect::new(track.left - 4., h - 22., track.right + 4., h - 2.), "zoom.track".into()));
        icon(c, "zoom_in", x + 152., h - 21., 18., 0x333333); self.hot.push((Rect::new(x + 152., h - 23., x + 172., h - 1.), "zoom.in".into())); text(c, &format!("{}%", (self.zoom * 100.) as i32), x + 176., h - 8., 12., false, 0x333333);
        // popups: quick access toolbar menu, ribbon context menu, command search
        if let Some(px) = self.qat_menu {
            let px = px.min(w - 270.); let hh = 40. + QAT_OPTIONS.len() as f32 * 26.; rrect(c, Rect::new(px - 1., 35., px + 261., 36. + hh), 8., 0xD0D3D9); rrect(c, Rect::new(px, 36., px + 260., 36. + hh - 1.), 7., 0xFFFFFF);
            text(c, "Customize Quick Access Toolbar", px + 12., 58., 12.5, true, 0x222222);
            for (i, id) in QAT_OPTIONS.iter().enumerate() { let y = 68. + i as f32 * 26.; let r = Rect::new(px + 4., y, px + 256., y + 26.); if inside(&r, self.hover) { rrect(c, r, 5., 0xEEF0F3); }
                if self.qat.iter().any(|q| q == id) { icon(c, "check", px + 10., y + 4., 18., 0x2E8B57); } text(c, cmd(id).map_or("", |c| c.label), px + 38., y + 18., 12.5, false, 0x222222); self.hot.push((r, format!("qat:{id}"))); }
        }
        if let Some((mx, my, id)) = self.ctx.clone() {
            let mut items: Vec<(String, String)> = vec![]; if let Some(id) = id.filter(|i| !self.qat.contains(i)) { items.push((format!("ctx:qat:{id}"), "Add to Quick Access Toolbar".into())); }
            items.push(("ctx:collapse".into(), if self.collapsed { "Expand the Ribbon" } else { "Collapse the Ribbon" }.into()));
            let (px, py) = (mx.min(w - 250.), my.min(h - 100.)); let hh = items.len() as f32 * 28. + 8.; rrect(c, Rect::new(px - 1., py - 1., px + 231., py + hh + 1.), 8., 0xD0D3D9); rrect(c, Rect::new(px, py, px + 230., py + hh), 7., 0xFFFFFF);
            for (i, (id, lab)) in items.into_iter().enumerate() { let y = py + 4. + i as f32 * 28.; let r = Rect::new(px + 4., y, px + 226., y + 28.); if inside(&r, self.hover) { rrect(c, r, 5., 0xEEF0F3); } text(c, &lab, px + 14., y + 19., 12.5, false, 0x222222); self.hot.push((r, id)); }
        }
        if let Some((q, sel)) = self.palette.clone() {
            let ql = q.to_lowercase(); let res: Vec<&Cmd> = CMDS.iter().filter(|c| c.label.to_lowercase().contains(&ql)).take(8).collect(); let px = (w - 460.) / 2.;
            tint(c, Rect::new(0., 0., w, h), 0x000000, 90); rrect(c, Rect::new(px, 60., px + 460., 112. + res.len() as f32 * 30.), 10., 0xFFFFFF); text(c, &format!("> {q}"), px + 16., 92., 14., true, 0x222222);
            for (i, cm) in res.iter().enumerate() { let y = 106. + i as f32 * 30.; let hi = i == sel.min(res.len().saturating_sub(1)); if hi { rrect(c, Rect::new(px + 6., y, px + 454., y + 28.), 6., ORANGE); }
                icon(c, cm.icon, px + 14., y + 4., 20., if hi { 0xFFFFFF } else { 0x333333 }); text(c, cm.label, px + 44., y + 19., 13., false, if hi { 0xFFFFFF } else { 0x222222 }); text(c, cm.tab, px + 360., y + 19., 12., false, if hi { 0xFFFFFF } else { 0x999999 }); }
        }
    }
    fn key(&mut self, e: &KeyEvent, m: ModifiersState) {
        self.toast.clear();
        let esc = matches!(e.logical_key, Key::Named(NamedKey::Escape));
        if self.mode == Mode::Present {
            match &e.logical_key { Key::Named(NamedKey::Escape) => self.mode = Mode::Normal, Key::Named(NamedKey::ArrowRight) | Key::Named(NamedKey::ArrowDown) | Key::Named(NamedKey::PageDown) | Key::Named(NamedKey::Space) | Key::Named(NamedKey::Enter) => self.go(1),
                Key::Named(NamedKey::ArrowLeft) | Key::Named(NamedKey::ArrowUp) | Key::Named(NamedKey::PageUp) | Key::Named(NamedKey::Backspace) => self.go(-1), _ => {} }
            return;
        }
        if esc { self.qat_menu = None; self.ctx = None; self.palette = None; }
        if let Some((q, sel)) = self.palette.as_mut() {
            match &e.logical_key { Key::Named(NamedKey::ArrowDown) => *sel += 1, Key::Named(NamedKey::ArrowUp) => *sel = sel.saturating_sub(1), Key::Named(NamedKey::Backspace) => { q.pop(); *sel = 0; }
                Key::Named(NamedKey::Enter) => { let ql = q.to_lowercase(); let id = CMDS.iter().filter(|c| c.label.to_lowercase().contains(&ql)).nth(*sel).map(|c| c.id); self.palette = None; if let Some(id) = id { self.run(id); } }
                _ => if let Some(t) = &e.text { if !m.control_key() { q.push_str(t); *sel = 0; } } }
            return;
        }
        if m.control_key() || m.super_key() {
            match &e.logical_key {
                Key::Character(k) => match (k.to_lowercase().as_str(), m.shift_key()) {
                    ("s", _) => self.save(false), ("o", _) => self.open_file(), ("i", _) => self.import_outline(), ("m", _) => self.run("slide.new"), ("d", _) => self.run("slide.dup"), ("z", _) => self.run("edit.undo"), ("y", _) => self.run("edit.redo"),
                    ("p", true) => self.run("palette.open"), _ => {} },
                Key::Named(NamedKey::Delete) => self.run("slide.del"), Key::Named(NamedKey::F1) => self.run("view.ribbon"), _ => {} }
            return;
        }
        let nb = self.deck.slides[self.cur].bullets.len();
        match &e.logical_key {
            Key::Named(NamedKey::F5) => self.run(if m.shift_key() { "show.current" } else { "show.begin" }),
            Key::Named(NamedKey::PageDown) => { self.cur = (self.cur + 1).min(self.deck.slides.len() - 1); self.sel = None; } Key::Named(NamedKey::PageUp) => { self.cur = self.cur.saturating_sub(1); self.sel = None; }
            Key::Named(NamedKey::ArrowDown) => self.focus = match self.focus { Field::Title if nb > 0 => Field::Bullet(0), Field::Bullet(i) if i + 1 < nb => Field::Bullet(i + 1), f => f },
            Key::Named(NamedKey::ArrowUp) => self.focus = match self.focus { Field::Bullet(0) => Field::Title, Field::Bullet(i) => Field::Bullet(i - 1), f => f },
            Key::Named(NamedKey::Delete) if matches!(self.focus, Field::Shape(_)) => if let Field::Shape(i) = self.focus { self.snap(); let s = self.slide(); if i < s.shapes.len() { s.shapes.remove(i); } self.sel = None; self.focus = Field::Title; },
            Key::Named(NamedKey::Enter) => match self.focus {
                Field::Title => { if nb == 0 { self.snap(); self.slide().bullets.push(String::new()); } self.focus = Field::Bullet(0); }
                Field::Bullet(i) => { self.snap(); self.slide().bullets.insert(i + 1, String::new()); self.focus = Field::Bullet(i + 1); }
                Field::Notes => self.type_text("\n"), Field::Shape(_) => {} },
            Key::Named(NamedKey::Backspace) => {
                if let Field::Bullet(i) = self.focus { if self.deck.slides[self.cur].bullets.get(i).map_or(false, |b| b.is_empty()) { self.snap(); if i < nb { self.slide().bullets.remove(i); } self.focus = if i == 0 { Field::Title } else { Field::Bullet(i - 1) }; return; } }
                if self.typing != Some((self.cur, self.focus)) { self.snap(); self.typing = Some((self.cur, self.focus)); } if let Some(f) = self.field() { f.pop(); }
            }
            _ => if let Some(t) = &e.text { if !t.chars().any(char::is_control) { self.type_text(t); } },
        }
    }
    fn text(&mut self, s: &str) { if self.mode != Mode::Present { if let Some((q, _)) = self.palette.as_mut() { q.push_str(s); } else { self.type_text(s); } } }
    fn moved(&mut self, x: f32, y: f32) {
        self.hover = (x, y);
        if self.drag.is_some() && self.drag_pending { self.snap(); self.drag_pending = false; }
        if let Some((i, ox, oy)) = self.drag { let r = self.canvas; let (fx, fy) = ((x - r.left) / r.width(), (y - r.top) / r.height());
            if let Some(sh) = self.slide().shapes.get_mut(i) { sh.x = (fx - ox).clamp(0., 1. - sh.w); sh.y = (fy - oy).clamp(0., 1. - sh.h); self.dirty = true; } }
    }
    fn mouse_up(&mut self) { self.drag = None; self.drag_pending = false; }
    fn wheel(&mut self, dy: f32) {
        if self.hover.1 < 36. + ribbon::TAB_H + ribbon::BODY_H && self.hover.1 > 36. { let max = (self.rwidth - self.size.0 + 20.).max(0.); self.rscroll = (self.rscroll - dy * 30.).clamp(0., max); }
        else { self.tscroll = (self.tscroll - dy * 30.).max(0.); }
    }
    fn mouse(&mut self, x: f32, y: f32, right: bool) {
        let m = (x, y);
        if self.mode == Mode::Present { self.go(1); return; }
        if self.palette.is_some() { self.palette = None; return; }
        let hit = self.hot.iter().rev().find(|(r, _)| inside(r, m)).map(|h| h.1.clone());
        if self.qat_menu.take().is_some() { if let Some(id) = hit.as_deref().and_then(|h| h.strip_prefix("qat:")) { let on = !self.qat.iter().any(|q| q == id); self.set_qat(id, on); self.qat_menu = Some(x.max(100.)); } return; }
        if self.ctx.take().is_some() {
            if let Some(h) = hit { if let Some(id) = h.strip_prefix("ctx:qat:") { self.set_qat(id, true); } else if h == "ctx:collapse" { self.run("view.ribbon"); } }
            return;
        }
        if right { if y < 36. + ribbon::TAB_H + if self.collapsed { 0. } else { ribbon::BODY_H } && y > 36. { self.ctx = Some((x, y, hit.filter(|h| cmd(h).is_some()))); } return; }
        if let Some(h) = hit {
            if h == "zoom.track" { if let Some((r, _)) = self.hot.iter().find(|(_, i)| i == "zoom.track") { self.zoom = (0.4 + (x - r.left - 4.) / (r.width() - 8.) * 0.6).clamp(0.4, 1.0); } }
            else if h.starts_with("tab:") && self.last_click.0.elapsed() < Duration::from_millis(400) && self.last_click.1 == usize::MAX - 1 { self.run("view.ribbon"); }
            else { if h.starts_with("tab:") { self.last_click = (Instant::now(), usize::MAX - 1); } self.run(&h); }
            return;
        }
        if let Some(i) = self.thumbs.iter().position(|r| inside(r, m)) {
            let dbl = self.last_click.0.elapsed() < Duration::from_millis(400) && self.last_click.1 == i; self.last_click = (Instant::now(), i);
            self.cur = i; self.sel = None; if self.mode == Mode::Sorter && dbl { self.mode = Mode::Normal; } return;
        }
        if self.mode == Mode::Normal && inside(&self.canvas, m) {
            let r = self.canvas; let (fx, fy) = ((x - r.left) / r.width(), (y - r.top) / r.height());
            let n = self.slide().shapes.len();
            if let Some(i) = (0..n).rev().find(|&i| { let s = &self.deck.slides[self.cur].shapes[i]; fx >= s.x - 0.02 && fx <= s.x + s.w + 0.02 && fy >= s.y - 0.02 && fy <= s.y + s.h + 0.02 }) {
                self.drag_pending = true; self.sel = Some(i); self.focus = Field::Shape(i); let s = &self.deck.slides[self.cur].shapes[i]; self.drag = Some((i, fx - s.x, fy - s.y)); return;
            }
            self.sel = None;
        }
        if let Some((_, f)) = self.fields.iter().find(|(r, _)| inside(r, m)).cloned() { self.focus = f; self.sel = None; }
    }
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut s = Show::new();
    if let Some(p) = std::env::args().nth(1) { let p = PathBuf::from(p); let d = std::fs::read(&p).unwrap_or_default(); if let Ok(d) = if p.extension().map_or(false, |e| e == "oms") { Deck::from_oms(&d) } else { Ok(Deck::from_markdown(&String::from_utf8_lossy(&d))) } { s.deck = d; s.path = Some(p); } }
    run(s, "OmniShow")
}
#[cfg(test)]
mod tests { use super::*;
    #[test] fn all_command_icons_exist() { for c in CMDS { assert!(has_icon(c.icon), "{} -> {}", c.id, c.icon); } for i in ["check", "dropdown", "search"] { assert!(has_icon(i)); } for q in QAT_OPTIONS { assert!(cmd(q).is_some()); } }
    #[test] fn every_tab_has_commands() { for t in TABS.iter().chain([SHAPE_TAB].iter()) { assert!(CMDS.iter().any(|c| c.tab == *t), "{t}"); } }
    #[test] fn png_export_renders_a_slide() {
        if skia_safe::FontMgr::default().count_families() == 0 { return; } // no system fonts on this machine
        let mut d = Deck::default(); d.slides[0].shapes.push(Shape::new(ShapeKind::Ellipse)); d.numbers = true;
        let b = render_png(&d, 0, 320).expect("png"); assert_eq!(omni_common::image::size(&b), Some((320, 180))); assert!(b.len() > 200);
    }
}
