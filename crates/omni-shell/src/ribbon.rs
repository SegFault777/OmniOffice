// SPDX-License-Identifier: GPL-3.0-only
//! Reusable Office-style ribbon: tab strip + groups of icon buttons (used by OmniShow).
use crate::{fill, icon, inside, measure, rrect, text, Canvas, Rect};
pub struct Btn { pub id: String, pub label: String, pub icon: &'static str, pub on: bool, pub enabled: bool }
pub struct Group { pub name: &'static str, pub btns: Vec<Btn> }
pub struct Tab { pub name: &'static str, pub groups: Vec<Group> }
pub struct Colors { pub accent: u32, pub bar: u32, pub bar2: u32, pub line: u32, pub ink: u32 }
pub const TAB_H: f32 = 28.;
pub const BODY_H: f32 = 84.;
/// Draw the ribbon at `y0`. Pushes hotspots ("tab:<name>" and button ids). Returns (bottom y, content width) for scrolling.
pub fn draw(c: &Canvas, w: f32, y0: f32, tabs: &[Tab], active: &str, collapsed: bool, scroll: f32, mouse: (f32, f32), col: &Colors, hot: &mut Vec<(Rect, String)>) -> (f32, f32) {
    let bottom = y0 + TAB_H + if collapsed { 0. } else { BODY_H };
    fill(c, Rect::new(0., y0, w, bottom), col.bar);
    let mut x = 10.;
    for t in tabs {
        let tw = measure(t.name, 12.5, false) + 24.; let r = Rect::new(x, y0 + 2., x + tw, y0 + TAB_H);
        text(c, t.name, x + 12., y0 + 20., 12.5, t.name == active, col.ink);
        if t.name == active { fill(c, Rect::new(x + 8., y0 + TAB_H - 3., x + tw - 8., y0 + TAB_H - 1.), col.accent); }
        hot.push((r, format!("tab:{}", t.name))); x += tw;
    }
    let mut content = 0.;
    if !collapsed {
        if let Some(t) = tabs.iter().find(|t| t.name == active) {
            let ry = y0 + TAB_H; let mut x = 10. - scroll;
            for g in &t.groups {
                let gx = x;
                for b in &g.btns {
                    let bw = (measure(&b.label, 11.5, false) + 16.).max(52.); let r = Rect::new(x, ry + 4., x + bw, ry + 60.);
                    let hov = b.enabled && inside(&r, mouse);
                    if b.on || hov { rrect(c, r, 6., if b.on { col.line } else { col.bar2 }); }
                    let ink = if !b.enabled { 0xB5BAC2 } else if b.on { col.accent } else { col.ink };
                    icon(c, b.icon, x + (bw - 26.) / 2., ry + 7., 26., ink); text(c, &b.label, x + (bw - measure(&b.label, 11.5, false)) / 2., ry + 50., 11.5, false, ink);
                    if b.enabled { hot.push((r, b.id.clone())); } x += bw + 4.;
                }
                let gw = measure(g.name, 11., false); text(c, g.name, gx + ((x - gx) - gw) / 2., ry + 76., 11., false, 0x707680);
                fill(c, Rect::new(x + 3., ry + 6., x + 4., ry + 72.), col.line); x += 12.;
            }
            content = x + scroll;
        }
    }
    (bottom, content)
}
