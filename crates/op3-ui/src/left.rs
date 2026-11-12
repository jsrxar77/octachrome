use crate::{draw_footer, note_name, theme::*, FooterCell, Mode, UiState, COMPONENT_PRESETS};
use op3_engine::{EngineStatus, PUNCH_SLOTS};
use op3_gfx::{line, rect_outline, text, Align, FontSize, FrameBuffer480x272, Rgb565};
use op3_seq::{brain::NOTE_NAMES, BrainMode, Chord, Project, StepComponent, MAX_STEPS, TRACKS};
use std::sync::atomic::Ordering::Relaxed;

const GRID_Y: i32 = 70;
const GRID_H: i32 = 84;
const CELL_W: i32 = 27;
const CELL_GAP: i32 = 2;
const GRID_X: i32 = (480 - (CELL_W + CELL_GAP) * 16 + CELL_GAP) / 2;
const LANES_Y: i32 = 170;

pub fn draw_left(fb: &mut FrameBuffer480x272, ui: &UiState, p: &Project, st: &EngineStatus) {
    fb.clear(BG);
    let accent = TRACK_COLORS[ui.track];
    top_bar(fb, ui, p, st, accent);
    track_strip(fb, ui, p);
    match (ui.mode, ui.inspect_step) {
        (Mode::Step, Some(step)) => inspector(fb, ui, p, step as usize, accent),
        (Mode::Brain, _) => brain_view(fb, ui, p, st, accent),
        (Mode::Punch, _) => punch_view(fb, ui, accent),
        _ => {
            tape(fb, ui, p, st, accent);
            lanes(fb, ui, p, st);
        }
    }
    let cells = left_footer(ui, p);
    draw_footer(fb, &cells, &ui.knob_touch[..4], accent);
}

fn top_bar(
    fb: &mut FrameBuffer480x272,
    ui: &UiState,
    p: &Project,
    st: &EngineStatus,
    accent: Rgb565,
) {
    let playing = st.playing.load(Relaxed);
    // play / stop glyph
    if playing {
        for i in 0..10 {
            fb.fill_rect(8 + i, 8 + i / 2, 1, 14 - i, FG);
        }
    } else {
        fb.fill_rect(8, 8, 12, 12, DIM);
    }
    text(
        fb,
        &format!("{:.1}", p.bpm),
        28,
        4,
        FontSize::Large,
        FG,
        Align::Left,
    );
    text(fb, "BPM", 92, 12, FontSize::Small, DIM, Align::Left);
    // Brain chord
    let chord = if playing {
        st.chord()
    } else {
        p.brain.chord_at(0)
    };
    rect_outline(
        fb,
        130,
        4,
        76,
        22,
        1,
        if p.brain.live.is_some() { accent } else { DIM },
    );
    text(
        fb,
        &chord.name(),
        168,
        7,
        FontSize::Medium,
        FG,
        Align::Center,
    );
    // mode
    let mode = match (ui.mode, ui.punch) {
        (Mode::Punch, Some((slot, v))) => format!("{} {:.0}%", PUNCH_SLOTS[slot].1, v * 100.0),
        (m, _) => m.label().to_string(),
    };
    text(fb, &mode, 300, 7, FontSize::Medium, accent, Align::Center);
    // playhead of the selected track
    let t = &p.tracks[ui.track];
    let ph = st.playhead[ui.track].load(Relaxed) as usize;
    text(
        fb,
        &format!("{:02}/{:02}", ph + 1, t.len()),
        472,
        7,
        FontSize::Medium,
        FG,
        Align::Right,
    );
    fb.fill_rect(0, 28, 480, 1, FAINT);
}

