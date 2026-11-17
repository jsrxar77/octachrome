//! MIXER: the main mode that takes both screens (design.md §4, "Mixer").
//!
//! **Left:** four channels in the four columns of the knobs under the screen, each a vertical fader (the level, in the colour of its
//! track) with its pan under it and the track's letter on top; the buttons 1 and 2 choose which four (`a–d` and `e–h`). The knobs under
//! the screen are their levels, and with SHIFT their pans.
//!
//! **Right:** the compressor on the master as a curve (what comes in across, what comes out up) with a dot where the sound is now and
//! a bar that says how much it is being brought down; buttons 5 and 6 choose its two pages, `comp` (threshold, ratio, attack,
//! release) and `output` (make-up gain, mix); button 7 is the third page, `eq`: the curve of the EQ of the master across the screen, with
//! the gains of the low and high shelves, the middle band's gain and its frequency under it. The values are under the screen, in the
//! colours of their knobs.
//!
//! A control that does nothing here shows nothing (rule 11).

use crate::theme::{button_strip, track_color, FOOTER_COL, HEADER_CY, TRACK_NAMES};
use op8_engine::{comp, eq, EngineStatus};
use op8_gfx::{
    knob,
    paint::{fill_circle, fill_round_rect, line_aa, ring, stroke_round_rect},
    text_v, theme, Align, FontSize, FrameBuffer480x272, VAlign,
};
use op8_seq::{MasterComp, MasterEq, Project};
use std::sync::atomic::Ordering::Relaxed;

/// What the mixer shows and the controller keeps.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MixerView {
    /// The page of the right screen: 0 `comp`, 1 `output`, 2 `eq`.
    pub page: u8,
    /// The four channels on the left: 0 is A to D, 1 is E to H.
    pub group: u8,
    /// The compressor's settings, how far it is bringing the sound down (dB, 0 or less) and how loud the sound going in is (dB): the
    /// controller keeps them from the project and the engine's status, so that the screen needs neither.
    pub comp: MasterComp,
    /// The EQ's settings, kept the same way.
    pub eq: MasterEq,
    pub reduction: f32,
    pub input: f32,
}

impl Default for MixerView {
    fn default() -> Self {
        Self { page: 0, group: 0, comp: MasterComp::default(), eq: MasterEq::default(), reduction: 0.0, input: -90.0 }
    }
}

/// The curve's box on the right screen: its left, top and side.
const CURVE: (f32, f32, f32) = (30.0, 52.0, 150.0);
/// The range of the curve, in dB, on both axes.
const FLOOR_DB: f32 = -48.0;

/// How many values the page has under the right screen (the second page has two).
pub fn page_values(page: u8) -> usize {
    match page {
        1 => 2,
        _ => 4,
    }
}

/// The compressor setting that knob `k` (4..8, the knobs under the right screen) edits on `page`, if there is one.
pub fn comp_index(page: u8, k: usize) -> Option<usize> {
    if page > 1 || k < 4 || k - 4 >= page_values(page) {
        return None;
    }
    Some(if page == 0 { k - 4 } else { 4 + k - 4 })
}

/// The EQ setting that knob `k` edits, if the page is the EQ's: low, mid, freq, high.
pub fn eq_index(page: u8, k: usize) -> Option<usize> {
    (page == 2 && (4..8).contains(&k)).then(|| k - 4)
}

