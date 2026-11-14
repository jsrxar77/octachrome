//! The pages of buttons 3, 4, 7 and 8: players and bar on the left screen, the track's effect and
//! its LFO on the right. Each is a picture that explains what the knobs do, and a footer for the four knobs.

use crate::{FooterCell, UiState, QUANTIZE};
use op8_gfx::{
    knob,
    paint::{fill_circle, fill_round_rect, line_aa, path, ring, stroke_round_rect},
    text_v, theme, Align, FontSize, FrameBuffer480x272, Rgb565, VAlign,
};
use op8_seq::{FxKind, LfoShape, PlayMode, Project};
use std::f32::consts::TAU;

// ---- footers ---------------------------------------------------------------------------------

/// Knobs 1–4 on the players page.
pub fn players_footer(ui: &UiState) -> [FooterCell; 4] {
    let p = &ui.players;
    if ui.kit {
        return [FooterCell::new("play", "off", None), FooterCell::empty(), FooterCell::empty(), FooterCell::empty()];
    }
    let mode = PlayMode::ALL.iter().position(|m| *m == p.mode).unwrap_or(0);
    let chord = op8_seq::ChordType::ALL.iter().position(|c| *c == p.chord).unwrap_or(0);
    let speed = if p.mode.is_arp() {
        FooterCell::new("rate", format!("{}/step", p.arp_count()), Some(p.speed as f32 / 4.0))
    } else if p.mode == PlayMode::Chord {
        FooterCell::new("strum", if p.speed == 0 { "together".into() } else { format!("{}", p.speed) }, Some(p.speed as f32 / 4.0))
    } else {
        FooterCell::empty()
    };
    [
        FooterCell::new("play", p.mode.label(), Some(mode as f32 / 5.0)),
        FooterCell::new("chord", p.chord.label(), Some(chord as f32 / 6.0)),
        speed,
        FooterCell::new("octaves", p.octaves.to_string(), Some((p.octaves as f32 - 1.0) / 2.0)),
    ]
}

/// Knobs 1–4 on the bar page.
pub fn bar_footer(ui: &UiState, p: &Project) -> [FooterCell; 4] {
    let t = &p.tracks[ui.track];
    [
        FooterCell::new("length", t.len().to_string(), Some(t.len() as f32 / 16.0)).with_glyph(crate::FooterGlyph::Length(t.len() as u8)),
        FooterCell::new("division", t.division.label(), Some(t.division as usize as f32 / 4.0)),
        FooterCell::new("shift", "turn", None),
        FooterCell::new("rec rounds", QUANTIZE[ui.quantize.min(2)], Some(ui.quantize as f32 / 2.0)),
    ]
}

/// Knobs 5–8 on the fx page.
pub fn fx_footer(ui: &UiState) -> [FooterCell; 4] {
    let fx = &ui.track_fx;
    let names = fx.kind.knobs();
    let k = fx.knobs();
    let kind = FxKind::ALL.iter().position(|x| *x == fx.kind).unwrap_or(0);
    let cell = |i: usize| {
        if fx.kind == FxKind::Off {
            FooterCell::empty()
        } else {
            FooterCell::new(names[i], format!("{:.0}%", k[i] * 100.0), Some(k[i]))
        }
    };
    [FooterCell::new("type", fx.kind.label(), Some(kind as f32 / 5.0)), cell(0), cell(1), cell(2)]
}

/// Knobs 5–8 on the mod page.
pub fn lfo_footer(ui: &UiState) -> [FooterCell; 4] {
    let l = &ui.lfo;
    let shape = LfoShape::ALL.iter().position(|x| *x == l.shape).unwrap_or(0);
    let hz = l.hz();
    [
        FooterCell::new("shape", l.shape.label(), Some(shape as f32 / 4.0)),
        FooterCell::new("rate", if hz < 1.0 { format!("{hz:.2}Hz") } else { format!("{hz:.1}Hz") }, Some(l.rate as f32 / 255.0)),
        FooterCell::new("depth", format!("{:.0}%", l.depth_f() * 100.0), Some(l.depth_f())),
        FooterCell::new("moves", ui.lfo_target.clone(), Some(l.target as f32 / (op8_seq::LFO_TARGETS - 1) as f32)),
    ]
}