fn track_strip(fb: &mut FrameBuffer480x272, ui: &UiState, p: &Project) {
    for i in 0..TRACKS {
        let x = i as i32 * 60;
        let c = TRACK_COLORS[i];
        let sel = i == ui.track;
        fb.fill_rect(x + 4, 32, 52, 3, if p.tracks[i].muted { FAINT } else { c });
        if sel {
            rect_outline(fb, x + 2, 30, 56, 32, 1, FG);
        }
        text(
            fb,
            TRACK_NAMES[i],
            x + 12,
            40,
            FontSize::Medium,
            if sel { FG } else { DIM },
            Align::Left,
        );
        if p.tracks[i].brain_link {
            text(fb, "B", x + 30, 43, FontSize::Small, c, Align::Left);
        }
        // activity LED
        let lit = ui.activity[i] > 0;
        fb.fill_rect(x + 44, 44, 6, 6, if lit { c } else { FAINT });
    }
    fb.fill_rect(0, 64, 480, 1, FAINT);
}

fn tape(fb: &mut FrameBuffer480x272, ui: &UiState, p: &Project, st: &EngineStatus, accent: Rgb565) {
    let t = &p.tracks[ui.track];
    let len = t.len();
    let ph = st.playhead[ui.track].load(Relaxed) as usize;
    let playing = st.playing.load(Relaxed);
    for s in 0..MAX_STEPS {
        let x = GRID_X + s as i32 * (CELL_W + CELL_GAP);
        let step = &t.steps[s];
        let in_len = s < len;
        let outline = if in_len {
            if s % 4 == 0 {
                FG
            } else {
                DIM
            }
        } else {
            FAINT
        };
        if step.active && in_len {
            let h = (GRID_H as f32 * (0.35 + 0.65 * step.velocity as f32 / 127.0)) as i32;
            fb.fill_rect(x, GRID_Y + GRID_H - h, CELL_W, h, accent);
        } else if step.active {
            fb.fill_rect(x, GRID_Y + GRID_H - 6, CELL_W, 6, FAINT);
        }
        rect_outline(fb, x, GRID_Y, CELL_W as u32, GRID_H as u32, 1, outline);
        // component dots
        for (k, _) in step.components.iter().enumerate().take(6) {
            fb.fill_rect(x + 3 + k as i32 * 4, GRID_Y + 3, 2, 2, FG);
        }
        if playing && s == ph {
            fb.fill_rect(x, GRID_Y + GRID_H + 3, CELL_W, 3, FG);
        }
    }
    // polymeter length mark
    let xe = GRID_X + len as i32 * (CELL_W + CELL_GAP) - CELL_GAP / 2 - 1;
    line(fb, xe, GRID_Y - 4, xe, GRID_Y + GRID_H + 6, 2, accent);
    text(
        fb,
        &format!("L{len}"),
        xe.min(460) + 3,
        GRID_Y - 4,
        FontSize::Small,
        accent,
        Align::Left,
    );
}

fn lanes(fb: &mut FrameBuffer480x272, ui: &UiState, p: &Project, st: &EngineStatus) {
    let playing = st.playing.load(Relaxed);
    for (i, t) in p.tracks.iter().enumerate() {
        let y = LANES_Y + i as i32 * 6;
        let c = TRACK_COLORS[i];
        let ph = st.playhead[i].load(Relaxed) as usize;
        for s in 0..t.len() {
            let x = GRID_X + s as i32 * (CELL_W + CELL_GAP);
            let col = if playing && s == ph {
                if t.is_empty() {
                    DIM
                } else {
                    FG
                }
            } else if t.steps[s].active {
                if i == ui.track {
                    c
                } else {
                    c.scale(0.6)
                }
            } else {
                FAINT
            };
            fb.fill_rect(x, y, CELL_W, 4, col);
        }
    }
}

