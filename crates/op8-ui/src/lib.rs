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
mod files;
mod left;
mod right;
mod settings;
mod theme;

pub use anim::{animate, Anim};
pub use left::{comp_glyph, draw_left, left_footer};
pub use right::draw_right;
pub use theme::*;

use op8_seq::StepComponent;

/// The main modes: what the two screens are for as a whole (buttons.md §1).
/// [`Mode`] is the pad mode inside `Instrument`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Main {
    /// Composing (CHANNEL): the sequencer on the left, the instrument on the right.
    #[default]
    Instrument,
    /// The machine's settings (SETTINGS): both screens.
    Settings,
    /// Projects (FILE): the sixteen pads open or save them, both screens.
    File,
    /// Instruments (PLUG-IN): the sixteen pads choose the instrument of the selected track, both screens.
    Plugin,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Mode {
    #[default]
    Step,
    Brain,
    Punch,
    /// PAD MODE: the pads play the track's instrument (a kit's sounds, or the scale).
    Pads,
    /// KEYBOARD: the pads are a chromatic keyboard.
    Keys,
}

impl Mode {
    pub const fn label(self) -> &'static str {
        match self {
            Mode::Step => "step",
            Mode::Brain => "brain",
            Mode::Punch => "punch",
            Mode::Pads => "pads",
            Mode::Keys => "keys",
        }
    }
}

/// Everything the screens need that is not in the project or engine status.
#[derive(Clone, Debug, Default)]
pub struct UiState {
    pub main: Main,
    pub mode: Mode,
    pub track: usize,
    /// Pad held long enough to open the Step Components inspector.
    pub inspect_step: Option<u8>,
    /// Which component slot knob 4 edits in the inspector (len = "add new").
    pub comp_cursor: usize,
    pub knob_touch: [bool; 8],
    /// Right screen parameter page: 0 = main, 1 = secondary.
    pub right_page: usize,
    /// Whether the selected track's instrument has a second page (the `shape` button).
    pub has_second: bool,
    /// The sixteen project slots shown in FILE, in reading order (top-left first): the name and
    /// whether there is a project in it. The top two rows are the factory projects.
    pub files: Vec<(String, bool)>,
    /// The sixteen places of PLUG-IN, in reading order: the name and whether it can be chosen (the top two
    /// rows are the factory instruments), and the one the selected track has now.
    pub plugs: Vec<(String, bool)>,
    pub plug_current: Option<usize>,
    pub plug_note: String,
    /// The slot last touched in FILE, and a line saying what happened.
    pub file_last: Option<usize>,
    pub file_note: String,
    /// REC is armed: what is played by hand while the sequencer runs is written into the track.
    pub rec: bool,
    /// KEYBOARD octave offset from C3 (pad 1 is C3 at 0).
    pub octave: i32,
    /// Pads held in PAD MODE / KEYBOARD (index = pad, bottom-left first), for the screen and the LEDs.
    pub pads_down: [bool; 16],
    /// The sound chosen on the right, as a lane 0..8, when the selected track's instrument is a kit (drums).
    pub lane: Option<usize>,
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

/// The note a pad plays in PAD MODE or KEYBOARD (pad 0 is bottom-left), if it plays one.
/// PAD MODE on a drum kit: pads 0–7 are the eight sounds (note 60 + lane), the rest are free;
/// on any other instrument the pads climb the Brain's scale from its key. KEYBOARD is chromatic
/// from C3, shifted by the octave.
pub fn pad_note(ui: &UiState, p: &op8_seq::Project, pad: u8) -> Option<u8> {
    let pad = pad as i32;
    let n = match ui.mode {
        Mode::Pads if ui.lane.is_some() => (pad < 8).then_some(op8_seq::step::KIT_BASE_NOTE as i32 + pad)?,
        Mode::Pads => {
            let iv = p.brain.scale.intervals();
            48 + p.brain.key as i32 + iv[(pad % 7) as usize] as i32 + 12 * (pad / 7)
        }
        Mode::Keys => 48 + 12 * ui.octave + pad,
        _ => return None,
    };
    (0..=127).contains(&n).then_some(n as u8)
}

/// The eight screen buttons above the two screens, in one table, so that what a
/// screen says about a button and what the button's LED does can never differ:
/// `None` = no function (no label, LED off); `Some((name, open))` = it does
/// something (a label; LED dim, or full when its page is the open one).
/// Buttons 1–4 are above the left screen, 5–8 above the right one.
pub fn screen_buttons(ui: &UiState) -> [Option<(&'static str, bool)>; 8] {
    let mut b = [None; 8];
    if ui.main != Main::Instrument {
        return b; // in settings the buttons do nothing
    }
    b[0] = Some(("step", ui.mode == Mode::Step && ui.inspect_step.is_none()));
    b[1] = Some(("components", ui.inspect_step.is_some()));
    // Button 5 is `shape` (the second page, if the instrument has one) and 6 is `sound` (the first).
    let shape = ui.right_page == 1 && ui.has_second;
    if ui.has_second {
        b[4] = Some(("shape", shape));
    }
    b[5] = Some(("sound", !shape));
    b
}

/// The labels of the four buttons above one screen (`0` = left, `1` = right), as the strip draws them.
pub fn strip_labels(ui: &UiState, screen: usize) -> [(&'static str, bool); 4] {
    let all = screen_buttons(ui);
    std::array::from_fn(|i| all[screen * 4 + i].unwrap_or(("", false)))
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
        op8_seq::brain::NOTE_NAMES[(n % 12) as usize],
        n as i32 / 12 - 1
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use op8_cartridge::InstrumentCartridge;
    use op8_engine::{EngineStatus, SharedCartridge};
    use op8_gfx::{DirtyTracker, FrameBuffer480x272, Rect};
    use op8_seq::{Project, Step};
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
        for mode in [Mode::Step, Mode::Brain, Mode::Punch, Mode::Pads, Mode::Keys] {
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
            Box::new(op8_cartridges::FmBass::new()) as Box<dyn InstrumentCartridge>
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
            Box::new(op8_cartridges::KickSyn::new()) as Box<dyn InstrumentCartridge>
        ));
        let mut fb = FrameBuffer480x272::new();
        for id in op8_gfx::ThemeId::ALL {
            op8_gfx::set_theme(id);
            let mut ui = UiState::default();
            ui.knob_touch[5] = true;
            for _ in 0..10 {
                animate(&mut ui, &p, &st);
            }
            draw_left(&mut fb, &ui, &p, &st);
            draw_right(&mut fb, &ui, Some(&c), &[0.5; 8]);
            assert!(fb.pixels().iter().filter(|&&px| px != 0).count() > 2000);
        }
        op8_gfx::set_theme(op8_gfx::ThemeId::Glossy);
    }
}
