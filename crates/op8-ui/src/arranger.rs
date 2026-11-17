//! ARRANGER: the song, a chain of scenes, on the sixteen pads (docs/design.md §4).
//!
//! **Left:** the grid of the pads, one cell for each step of the song (`scene × bars`), a step that exists is filled and the one that
//! plays has the thick outline. **Right:** the step that is chosen, what a knob does, and what just happened; the four knobs under the
//! screen: the scene of the step, how many bars it lasts, whether the song is followed, and how many steps there are.
use op8_engine::EngineStatus;
use op8_gfx::{knob, text_v, theme, Align, FontSize, FrameBuffer480x272, VAlign};
use std::sync::atomic::Ordering::Relaxed;

/// The song as the screens show it (the controller copies it from the project every frame).
#[derive(Clone, Debug, Default)]
pub struct SongView {
    /// For each step that exists: the scene (0 to 15) and the bars (1 to 16).
    pub steps: Vec<(u8, u8)>,
    pub on: bool,
    /// The step that is chosen (where the 4D is).
    pub sel: usize,
}

pub fn draw_left(fb: &mut FrameBuffer480x272, ui: &crate::UiState, st: &EngineStatus) {
    let cells: Vec<(String, bool)> = (0..op8_seq::SONG_STEPS)
        .map(|i| match ui.arr.steps.get(i) {
            Some((s, b)) => (format!("{} x{}", s + 1, b), true),
            None => ((i + 1).to_string(), false),
        })
        .collect();
    let playing = st.playing.load(Relaxed).then(|| st.song_step.load(Relaxed) as usize).filter(|&p| p < op8_seq::SONG_STEPS);
    crate::files::pick_left_rows(fb, ui, st, "song", &cells, playing, false);
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
    text_v(fb, "tap a pad", 30, 140, FontSize::Large, t.fg, Align::Left, VAlign::Middle);
    text_v(fb, "chooses a step; erase + tap takes it out", 30, 164, FontSize::Medium, t.dim, Align::Left, VAlign::Middle);
    if !ui.arr_note.is_empty() {
        text_v(fb, &ui.arr_note, 30, 208, FontSize::Medium, t.fg, Align::Left, VAlign::Middle);
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
