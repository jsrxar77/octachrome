//! PATTERN: the sixteen patterns of the selected track, as the sixteen pads (docs/design.md §4).
//!
//! **Left:** the grid of the pads, one cell for each pattern (its number): a cell with something in it is filled, the pattern that is
//! shown (and edited) has the thick outline, and the one that is playing, when it is another because the change waits for the start of
//! the track's next loop, has a small play mark in its corner. **Right:** what a pad does, and what just happened.
use crate::nav::{CELL_H, CELL_W, GAP};
use crate::theme::TRACK_NAMES;
use op8_engine::EngineStatus;
use op8_gfx::{paint::fill_convex, theme, FrameBuffer480x272};
use op8_seq::{Project, PATTERNS};
use std::sync::atomic::Ordering::Relaxed;

pub fn draw_left(fb: &mut FrameBuffer480x272, ui: &crate::UiState, p: &Project, st: &EngineStatus) {
    let t = theme();
    let track = ui.track;
    let cells: Vec<(String, bool)> = (0..PATTERNS).map(|i| ((i + 1).to_string(), p.pattern(track, i).is_some_and(|x| !x.is_empty()))).collect();
    let title = format!("patterns · {}", TRACK_NAMES[track]);
    crate::files::pick_left_rows(fb, ui, st, &title, &cells, Some(p.current[track] as usize), false);
    let playing = st.pattern[track].load(Relaxed) as usize;
    if st.playing.load(Relaxed) && playing != p.current[track] as usize {
        let (col, row) = ((playing % 4) as f32, (playing / 4) as f32);
        let (x, y) = (12.0 + col * (CELL_W + GAP), 44.0 + row * (CELL_H + GAP));
        fill_convex(fb, &[(x + CELL_W - 18.0, y + 8.0), (x + CELL_W - 8.0, y + 14.0), (x + CELL_W - 18.0, y + 20.0)], t.fg);
    }
}

pub fn draw_right(fb: &mut FrameBuffer480x272, ui: &crate::UiState) {
    crate::files::pick_right(
        fb,
        &format!("patterns · {}", TRACK_NAMES[ui.track]),
        &[("tap a pad", "shows it, plays from the next loop"), ("shift + tap", "plays it at once"), ("hold + tap another", "copies it; erase + tap empties")],
        &ui.pattern_note,
    );
}