fn inspector(fb: &mut FrameBuffer480x272, ui: &UiState, p: &Project, step: usize, accent: Rgb565) {
    let s = &p.tracks[ui.track].steps[step];
    text(
        fb,
        &format!("STEP {:02}", step + 1),
        12,
        72,
        FontSize::Large,
        accent,
        Align::Left,
    );
    text(
        fb,
        if s.active { "ON" } else { "OFF" },
        130,
        76,
        FontSize::Medium,
        if s.active { FG } else { DIM },
        Align::Left,
    );
    text(
        fb,
        &format!(
            "{}  VEL {}  GATE {}%",
            note_name(s.note),
            s.velocity,
            s.gate
        ),
        180,
        76,
        FontSize::Medium,
        FG,
        Align::Left,
    );
    text(
        fb,
        "STEP COMPONENTS",
        12,
        100,
        FontSize::Small,
        DIM,
        Align::Left,
    );
    let n = s.components.len();
    for (i, c) in s.components.iter().enumerate() {
        let (x, y) = (12 + (i as i32 % 3) * 156, 114 + (i as i32 / 3) * 26);
        let sel = i == ui.comp_cursor;
        if sel {
            fb.fill_rect(x - 2, y - 2, 150, 20, accent);
        } else {
            rect_outline(fb, x - 2, y - 2, 150, 20, 1, DIM);
        }
        text(
            fb,
            &c.label(),
            x + 4,
            y + 1,
            FontSize::Medium,
            if sel { BG } else { FG },
            Align::Left,
        );
    }
    if n < op3_seq::MAX_COMPONENTS {
        let (x, y) = (12 + (n as i32 % 3) * 156, 114 + (n as i32 / 3) * 26);
        let sel = ui.comp_cursor >= n;
        rect_outline(
            fb,
            x - 2,
            y - 2,
            150,
            20,
            1,
            if sel { accent } else { FAINT },
        );
        text(
            fb,
            "+ ADD",
            x + 4,
            y + 1,
            FontSize::Medium,
            if sel { accent } else { DIM },
            Align::Left,
        );
    }
    text(
        fb,
        "JOG: select   KNOB 4: set",
        12,
        196,
        FontSize::Small,
        DIM,
        Align::Left,
    );
}

fn brain_view(
    fb: &mut FrameBuffer480x272,
    ui: &UiState,
    p: &Project,
    st: &EngineStatus,
    accent: Rgb565,
) {
    let b = &p.brain;
    text(
        fb,
        &format!("KEY {} {}", NOTE_NAMES[b.key as usize], b.scale.name()),
        12,
        70,
        FontSize::Medium,
        FG,
        Align::Left,
    );
    let active = if st.playing.load(Relaxed) {
        st.chord_slot.load(Relaxed) as usize
    } else {
        0
    };
    let n = b.len as usize;
    let w = 464 / n.max(1) as i32;
    for i in 0..n {
        let slot = b.progression[i];
        let c = Chord::diatonic(b.key, b.scale, slot.degree, slot.kind);
        let x = 8 + i as i32 * w;
        let on = i == active && b.live.is_none();
        if on {
            fb.fill_rect(x, 92, w - 4, 50, accent);
        } else {
            rect_outline(fb, x, 92, (w - 4) as u32, 50, 1, DIM);
        }
        text(
            fb,
            &c.name(),
            x + (w - 4) / 2,
            100,
            FontSize::Medium,
            if on { BG } else { FG },
            Align::Center,
        );
        text(
            fb,
            &format!("{} st", slot.length),
            x + (w - 4) / 2,
            122,
            FontSize::Small,
            if on { BG } else { DIM },
            Align::Center,
        );
    }
    // live chord pads legend: bottom row triads, row 2 sevenths
    text(
        fb,
        "PADS 1-7 TRIADS  9-15 SEVENTHS  HOLD = LIVE",
        12,
        152,
        FontSize::Small,
        DIM,
        Align::Left,
    );
    if let Some(live) = b.live {
        text(
            fb,
            &format!("LIVE {}", live.name()),
            12,
            170,
            FontSize::Large,
            accent,
            Align::Left,
        );
    }
    let linked: Vec<&str> = (0..TRACKS)
        .filter(|&t| p.tracks[t].brain_link)
        .map(|t| TRACK_NAMES[t])
        .collect();
    text(
        fb,
        &format!(
            "LINKED: {}",
            if linked.is_empty() {
                "-".into()
            } else {
                linked.join(" ")
            }
        ),
        12,
        200,
        FontSize::Small,
        FG,
        Align::Left,
    );
    let _ = ui;
}