/// The buttons over the screens in MIXER: 1 and 2 choose the channels, 5 and 6 the page of the compressor.
pub fn buttons(m: &MixerView) -> [Option<(&'static str, bool)>; 8] {
    let mut b = [None; 8];
    b[0] = Some(("a–d", m.group == 0));
    b[1] = Some(("e–h", m.group == 1));
    b[4] = Some(("comp", m.page == 0));
    b[5] = Some(("output", m.page == 1));
    b[6] = Some(("eq", m.page == 2));
    b
}

fn bar(fb: &mut FrameBuffer480x272, title: &str) {
    let t = theme();
    fill_circle(fb, 16.0, HEADER_CY, 6.0, t.fg);
    text_v(fb, title, 30, HEADER_CY as i32, FontSize::Large, t.fg, Align::Left, VAlign::Middle);
}

/// The box of the fader of channel column `c` (0..4): where the level is drawn and where the focus rings it.
pub fn fader_rect(c: usize) -> (f32, f32, f32, f32) {
    (c as f32 * FOOTER_COL as f32 + 6.0, 44.0, FOOTER_COL as f32 - 12.0, 176.0)
}

pub fn draw_left(fb: &mut FrameBuffer480x272, ui: &crate::UiState, p: &Project, st: &EngineStatus) {
    let t = theme();
    fb.clear(t.bg);
    button_strip(fb, &crate::strip_labels(ui, 0));
    bar(fb, "mixer");
    if st.playing.load(Relaxed) {
        op8_gfx::paint::fill_convex(fb, &[(460.0, HEADER_CY - 7.0), (470.0, HEADER_CY - 1.0), (460.0, HEADER_CY + 5.0)], t.fg);
    } else {
        fill_round_rect(fb, 460.0, HEADER_CY - 6.0, 10.0, 10.0, 1.5, t.dim);
    }
    for c in 0..4 {
        let i = ui.mixer.group as usize * 4 + c;
        let track = &p.tracks[i];
        let cx = c as f32 * FOOTER_COL as f32 + FOOTER_COL as f32 / 2.0;
        // the letter of the track, in its colour
        text_v(fb, TRACK_NAMES[i], cx as i32 - 14, 62, FontSize::Value, track_color(i), Align::Center, VAlign::Middle);
        if track.muted {
            text_v(fb, "mute", cx as i32 + 8, 62, FontSize::Small, t.dim, Align::Left, VAlign::Middle);
        }
        // the fader: a rounded slot with the level filled from below
        let (fx, fy, fw, fh) = (cx - 9.0, 78.0, 18.0, 96.0);
        fill_round_rect(fb, fx, fy, fw, fh, 9.0, t.faint);
        let lv = (track.level as f32 / 127.0).clamp(0.0, 1.0);
        let h = (fh * lv).max(if track.level > 0 { 6.0 } else { 0.0 });
        if h > 0.0 {
            fill_round_rect(fb, fx, fy + fh - h, fw, h, 9.0, if track.muted { t.dim } else { track_color(i) });
        }
        // the pan: a short line with a dot, centre marked
        let (px0, px1, py) = (cx - 36.0, cx + 36.0, 192.0);
        line_aa(fb, px0, py, px1, py, 1.0, t.faint);
        line_aa(fb, cx, py - 4.0, cx, py + 4.0, 1.0, t.dim);
        fill_circle(fb, px0 + (px1 - px0) * track.pan as f32 / 127.0, py, 4.0, t.fg);
        // under the screen: what the knob turns, with SHIFT or without
        let k = knob(c);
        let (name, value) = if ui.shift { ("pan", pan_text(track.pan)) } else { ("level", format!("{}%", (track.level as f32 / 127.0 * 100.0).round())) };
        text_v(fb, name, cx as i32, 236, FontSize::Small, t.dim, Align::Center, VAlign::Middle);
        text_v(fb, &value, cx as i32, 254, FontSize::Medium, k, Align::Center, VAlign::Middle);
    }
    if let Some(it) = crate::nav::focused(ui).filter(|it| it.screen == 0 && ui.main == crate::Main::Mixer) {
        crate::nav::draw_ring(fb, &it, ui);
    }
}

/// A pan as text: `c` for the centre, else the side and how far.
pub fn pan_text(pan: u8) -> String {
    let d = pan as i32 - 64;
    match d {
        0 => "c".to_string(),
        d if d < 0 => format!("l{}", -d * 100 / 64),
        d => format!("r{}", d * 100 / 63),
    }
}

/// Where a level in dB (clamped to the box) is, as a fraction of the box: 0 at the floor, 1 at 0 dB.
fn frac(db: f32) -> f32 {
    ((db - FLOOR_DB) / -FLOOR_DB).clamp(0.0, 1.0)
}

/// The output of the curve for an input in dB, with the make-up gain and the mix as they sound (the mix is of the levels, not of the dB).
pub fn curve_out(in_db: f32, s: &MasterComp) -> f32 {
    let wet = in_db + comp::reduction_db(in_db, comp::threshold_db(s.threshold), comp::ratio(s.ratio)) + comp::makeup_db(s.makeup);
    let m = comp::mix(s.mix);
    let lin = (1.0 - m) * 10f32.powf(in_db / 20.0) + m * 10f32.powf(wet / 20.0);
    20.0 * lin.max(1e-6).log10()
}

pub fn draw_right(fb: &mut FrameBuffer480x272, ui: &crate::UiState) {
    if ui.mixer.page == 2 {
        return draw_eq(fb, ui);
    }
    let t = theme();
    let s = &ui.mixer.comp;
    fb.clear(t.bg);
    button_strip(fb, &crate::strip_labels(ui, 1));
    bar(fb, "compressor");
    let (x0, y0, side) = CURVE;
    // the box, the straight line of a sound that is left alone, and the curve
    fill_round_rect(fb, x0, y0, side, side, 6.0, t.panel);
    let at = |in_db: f32, out_db: f32| (x0 + side * frac(in_db), y0 + side * (1.0 - frac(out_db)));
    let (ax, ay) = at(FLOOR_DB, FLOOR_DB);
    let (bx, by) = at(0.0, 0.0);
    line_aa(fb, ax, ay, bx, by, 1.0, t.faint);
    let steps = 48;
    let mut prev = at(FLOOR_DB, curve_out(FLOOR_DB, s));
    for k in 1..=steps {
        let in_db = FLOOR_DB + -FLOOR_DB * k as f32 / steps as f32;
        let now = at(in_db, curve_out(in_db, s));
        line_aa(fb, prev.0, prev.1, now.0, now.1, 2.0, t.fg);
        prev = now;
    }
    text_v(fb, "in", (x0 + side) as i32, (y0 + side + 14.0) as i32, FontSize::Small, t.dim, Align::Right, VAlign::Middle);
    text_v(fb, "out", (x0 + 6.0) as i32, (y0 + 10.0) as i32, FontSize::Small, t.dim, Align::Left, VAlign::Middle);
    // the threshold, on the way in, in the colour of its knob
    let thr = comp::threshold_db(s.threshold);
    let tx = x0 + side * frac(thr);
    line_aa(fb, tx, y0 + side, tx, y0 + side + 8.0, 2.0, knob(4));
    // where the sound is now: a dot on the curve
    let input = ui.mixer.input;
    if input > FLOOR_DB {
        let (dx, dy) = at(input, curve_out(input, s));
        fill_circle(fb, dx, dy, 4.0, t.fg);
    }
    // how much it is bringing down: a bar from the left, in dB, to 24
    let (rx, ry, rw) = (216.0f32, 100.0f32, 234.0f32);
    text_v(fb, "reduction", rx as i32, ry as i32 - 16, FontSize::Small, t.dim, Align::Left, VAlign::Middle);
    fill_round_rect(fb, rx, ry, rw, 8.0, 4.0, t.faint);
    let red = (-ui.mixer.reduction).clamp(0.0, 24.0);
    if red > 0.05 {
        fill_round_rect(fb, rx, ry, (rw * red / 24.0).max(8.0), 8.0, 4.0, t.fg);
    }
    text_v(fb, &format!("{red:.1} dB"), (rx + rw) as i32, ry as i32 + 24, FontSize::Medium, t.dim, Align::Right, VAlign::Middle);
    if s.threshold == 255 && s.makeup == 0 {
        text_v(fb, "it does nothing yet: lower the threshold", rx as i32, 160, FontSize::Small, t.dim, Align::Left, VAlign::Middle);
    }
    // the values under the screen
    let values: Vec<(&str, String)> = if ui.mixer.page == 0 {
        vec![
            ("threshold", format!("{:.0} dB", comp::threshold_db(s.threshold))),
            ("ratio", format!("{:.1}:1", comp::ratio(s.ratio))),
            ("attack", ms_text(comp::attack_ms(s.attack))),
            ("release", ms_text(comp::release_ms(s.release))),
        ]
    } else {
        vec![("make-up", format!("+{:.0} dB", comp::makeup_db(s.makeup))), ("mix", format!("{:.0}%", comp::mix(s.mix) * 100.0))]
    };
    for (i, (name, v)) in values.iter().enumerate() {
        let x = 60 + i as i32 * 120;
        text_v(fb, name, x, 236, FontSize::Small, t.dim, Align::Center, VAlign::Middle);
        text_v(fb, v, x, 254, FontSize::Medium, knob(4 + i), Align::Center, VAlign::Middle);
    }
    if let Some(it) = crate::nav::focused(ui).filter(|it| it.screen == 1 && ui.main == crate::Main::Mixer) {
        crate::nav::draw_ring(fb, &it, ui);
    }
    let _ = (ring, stroke_round_rect);
}

/// The EQ's page: the response of the three bands across the screen, on a scale of frequency that is logarithmic, with a dot where each
/// band's gain is set and the values under the screen.
fn draw_eq(fb: &mut FrameBuffer480x272, ui: &crate::UiState) {
    let t = theme();
    let s = &ui.mixer.eq;
    fb.clear(t.bg);
    button_strip(fb, &crate::strip_labels(ui, 1));
    bar(fb, "eq");
    let (x0, y0, w, h) = (30.0f32, 52.0f32, 420.0f32, 130.0f32);
    let (lo, hi) = (20.0f32.ln(), 20_000.0f32.ln());
    let fx = |hz: f32| x0 + w * ((hz.ln() - lo) / (hi - lo)).clamp(0.0, 1.0);
    let gy = |db: f32| y0 + h / 2.0 - h / 2.0 * (db / 15.0).clamp(-1.0, 1.0);
    fill_round_rect(fb, x0, y0, w, h, 6.0, t.panel);
    // the zero line and a line at each decade, with its number
    line_aa(fb, x0, gy(0.0), x0 + w, gy(0.0), 1.0, t.faint);
    for (hz, label) in [(100.0f32, "100"), (1_000.0, "1k"), (10_000.0, "10k")] {
        line_aa(fb, fx(hz), y0 + 4.0, fx(hz), y0 + h - 4.0, 1.0, t.faint);
        text_v(fb, label, fx(hz) as i32, (y0 + h + 10.0) as i32, FontSize::Small, t.dim, Align::Center, VAlign::Middle);
    }
    // the response of the three bands together
    let rate = op8_cartridge::SAMPLE_RATE;
    let steps = 84;
    let mut prev = (x0, gy(eq::response_db(s, 20.0, rate)));
    for k in 1..=steps {
        let hz = (lo + (hi - lo) * k as f32 / steps as f32).exp();
        let now = (fx(hz), gy(eq::response_db(s, hz, rate)));
        line_aa(fb, prev.0, prev.1, now.0, now.1, 2.0, t.fg);
        prev = now;
    }
    // where each band is set: a dot in the colour of its knob (the middle band's frequency has its own mark under the box)
    fill_circle(fb, fx(100.0), gy(eq::gain_db(s.low)), 5.0, knob(4));
    fill_circle(fb, fx(eq::freq_hz(s.freq)), gy(eq::gain_db(s.mid)), 5.0, knob(5));
    fill_circle(fb, fx(8_000.0), gy(eq::gain_db(s.high)), 5.0, knob(7));
    line_aa(fb, fx(eq::freq_hz(s.freq)), y0 + h - 6.0, fx(eq::freq_hz(s.freq)), y0 + h + 2.0, 2.0, knob(6));
    if eq::is_flat(s) {
        text_v(fb, "flat: it does nothing yet", 240, (y0 + h + 28.0) as i32, FontSize::Small, t.dim, Align::Center, VAlign::Middle);
    }
    let db = |v: u8| format!("{:+.0} dB", eq::gain_db(v));
    let hz = eq::freq_hz(s.freq);
    let values = [("low", db(s.low)), ("mid", db(s.mid)), ("freq", if hz >= 1000.0 { format!("{:.1} kHz", hz / 1000.0) } else { format!("{hz:.0} Hz") }), ("high", db(s.high))];
    for (i, (name, v)) in values.iter().enumerate() {
        let x = 60 + i as i32 * 120;
        text_v(fb, name, x, 236, FontSize::Small, t.dim, Align::Center, VAlign::Middle);
        text_v(fb, v, x, 254, FontSize::Medium, knob(4 + i), Align::Center, VAlign::Middle);
    }
    if let Some(it) = crate::nav::focused(ui).filter(|it| it.screen == 1 && ui.main == crate::Main::Mixer) {
        crate::nav::draw_ring(fb, &it, ui);
    }
}

fn ms_text(ms: f32) -> String {
    if ms >= 100.0 {
        format!("{ms:.0} ms")
    } else if ms >= 10.0 {
        format!("{ms:.0} ms")
    } else {
        format!("{ms:.1} ms")
    }
}
