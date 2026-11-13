//! Screen layouts. Pure drawing: give it state, get pixels. Every rule here
//! comes from `docs/design.md` (UX/UI treatise).
//!
//! Left screen (fixed: time, encoders 1–4)
//! ```text
//! y   0 ┌ ▶ tempo · chord · track dots ─────────────────────┐
//! y  32 │ polymeter rings          pad mirror (4×4)          │
//! y     │   (inspector / brain / punch replace this area)   │
//! y 224 ├ four dials in the colours of encoders 1–4 ────────┤
//! ```
//! Right screen (active instrument, encoders 5–8)
//! ```text
//! y   0 ┌ ● instrument name                    page dots ───┐
//! y  32 │ cartridge's animated drawing (draw_ui)            │
//! y 224 ├ four dials in the colours of encoders 5–8 ────────┤
//! ```

mod anim;
mod left;
mod right;
mod theme;

pub use anim::{animate, Anim};
pub use left::{comp_glyph, draw_left, left_footer};
pub use right::draw_right;
pub use theme::*;

use op3_seq::StepComponent;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Mode {
    #[default]
    Step,
    Brain,
    Punch,
}

impl Mode {
    pub const fn label(self) -> &'static str {
        match self {
            Mode::Step => "step",
            Mode::Brain => "brain",
            Mode::Punch => "punch",
        }
    }
}

/// Everything the screens need that is not in the project or engine status.
#[derive(Clone, Debug, Default)]
pub struct UiState {
    pub mode: Mode,
    pub track: usize,
    /// Pad held long enough to open the Step Components inspector.
    pub inspect_step: Option<u8>,
    /// Which component slot knob 4 edits in the inspector (len = "add new").
    pub comp_cursor: usize,
    pub knob_touch: [bool; 8],
    /// Right screen parameter page: 0 = main, 1 = secondary.
    pub right_page: usize,
    /// Active Punch-in FX: (slot, intensity).
    pub punch: Option<(usize, f32)>,
    pub strip: Option<f32>,
    /// Brain pad held for a live chord.
    pub live_chord_pad: Option<u8>,
    /// Activity LED per track: frames left lit.
    pub activity: [u8; 8],
    pub shift: bool,
    /// Smoothed animation state, advanced by [`animate`].
    pub anim: Anim,
}

/// The menu knob 4 scrolls through in the inspector.
pub const COMPONENT_PRESETS: [StepComponent; 20] = [
    StepComponent::None,
    StepComponent::Ratchet(2),
    StepComponent::Ratchet(3),
    StepComponent::Ratchet(4),
    StepComponent::Chance(75),
    StepComponent::Chance(50),
    StepComponent::Chance(25),
    StepComponent::Every { n: 1, of: 2 },
    StepComponent::Every { n: 2, of: 2 },
    StepComponent::Every { n: 1, of: 4 },
    StepComponent::Every { n: 4, of: 4 },
    StepComponent::Velocity(-40),
    StepComponent::Velocity(20),
    StepComponent::Transpose(7),
    StepComponent::Transpose(-5),
    StepComponent::Octave(1),
    StepComponent::Octave(-1),
    StepComponent::Hold(1),
    StepComponent::Delay(8),
    StepComponent::RandomNote(12),
];

pub fn note_name(n: u8) -> String {
    format!(
        "{}{}",
        op3_seq::brain::NOTE_NAMES[(n % 12) as usize],
        n as i32 / 12 - 1
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use op3_cartridge::InstrumentCartridge;
    use op3_engine::{EngineStatus, SharedCartridge};
    use op3_gfx::{DirtyTracker, FrameBuffer480x272, Rect};
    use op3_seq::{Project, Step};
    use std::sync::{Arc, Mutex};

    /// The theme is global; tests that draw take this lock so one test
    /// switching themes cannot change another test's pixels mid-way.
    static THEME_LOCK: Mutex<()> = Mutex::new(());
    fn lock() -> std::sync::MutexGuard<'static, ()> {
        THEME_LOCK.lock().unwrap_or_else(|e| e.into_inner())
    }

    fn project() -> Project {
        let mut p = Project::default();
        p.tracks[0].steps[0] = Step::on(36).with(StepComponent::Ratchet(2));
        p.tracks[0].steps[8] = Step::on(36);
        p.tracks[1].length = 7;
        p.tracks[1].brain_link = true;
        p
    }

    #[test]
    fn left_screen_draws_every_mode_and_inspector() {
        let _guard = lock();
        let p = project();
        let st = EngineStatus::default();
        let mut fb = FrameBuffer480x272::new();
        for mode in [Mode::Step, Mode::Brain, Mode::Punch] {
            for inspect in [None, Some(0)] {
                let ui = UiState {
                    mode,
                    inspect_step: inspect,
                    punch: Some((0, 0.5)),
                    ..Default::default()
                };
                draw_left(&mut fb, &ui, &p, &st);
                assert!(fb.pixels().iter().filter(|&&px| px != 0).count() > 2000);
            }
        }
    }

    #[test]
    fn right_screen_with_and_without_cartridge() {
        let _guard = lock();
        let c: SharedCartridge = Arc::new(Mutex::new(
            Box::new(op3_cartridges::FmBass::new()) as Box<dyn InstrumentCartridge>
        ));
        let mut fb = FrameBuffer480x272::new();
        draw_right(&mut fb, &UiState::default(), Some(&c), &[0.5; 8]);
        assert!(fb.pixels().iter().any(|&p| p != 0));
        draw_right(&mut fb, &UiState::default(), None, &[0.5; 8]);
    }

    #[test]
    fn moving_playhead_is_a_small_update() {
        let _guard = lock();
        let p = project();
        let st = EngineStatus::default();
        st.playing.store(true, std::sync::atomic::Ordering::Relaxed);
        let mut ui = UiState::default();
        let mut fb = FrameBuffer480x272::new();
        let mut dirty = DirtyTracker::new();
        animate(&mut ui, &p, &st);
        draw_left(&mut fb, &ui, &p, &st);
        dirty.diff(&fb);
        st.playhead[0].store(1, std::sync::atomic::Ordering::Relaxed);
        st.playhead[1].store(1, std::sync::atomic::Ordering::Relaxed);
        animate(&mut ui, &p, &st);
        draw_left(&mut fb, &ui, &p, &st);
        let px = dirty.diff(&fb).pixel_count();
        assert!(px > 0 && px < Rect::FULL.area() / 4, "{px}");
    }

    #[test]
    fn every_theme_draws_both_screens() {
        let _guard = lock();
        let p = project();
        let st = EngineStatus::default();
        let c: SharedCartridge = Arc::new(Mutex::new(
            Box::new(op3_cartridges::KickSyn::new()) as Box<dyn InstrumentCartridge>
        ));
        let mut fb = FrameBuffer480x272::new();
        for id in op3_gfx::ThemeId::ALL {
            op3_gfx::set_theme(id);
            let mut ui = UiState::default();
            ui.knob_touch[5] = true;
            for _ in 0..10 {
                animate(&mut ui, &p, &st);
            }
            draw_left(&mut fb, &ui, &p, &st);
            draw_right(&mut fb, &ui, Some(&c), &[0.5; 8]);
            assert!(fb.pixels().iter().filter(|&&px| px != 0).count() > 2000);
        }
        op3_gfx::set_theme(op3_gfx::ThemeId::Op1);
    }
}
