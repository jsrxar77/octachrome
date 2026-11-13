//! Left screen (fixed): time. Encoders 1–4 (design.md §4).
//!
//! Step view: concentric polymeter rings (track A outermost) and a mirror of
//! the 16 pads. Holding a pad opens the inspector; Brain and Punch modes
//! replace the drawing area.

use crate::{
    draw_footer, note_name, theme::*, touch_overlay, FooterCell, FooterGlyph, Mode, UiState,
    COMPONENT_PRESETS,
};
use op3_engine::{EngineStatus, PunchFx, PUNCH_SLOTS};
use op3_gfx::{
    knob,
    paint::{
        arc, fill_circle, fill_convex, fill_round_rect, line_aa, path, ring, stroke_round_rect,
    },
    text_v, theme, Align, FontSize, FrameBuffer480x272, Rgb565, VAlign,
};
use op3_seq::{brain::NOTE_NAMES, BrainMode, Chord, Project, StepComponent, MAX_STEPS, TRACKS};
use std::f32::consts::{FRAC_PI_2, PI, TAU};
use std::sync::atomic::Ordering::Relaxed;

const RING_CX: f32 = 118.0;
const RING_CY: f32 = 126.0;
const RING_R0: f32 = 26.0;
const RING_STEP: f32 = 11.0;
const GRID_X: i32 = 262;
const GRID_Y: i32 = 38;
const CELL: i32 = 40;
const GAP: i32 = 6;

pub fn draw_left(fb: &mut FrameBuffer480x272, ui: &UiState, p: &Project, st: &EngineStatus) {
    let t = theme();
    fb.clear(t.bg);
    let accent = TRACK_COLORS[ui.track];
    top_bar(fb, ui, p, st, accent);
    match (ui.mode, ui.inspect_step) {
        (Mode::Step, Some(step)) => inspector(fb, ui, p, step as usize, accent),
        (Mode::Brain, _) => brain_view(fb, p, st, accent),
        (Mode::Punch, _) => punch_view(fb, ui, accent),
        _ => {
            rings(fb, ui, p, st);
            pad_grid(fb, ui, p, st, accent);
        }
    }
    let cells = left_footer(ui, p);
    draw_footer(fb, &cells, 0);
    touch_overlay(fb, &cells, 0, &ui.anim);
}

fn top_bar(
    fb: &mut FrameBuffer480x272,
    ui: &UiState,
    p: &Project,
    st: &EngineStatus,
    accent: Rgb565,
) {
    let t = theme();
    let playing = st.playing.load(Relaxed);
    if playing {
        fill_convex(fb, &[(10.0, 8.0), (20.0, 14.0), (10.0, 20.0)], t.fg);
    } else {
        fill_round_rect(fb, 10.0, 9.0, 10.0, 10.0, 1.5, t.dim);
    }
    let bpm = format!("{:.0}", p.bpm);
    let end = text_v(
        fb,
        &bpm,
        28,
        14,
        FontSize::Large,
        t.fg,
        Align::Left,
        VAlign::Middle,
    );
    // beat dot: flashes on every quarter
    let beat = if playing && st.playhead[0].load(Relaxed).is_multiple_of(4) {
        ui.anim.hit[0]
    } else {
        0.0
    };
    fill_circle(
        fb,
        end as f32 + 8.0,
        15.0,
        3.0,
        t.fg.scale(0.25 + 0.75 * beat),
    );
    // Brain chord, centred; brightens when it changes
    let chord = if playing {
        st.chord()
    } else {
        p.brain.chord_at(0)
    };
    let c = if p.brain.live.is_some() { accent } else { t.fg };
    let c = c
        .scale(0.8 + 0.2 * ui.anim.chord_pulse.min(1.0))
        .lerp(t.fg, ui.anim.chord_pulse * 0.3);
    text_v(
        fb,
        &chord.name(),
        240,
        14,
        FontSize::Large,
        c,
        Align::Center,
        VAlign::Middle,
    );
    // track dots
    #[allow(clippy::needless_range_loop)]
    for i in 0..TRACKS {
        let (x, y) = (338.0 + i as f32 * 18.0, 15.0);
        let (col, sel, tr) = (TRACK_COLORS[i], i == ui.track, &p.tracks[i]);
        let r = if sel { 6.0 } else { 4.0 } + ui.anim.hit[i] * 2.0;
        let fill = if tr.muted {
            t.faint
        } else if sel {
            col
        } else {
            col.scale(0.55)
        };
        fill_circle(fb, x, y, r, fill);
        if tr.brain_link {
            ring(fb, x, y, if sel { 8.5 } else { 6.5 }, 1.0, col);
        }
    }
}

