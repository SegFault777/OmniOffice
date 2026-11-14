// SPDX-License-Identifier: GPL-3.0-only
//! OmniOffice launcher: pick OmniDoc, OmniCel or OmniShow (or pass a file to open it in the matching app).
use omni_shell::*;
use std::process::Command;

const APPS: [(&str, &str, &str, u32, &str); 3] = [
    ("OmniDoc", "omnidoc", "D", 0x2B579A, "Word processor and markup editor  .omd .docx .odt .rtf .doc .md ..."),
    ("OmniCel", "omnicel", "C", 0x1D6F42, "Spreadsheet with formulas  .omc .csv"),
    ("OmniShow", "omnishow", "S", 0xC55A11, "Presentations and slideshows  .oms .md"),
];
struct Launcher { mouse: (f32, f32), toast: String, cards: Vec<Rect>, quit: bool }
fn launch(bin: &str, arg: Option<&str>) -> Result<(), String> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?.with_file_name(format!("{bin}{}", std::env::consts::EXE_SUFFIX));
    let mut c = Command::new(&exe); if let Some(a) = arg { c.arg(a); }
    c.spawn().map(|_| ()).map_err(|e| format!("Cannot start {}: {e}", exe.display()))
}
/// Which application handles a file extension.
fn app_for(path: &str) -> &'static str {
    match path.rsplit('.').next().unwrap_or("").to_lowercase().as_str() { "omc" | "csv" => "omnicel", "oms" => "omnishow", _ => "omnidoc" }
}
impl App for Launcher {
    fn title(&self) -> String { format!("OmniOffice {}", omni_common_id()) }
    fn wants_quit(&self) -> bool { self.quit }
    fn draw(&mut self, c: &Canvas, w: f32, h: f32) {
        fill(c, Rect::new(0., 0., w, h), 0xF3F4F7); text(c, "OmniOffice", 48., 90., 34., true, 0x222222); text(c, "Choose an application  (press 1, 2 or 3)", 50., 120., 14., false, 0x666666);
        self.cards.clear(); let cw = ((w - 100. - 40.) / 3.).min(300.);
        for (i, (name, _, letter, color, desc)) in APPS.iter().enumerate() {
            let r = Rect::new(50. + i as f32 * (cw + 20.), 160., 50. + i as f32 * (cw + 20.) + cw, 420.); let hov = inside(&r, self.mouse);
            rrect(c, r.with_outset((if hov { 3. } else { 0. }, if hov { 3. } else { 0. })), 14., if hov { *color } else { 0xDDE0E6 }); rrect(c, r, 12., 0xFFFFFF);
            rrect(c, Rect::new(r.left + 24., r.top + 24., r.left + 104., r.top + 104.), 16., *color); text(c, letter, r.left + 24. + (80. - measure(letter, 44., true)) / 2., r.top + 82., 44., true, 0xFFFFFF);
            text(c, name, r.left + 24., r.top + 150., 24., true, 0x222222); text(c, &format!("{}", i + 1), r.right - 34., r.top + 40., 18., true, 0x999999);
            let mut y = r.top + 182.; let mut line = String::new(); for word in desc.split(' ') { if measure(&format!("{line} {word}"), 13., false) > cw - 48. { text(c, &line, r.left + 24., y, 13., false, 0x555555); y += 20.; line.clear(); } if !line.is_empty() { line.push(' '); } line.push_str(word); } text(c, &line, r.left + 24., y, 13., false, 0x555555);
            self.cards.push(r);
        }
        text(c, &self.toast, 50., h - 24., 13., false, 0xC0392B);
    }
    fn mouse(&mut self, x: f32, y: f32, _r: bool) { if let Some(i) = self.cards.iter().position(|r| inside(r, (x, y))) { self.go(i); } }
    fn moved(&mut self, x: f32, y: f32) { self.mouse = (x, y); }
    fn key(&mut self, e: &KeyEvent, _m: ModifiersState) {
        match &e.logical_key { Key::Character(k) => if let Some(i) = k.chars().next().and_then(|c| c.to_digit(10)).filter(|d| (1..=3).contains(d)) { self.go(i as usize - 1); }, Key::Named(NamedKey::Escape) => self.quit = true, _ => {} }
    }
}
impl Launcher { fn go(&mut self, i: usize) { match launch(APPS[i].1, None) { Ok(()) => {} Err(e) => self.toast = e } } }
fn omni_common_id() -> &'static str { omni_common::BUILD_ID }
fn main() -> Result<(), Box<dyn std::error::Error>> {
    if let Some(f) = std::env::args().nth(1) { return launch(app_for(&f), Some(&f)).map_err(Into::into); }
    run(Launcher { mouse: (0., 0.), toast: String::new(), cards: vec![], quit: false }, "OmniOffice")
}
#[cfg(test)]
mod tests { #[test] fn routing() { assert_eq!(super::app_for("a.OMC"), "omnicel"); assert_eq!(super::app_for("b.oms"), "omnishow"); assert_eq!(super::app_for("c.hwpx"), "omnidoc"); } }
