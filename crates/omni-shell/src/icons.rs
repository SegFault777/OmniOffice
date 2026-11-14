// SPDX-License-Identifier: GPL-3.0-only
//! Material Symbols icons (Apache-2.0, see THIRD_PARTY.md) drawn as Skia paths.
use crate::{col, icons_data::ICONS};
use skia_safe::{utils::parse_path, Canvas, Paint, Path};
use std::{cell::RefCell, collections::HashMap};

thread_local! { static CACHE: RefCell<HashMap<&'static str, Option<Path>>> = RefCell::new(HashMap::new()); }
/// True when an icon with this name exists.
pub fn has_icon(name: &str) -> bool { ICONS.iter().any(|i| i.0 == name) }
/// Draw icon `name` with its top-left corner at (x, y), `size` pixels square (viewBox is 0 -960 960 960).
pub fn icon(c: &Canvas, name: &str, x: f32, y: f32, size: f32, color: u32) {
    let Some(&(key, d)) = ICONS.iter().find(|i| i.0 == name) else { return };
    CACHE.with(|m| {
        let mut m = m.borrow_mut();
        let Some(path) = m.entry(key).or_insert_with(|| parse_path::from_svg(d)).as_ref() else { return };
        let mut p = Paint::default(); p.set_anti_alias(true); p.set_color(col(color));
        c.save(); c.translate((x, y)); let s = size / 960.; c.scale((s, s)); c.translate((0., 960.)); c.draw_path(path, &p); c.restore();
    });
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn all_icon_paths_parse() { for (n, d) in ICONS { assert!(parse_path::from_svg(d).is_some(), "icon {n} does not parse"); } assert!(has_icon("doc") && has_icon("sheet") && has_icon("slides")); }
}