fn polar(cx: f32, cy: f32, r: f32, a: f32) -> (f32, f32) {
    (cx + a.cos() * r, cy + a.sin() * r)
}

fn rings(fb: &mut FrameBuffer480x272, ui: &UiState, p: &Project, st: &EngineStatus) {
    let t = theme();
    let playing = st.playing.load(Relaxed);
    for i in (0..TRACKS).rev() {
        let tr = &p.tracks[i];
        let (r, len, sel, col) = (
            RING_R0 + (7 - i) as f32 * RING_STEP,
            tr.len(),
            i == ui.track,
            TRACK_COLORS[i],
        );
        ring(
            fb,
            RING_CX,
            RING_CY,
            r,
            if sel { 2.0 } else { 1.0 },
            if sel {
                col.scale(0.35)
            } else {
                t.faint.scale(0.6)
            },
        );
        for s in 0..len {
            let (x, y) = polar(
                RING_CX,
                RING_CY,
                r,
                -FRAC_PI_2 + s as f32 / len as f32 * TAU,
            );
            if tr.steps[s].active {
                let c = if tr.muted {
                    t.faint
                } else if sel {
                    col
                } else {
                    col.scale(0.7)
                };
                fill_circle(fb, x, y, if sel { 3.6 } else { 2.6 }, c);
            } else if sel {
                fill_circle(fb, x, y, 1.2, t.dim);
            }
        }
        if playing {
            let (x, y) = polar(
                RING_CX,
                RING_CY,
                r,
                -FRAC_PI_2 + ui.anim.ph[i] / len as f32 * TAU,
            );
            fill_circle(
                fb,
                x,
                y,
                if sel { 2.4 } else { 1.6 } + ui.anim.hit[i] * 2.2,
                t.fg,
            );
        }
    }
    text_v(
        fb,
        TRACK_NAMES[ui.track],
        RING_CX as i32,
        RING_CY as i32,
        FontSize::Large,
        TRACK_COLORS[ui.track],
        Align::Center,
        VAlign::Middle,
    );
}

/// Pad mirror: step 1 top-left, like the hardware pads.
fn pad_grid(
    fb: &mut FrameBuffer480x272,
    ui: &UiState,
    p: &Project,
    st: &EngineStatus,
    accent: Rgb565,
) {
    let t = theme();
    let tr = &p.tracks[ui.track];
    let playing = st.playing.load(Relaxed);
    let ph = st.playhead[ui.track].load(Relaxed) as usize;
    for s in 0..MAX_STEPS {
        let x = (GRID_X + (s as i32 % 4) * (CELL + GAP)) as f32;
        let y = (GRID_Y + (s as i32 / 4) * (CELL + GAP)) as f32;
        let cs = CELL as f32;
        if s >= tr.len() {
            fill_circle(fb, x + cs / 2.0, y + cs / 2.0, 1.5, t.faint);
            continue;
        }
        let step = &tr.steps[s];
        let is_ph = playing && s == ph;
        if step.active {
            let k = if is_ph {
                1.0
            } else {
                0.45 + 0.55 * step.velocity as f32 / 127.0
            };
            fill_round_rect(fb, x, y, cs, cs, 6.0, accent.scale(k));
            for c in 0..step.components.len() as i32 {
                fb.fill_rect(x as i32 + 6 + c * 5, y as i32 + CELL - 7, 3, 3, t.bg);
            }
        } else {
            stroke_round_rect(fb, x, y, cs, cs, 6.0, 1.0, t.faint);
        }
        if is_ph {
            stroke_round_rect(fb, x - 3.0, y - 3.0, cs + 6.0, cs + 6.0, 8.0, 2.0, t.fg);
        }
    }
}

