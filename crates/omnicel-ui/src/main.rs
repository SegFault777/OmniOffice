// SPDX-License-Identifier: GPL-3.0-only
//! OmniCel: spreadsheet UI (ribbon, formula bar, formatted grid, charts, selection statistics).
use omni_common::config;
use omni_shell::ribbon::{self, Btn, Colors, Group, Tab};
use omni_shell::*;
use omnicel_core::{col_name, Chart, ChartKind, NumFmt, Sheet, Value};
use std::path::PathBuf;

const GREEN: u32 = 0x1D6F42; const CW: f32 = 104.; const RH: f32 = 24.; const HW: f32 = 48.;
const PALETTE: [u32; 6] = [0x2B7FFF, 0xED7D31, 0x4CAF50, 0xFFC000, 0x9C27B0, 0x00ACC1];
struct Cmd { id: &'static str, label: &'static str, icon: &'static str, tab: &'static str, group: &'static str }
const fn c(id: &'static str, label: &'static str, icon: &'static str, tab: &'static str, group: &'static str) -> Cmd { Cmd { id, label, icon, tab, group } }
const TABS: [&str; 5] = ["File", "Home", "Insert", "Data", "View"];
const CMDS: &[Cmd] = &[
    c("file.open", "Open", "folder_open", "File", "File"), c("file.save", "Save", "save", "File", "File"), c("file.saveas", "Save As", "save_as", "File", "File"),
    c("edit.undo", "Undo", "undo", "Home", "Undo"), c("edit.redo", "Redo", "redo", "Home", "Undo"),
    c("edit.cut", "Cut", "content_cut", "Home", "Clipboard"), c("edit.copy", "Copy", "content_copy", "Home", "Clipboard"), c("edit.paste", "Paste", "content_paste", "Home", "Clipboard"),
    c("fmt.bold", "Bold", "bold", "Home", "Font"), c("color.black", "Black", "text_color", "Home", "Font"), c("color.red", "Red", "text_color", "Home", "Font"), c("color.blue", "Blue", "text_color", "Home", "Font"),
    c("fill.none", "No Fill", "fill", "Home", "Fill"), c("fill.yellow", "Yellow", "fill", "Home", "Fill"), c("fill.green", "Green", "fill", "Home", "Fill"), c("fill.blue", "Blue", "fill", "Home", "Fill"), c("fill.red", "Red", "fill", "Home", "Fill"),
    c("align.left", "Left", "format_align_left", "Home", "Alignment"), c("align.center", "Center", "format_align_center", "Home", "Alignment"), c("align.right", "Right", "format_align_right", "Home", "Alignment"),
    c("num.general", "General", "decimal", "Home", "Number"), c("num.fixed", "0.00", "decimal", "Home", "Number"), c("num.percent", "Percent", "percent", "Home", "Number"), c("num.currency", "Currency", "currency", "Home", "Number"), c("num.thousands", "1,000", "decimal", "Home", "Number"),
    c("edit.autosum", "AutoSum", "sigma", "Home", "Editing"), c("edit.filldown", "Fill Down", "fill_down", "Home", "Editing"), c("edit.fillright", "Fill Right", "fill_right", "Home", "Editing"),
    c("chart.bar", "Column", "bar_chart", "Insert", "Charts"), c("chart.line", "Line", "line_chart", "Insert", "Charts"), c("chart.pie", "Pie", "pie_chart", "Insert", "Charts"), c("chart.delete", "Delete Chart", "delete", "Insert", "Charts"),
    c("sort.asc", "Sort A to Z", "sort_az", "Data", "Sort"), c("sort.desc", "Sort Z to A", "sort_za", "Data", "Sort"),
    c("view.grid", "Gridlines", "grid", "View", "Show"), c("view.ribbon", "Collapse Ribbon", "collapse", "View", "Show"),
];
struct Cel {
    sh: Sheet, path: Option<PathBuf>, sel: (u32, u32), anchor: Option<(u32, u32)>, edit: Option<String>, sc: (u32, u32), dirty: bool, toast: String, undo: Vec<Sheet>, redo: Vec<Sheet>, down: bool,
    hot: Vec<(Rect, String)>, hover: (f32, f32), size: (f32, f32), tab: &'static str, collapsed: bool, rscroll: f32, rwidth: f32, grid: bool, chart: Option<usize>, chart_drag: Option<(i64, i64)>, drag_pending: bool,
}
impl Cel {
    fn top(&self) -> f32 { 36. + ribbon::TAB_H + if self.collapsed { 0. } else { ribbon::BODY_H } + 30. + 24. }
    fn snap(&mut self) { self.undo.push(self.sh.clone()); if self.undo.len() > 100 { self.undo.remove(0); } self.redo.clear(); self.dirty = true; }
    fn commit(&mut self) { if let Some(b) = self.edit.take() { self.snap(); self.sh.set(self.sel.0, self.sel.1, &b); } }
    /// Selected range as ((top, left), (bottom, right)).
    fn range(&self) -> ((u32, u32), (u32, u32)) { let a = self.anchor.unwrap_or(self.sel); ((a.0.min(self.sel.0), a.1.min(self.sel.1)), (a.0.max(self.sel.0), a.1.max(self.sel.1))) }
    fn goto(&mut self, r: i64, cc: i64) {
        self.commit(); self.sel = (r.clamp(0, 99_999) as u32, cc.clamp(0, 999) as u32);
        let (vr, vc) = (((self.size.1 - self.top() - 24.) / RH) as u32, ((self.size.0 - HW) / CW) as u32);
        if self.sel.0 < self.sc.0 { self.sc.0 = self.sel.0 } else if self.sel.0 >= self.sc.0 + vr.max(1) { self.sc.0 = self.sel.0 + 1 - vr.max(1) }
        if self.sel.1 < self.sc.1 { self.sc.1 = self.sel.1 } else if self.sel.1 >= self.sc.1 + vc.max(1) { self.sc.1 = self.sel.1 + 1 - vc.max(1) }
    }
    fn step(&mut self, r: i64, cc: i64, extend: bool) { let a = self.anchor.or(Some(self.sel)); self.goto(r, cc); self.anchor = if extend { a } else { None }; }
    fn load(&mut self, p: PathBuf) {
        let d = std::fs::read(&p).unwrap_or_default();
        match if p.extension().map_or(false, |e| e == "omc") { Sheet::from_omc(&d) } else { Ok(Sheet::from_csv(&String::from_utf8_lossy(&d))) } {
            Ok(s) => { self.sh = s; self.path = Some(p); self.dirty = false; self.sel = (0, 0); self.anchor = None; self.sc = (0, 0); self.undo.clear(); self.redo.clear(); self.chart = None; self.toast = "Opened".into() } Err(e) => self.toast = e }
    }
    fn save(&mut self, as_new: bool) {
        self.commit();
        if self.path.is_none() || as_new { self.path = rfd::FileDialog::new().add_filter("OmniCel", &["omc"]).add_filter("CSV (values only)", &["csv"]).set_file_name("sheet.omc").save_file().or(self.path.take()); }
        let Some(p) = self.path.clone() else { return };
        let csv = p.extension().map_or(false, |e| e == "csv"); let bytes = if csv { self.sh.to_csv().into_bytes() } else { self.sh.to_omc() };
        match std::fs::write(&p, bytes) { Ok(()) => { self.dirty = false; self.toast = if csv { "Saved (CSV keeps values and formulas only, not formats or charts)".into() } else { "Saved".into() }; let mut r = config::recent_load("omnicel"); config::recent_push(&mut r, &p.to_string_lossy(), 8); config::recent_save("omnicel", &r); } Err(e) => self.toast = e.to_string() }
    }
    fn run(&mut self, id: &str) {
        if let Some(t) = id.strip_prefix("tab:") { if let Some(t) = TABS.iter().find(|x| **x == t) { self.tab = t; } return; }
        self.commit(); let (a, b) = self.range();
        match id {
            "file.open" => if let Some(p) = rfd::FileDialog::new().add_filter("OmniCel / CSV", &["omc", "csv"]).pick_file() { self.load(p); }, "file.save" => self.save(false), "file.saveas" => self.save(true),
            "edit.undo" => if let Some(s) = self.undo.pop() { self.redo.push(std::mem::replace(&mut self.sh, s)); self.dirty = true; },
            "edit.redo" => if let Some(s) = self.redo.pop() { self.undo.push(std::mem::replace(&mut self.sh, s)); self.dirty = true; },
            "edit.copy" | "edit.cut" => { let t = self.sh.range_tsv(a.0, a.1, b.0, b.1); if let Ok(mut cb) = arboard::Clipboard::new() { let _ = cb.set_text(t); } if id == "edit.cut" { self.snap(); self.sh.clear_range(a.0, a.1, b.0, b.1); } }
            "edit.paste" => if let Ok(t) = arboard::Clipboard::new().and_then(|mut cb| cb.get_text()) { self.snap(); let (rows, cols) = self.sh.paste_tsv(a.0, a.1, &t); self.sel = a; self.anchor = Some((a.0 + rows.saturating_sub(1), a.1 + cols.saturating_sub(1))); },
            "edit.selectall" => { let (r, cc) = self.sh.dims(); self.anchor = Some((0, 0)); self.sel = (r.saturating_sub(1), cc.saturating_sub(1)); }
            "edit.autosum" => { self.snap(); if !self.sh.autosum(self.sel.0, self.sel.1) { self.toast = "Select the empty cell below or to the right of the numbers".into(); } }
            "edit.filldown" => { self.snap(); self.sh.fill_down(a.0, a.1, b.0.max(a.0 + 1), b.1); } "edit.fillright" => { self.snap(); self.sh.fill_right(a.0, a.1, b.0, b.1.max(a.1 + 1)); }
            "fmt.bold" => { self.snap(); let on = !(a.0..=b.0).all(|r| (a.1..=b.1).all(|cc| self.sh.fmt(r, cc).bold)); self.sh.update_fmt(a.0, a.1, b.0, b.1, |f| f.bold = on); }
            "view.grid" => self.grid = !self.grid, "view.ribbon" => self.collapsed = !self.collapsed,
            "chart.delete" => if let Some(i) = self.chart.take() { if i < self.sh.charts.len() { self.snap(); self.sh.charts.remove(i); } },
            "sort.asc" | "sort.desc" => { self.snap();
                let ((mut r1, c1), (r2, c2)) = if self.anchor.is_some() { (a, b) } else { ((0, 0), { let d = self.sh.dims(); (d.0.saturating_sub(1), d.1.saturating_sub(1)) }) };
                let header = (c1..=c2).all(|cc| matches!(self.sh.eval(r1, cc), Value::Text(ref t) if !t.is_empty())) && r2 > r1 && (c1..=c2).any(|cc| matches!(self.sh.eval(r1 + 1, cc), Value::Num(_))); if header { r1 += 1; }
                self.sh.sort_rows(r1, c1, r2, c2, self.sel.1.clamp(c1, c2), id == "sort.asc"); }
            _ => { let (r, cc) = (self.sel.0, self.sel.1); let _ = (r, cc);
                if let Some(k) = id.strip_prefix("chart.").and_then(|k| ["bar", "line", "pie"].iter().position(|x| *x == k)) {
                    self.snap(); let kind = [ChartKind::Bar, ChartKind::Line, ChartKind::Pie][k]; let rg = if self.anchor.is_some() { (a.0, a.1, b.0, b.1) } else { let d = self.sh.dims(); (0, 0, d.0.saturating_sub(1), d.1.saturating_sub(1)) };
                    self.sh.charts.push(Chart { kind, range: rg, at: (rg.0, rg.3 + 2), title: self.path.as_ref().and_then(|p| p.file_stem()).and_then(|n| n.to_str()).unwrap_or("Chart").to_string() }); self.chart = Some(self.sh.charts.len() - 1);
                } else if let Some(n) = id.strip_prefix("num.") { let nf = match n { "fixed" => NumFmt::Fixed2, "percent" => NumFmt::Percent, "currency" => NumFmt::Currency, "thousands" => NumFmt::Thousands, _ => NumFmt::General }; self.snap(); self.sh.update_fmt(a.0, a.1, b.0, b.1, |f| f.num = nf); }
                else if let Some(n) = id.strip_prefix("fill.") { let v = match n { "yellow" => Some(0xFFF2CC), "green" => Some(0xD9EAD3), "blue" => Some(0xD0E2F3), "red" => Some(0xF4CCCC), _ => None }; self.snap(); self.sh.update_fmt(a.0, a.1, b.0, b.1, |f| f.fill = v); }
                else if let Some(n) = id.strip_prefix("color.") { let v = match n { "red" => Some(0xC00000), "blue" => Some(0x1F4E9E), _ => None }; self.snap(); self.sh.update_fmt(a.0, a.1, b.0, b.1, |f| f.color = v); }
                else if let Some(n) = id.strip_prefix("align.") { let v = match n { "left" => 1, "center" => 2, _ => 3 }; self.snap(); self.sh.update_fmt(a.0, a.1, b.0, b.1, |f| f.align = v); } }
        }
    }
    fn cell_at(&self, x: f32, y: f32) -> Option<(u32, u32)> { if x < HW || y < self.top() { return None; } Some((self.sc.0 + ((y - self.top()) / RH) as u32, self.sc.1 + ((x - HW) / CW) as u32)) }
    fn chart_rect(&self, ch: &Chart) -> Rect { let x = HW + (ch.at.1 as f32 - self.sc.1 as f32) * CW; let y = self.top() + (ch.at.0 as f32 - self.sc.0 as f32) * RH; Rect::new(x, y, x + 420., y + 260.) }
    fn is_on(&self, id: &str) -> bool {
        let (a, b) = self.range(); let all = |f: &dyn Fn(omnicel_core::CellFmt) -> bool| (a.0..=b.0).all(|r| (a.1..=b.1).all(|cc| f(self.sh.fmt(r, cc))));
        match id { "fmt.bold" => all(&|f| f.bold), "view.grid" => self.grid, "view.ribbon" => self.collapsed, "align.left" => all(&|f| f.align == 1), "align.center" => all(&|f| f.align == 2), "align.right" => all(&|f| f.align == 3),
            "num.fixed" => all(&|f| f.num == NumFmt::Fixed2), "num.percent" => all(&|f| f.num == NumFmt::Percent), "num.currency" => all(&|f| f.num == NumFmt::Currency), "num.thousands" => all(&|f| f.num == NumFmt::Thousands), _ => false }
    }
    fn draw_chart(&self, cv: &Canvas, ch: &Chart, r: Rect, selected: bool) {
        rrect(cv, r.with_outset((1., 1.)), 6., if selected { 0x2B7FFF } else { 0xC8CFCB }); rrect(cv, r, 5., 0xFFFFFF);
        text(cv, &ch.title, r.left + 12., r.top + 22., 14., true, 0x222222);
        let (labels, series) = self.sh.chart_data(ch); if series.is_empty() || labels.is_empty() { return; }
        let plot = Rect::new(r.left + 40., r.top + 40., r.right - 120., r.bottom - 34.);
        for (i, (name, _)) in series.iter().enumerate() { let y = r.top + 50. + i as f32 * 20.; fill(cv, Rect::new(r.right - 108., y - 9., r.right - 98., y + 1.), PALETTE[i % 6]); text(cv, &name.chars().take(10).collect::<String>(), r.right - 92., y, 11., false, 0x333333); }
        if ch.kind == ChartKind::Pie {
            let vals = &series[0].1; let total: f64 = vals.iter().map(|v| v.max(0.)).sum(); if total <= 0. { return; }
            let d = plot.width().min(plot.height()); let oval = Rect::new(plot.left + 20., plot.top, plot.left + 20. + d, plot.top + d); let mut a = -90f32;
            for (i, v) in vals.iter().enumerate() { let sw = (v.max(0.) / total * 360.) as f32; arc(cv, oval, a, sw, PALETTE[i % 6]); a += sw; text(cv, &labels[i].chars().take(8).collect::<String>(), r.right - 108., r.top + 50. + (series.len() + i) as f32 * 18. + 8., 11., false, 0x333333); }
            return;
        }
        let max = series.iter().flat_map(|s| s.1.iter()).cloned().fold(0f64, f64::max).max(1e-9); let n = labels.len();
        line(cv, (plot.left, plot.bottom), (plot.right, plot.bottom), 0x888888, 1.); line(cv, (plot.left, plot.top), (plot.left, plot.bottom), 0x888888, 1.); text(cv, &Value::Num(max).show(), r.left + 6., plot.top + 4., 10., false, 0x666666);
        let slot = plot.width() / n as f32;
        for (li, l) in labels.iter().enumerate() { text(cv, &l.chars().take(7).collect::<String>(), plot.left + li as f32 * slot + 2., plot.bottom + 14., 10., false, 0x444444); }
        let ns = series.len() as f32;
        for (si, (_, vals)) in series.iter().enumerate() {
            let pts: Vec<(f32, f32)> = vals.iter().enumerate().map(|(i, v)| (plot.left + (i as f32 + 0.5) * slot, plot.bottom - (v.max(0.) / max) as f32 * plot.height())).collect();
            if ch.kind == ChartKind::Bar { for (i, p) in pts.iter().enumerate() { let bw = slot * 0.7 / ns; let x = plot.left + i as f32 * slot + slot * 0.15 + si as f32 * bw; fill(cv, Rect::new(x, p.1, x + bw - 1., plot.bottom), PALETTE[si % 6]); } }
            else { for w in pts.windows(2) { line(cv, w[0], w[1], PALETTE[si % 6], 2.); } for p in &pts { oval(cv, Rect::new(p.0 - 3., p.1 - 3., p.0 + 3., p.1 + 3.), PALETTE[si % 6]); } }
        }
    }
}
impl App for Cel {
    fn title(&self) -> String { format!("{}{} - OmniCel {}", self.path.as_ref().and_then(|p| p.file_name()).and_then(|n| n.to_str()).unwrap_or("Untitled"), if self.dirty { " *" } else { "" }, omni_common::BUILD_ID) }
    fn draw(&mut self, cv: &Canvas, w: f32, h: f32) {
        self.size = (w, h); self.hot.clear(); fill(cv, Rect::new(0., 0., w, h), 0xFFFFFF);
        fill(cv, Rect::new(0., 0., w, 36.), GREEN); text(cv, &format!("OmniCel  |  {}{}", self.path.as_ref().and_then(|p| p.file_name()).and_then(|n| n.to_str()).unwrap_or("Untitled"), if self.dirty { "  - Unsaved" } else { "" }), 12., 23., 13., true, 0xFFFFFF);
        let tabs: Vec<Tab> = TABS.iter().map(|t| { let mut groups: Vec<Group> = vec![]; for cm in CMDS.iter().filter(|c| c.tab == *t) {
            if groups.last().map_or(true, |g| g.name != cm.group) { groups.push(Group { name: cm.group, btns: vec![] }); }
            groups.last_mut().unwrap().btns.push(Btn { id: cm.id.into(), label: cm.label.into(), icon: cm.icon, on: self.is_on(cm.id), enabled: match cm.id { "edit.undo" => !self.undo.is_empty(), "edit.redo" => !self.redo.is_empty(), "chart.delete" => self.chart.is_some(), _ => true } }); }
            Tab { name: t, groups } }).collect();
        let col = Colors { accent: GREEN, bar: 0xF3F5F4, bar2: 0xFFFFFF, line: 0xDDE3DF, ink: 0x222222 };
        let (rb, rw) = ribbon::draw(cv, w, 36., &tabs, self.tab, self.collapsed, self.rscroll, self.hover, &col, &mut self.hot); self.rwidth = rw;
        let top = self.top();
        // formula bar
        fill(cv, Rect::new(0., rb, w, rb + 30.), 0xF3F5F4); text(cv, &format!("{}{}", col_name(self.sel.1), self.sel.0 + 1), 12., rb + 20., 13., true, GREEN); text(cv, "fx", 80., rb + 20., 13., false, 0x888888);
        text(cv, &self.edit.clone().unwrap_or_else(|| self.sh.raw(self.sel.0, self.sel.1).to_string()), 110., rb + 20., 13., false, 0x222222);
        let (vr, vc) = (((h - top - 24.) / RH) as u32 + 1, ((w - HW) / CW) as u32 + 1);
        fill(cv, Rect::new(0., rb + 30., w, top), 0xE9EDEB); fill(cv, Rect::new(0., top, HW, h - 24.), 0xE9EDEB); let ((ra, ca), (rbm, cb)) = self.range();
        for j in 0..vc { let x = HW + j as f32 * CW; let cc = self.sc.1 + j; if (ca..=cb).contains(&cc) { fill(cv, Rect::new(x, top - 24., x + CW, top), 0xCFE5D8); } let nm = col_name(cc); text(cv, &nm, x + (CW - measure(&nm, 12., true)) / 2., top - 8., 12., true, 0x333333); stroke(cv, Rect::new(x, top - 24., x + CW, top), 0xC8CFCB, 1.); }
        for i in 0..vr { let y = top + i as f32 * RH; let r = self.sc.0 + i; if y + RH > h - 24. { break; }
            if (ra..=rbm).contains(&r) { fill(cv, Rect::new(0., y, HW, y + RH), 0xCFE5D8); } text(cv, &(r + 1).to_string(), 8., y + 17., 12., true, 0x333333); stroke(cv, Rect::new(0., y, HW, y + RH), 0xC8CFCB, 1.);
            for j in 0..vc { let x = HW + j as f32 * CW; let cc = self.sc.1 + j; let rect = Rect::new(x, y, x + CW, y + RH); let f = self.sh.fmt(r, cc);
                if let Some(fc) = f.fill { fill(cv, rect, fc); } if self.grid { stroke(cv, rect, 0xE1E5E3, 1.); }
                let v = self.sh.eval(r, cc); if matches!(&v, Value::Text(t) if t.is_empty()) { continue; }
                let s = self.sh.display(r, cc); let tw = measure(&s, 13., f.bold);
                let right = match f.align { 1 => false, 2 => false, 3 => true, _ => matches!(v, Value::Num(_)) };
                let tx = if f.align == 2 { x + (CW - tw) / 2. } else if right { x + CW - 6. - tw } else { x + 6. };
                cv.save(); cv.clip_rect(rect.with_inset((2., 0.)), None, false); text(cv, &s, tx, y + 17., 13., f.bold, if matches!(v, Value::Err(_)) { 0xC0392B } else { f.color.unwrap_or(0x1A1A1A) }); cv.restore(); } }
        if self.anchor.is_some() { let (x0, y0) = (HW + (ca as f32 - self.sc.1 as f32) * CW, top + (ra as f32 - self.sc.0 as f32) * RH); let (x1, y1) = (HW + (cb as f32 + 1. - self.sc.1 as f32) * CW, top + (rbm as f32 + 1. - self.sc.0 as f32) * RH);
            cv.save(); cv.clip_rect(Rect::new(HW, top, w, h - 24.), None, false); tint(cv, Rect::new(x0, y0, x1, y1), GREEN, 40); stroke(cv, Rect::new(x0, y0, x1, y1), GREEN, 1.5); cv.restore(); }
        let sr = Rect::new(HW + (self.sel.1 as f32 - self.sc.1 as f32) * CW, top + (self.sel.0 as f32 - self.sc.0 as f32) * RH, 0., 0.); let sr = Rect::new(sr.left, sr.top, sr.left + CW, sr.top + RH);
        if let Some(b) = &self.edit { fill(cv, sr, 0xFFFFFF); text(cv, &format!("{b}|"), sr.left + 6., sr.top + 17., 13., false, 0x000000); } stroke(cv, sr, GREEN, 2.);
        cv.save(); cv.clip_rect(Rect::new(HW, top, w, h - 24.), None, false);
        for (i, ch) in self.sh.charts.iter().enumerate() { self.draw_chart(cv, ch, self.chart_rect(ch), self.chart == Some(i)); } cv.restore();
        // status bar: selection statistics (like Excel's status bar)
        fill(cv, Rect::new(0., h - 24., w, h), 0xF3F5F4);
        let nums: Vec<f64> = (ra..=rbm).flat_map(|r| (ca..=cb).map(move |c| (r, c))).filter_map(|(r, c)| if let Value::Num(n) = self.sh.eval(r, c) { Some(n) } else { None }).collect();
        let stat = if nums.len() > 1 { format!("Average: {}    Count: {}    Sum: {}", Value::Num(nums.iter().sum::<f64>() / nums.len() as f64).show(), nums.len(), Value::Num(nums.iter().sum()).show()) } else { format!("{}", self.sh.display(self.sel.0, self.sel.1)) };
        text(cv, &format!("{}    {}", stat, self.toast), 10., h - 8., 12., false, 0x333333);
    }
    fn key(&mut self, e: &KeyEvent, m: ModifiersState) {
        let (r, cc) = (self.sel.0 as i64, self.sel.1 as i64); self.toast.clear();
        if m.control_key() || m.super_key() { if let Key::Character(k) = &e.logical_key { match k.to_lowercase().as_str() {
            "s" => self.run("file.save"), "o" => self.run("file.open"), "z" => self.run("edit.undo"), "y" => self.run("edit.redo"), "c" => self.run("edit.copy"), "x" => self.run("edit.cut"), "v" => self.run("edit.paste"),
            "a" => self.run("edit.selectall"), "b" => self.run("fmt.bold"), "d" => self.run("edit.filldown"), "r" => self.run("edit.fillright"), _ => {} } } else if matches!(e.logical_key, Key::Named(NamedKey::F1)) { self.run("view.ribbon"); } return; }
        if m.alt_key() { if let Key::Character(k) = &e.logical_key { if k.as_str() == "=" { self.run("edit.autosum"); } } return; }
        match &e.logical_key {
            Key::Named(NamedKey::ArrowUp) => self.step(r - 1, cc, m.shift_key()), Key::Named(NamedKey::ArrowDown) => self.step(r + 1, cc, m.shift_key()), Key::Named(NamedKey::ArrowLeft) => self.step(r, cc - 1, m.shift_key()), Key::Named(NamedKey::ArrowRight) => self.step(r, cc + 1, m.shift_key()),
            Key::Named(NamedKey::Enter) => self.step(r + 1, cc, false), Key::Named(NamedKey::Tab) => self.step(r, cc + 1, false),
            Key::Named(NamedKey::Escape) => { self.edit = None; self.chart = None; } Key::Named(NamedKey::F2) => self.edit = Some(self.sh.raw(self.sel.0, self.sel.1).to_string()),
            Key::Named(NamedKey::Delete) => { if self.chart.is_some() { self.run("chart.delete"); } else { self.snap(); let (a, b) = self.range(); self.sh.clear_range(a.0, a.1, b.0, b.1); } }
            Key::Named(NamedKey::Backspace) => { if let Some(b) = self.edit.as_mut() { b.pop(); } else { self.edit = Some(String::new()); } }
            _ => if let Some(t) = &e.text { if !t.chars().any(char::is_control) { self.edit.get_or_insert_with(String::new).push_str(t); } },
        }
    }
    fn text(&mut self, s: &str) { self.edit.get_or_insert_with(String::new).push_str(s); }
    fn moved(&mut self, x: f32, y: f32) {
        self.hover = (x, y);
        if let (Some(i), Some((dr, dc))) = (self.chart, self.chart_drag) { if self.drag_pending { self.snap(); self.drag_pending = false; } if let Some(rc) = self.cell_at(x, y) { if let Some(ch) = self.sh.charts.get_mut(i) { ch.at = ((rc.0 as i64 - dr).max(0) as u32, (rc.1 as i64 - dc).max(0) as u32); self.dirty = true; } } }
        else if self.down { if let Some(rc) = self.cell_at(x, y) { self.sel = rc; } }
    }
    fn mouse_up(&mut self) { self.down = false; self.chart_drag = None; self.drag_pending = false; }
    fn wheel(&mut self, dy: f32) {
        if self.hover.1 > 36. && self.hover.1 < 36. + ribbon::TAB_H + ribbon::BODY_H && !self.collapsed { let max = (self.rwidth - self.size.0 + 20.).max(0.); self.rscroll = (self.rscroll - dy * 30.).clamp(0., max); }
        else { self.sc.0 = (self.sc.0 as i64 - dy as i64).max(0) as u32; }
    }
    fn mouse(&mut self, x: f32, y: f32, _right: bool) {
        if let Some(id) = self.hot.iter().rev().find(|(r, _)| inside(r, (x, y))).map(|h| h.1.clone()) { self.run(&id); return; }
        if let Some(i) = (0..self.sh.charts.len()).rev().find(|&i| inside(&self.chart_rect(&self.sh.charts[i]), (x, y))) {
            self.chart = Some(i); if let Some(rc) = self.cell_at(x, y) { let at = self.sh.charts[i].at; self.chart_drag = Some((rc.0 as i64 - at.0 as i64, rc.1 as i64 - at.1 as i64)); self.drag_pending = true; } return;
        }
        self.chart = None;
        if let Some((r, cc)) = self.cell_at(x, y) { self.goto(r as i64, cc as i64); self.anchor = Some((r, cc)); self.down = true; }
    }
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut a = Cel { sh: Sheet::default(), path: None, sel: (0, 0), anchor: None, edit: None, sc: (0, 0), dirty: false, toast: String::new(), undo: vec![], redo: vec![], down: false, hot: vec![], hover: (0., 0.), size: (1180., 760.),
        tab: "Home", collapsed: false, rscroll: 0., rwidth: 0., grid: true, chart: None, chart_drag: None, drag_pending: false };
    if let Some(p) = std::env::args().nth(1) { a.load(PathBuf::from(p)); }
    run(a, "OmniCel")
}
#[cfg(test)]
mod tests { use super::*;
    #[test] fn command_icons_exist() { for c in CMDS { assert!(has_icon(c.icon), "{} -> {}", c.id, c.icon); } for t in TABS { assert!(CMDS.iter().any(|c| c.tab == t), "{t}"); } }
}