// ---- the left screen -------------------------------------------------------------------------

const KEY_X: f32 = 30.0;
const KEY_W: f32 = 30.0;
const KEY_Y: f32 = 96.0;
const KEY_H: f32 = 104.0;

fn is_black(pc: i32) -> bool {
    matches!(pc.rem_euclid(12), 1 | 3 | 6 | 8 | 10)
}

/// The two octaves from C3: where the key for semitone `s` (0..24) is, and whether it is a black one.
fn key_rect(s: i32) -> (f32, f32, f32, bool) {
    let whites_before = (0..s).filter(|&i| !is_black(i)).count() as f32;
    if is_black(s) {
        let w = KEY_W * 0.6;
        (KEY_X + whites_before * KEY_W - w / 2.0, w, KEY_H * 0.6, true)
    } else {
        (KEY_X + whites_before * KEY_W, KEY_W, KEY_H, false)
    }
}

/// Button 3: a keyboard that shows the chord the players make on the note of the first step.
pub fn players_view(fb: &mut FrameBuffer480x272, ui: &UiState, p: &Project, accent: Rgb565) {
    let t = theme();
    let track = &p.tracks[ui.track];
    let pl = &ui.players;
    let root = track.steps.iter().take(track.len()).find(|s| s.active).map_or(48, |s| s.note);
    let what = if ui.kit {
        "a drum kit plays its lanes".to_string()
    } else if pl.mode == PlayMode::Off {
        "off: each step plays its note".to_string()
    } else {
        format!("{} · {}", pl.mode.label(), pl.chord.label())
    };
    text_v(fb, &what, 240, 62, FontSize::Large, t.dim, Align::Center, VAlign::Middle);
    let mut notes = [0u8; op8_seq::rack::MAX_PLAYED];
    let n = if ui.kit || pl.mode == PlayMode::Off {
        notes[0] = root;
        1
    } else {
        pl.chord_notes(root, &mut notes)
    };
    // an arpeggio walks through its notes; a chord lights them all
    let walking = pl.mode.is_arp() && !ui.kit;
    let now = (ui.anim.frame / 7) as usize % n.max(1);
    let lit = |s: i32| -> u8 {
        let mut level = 0;
        for (i, &nt) in notes[..n].iter().enumerate() {
            if (nt as i32 - 48).rem_euclid(24) == s {
                level = level.max(if walking && i == now { 2 } else { 1 });
            }
        }
        level
    };
    // white keys first, the black ones over them
    for black in [false, true] {
        for s in 0..24 {
            let (x, w, h, b) = key_rect(s);
            if b != black {
                continue;
            }
            let fill = match lit(s) {
                2 => t.fg,
                1 => accent,
                _ if b => t.bg,
                _ => t.panel,
            };
            fill_round_rect(fb, x + 1.0, KEY_Y, w - 2.0, h, 4.0, fill);
            if lit(s) == 0 {
                stroke_round_rect(fb, x + 1.0, KEY_Y, w - 2.0, h, 4.0, 1.0, t.faint);
            }
        }
    }
}

/// Button 4: the track as a bar of steps, its beats marked, with the playhead.
pub fn bar_view(fb: &mut FrameBuffer480x272, ui: &UiState, p: &Project, accent: Rgb565) {
    let t = theme();
    let tr = &p.tracks[ui.track];
    let len = tr.len();
    let per_beat = (96 / tr.division.ticks()).max(1) as usize;
    let (x0, cw, y) = (24.0, 27.0, 100.0);
    for i in 0..16usize {
        let x = x0 + i as f32 * cw;
        let inside = i < len;
        let beat = i % per_beat == 0;
        let top = if beat { y - 10.0 } else { y };
        let fill = if !inside {
            t.bg
        } else if tr.steps[i].active {
            accent
        } else {
            t.panel
        };
        fill_round_rect(fb, x + 1.5, top, cw - 3.0, 54.0 + y - top, 4.0, fill);
        if !inside || !tr.steps[i].active {
            stroke_round_rect(fb, x + 1.5, top, cw - 3.0, 54.0 + y - top, 4.0, 1.0, if inside { t.faint } else { t.panel });
        }
    }
    let ph = ui.anim.ph[ui.track];
    let px = x0 + ph * cw + cw / 2.0;
    fill_circle(fb, px, y + 70.0, 3.5, t.fg);
    let beats = len as f32 * tr.division.ticks() as f32 / 96.0;
    let text = format!("{len} steps of {} = {:.2} beats", tr.division.label(), beats);
    text_v(fb, &text, 240, 62, FontSize::Large, t.dim, Align::Center, VAlign::Middle);
    text_v(fb, "the tall steps are the beats", 240, 188, FontSize::Medium, t.faint, Align::Center, VAlign::Middle);
}