/// Small picture of a step component in a 34×22 box at (x, y).
pub fn comp_glyph(fb: &mut FrameBuffer480x272, c: StepComponent, x: f32, y: f32, col: Rgb565) {
    let (w, h) = (34.0, 22.0);
    match c {
        StepComponent::None => {}
        StepComponent::Ratchet(n) => {
            let n = n.clamp(1, 8) as f32;
            for i in 0..n as i32 {
                let bw = ((w - 4.0) / n - 3.0).max(2.0);
                fill_round_rect(
                    fb,
                    x + 2.0 + i as f32 * (w - 4.0) / n,
                    y + 3.0,
                    bw,
                    h - 6.0,
                    1.0,
                    col,
                );
            }
        }
        StepComponent::Chance(p) => {
            let (cx, cy) = (x + w / 2.0, y + h / 2.0);
            ring(fb, cx, cy, 9.0, 2.0, col.scale(0.4));
            arc(
                fb,
                cx,
                cy,
                9.0,
                2.0,
                -FRAC_PI_2,
                -FRAC_PI_2 + TAU * p as f32 / 100.0,
                col,
            );
        }
        StepComponent::Every { n, of } => {
            let of = of.clamp(1, 8);
            for i in 0..of {
                let px = x + 6.0 + i as f32 * (w - 12.0) / (of.max(2) - 1) as f32;
                if i + 1 == n {
                    fill_circle(fb, px, y + h / 2.0, 4.0, col);
                } else {
                    ring(fb, px, y + h / 2.0, 3.5, 1.0, col);
                }
            }
        }
        StepComponent::Velocity(v) => {
            let up = v > 0;
            line_aa(
                fb,
                x + 4.0,
                y + if up { h - 4.0 } else { 4.0 },
                x + w - 4.0,
                y + if up { 4.0 } else { h - 4.0 },
                2.0,
                col,
            );
        }
        StepComponent::Transpose(v) | StepComponent::Octave(v) => {
            let n = if matches!(c, StepComponent::Octave(_)) {
                2
            } else {
                1
            };
            let up = v > 0;
            for i in 0..n {
                let ox = x + w / 2.0 - (n - 1) as f32 * 5.0 + i as f32 * 10.0;
                let (tip, base) = if up {
                    (y + 3.0, y + 13.0)
                } else {
                    (y + h - 3.0, y + h - 13.0)
                };
                fill_convex(fb, &[(ox, tip), (ox + 6.0, base), (ox - 6.0, base)], col);
            }
        }
        StepComponent::Hold(_) => {
            fill_round_rect(fb, x + 3.0, y + h / 2.0 - 3.0, w - 6.0, 6.0, 3.0, col)
        }
        StepComponent::Delay(_) => {
            fb.fill_rect(
                x as i32 + 3,
                (y + h) as i32 - 4,
                w as i32 - 6,
                1,
                col.scale(0.5),
            );
            fill_round_rect(fb, x + 14.0, y + 4.0, 4.0, h - 8.0, 1.0, col);
        }
        StepComponent::RandomNote(_) => {
            for (dx, dy) in [
                (6.0, 6.0),
                (16.0, 15.0),
                (25.0, 8.0),
                (11.0, 17.0),
                (28.0, 17.0),
            ] {
                fill_circle(fb, x + dx, y + dy, 2.0, col);
            }
        }
        StepComponent::ParamLock { value, .. } => {
            let v = value as f32 / 127.0;
            arc(
                fb,
                x + w / 2.0,
                y + h / 2.0,
                9.0,
                2.0,
                op3_gfx::paint::DIAL_A0,
                op3_gfx::paint::DIAL_A0
                    + (op3_gfx::paint::DIAL_A1 - op3_gfx::paint::DIAL_A0) * v.max(0.02),
                col,
            );
        }
    }
}

fn comp_label(c: StepComponent) -> String {
    c.label().to_lowercase()
}

