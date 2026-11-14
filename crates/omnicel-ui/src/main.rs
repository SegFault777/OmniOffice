// SPDX-License-Identifier: GPL-3.0-only
//! OmniCel: spreadsheet UI (Excel/Hancom Cell style grid, formula bar, status bar).
use omni_common::config;
use omni_shell::*;
use omnicel_core::{col_name, Sheet, Value};
use std::{path::PathBuf, time::Instant};

const CW: f32 = 104.; const RH: f32 = 24.; const HW: f32 = 48.; const TOP: f32 = 100.;
struct Cel { sh: Sheet, path: Option<PathBuf>, sel: (u32, u32), edit: Option<String>, sc: (u32, u32), dirty: bool, toast: String, drag_from: Option<(u32, u32)>, size: (f32, f32), _t: Instant, hot: Vec<(Rect, &'static str, &'static str)>, hover: (f32, f32) }
impl Cel {
    fn commit(&mut self) { if let Some(b) = self.edit.take() { self.sh.set(self.sel.0, self.sel.1, &b); self.dirty = true; } }
    fn goto(&mut self, r: i64, c: i64) {
        self.commit(); self.sel = (r.clamp(0, 99_999) as u32, c.clamp(0, 999) as u32);
        let (vr, vc) = (((self.size.1 - TOP - 24.) / RH) as u32, ((self.size.0 - HW) / CW) as u32);
        if self.sel.0 < self.sc.0 { self.sc.0 = self.sel.0 } else if self.sel.0 >= self.sc.0 + vr.max(1) { self.sc.0 = self.sel.0 + 1 - vr.max(1) }
        if self.sel.1 < self.sc.1 { self.sc.1 = self.sel.1 } else if self.sel.1 >= self.sc.1 + vc.max(1) { self.sc.1 = self.sel.1 + 1 - vc.max(1) }
    }
    fn open(&mut self) {
        let Some(p) = rfd::FileDialog::new().add_filter("OmniCel / CSV", &["omc", "csv"]).pick_file() else { return };
        let d = std::fs::read(&p).unwrap_or_default();
        let r = if p.extension().map_or(false, |e| e == "omc") { Sheet::from_omc(&d) } else { Ok(Sheet::from_csv(&String::from_utf8_lossy(&d))) };
        match r { Ok(s) => { self.sh = s; self.path = Some(p); self.dirty = false; self.sel = (0, 0); self.sc = (0, 0); self.toast = "Opened".into() } Err(e) => self.toast = e }
    }
    fn save(&mut self) {
        self.commit();
        if self.path.is_none() { self.path = rfd::FileDialog::new().add_filter("OmniCel", &["omc"]).add_filter("CSV", &["csv"]).set_file_name("sheet.omc").save_file(); }
        let Some(p) = self.path.clone() else { return };
        let bytes = if p.extension().map_or(false, |e| e == "csv") { self.sh.to_csv().into_bytes() } else { self.sh.to_omc() };
        match std::fs::write(&p, bytes) { Ok(()) => { self.dirty = false; self.toast = "Saved".into(); let mut r = config::recent_load("omnicel"); config::recent_push(&mut r, &p.to_string_lossy(), 8); config::recent_save("omnicel", &r); } Err(e) => self.toast = e.to_string() }
    }
    fn cell_at(&self, x: f32, y: f32) -> Option<(u32, u32)> { if x < HW || y < TOP { return None; } Some((self.sc.0 + ((y - TOP) / RH) as u32, self.sc.1 + ((x - HW) / CW) as u32)) }
}
impl App for Cel {
    fn title(&self) -> String { format!("{}{} - OmniCel {}", self.path.as_ref().and_then(|p| p.file_name()).and_then(|n| n.to_str()).unwrap_or("Untitled"), if self.dirty { " *" } else { "" }, omni_common::BUILD_ID) }
    fn draw(&mut self, c: &Canvas, w: f32, h: f32) {
        self.size = (w, h); fill(c, Rect::new(0., 0., w, h), 0xFFFFFF);
        fill(c, Rect::new(0., 0., w, 36.), 0x1D6F42); text(c, &format!("OmniCel  |  {}{}", self.path.as_ref().and_then(|p| p.file_name()).and_then(|n| n.to_str()).unwrap_or("Untitled"), if self.dirty { "  - Unsaved" } else { "" }), 12., 23., 13., true, 0xFFFFFF);
        self.hot.clear(); let mut hov = "";
        for (i, (id, ic, tip)) in [("open", "folder_open", "Open (Ctrl+O)"), ("save", "save", "Save (Ctrl+S)")].into_iter().enumerate() {
            let r = Rect::new(250. + i as f32 * 36., 5., 250. + i as f32 * 36. + 30., 31.); if inside(&r, self.hover) { rrect(c, r, 6., 0x2E8B57); hov = tip; }
            icon(c, ic, r.left + 3., r.top + 1., 24., 0xFFFFFF); self.hot.push((r, id, tip));
        }
        text(c, if hov.is_empty() { "F2 edit   Del clear   arrows/Tab/Enter move" } else { hov }, 340., 23., 12., false, 0xE6F2EA);
        fill(c, Rect::new(0., 36., w, 66.), 0xF3F5F4); text(c, &format!("{}{}", col_name(self.sel.1), self.sel.0 + 1), 12., 56., 13., true, 0x1D6F42); text(c, "fx", 70., 56., 13., false, 0x888888);
        let raw = self.edit.clone().unwrap_or_else(|| self.sh.raw(self.sel.0, self.sel.1).to_string()); text(c, &raw, 100., 56., 13., false, 0x222222);
        let (vr, vc) = (((h - TOP - 24.) / RH) as u32 + 1, ((w - HW) / CW) as u32 + 1);
        fill(c, Rect::new(0., 66., w, TOP), 0xE9EDEB); fill(c, Rect::new(0., TOP, HW, h - 24.), 0xE9EDEB);
        for j in 0..vc { let x = HW + j as f32 * CW; let cc = self.sc.1 + j; let on = cc == self.sel.1;
            if on { fill(c, Rect::new(x, 66.0 + 10., x + CW, TOP), 0xCFE5D8); } let nm = col_name(cc); text(c, &nm, x + (CW - measure(&nm, 12., true)) / 2., 90., 12., true, 0x333333); stroke(c, Rect::new(x, 76., x + CW, TOP), 0xC8CFCB, 1.); }
        for i in 0..vr { let y = TOP + i as f32 * RH; let r = self.sc.0 + i; if y + RH > h - 24. { break; }
            if r == self.sel.0 { fill(c, Rect::new(0., y, HW, y + RH), 0xCFE5D8); } text(c, &(r + 1).to_string(), 8., y + 17., 12., true, 0x333333); stroke(c, Rect::new(0., y, HW, y + RH), 0xC8CFCB, 1.);
            for j in 0..vc { let x = HW + j as f32 * CW; let cc = self.sc.1 + j; let rect = Rect::new(x, y, x + CW, y + RH); stroke(c, rect, 0xE1E5E3, 1.);
                let v = self.sh.eval(r, cc); if matches!(&v, Value::Text(t) if t.is_empty()) { continue; }
                let s = v.show(); let (colr, right) = match &v { Value::Err(_) => (0xC0392B, false), Value::Num(_) => (0x1A1A1A, true), _ => (0x1A1A1A, false) };
                c.save(); c.clip_rect(rect.with_inset((2., 0.)), None, false); let tx = if right { x + CW - 6. - measure(&s, 13., false) } else { x + 6. }; text(c, &s, tx, y + 17., 13., false, colr); c.restore(); } }
        let sr = Rect::new(HW + (self.sel.1 as f32 - self.sc.1 as f32) * CW, TOP + (self.sel.0 as f32 - self.sc.0 as f32) * RH, 0., 0.); let sr = Rect::new(sr.left, sr.top, sr.left + CW, sr.top + RH);
        if let Some(b) = &self.edit { fill(c, sr, 0xFFFFFF); text(c, &format!("{b}|"), sr.left + 6., sr.top + 17., 13., false, 0x000000); } stroke(c, sr, 0x1D6F42, 2.);
        fill(c, Rect::new(0., h - 24., w, h), 0xF3F5F4); let (rows, cols) = self.sh.dims(); text(c, &format!("{}    Used range: {} rows x {} columns    {}", self.sh.eval(self.sel.0, self.sel.1).show(), rows, cols, self.toast), 10., h - 8., 12., false, 0x333333);
    }
    fn key(&mut self, e: &KeyEvent, m: ModifiersState) {
        let (r, c) = (self.sel.0 as i64, self.sel.1 as i64); self.toast.clear();
        if m.control_key() || m.super_key() { if let Key::Character(k) = &e.logical_key { match k.to_lowercase().as_str() { "s" => self.save(), "o" => self.open(), _ => {} } } return; }
        match &e.logical_key {
            Key::Named(NamedKey::ArrowUp) => self.goto(r - 1, c), Key::Named(NamedKey::ArrowDown) => self.goto(r + 1, c), Key::Named(NamedKey::ArrowLeft) => self.goto(r, c - 1), Key::Named(NamedKey::ArrowRight) => self.goto(r, c + 1),
            Key::Named(NamedKey::Enter) => self.goto(r + 1, c), Key::Named(NamedKey::Tab) => self.goto(r, c + 1),
            Key::Named(NamedKey::Escape) => self.edit = None, Key::Named(NamedKey::F2) => self.edit = Some(self.sh.raw(self.sel.0, self.sel.1).to_string()),
            Key::Named(NamedKey::Delete) => { self.sh.set(self.sel.0, self.sel.1, ""); self.dirty = true; }
            Key::Named(NamedKey::Backspace) => { if let Some(b) = self.edit.as_mut() { b.pop(); } else { self.edit = Some(String::new()); } }
            _ => if let Some(t) = &e.text { if !t.chars().any(char::is_control) { self.edit.get_or_insert_with(String::new).push_str(t); } },
        }
    }
    fn text(&mut self, s: &str) { self.edit.get_or_insert_with(String::new).push_str(s); }
    fn moved(&mut self, x: f32, y: f32) { self.hover = (x, y); }
    fn mouse(&mut self, x: f32, y: f32, _r: bool) {
        if let Some(id) = self.hot.iter().find(|h| inside(&h.0, (x, y))).map(|h| h.1) { if id == "open" { self.open() } else { self.save() } return; } if let Some((r, c)) = self.cell_at(x, y) { self.goto(r as i64, c as i64); self.drag_from = Some((r, c)); } }
    fn wheel(&mut self, dy: f32) { self.sc.0 = (self.sc.0 as i64 - dy as i64).max(0) as u32; }
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut a = Cel { sh: Sheet::default(), path: None, sel: (0, 0), edit: None, sc: (0, 0), dirty: false, toast: String::new(), drag_from: None, size: (1180., 760.), _t: Instant::now(), hot: vec![], hover: (0., 0.) };
    if let Some(p) = std::env::args().nth(1) { let p = PathBuf::from(p); let d = std::fs::read(&p).unwrap_or_default(); let r = if p.extension().map_or(false, |e| e == "omc") { Sheet::from_omc(&d) } else { Ok(Sheet::from_csv(&String::from_utf8_lossy(&d))) }; if let Ok(s) = r { a.sh = s; a.path = Some(p); } }
    run(a, "OmniCel")
}
