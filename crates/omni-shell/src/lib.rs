// SPDX-License-Identifier: GPL-3.0-only
//! Window shell shared by OmniCel, OmniShow and the launcher: winit window, CPU Skia surface, softbuffer present.
pub use skia_safe::{Canvas, Rect};
pub use winit::{event::KeyEvent, keyboard::{Key, ModifiersState, NamedKey}};
use skia_safe::{surfaces, textlayout::*, Color, Font, FontMgr, FontStyle, Paint, PaintStyle};
use std::{num::NonZeroU32, sync::Arc, time::{Duration, Instant}};
use winit::{dpi::LogicalSize, event::*, event_loop::{ControlFlow, EventLoop}, window::WindowBuilder};

const FAMS: &[&str] = &["Noto Sans CJK KR", "Noto Sans KR", "Malgun Gothic", "Apple SD Gothic Neo", "Segoe UI", "Helvetica Neue", "Arial", "Noto Sans", "sans-serif"];
thread_local! { static FC: FontCollection = { let mut f = FontCollection::new(); f.set_default_font_manager(FontMgr::default(), None); f }; }
pub fn col(c: u32) -> Color { Color::from_argb(255, (c >> 16) as u8, (c >> 8) as u8, c as u8) }
pub fn inside(r: &Rect, p: (f32, f32)) -> bool { p.0 >= r.left && p.0 <= r.right && p.1 >= r.top && p.1 <= r.bottom }
pub fn font(size: f32, bold: bool) -> Font {
    let m = FontMgr::default(); let st = if bold { FontStyle::bold() } else { FontStyle::normal() };
    Font::new(FAMS.iter().find_map(|f| m.match_family_style(f, st)).or_else(|| m.legacy_make_typeface(None, st)).expect("no system fonts"), size)
}
fn para(s: &str, size: f32, bold: bool, color: u32) -> Paragraph {
    FC.with(|fc| {
        let mut b = ParagraphBuilder::new(&ParagraphStyle::new(), fc); let mut t = TextStyle::new();
        t.set_font_size(size); t.set_color(col(color)); t.set_font_families(FAMS); t.set_font_style(if bold { FontStyle::bold() } else { FontStyle::normal() });
        b.push_style(&t); b.add_text(s); b.pop(); let mut p = b.build(); p.layout(100000.); p
    })
}
/// Draw text with its baseline at `base`. Non-ASCII text goes through Skia's paragraph layout for CJK font fallback.
pub fn text(c: &Canvas, s: &str, x: f32, base: f32, size: f32, bold: bool, color: u32) {
    if s.is_ascii() { let mut p = Paint::default(); p.set_anti_alias(true); p.set_color(col(color)); c.draw_str(s, (x, base), &font(size, bold), &p); }
    else { para(s, size, bold, color).paint(c, (x, base - size * 0.95)); }
}
pub fn measure(s: &str, size: f32, bold: bool) -> f32 { if s.is_ascii() { font(size, bold).measure_str(s, None).0 } else { para(s, size, bold, 0).max_intrinsic_width() } }
pub fn fill(c: &Canvas, r: Rect, color: u32) { let mut p = Paint::default(); p.set_color(col(color)); c.draw_rect(r, &p); }
pub fn rrect(c: &Canvas, r: Rect, rad: f32, color: u32) { let mut p = Paint::default(); p.set_anti_alias(true); p.set_color(col(color)); c.draw_round_rect(r, rad, rad, &p); }
pub fn stroke(c: &Canvas, r: Rect, color: u32, w: f32) { let mut p = Paint::default(); p.set_style(PaintStyle::Stroke); p.set_stroke_width(w); p.set_color(col(color)); c.draw_rect(r, &p); }

pub trait App {
    fn draw(&mut self, c: &Canvas, w: f32, h: f32);
    fn title(&self) -> String;
    fn key(&mut self, _e: &KeyEvent, _m: ModifiersState) {}
    fn text(&mut self, _s: &str) {}               // IME commit
    fn mouse(&mut self, _x: f32, _y: f32, _right: bool) {}
    fn moved(&mut self, _x: f32, _y: f32) {}
    fn wheel(&mut self, _dy: f32) {}
    fn tick(&mut self) {}
    fn wants_quit(&self) -> bool { false }
}
pub fn run(mut app: impl App + 'static, title: &str) -> Result<(), Box<dyn std::error::Error>> {
    let el = EventLoop::new()?;
    let win = Arc::new(WindowBuilder::new().with_title(title).with_inner_size(LogicalSize::new(1180., 760.)).build(&el)?);
    win.set_ime_allowed(true);
    let ctx = softbuffer::Context::new(win.clone())?; let mut sb = softbuffer::Surface::new(&ctx, win.clone())?;
    let (mut mods, mut mouse) = (ModifiersState::empty(), (0f32, 0f32));
    el.run(move |ev, elwt| {
        elwt.set_control_flow(ControlFlow::WaitUntil(Instant::now() + Duration::from_secs(30))); app.tick();
        let Event::WindowEvent { event, .. } = ev else { return };
        let sf = win.scale_factor();
        match event {
            WindowEvent::CloseRequested => elwt.exit(),
            WindowEvent::ModifiersChanged(m) => mods = m.state(),
            WindowEvent::KeyboardInput { event, .. } if event.state == ElementState::Pressed => { app.key(&event, mods); win.request_redraw() }
            WindowEvent::Ime(Ime::Commit(s)) => { app.text(&s); win.request_redraw() }
            WindowEvent::CursorMoved { position, .. } => { mouse = ((position.x / sf) as f32, (position.y / sf) as f32); app.moved(mouse.0, mouse.1); win.request_redraw() }
            WindowEvent::MouseInput { state: ElementState::Pressed, button, .. } => { app.mouse(mouse.0, mouse.1, button == MouseButton::Right); win.request_redraw() }
            WindowEvent::MouseWheel { delta, .. } => { app.wheel(match delta { MouseScrollDelta::LineDelta(_, y) => y * 3., MouseScrollDelta::PixelDelta(p) => p.y as f32 / 16. }); win.request_redraw() }
            WindowEvent::Resized(_) | WindowEvent::ScaleFactorChanged { .. } => win.request_redraw(),
            WindowEvent::RedrawRequested => {
                let sz = win.inner_size(); let (Some(w), Some(h)) = (NonZeroU32::new(sz.width), NonZeroU32::new(sz.height)) else { return };
                let Some(mut surf) = surfaces::raster_n32_premul((w.get() as i32, h.get() as i32)) else { return };
                { let c = surf.canvas(); c.scale((sf as f32, sf as f32)); app.draw(c, sz.width as f32 / sf as f32, sz.height as f32 / sf as f32); }
                let info = surf.image_info(); let mut px = vec![0u8; (w.get() * h.get() * 4) as usize];
                if !surf.read_pixels(&info, &mut px, (w.get() * 4) as usize, (0, 0)) || sb.resize(w, h).is_err() { return; }
                if let Ok(mut b) = sb.buffer_mut() { for (i, p) in px.chunks_exact(4).enumerate() { b[i] = u32::from_le_bytes([p[0], p[1], p[2], p[3]]); } let _ = b.present(); }
                win.set_title(&app.title()); if app.wants_quit() { elwt.exit(); }
            }
            _ => {}
        }
    })?;
    Ok(())
}