fn inspector(fb: &mut FrameBuffer480x272, ui: &UiState, p: &Project, step: usize, accent: Rgb565) {
    let t = theme();
    let s = &p.tracks[ui.track].steps[step];
    text_v(
        fb,
        &(step + 1).to_string(),
        30,
        66,
        FontSize::Huge,
        if s.active { accent } else { t.dim },
        Align::Left,
        VAlign::Middle,
    );
    text_v(
        fb,
        &note_name(s.note),
        30,
        116,
        FontSize::Large,
        knob(0),
        Align::Left,
        VAlign::Middle,
    );
    // velocity: vertical bar (knob 2); gate: horizontal bar (knob 3)
    fb.fill_rect(120, 44, 10, 100, t.faint);
    let vh = (100.0 * s.velocity as f32 / 127.0) as i32;
    fb.fill_rect(120, 144 - vh, 10, vh, knob(1));
    fb.fill_rect(30, 140, 80, 6, t.faint);
    fb.fill_rect(30, 140, (80.0 * s.gate as f32 / 100.0) as i32, 6, knob(2));
    // component stack (knob 4)
    let (x0, y0) = (160.0, 42.0);
    let comps = s.components.as_slice();
    for (i, &c) in comps.iter().enumerate() {
        let (x, y) = (x0 + (i % 3) as f32 * 104.0, y0 + (i / 3) as f32 * 60.0);
        let sel = i == ui.comp_cursor;
        let col = if sel { knob(3) } else { t.faint };
        stroke_round_rect(fb, x, y, 96.0, 52.0, 8.0, if sel { 2.0 } else { 1.0 }, col);
        comp_glyph(fb, c, x + 8.0, y + 6.0, if sel { knob(3) } else { t.fg });
        op3_gfx::text(
            fb,
            &comp_label(c),
            x as i32 + 8,
            y as i32 + 32,
            FontSize::Small,
            if sel { knob(3) } else { t.dim },
            Align::Left,
        );
    }
    let n = comps.len();
    if n < op3_seq::MAX_COMPONENTS {
        let (x, y) = (x0 + (n % 3) as f32 * 104.0, y0 + (n / 3) as f32 * 60.0);
        let sel = ui.comp_cursor >= n;
        let col = if sel { knob(3) } else { t.faint };
        stroke_round_rect(fb, x, y, 96.0, 52.0, 8.0, if sel { 2.0 } else { 1.0 }, col);
        text_v(
            fb,
            "+",
            x as i32 + 48,
            y as i32 + 26,
            FontSize::Big,
            if sel { knob(3) } else { t.dim },
            Align::Center,
            VAlign::Middle,
        );
    }
}

fn brain_view(fb: &mut FrameBuffer480x272, p: &Project, st: &EngineStatus, accent: Rgb565) {
    let t = theme();
    let b = &p.brain;
    let active = if st.playing.load(Relaxed) {
        st.chord_slot.load(Relaxed) as usize
    } else {
        0
    };
    let n = (b.len as usize).max(1);
    let total: f32 = b.progression[..n]
        .iter()
        .map(|s| s.length.max(1) as f32)
        .sum();
    let mut x = 20.0;
    for (i, slot) in b.progression[..n].iter().enumerate() {
        let w = 440.0 * slot.length.max(1) as f32 / total;
        let cx = x + w / 2.0;
        let on = i == active && b.live.is_none();
        let c = Chord::diatonic(b.key, b.scale, slot.degree, slot.kind);
        let r = (w / 2.0 - 4.0).min(30.0);
        if on {
            fill_circle(fb, cx, 82.0, r, accent);
        } else {
            ring(fb, cx, 82.0, r, 1.5, t.dim);
        }
        text_v(
            fb,
            &c.name(),
            cx as i32,
            82,
            FontSize::Medium,
            if on { t.bg } else { t.fg },
            Align::Center,
            VAlign::Middle,
        );
        x += w;
    }
    // the sounding chord on a 12-tone circle
    let (cx, cy, r) = (240.0, 168.0, 38.0);
    let ch = st.chord();
    let ch = if ch.mask == 0 { b.chord_at(0) } else { ch };
    let scale_mask = b.scale.mask(b.key);
    ring(fb, cx, cy, r, 1.0, t.faint);
    let line_col = if b.live.is_some() { accent } else { t.fg };
    let pts: Vec<(f32, f32)> = (0..12u8)
        .filter(|&pc| ch.contains(pc))
        .map(|pc| polar(cx, cy, r, -FRAC_PI_2 + pc as f32 / 12.0 * TAU))
        .collect();
    path(fb, &pts, 1.5, true, line_col);
    for pc in 0..12u8 {
        let (px, py) = polar(cx, cy, r, -FRAC_PI_2 + pc as f32 / 12.0 * TAU);
        if ch.contains(pc) {
            fill_circle(
                fb,
                px,
                py,
                if pc == ch.root % 12 { 7.0 } else { 5.0 },
                line_col,
            );
        } else if scale_mask & (1 << pc) != 0 {
            fill_circle(fb, px, py, 2.5, t.dim);
        } else {
            fill_circle(fb, px, py, 1.2, t.faint);
        }
    }
    for (i, tr) in p.tracks.iter().enumerate() {
        if tr.brain_link {
            fill_circle(fb, 30.0 + i as f32 * 14.0, 168.0, 4.0, TRACK_COLORS[i]);
        }
    }
    let label = format!(
        "{} {}",
        NOTE_NAMES[b.key as usize],
        b.scale.name().to_lowercase()
    );
    text_v(
        fb,
        &label,
        450,
        168,
        FontSize::Small,
        t.dim,
        Align::Right,
        VAlign::Middle,
    );
}

