//! Settings: the main mode that takes both screens (design.md §4).
//!
//! Left: one card per theme, each drawn in that theme's own colours (its eight
//! brand keys, the backlight it asks for and its LED level), the active one outlined. Right: the eight encoder colours as dials and the track
//! colours, in the active theme. The theme applies live, so the screens are
//! their own preview. A control that does nothing here shows nothing (rule 11).

use crate::theme::{dial, TRACK_COLORS};
use op8_engine::EngineStatus;
use op8_gfx::{
    knob,
    paint::{fill_circle, fill_convex, fill_round_rect, stroke_round_rect},
    text, text_v, theme, theme_id, theme_of, Align, FontSize, FrameBuffer480x272, ThemeId, VAlign,
};
use std::sync::atomic::Ordering::Relaxed;

const CARD_W: f32 = 136.0;
const CARD_H: f32 = 160.0;
const CARD_GAP: f32 = 18.0;
const CARD_Y: f32 = 62.0;

/// Title with a dot, at the top left of both settings screens.
fn bar(fb: &mut FrameBuffer480x272, title: &str) {
    let t = theme();
    fill_circle(fb, 16.0, 15.0, 6.0, t.fg);
    text_v(fb, title, 30, 15, FontSize::Large, t.fg, Align::Left, VAlign::Middle);
}

pub fn draw_left(fb: &mut FrameBuffer480x272, st: &EngineStatus) {
    let t = theme();
    fb.clear(t.bg);
    bar(fb, "settings");
    // Transport stays visible: PLAY keeps working here.
    if st.playing.load(Relaxed) {
        fill_convex(fb, &[(460.0, 8.0), (470.0, 14.0), (460.0, 20.0)], t.fg);
    } else {
        fill_round_rect(fb, 460.0, 9.0, 10.0, 10.0, 1.5, t.dim);
    }
    let x0 = (480.0 - (3.0 * CARD_W + 2.0 * CARD_GAP)) / 2.0;
    let active = theme_id();
    for (n, id) in ThemeId::ALL.into_iter().enumerate() {
        let c = theme_of(id);
        let on = id == active;
        let x = x0 + n as f32 * (CARD_W + CARD_GAP);
        fill_round_rect(fb, x, CARD_Y, CARD_W, CARD_H, 8.0, c.panel);
        let (w, col) = if on { (2.0, t.fg) } else { (1.0, t.faint) };
        stroke_round_rect(fb, x, CARD_Y, CARD_W, CARD_H, 8.0, w, col);
        // The theme's eight colours as the brand keys, four and four.
        for (i, k) in c.brand.iter().enumerate() {
            let (kx, ky) = (x + 14.0 + (i % 4) as f32 * 28.0, CARD_Y + 20.0 + (i / 4) as f32 * 18.0);
            fill_round_rect(fb, kx, ky, 24.0, 13.0, 3.0, *k);
        }
        // Its light: the backlight it asks for, then the level of the pad LEDs.
        let track_w = CARD_W - 28.0;
        fill_round_rect(fb, x + 14.0, CARD_Y + 72.0, track_w, 5.0, 2.0, c.faint);
        fill_round_rect(fb, x + 14.0, CARD_Y + 72.0, (track_w * c.light.backlight).max(4.0), 5.0, 2.0, c.fg);
        for lvl in 0..3u8 {
            let col = if lvl < c.light.led_level { c.fg } else { c.faint };
            fill_round_rect(fb, x + 14.0 + lvl as f32 * 16.0, CARD_Y + 90.0, 12.0, 12.0, 3.0, col);
        }
        text(
            fb,
            c.name,
            (x + CARD_W / 2.0) as i32,
            (CARD_Y + 128.0) as i32,
            FontSize::Medium,
            if on { t.fg } else { t.dim },
            Align::Center,
        );
    }
}

pub fn draw_right(fb: &mut FrameBuffer480x272) {
    let t = theme();
    fb.clear(t.bg);
    bar(fb, "theme");
    // The eight encoders in this theme's colours (values only suggest a dial).
    const V: [f32; 8] = [0.35, 0.7, 0.5, 0.85, 0.45, 0.75, 0.3, 0.6];
    for (i, v) in V.into_iter().enumerate() {
        let (cx, cy) = (60.0 + (i % 4) as f32 * 120.0, 96.0 + (i / 4) as f32 * 66.0);
        dial(fb, cx, cy, 20.0, v, knob(i));
    }
    // The track colours, which do not change with the theme.
    for (i, c) in TRACK_COLORS.iter().enumerate() {
        fill_circle(fb, 240.0 + (i as f32 - 3.5) * 40.0, 224.0, 8.0, *c);
    }
}
