//! SCENE: the sixteen scenes as the sixteen pads (docs/design.md §4).
//!
//! **Left:** the grid of the pads, one cell for each scene (its number): a scene that has been stored is filled and the one launched or
//! stored last has the thick outline. **Right:** what a pad does, and what just happened.
use op8_engine::EngineStatus;
use op8_gfx::FrameBuffer480x272;
use op8_seq::{Project, SCENES};

pub fn draw_left(fb: &mut FrameBuffer480x272, ui: &crate::UiState, p: &Project, st: &EngineStatus) {
    let cells: Vec<(String, bool)> = (0..SCENES).map(|i| ((i + 1).to_string(), p.scenes[i].used)).collect();
    crate::files::pick_left_rows(fb, ui, st, "scenes", &cells, ui.scene_last, false);
}

pub fn draw_right(fb: &mut FrameBuffer480x272, ui: &crate::UiState) {
    crate::files::pick_right(
        fb,
        "scenes",
        &[("tap a pad", "every track goes to the pattern it had, from its next loop; the mutes at once"), ("shift + tap", "stores the patterns and mutes that are now"), ("erase + tap", "empties it")],
        &ui.scene_note,
    );
}
