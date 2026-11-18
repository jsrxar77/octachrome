//! ARRANGER: the song, a chain of scenes, on the sixteen pads (docs/design.md §4).
//!
//! **Left:** the grid of the pads, one cell for each step of the song (`scene × bars`), a step that exists is filled and the one that
//! plays has the thick outline. **Right:** the step that is chosen, what a knob does, and what just happened; the four knobs under the
//! screen: the scene of the step, how many bars it lasts, whether the song is followed, and how many steps there are.
use octachrome_engine::EngineStatus;
use octachrome_gfx::{knob, text_v, theme, Align, FontSize, FrameBuffer480x272, VAlign};
use std::sync::atomic::Ordering::Relaxed;

/// The song as the screens show it (the controller copies it from the project every frame).
#[derive(Clone, Debug, Default)]
pub struct SongView {
    /// For each step that exists: the scene (0 to 15) and the bars (1 to 16).
    pub steps: Vec<(u8, u8)>,
    pub on: bool,
    /// The step that is chosen (where the 4D is).
    pub sel: usize,
    /// The step that plays, the bar it is in and its bars (copied from the engine every frame, so the screen counts).
    pub playing: Option<(usize, u32, u32)>,
}

pub fn draw_left(fb: &mut FrameBuffer480x272, ui: &crate::UiState, st: &EngineStatus) {
    let cells: Vec<(String, bool)> = (0..octachrome_seq::SONG_STEPS)
        .map(|i| match ui.arr.steps.get(i) {
            Some((s, b)) => (format!("scene {}\n{} bar{}", s + 1, b, if *b == 1 { "" } else { "s" }), true),
            None => ((i + 1).to_string(), false),
        })
        .collect();
    let playing = st.playing.load(Relaxed).then(|| st.song_step.load(Relaxed) as usize).filter(|&p| p < octachrome_seq::SONG_STEPS);
    crate::files::pick_left_rows(fb, ui, st, "song", &cells, playing, false);
    // where the step that plays is in its bars: a bar along the bottom of its cell that grows, so it can be seen going
    if let Some(p) = playing {
        if let Some((_, bars)) = ui.arr.steps.get(p) {
            let (col, row) = ((p % 4) as f32, (p / 4) as f32);
            let (x, y) = (12.0 + col * (crate::nav::CELL_W + crate::nav::GAP), 44.0 + row * (crate::nav::CELL_H + crate::nav::GAP));
            let done = (st.song_tick.load(Relaxed) as f32 / (*bars as f32 * 384.0)).clamp(0.0, 1.0);
            let (x0, x1, y0, y1) = (x + 8.0, x + 8.0 + (crate::nav::CELL_W - 16.0) * done, y + crate::nav::CELL_H - 6.0, y + crate::nav::CELL_H - 3.0);
            octachrome_gfx::paint::fill_convex(fb, &[(x0, y0), (x1, y0), (x1, y1), (x0, y1)], theme().fg);
        }
    }
    // knob 4 (the last under this screen) moves the step that is chosen
    let t = theme();
    if ui.arr.steps.len() > 1 {
        text_v(fb, "move", 420, 236, FontSize::Small, t.dim, Align::Center, VAlign::Middle);
        text_v(fb, &format!("step {}", ui.arr.sel + 1), 420, 254, FontSize::Medium, knob(3), Align::Center, VAlign::Middle);
    }
}

pub fn draw_right(fb: &mut FrameBuffer480x272, ui: &crate::UiState) {
    let t = theme();
    fb.clear(t.bg);
    crate::files::bar(fb, "song");
    let a = &ui.arr;
    let (l1, l2) = match a.steps.get(a.sel) {
        Some((s, b)) => (format!("step {} of {}", a.sel + 1, a.steps.len()), format!("scene {} for {} bar{}", s + 1, b, if *b == 1 { "" } else { "s" })),
        None => (format!("step {} is free", a.sel + 1), "turn knob 5 to put a scene in it".to_string()),
    };
    text_v(fb, &l1, 30, 76, FontSize::Large, t.fg, Align::Left, VAlign::Middle);
    text_v(fb, &l2, 30, 100, FontSize::Medium, t.dim, Align::Left, VAlign::Middle);
    text_v(fb, "tap a pad", 30, 128, FontSize::Large, t.fg, Align::Left, VAlign::Middle);
    text_v(fb, "chooses it; while it plays, goes there next bar", 30, 150, FontSize::Medium, t.dim, Align::Left, VAlign::Middle);
    text_v(fb, "shift + tap: now (stopped: puts one in)", 30, 168, FontSize::Medium, t.dim, Align::Left, VAlign::Middle);
    text_v(fb, "hold + tap another: moves it", 30, 186, FontSize::Medium, t.dim, Align::Left, VAlign::Middle);
    // what plays now (it counts the bars, so it moves) or what just happened
    if !ui.arr_note.is_empty() {
        text_v(fb, &ui.arr_note, 30, 210, FontSize::Medium, t.fg, Align::Left, VAlign::Middle);
    } else if let Some((p, bar, bars)) = a.playing {
        text_v(fb, &format!("playing step {} · bar {} of {}", p + 1, bar, bars), 30, 210, FontSize::Medium, t.fg, Align::Left, VAlign::Middle);
    }
    let values = [
        ("scene", a.steps.get(a.sel).map_or("-".to_string(), |s| (s.0 + 1).to_string())),
        ("bars", a.steps.get(a.sel).map_or("-".to_string(), |s| s.1.to_string())),
        ("song", if a.on { "on" } else { "off" }.to_string()),
        ("steps", a.steps.len().to_string()),
    ];
    for (i, (name, v)) in values.iter().enumerate() {
        let x = 60 + i as i32 * 120;
        text_v(fb, name, x, 236, FontSize::Small, t.dim, Align::Center, VAlign::Middle);
        text_v(fb, v, x, 254, FontSize::Medium, knob(4 + i), Align::Center, VAlign::Middle);
    }
}