fn punch_view(fb: &mut FrameBuffer480x272, ui: &UiState, accent: Rgb565) {
    for (i, (_, name)) in PUNCH_SLOTS.iter().enumerate() {
        // pads laid out like the hardware: pad 0 bottom-left
        let col = (i % 4) as i32;
        let row = 3 - (i / 4) as i32;
        let (x, y) = (8 + col * 118, 70 + row * 38);
        let on = ui.punch.map(|(s, _)| s) == Some(i);
        if on {
            let v = ui.punch.unwrap().1;
            fb.fill_rect(x, y, 112, 34, accent.scale(0.4 + 0.6 * v));
        } else {
            rect_outline(fb, x, y, 112, 34, 1, DIM);
        }
        text(
            fb,
            name,
            x + 56,
            y + 10,
            FontSize::Medium,
            if on { BG } else { FG },
            Align::Center,
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
            [
                FooterCell::new("NOTE", note_name(st.note), Some(st.note as f32 / 127.0)),
                FooterCell::new(
                    "VELOCITY",
                    st.velocity.to_string(),
                    Some(st.velocity as f32 / 127.0),
                ),
                FooterCell::new(
                    "GATE",
                    format!("{}%", st.gate),
                    Some(st.gate as f32 / 100.0),
                ),
                FooterCell::new(
                    "COMPONENT",
                    if ui.comp_cursor >= st.components.len() {
                        "+ ADD".into()
                    } else {
                        comp.label()
                    },
                    Some(
                        COMPONENT_PRESETS
                            .iter()
                            .position(|c| *c == comp)
                            .unwrap_or(0) as f32
                            / (COMPONENT_PRESETS.len() - 1) as f32,
                    ),
                ),
            ]
        }
        (Mode::Brain, _) => [
            FooterCell::new(
                "KEY",
                NOTE_NAMES[p.brain.key as usize],
                Some(p.brain.key as f32 / 11.0),
            ),
            FooterCell::new("SCALE", p.brain.scale.name(), None),
            FooterCell::new(
                "MODE",
                match p.brain.mode {
                    BrainMode::Transpose => "TRANSP",
                    BrainMode::Chord => "CHORD",
                    BrainMode::Scale => "SCALE",
                },
                None,
            ),
            FooterCell::new(
                "CHORDS",
                p.brain.len.to_string(),
                Some(p.brain.len as f32 / 8.0),
            ),
        ],
        (Mode::Punch, _) => [
            FooterCell::new("FX", ui.punch.map_or("-", |(s, _)| PUNCH_SLOTS[s].1), None),
            FooterCell::new(
                "AMOUNT",
                ui.punch
                    .map_or("-".into(), |(_, v)| format!("{:.0}%", v * 100.0)),
                ui.punch.map(|(_, v)| v),
            ),
            FooterCell::new(
                "STRIP",
                ui.strip
                    .map_or("-".into(), |v| format!("{:.0}%", v * 100.0)),
                ui.strip,
            ),
            FooterCell::new("BPM", format!("{:.1}", p.bpm), None),
        ],
        _ => [
            FooterCell::new("LENGTH", t.len().to_string(), Some(t.len() as f32 / 16.0)),
            FooterCell::new("DIVISION", t.division.label(), None),
            FooterCell::new(
                "SWING",
                format!("{:.0}%", p.swing * 100.0),
                Some((p.swing - 0.5) * 4.0),
            ),
            FooterCell::new("BPM", format!("{:.1}", p.bpm), Some((p.bpm - 20.0) / 280.0)),
        ],
    }
}