// ---- the right screen ------------------------------------------------------------------------

const AX: f32 = 40.0;
const AW: f32 = 400.0;
const AY: f32 = 66.0;
const AH: f32 = 140.0;

/// Button 7: a picture of the effect, moving.
pub fn fx_view(fb: &mut FrameBuffer480x272, ui: &UiState) {
    let t = theme();
    let fx = &ui.track_fx;
    let [a, b, c] = fx.knobs();
    let frame = ui.anim.frame as f32;
    let mid = AY + AH / 2.0;
    text_v(fb, fx.kind.label(), 240, 50, FontSize::Large, t.dim, Align::Center, VAlign::Middle);
    match fx.kind {
        FxKind::Off => {
            ring(fb, 240.0, mid, 40.0, 2.0, t.faint);
            line_aa(fb, 212.0, mid + 28.0, 268.0, mid - 28.0, 2.0, t.faint);
            text_v(fb, "turn the first knob to choose an effect", 240, (AY + AH - 6.0) as i32, FontSize::Medium, t.faint, Align::Center, VAlign::Middle);
        }
        FxKind::Delay => {
            // the sound, then its echoes: the gap is the time, each is as tall as the feedback leaves it
            let gap = 22.0 + a * 60.0;
            let mut h = 80.0;
            let mut x = AX + 12.0;
            let beat = (frame / 5.0) as usize % 6;
            let mut i = 0;
            while x < AX + AW && h > 4.0 {
                let col = if i == 0 { t.fg } else { knob(5) };
                let lit = i == beat;
                let hh = if lit { h * 1.08 } else { h };
                fill_round_rect(fb, x - 5.0, mid + 40.0 - hh, 10.0, hh, 4.0, if lit { t.fg } else { col.scale(if i == 0 { 1.0 } else { 0.35 + 0.65 * c }) });
                h *= 0.25 + 0.7 * b;
                x += gap;
                i += 1;
            }
        }
        FxKind::Reverb => {
            // rings spreading out: the size is how far, the tone how crisp, the mix how bright
            let n = 6;
            for i in 0..n {
                let ph = ((frame / 70.0) + i as f32 / n as f32).fract();
                let r = 8.0 + ph * (30.0 + a * 62.0);
                let w = 1.0 + (1.0 - ph) * 2.0 * b;
                ring(fb, 240.0, mid + 4.0, r, w, knob(5).scale((1.0 - ph) * (0.25 + 0.75 * c)));
            }
            fill_circle(fb, 240.0, mid + 4.0, 5.0, t.fg);
        }
        FxKind::Chorus => {
            // two copies of one wave drifting apart: the depth is how far, the rate how fast
            let speed = 0.02 + a * 0.2;
            for (k, col) in [(0.0, t.fg), (1.0, knob(5))] {
                let pts: Vec<(f32, f32)> = (0..=80)
                    .map(|i| {
                        let x = AX + AW * i as f32 / 80.0;
                        let drift = k * (0.35 * b * (frame * speed).sin() + 0.1);
                        (x, mid + (i as f32 / 80.0 * TAU * 3.0 + frame * 0.05 + drift * TAU).sin() * 38.0)
                    })
                    .collect();
                path(fb, &pts, 2.0, false, col.scale(if k == 0.0 { 1.0 } else { 0.3 + 0.7 * c }));
            }
        }
        FxKind::Filter => {
            // the response curve: a low-pass bends down on the right, a high-pass on the left; the peak is the resonance
            let low = a < 0.5;
            let pos = if low { a * 2.0 } else { (a - 0.5) * 2.0 };
            let cut = if low { 0.15 + pos * 0.8 } else { 0.05 + pos * 0.8 };
            let pts: Vec<(f32, f32)> = (0..=120)
                .map(|i| {
                    let f = i as f32 / 120.0;
                    let d = (f - cut) * 9.0;
                    let slope = if low { 1.0 / (1.0 + (d.max(0.0)).powi(2)) } else { 1.0 / (1.0 + ((-d).max(0.0)).powi(2)) };
                    let peak = b * 0.6 * (-(d * d) * 2.0).exp();
                    (AX + AW * f, AY + AH - 14.0 - (slope + peak).min(1.5) * 80.0)
                })
                .collect();
            path(fb, &pts, 3.0, false, knob(5).scale(0.4 + 0.6 * c));
            let x = AX + AW * cut;
            line_aa(fb, x, AY + 10.0, x, AY + AH, 1.0, t.faint);
            let sweep = (frame * 0.03).sin() * 0.5 + 0.5;
            fill_circle(fb, AX + AW * sweep, AY + AH - 8.0, 3.0, t.dim);
        }
        FxKind::Drive => {
            // the curve that bends a wave: the harder it is driven, the flatter its tops
            let gain = 1.0 + a * 24.0;
            let curve: Vec<(f32, f32)> = (0..=80)
                .map(|i| {
                    let x = i as f32 / 80.0 * 2.0 - 1.0;
                    (240.0 + x * 60.0, mid - (x * gain).tanh() / gain.tanh() * 60.0)
                })
                .collect();
            path(fb, &curve, 2.0, false, t.faint);
            let ph = (frame * 0.04).fract();
            let x = (ph * TAU).sin();
            let y = (x * gain).tanh() / gain.tanh();
            fill_circle(fb, 240.0 + x * 60.0, mid - y * 60.0, 5.0, knob(5).scale(0.4 + 0.6 * c));
            let wave: Vec<(f32, f32)> = (0..=60)
                .map(|i| {
                    let f = i as f32 / 60.0;
                    let s = ((f + ph) * TAU * 1.5).sin();
                    (AX + 330.0 + f * 70.0 - 70.0 + 10.0, mid - ((s * gain).tanh() / gain.tanh()) * 40.0 * (0.4 + 0.6 * (1.0 - b * 0.5)))
                })
                .collect();
            path(fb, &wave, 2.0, false, knob(5).scale(0.5 + 0.5 * c));
        }
    }
}

