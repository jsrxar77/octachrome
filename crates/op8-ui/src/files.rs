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

use crate::nav::{CELL_H, CELL_W, GAP};

pub(crate) fn bar(fb: &mut FrameBuffer480x272, title: &str) {
    let t = theme();
    fill_circle(fb, 16.0, 15.0, 6.0, t.fg);
    text_v(fb, title, 30, 15, FontSize::Large, t.fg, Align::Left, VAlign::Middle);
}

pub fn draw_left(fb: &mut FrameBuffer480x272, ui: &crate::UiState, st: &EngineStatus) {
    pick_left(fb, ui, st, if ui.shift { "save" } else { "open" }, &ui.files, ui.file_last);
}

/// A grid of sixteen choices as the pads are (reading order), the top two rows the factory ones: used by
/// FILE (projects) and PLUG-IN (instruments). A cell is `(name, available)`; the one in `last` is outlined.
pub fn pick_left(fb: &mut FrameBuffer480x272, ui: &crate::UiState, st: &EngineStatus, title: &str, cells: &[(String, bool)], last: Option<usize>) {
    pick_left_rows(fb, ui, st, title, cells, last, true)
}

/// The same, with the first two rows told apart from the rest (what comes with the machine) or not.
pub fn pick_left_rows(fb: &mut FrameBuffer480x272, ui: &crate::UiState, st: &EngineStatus, title: &str, cells: &[(String, bool)], last: Option<usize>, factory_rows: bool) {
    let t = theme();
    fb.clear(t.bg);
    bar(fb, title);
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
        let (name, has) = cells.get(i).cloned().unwrap_or_default();
        let factory = factory_rows && i < 8;
        let last = last == Some(i);
        if has {
            fill_round_rect(fb, x, y, CELL_W, CELL_H, 8.0, if factory { t.panel } else { t.faint });
        }
        let col = if last { t.fg } else if has { t.dim } else { t.faint };
        stroke_round_rect(fb, x, y, CELL_W, CELL_H, 8.0, if last { 2.0 } else { 1.0 }, col);
        let marked = ui.confirm == Some(i);
        // the place ERASE has marked is red, with the ink that reads on red
        let (label, ink) = if marked {
            let (fill, ink) = op8_gfx::readable(t.rec);
            fill_round_rect(fb, x, y, CELL_W, CELL_H, 8.0, fill);
            ("erase?".to_string(), ink)
        } else {
            (if name.is_empty() { "empty".to_string() } else { name }, if has { t.fg } else { t.dim })
        };
        text_v(fb, &label, (x + CELL_W / 2.0) as i32, (y + CELL_H / 2.0) as i32, FontSize::Medium, ink, Align::Center, VAlign::Middle);
    }
    if let Some(it) = crate::nav::focused(ui).filter(|it| it.screen == 0) {
        crate::nav::draw_ring(fb, &it, ui);
    }
}

pub fn draw_right(fb: &mut FrameBuffer480x272, ui: &crate::UiState) {
    pick_right(fb, "projects", &[("tap a pad", "opens it"), ("shift + tap", "saves into your slots"), ("erase + tap twice", "erases one of yours")], &ui.file_note);
}

/// The right screen of a picker: its title, what the pads do, and what just happened.
pub fn pick_right(fb: &mut FrameBuffer480x272, title: &str, lines: &[(&str, &str)], note: &str) {
    let t = theme();
    fb.clear(t.bg);
    bar(fb, title);
    for (i, (a, b)) in lines.iter().enumerate() {
        let y = 76 + i as i32 * 50;
        text_v(fb, a, 30, y, FontSize::Large, t.fg, Align::Left, VAlign::Middle);
        text_v(fb, b, 30, y + 24, FontSize::Medium, t.dim, Align::Left, VAlign::Middle);
    }
    if !note.is_empty() {
        text_v(fb, note, 30, 236, FontSize::Large, t.fg, Align::Left, VAlign::Middle);
    }
}