#[allow(clippy::too_many_arguments)]
fn fx_glyph(
    fb: &mut FrameBuffer480x272,
    fx: PunchFx,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    col: Rgb565,
    amt: f32,
) {
    let (mx, my) = (x + w / 2.0, y + h / 2.0);
    match fx {
        PunchFx::LowPass | PunchFx::HighPass => {
            let cut = 0.65 - amt * 0.4;
            let pts: Vec<(f32, f32)> = (0..=w as i32)
                .step_by(2)
                .map(|i| {
                    let u = i as f32 / w;
                    let v = if fx == PunchFx::LowPass { u } else { 1.0 - u };
                    let yy = if v < cut {
                        0.0
                    } else {
                        ((v - cut) * 3.0).min(1.0).powf(1.5)
                    };
                    (x + i as f32, y + 4.0 + yy * (h - 8.0))
                })
                .collect();
            path(fb, &pts, 2.0, false, col);
        }
        PunchFx::Crush | PunchFx::Downsample => {
            let n = if fx == PunchFx::Crush { 6 } else { 3 };
            let mut pts = vec![(x, y + h - 2.0)];
            for i in 0..n {
                let px = x + i as f32 * w / n as f32;
                let py = y + h - 2.0 - (i + 1) as f32 * (h - 4.0) / n as f32;
                pts.push((px, py));
                pts.push((px + w / n as f32, py));
            }
            path(fb, &pts, 2.0, false, col);
        }
        PunchFx::Gate16 | PunchFx::Gate8 => {
            let n = if fx == PunchFx::Gate16 { 4 } else { 2 };
            let seg = w / n as f32;
            let mut pts = vec![(x, y + h - 2.0)];
            for i in 0..n {
                let px = x + i as f32 * seg;
                pts.extend([
                    (px, y + 2.0),
                    (px + seg / 2.0, y + 2.0),
                    (px + seg / 2.0, y + h - 2.0),
                    (px + seg, y + h - 2.0),
                ]);
            }
            path(fb, &pts, 2.0, false, col);
        }
        PunchFx::TapeStop => {
            let pts: Vec<(f32, f32)> = (0..95)
                .map(|i| {
                    let a = i as f32 * 0.2;
                    let r = 2.0 + a * 1.1;
                    (mx + a.cos() * r * w / 50.0, my + a.sin() * r * h / 50.0)
                })
                .collect();
            path(fb, &pts, 2.0, false, col);
        }
        PunchFx::Kill => {
            line_aa(fb, mx - 9.0, my - 9.0, mx + 9.0, my + 9.0, 3.0, col);
            line_aa(fb, mx + 9.0, my - 9.0, mx - 9.0, my + 9.0, 3.0, col);
        }
        PunchFx::Echo => {
            for i in 0..4 {
                ring(
                    fb,
                    mx - 14.0 + i as f32 * 9.0,
                    my,
                    7.0 - i as f32 * 1.2,
                    2.0 - i as f32 * 0.3,
                    col,
                );
            }
        }
        PunchFx::Repeat(_) | PunchFx::Stutter(_) => {
            arc(fb, mx, my, w.min(h) / 2.0 - 3.0, 2.0, -0.4, PI * 1.6, col);
        }
    }
}

fn punch_view(fb: &mut FrameBuffer480x272, ui: &UiState, accent: Rgb565) {
    let t = theme();
    for (i, &(fx, name)) in PUNCH_SLOTS.iter().enumerate() {
        // pads laid out like the hardware: pad 0 bottom-left
        let (col, row) = ((i % 4) as f32, (3 - i / 4) as f32);
        let (x, y) = (12.0 + col * 116.0, 36.0 + row * 46.0);
        let on = ui.punch.map(|(s, _)| s) == Some(i);
        let amt = ui.punch.map_or(0.0, |(_, v)| v);
        if on {
            fill_round_rect(fb, x, y, 108.0, 40.0, 8.0, accent.scale(0.35 + 0.65 * amt));
        } else {
            stroke_round_rect(fb, x, y, 108.0, 40.0, 8.0, 1.0, t.faint);
        }
        let ink = if on { t.bg } else { t.fg };
        let mut words = name.split(' ');
        let head = words.next().unwrap_or("").to_lowercase();
        let sub = words.next();
        // repeat / stutter: loop glyph, then the length as text
        let gw = if sub.is_some() { 22.0 } else { 40.0 };
        fx_glyph(
            fb,
            fx,
            x + 10.0,
            y + 8.0,
            gw,
            24.0,
            ink,
            if on { amt } else { 0.5 },
        );
        if let Some(sub) = sub {
            text_v(
                fb,
                sub,
                x as i32 + 38,
                y as i32 + 20,
                FontSize::Small,
                ink,
                Align::Left,
                VAlign::Middle,
            );
        }
        text_v(
            fb,
            &head,
            x as i32 + 98,
            y as i32 + 20,
            FontSize::Small,
            if on { t.bg } else { t.dim },
            Align::Right,
            VAlign::Middle,
        );
    }
}