/// Button 8: the wave of the LFO going by, and what it moves.
pub fn lfo_view(fb: &mut FrameBuffer480x272, ui: &UiState) {
    let t = theme();
    let l = &ui.lfo;
    let mid = AY + AH / 2.0 - 4.0;
    let depth = l.depth_f();
    let amp = 8.0 + depth * 50.0;
    text_v(fb, &format!("moves {}", ui.lfo_target), 240, 50, FontSize::Large, t.dim, Align::Center, VAlign::Middle);
    line_aa(fb, AX, mid, AX + AW, mid, 1.0, t.faint);
    // two cycles across; a random wave is drawn as the steps of a held value
    let cycles = 2.0;
    let pts: Vec<(f32, f32)> = (0..=160)
        .map(|i| {
            let f = i as f32 / 160.0;
            let ph = (f * cycles).fract();
            let v = match l.shape {
                LfoShape::Random => {
                    let cyc = (f * cycles) as u32;
                    ((cyc.wrapping_mul(2654435761) >> 8) % 200) as f32 / 100.0 - 1.0
                }
                s => s.at(ph),
            };
            (AX + AW * f, mid - v * amp)
        })
        .collect();
    path(fb, &pts, 2.5, false, knob(5).scale(if l.target == 0 { 0.3 } else { 1.0 }));
    // the dot runs along at the rate of the LFO
    let ph = (ui.anim.frame as f32 * l.hz() / 60.0).fract();
    let v = match l.shape {
        LfoShape::Random => 0.0,
        s => s.at(ph),
    };
    fill_circle(fb, AX + AW * ph / cycles, mid - v * amp, 5.0, t.fg);
    if l.target == 0 {
        text_v(fb, "choose what it moves with the last knob", 240, (AY + AH - 6.0) as i32, FontSize::Medium, t.faint, Align::Center, VAlign::Middle);
    }
}
