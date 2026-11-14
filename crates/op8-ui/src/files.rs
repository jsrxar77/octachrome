//! FILE: the main mode that takes both screens (design.md §4). The left shows the sixteen project
//! slots as the pads are (the factory projects in the top two rows, the player's in the bottom
//! two); the right says what the pads do and what just happened.

use crate::theme::HEADER_CY;
use op8_engine::EngineStatus;
use op8_gfx::{
    paint::{fill_circle, fill_convex, fill_round_rect, stroke_round_rect},
    text_v, theme, Align, FontSize, FrameBuffer480x272, VAlign,
};
use std::sync::atomic::Ordering::Relaxed;

const CELL_W: f32 = 108.0;
const CELL_H: f32 = 42.0;
const GAP: f32 = 6.0;

fn bar(fb: &mut FrameBuffer480x272, title: &str) {
    let t = theme();
    fill_circle(fb, 16.0, 15.0, 6.0, t.fg);
    text_v(fb, title, 30, 15, FontSize::Large, t.fg, Align::Left, VAlign::Middle);
}

pub fn draw_left(fb: &mut FrameBuffer480x272, ui: &crate::UiState, st: &EngineStatus) {
    let t = theme();
    fb.clear(t.bg);
    bar(fb, if ui.shift { "save" } else { "open" });
    // transport stays visible: PLAY keeps working
    if st.playing.load(Relaxed) {
        fill_convex(fb, &[(460.0, 8.0), (470.0, 14.0), (460.0, 20.0)], t.fg);
    } else {
        fill_round_rect(fb, 460.0, 9.0, 10.0, 10.0, 1.5, t.dim);
    }
    let _ = HEADER_CY;
    for i in 0..16usize {
        let (col, row) = ((i % 4) as f32, (i / 4) as f32);
        let (x, y) = (12.0 + col * (CELL_W + GAP), 44.0 + row * (CELL_H + GAP));
        let (name, has) = ui.files.get(i).cloned().unwrap_or_default();
        let factory = i < 8;
        let last = ui.file_last == Some(i);
        if has {
            fill_round_rect(fb, x, y, CELL_W, CELL_H, 8.0, if factory { t.panel } else { t.faint });
        }
        let col = if last { t.fg } else if has { t.dim } else { t.faint };
        stroke_round_rect(fb, x, y, CELL_W, CELL_H, 8.0, if last { 2.0 } else { 1.0 }, col);
        let label = if name.is_empty() { "empty".to_string() } else { name };
        text_v(fb, &label, (x + CELL_W / 2.0) as i32, (y + CELL_H / 2.0) as i32, FontSize::Medium, if has { t.fg } else { t.dim }, Align::Center, VAlign::Middle);
    }
}

pub fn draw_right(fb: &mut FrameBuffer480x272, ui: &crate::UiState) {
    let t = theme();
    fb.clear(t.bg);
    bar(fb, "projects");
    let lines = [
        ("tap a pad", "opens it"),
        ("shift + tap", "saves into your slots"),
    ];
    for (i, (a, b)) in lines.iter().enumerate() {
        let y = 90 + i as i32 * 56;
        text_v(fb, a, 30, y, FontSize::Large, t.fg, Align::Left, VAlign::Middle);
        text_v(fb, b, 30, y + 24, FontSize::Medium, t.dim, Align::Left, VAlign::Middle);
    }
    if !ui.file_note.is_empty() {
        text_v(fb, &ui.file_note, 30, 220, FontSize::Large, t.fg, Align::Left, VAlign::Middle);
    }
}