/// Footer for encoders 1–4. The controller uses the same layout to decide
/// what each knob does.
pub fn left_footer(ui: &UiState, p: &Project) -> [FooterCell; 4] {
    let t = &p.tracks[ui.track];
    match (ui.mode, ui.inspect_step) {
        (Mode::Step, Some(s)) => {
            let st = &t.steps[s as usize];
            let comp = st
                .components
                .as_slice()
                .get(ui.comp_cursor)
                .copied()
                .unwrap_or(StepComponent::None);
            let adding = ui.comp_cursor >= st.components.len();
            [
                FooterCell::new("note", note_name(st.note), Some(st.note as f32 / 127.0)),
                FooterCell::new(
                    "velocity",
                    st.velocity.to_string(),
                    Some(st.velocity as f32 / 127.0),
                ),
                FooterCell::new(
                    "gate",
                    format!("{}%", st.gate),
                    Some(st.gate as f32 / 100.0),
                ),
                FooterCell::new(
                    "component",
                    if adding { "+".into() } else { comp_label(comp) },
                    Some(
                        COMPONENT_PRESETS
                            .iter()
                            .position(|c| *c == comp)
                            .unwrap_or(0) as f32
                            / (COMPONENT_PRESETS.len() - 1) as f32,
                    ),
                )
                .with_glyph(FooterGlyph::Component(if adding {
                    StepComponent::None
                } else {
                    comp
                })),
            ]
        }
        (Mode::Brain, _) => [
            FooterCell::new(
                "key",
                NOTE_NAMES[p.brain.key as usize],
                Some(p.brain.key as f32 / 11.0),
            ),
            FooterCell::new(
                "scale",
                p.brain.scale.name().to_lowercase(),
                Some(p.brain.scale as usize as f32 / 6.0),
            ),
            FooterCell::new(
                "mode",
                match p.brain.mode {
                    BrainMode::Transpose => "transpose",
                    BrainMode::Chord => "chord",
                    BrainMode::Scale => "scale",
                },
                Some(p.brain.mode as usize as f32 / 2.0),
            ),
            FooterCell::new(
                "chords",
                p.brain.len.to_string(),
                Some(p.brain.len as f32 / 8.0),
            ),
        ],
        (Mode::Punch, _) => [
            FooterCell::new(
                "fx",
                ui.punch
                    .map_or("-".into(), |(s, _)| PUNCH_SLOTS[s].1.to_lowercase()),
                None,
            ),
            FooterCell::new(
                "amount",
                ui.punch
                    .map_or("-".into(), |(_, v)| format!("{:.0}%", v * 100.0)),
                Some(ui.punch.map_or(0.0, |(_, v)| v)),
            ),
            FooterCell::new(
                "strip",
                ui.strip
                    .map_or("-".into(), |v| format!("{:.0}%", v * 100.0)),
                Some(ui.strip.unwrap_or(0.5)),
            ),
            FooterCell::new(
                "tempo",
                format!("{:.1}", p.bpm),
                Some((p.bpm - 20.0) / 280.0),
            ),
        ],
        _ => [
            FooterCell::new("length", t.len().to_string(), Some(t.len() as f32 / 16.0))
                .with_glyph(FooterGlyph::Length(t.len() as u8)),
            FooterCell::new(
                "division",
                t.division.label(),
                Some(t.division as usize as f32 / 4.0),
            ),
            FooterCell::new(
                "swing",
                format!("{:.0}%", p.swing * 100.0),
                Some((p.swing - 0.5) * 4.0),
            ),
            FooterCell::new(
                "tempo",
                format!("{:.1}", p.bpm),
                Some((p.bpm - 20.0) / 280.0),
            ),
        ],
    }
}
