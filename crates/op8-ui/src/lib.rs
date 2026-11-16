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
mod nav;
mod pages;
mod right;
mod sampling;
mod settings;
mod theme;

pub use anim::{animate, Anim};
pub use left::{comp_glyph, draw_left, left_footer};
pub use nav::{focused as nav_focused, index as nav_index, items as nav_items, move_dir as nav_move_dir, punch_slot as nav_punch_slot, step as nav_step, NavItem, NavKind};
pub use pages::{fx_footer, lfo_footer};
pub use right::draw_right;
pub use sampling::{peaks_of, SamplingView, COLUMNS as SAMPLING_COLUMNS, SOURCES as SAMPLING_SOURCES};
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
    /// Recording a sound (SAMPLING): both screens.
    Sampling,
    /// The presets of the selected track's instrument (BROWSER): both screens.
    Browser,
}

/// What the left screen shows when the step view is open: the steps, or one of the two pages of buttons 3 and 4.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum LeftPage {
    #[default]
    Steps,
    /// Button 3: the arpeggio or chord over each note.
    Players,
    /// Button 4: the length, division and shift of the track, and how REC rounds.
    Bar,
}

/// How REC rounds what is played by hand to a step (knob 4 of the bar page).
pub const QUANTIZE: [&str; 3] = ["nearest", "down", "up"];

/// What the 4D encoder is turning while its button is held.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Held {
    Tempo,
    Swing,
    Volume,
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
    /// How many components the step in the inspector has (the controller keeps it: the list of the 4D's items depends on it).
    pub comp_count: usize,
    pub knob_touch: [bool; 8],
    /// Right screen page: 0 = sound, 1 = shape (the second page of the instrument), 2 = fx, 3 = mod.
    pub right_page: usize,
    /// The left screen page (buttons 1–4).
    pub left_page: LeftPage,
    /// How REC rounds to a step: an index into [`QUANTIZE`].
    pub quantize: usize,
    /// The selected track's players, effect and LFO, copied from the project every frame for the screens.
    pub players: op8_seq::Players,
    pub track_fx: op8_seq::TrackFx,
    pub lfo: op8_seq::Lfo,
    /// What the LFO moves, in words (the name of the instrument's knob, or "volume").
    pub lfo_target: String,
    /// The selected track is a drum kit (its players are off).
    pub kit: bool,
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
    /// BROWSER: the sixteen presets of the instrument (name, and whether there is one), its name, the last pad touched and a line saying what happened.
    pub browse: Vec<(String, bool)>,
    pub browse_inst: String,
    pub browse_last: Option<usize>,
    pub browse_note: String,
    /// The item that has the focus in the view that is open, and whether it is grabbed (a value being turned).
    pub focus: NavKind,
    pub grabbed: bool,
    /// SAMPLING: the take, its part and the knobs.
    pub smp: SamplingView,
    /// The slot last touched in FILE, and a line saying what happened.
    pub file_last: Option<usize>,
    pub file_note: String,
    /// TEMPO, SWING or VOLUME is held: the 4D encoder turns it and the left screen shows it big.
    pub held: Option<Held>,
    /// The metronome is on (SHIFT + TAP).
    pub metronome: bool,
    /// FIXED VEL is on: the pads always play at the same velocity.
    pub fixed_vel: bool,
    /// The master volume, 0.0..=1.0.
    pub master: f32,
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
        // on a drum kit KEYBOARD is PAD MODE: the eight sounds, a chromatic scale of them would mean nothing
        Mode::Pads | Mode::Keys if ui.lane.is_some() => (pad < 8).then_some(op8_seq::step::KIT_BASE_NOTE as i32 + pad)?,
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
    let steps = ui.mode == Mode::Step && ui.inspect_step.is_none();
    b[0] = Some(("step", steps && ui.left_page == LeftPage::Steps));
    b[1] = Some(("components", ui.inspect_step.is_some()));
    b[2] = Some(("players", steps && ui.left_page == LeftPage::Players));
    b[3] = Some(("bar", steps && ui.left_page == LeftPage::Bar));
    // Button 5 is `sound` (the first page, the one a track opens on), 6 is `shape` (the second, if the
    // instrument has one), 7 the track's effect and 8 its LFO.
    let page = right_page(ui);
    b[4] = Some(("sound", page == 0));
    if ui.has_second {
        b[5] = Some(("shape", page == 1));
    }
    b[6] = Some(("fx", page == 2));
    b[7] = Some(("mod", page == 3));
    b
}

/// The page of the right screen that is really showing: the shape page only exists when the instrument has one.
pub fn right_page(ui: &UiState) -> usize {
    if ui.right_page == 1 && !ui.has_second {
        0
    } else {
        ui.right_page.min(3)
    }
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
        op8_gfx::set_theme(op8_gfx::ThemeId::Matte);
    }
    // F-D19: ink over a colour always reads: whatever the theme, the track and the intensity of an effect, the ink on the fill that
    // `readable` returns has at least the contrast the design asks for.
    #[test]
    fn f_d19_ink_over_colour_reads() {
        let _guard = lock();
        for id in op8_gfx::ThemeId::ALL {
            op8_gfx::set_theme(id);
            for track in 0..8 {
                for pct in 0..=100 {
                    let want = track_color(track).scale(0.35 + 0.65 * pct as f32 / 100.0);
                    let (fill, ink) = op8_gfx::readable(want);
                    assert!(fill.contrast(ink) >= op8_gfx::MIN_INK_CONTRAST, "{} track {track} at {pct} %: {:.2}", id.name(), fill.contrast(ink));
                }
                let (fill, ink) = op8_gfx::readable(track_color(track));
                assert!(fill.contrast(ink) >= op8_gfx::MIN_INK_CONTRAST, "{} track {track} solid", id.name());
            }
        }
        op8_gfx::set_theme(op8_gfx::ThemeId::Matte);
    }

    // F-N13: the ring of every item of every view is inside the screen, and the ring of a screen button is on the capsule that is drawn.
    #[test]
    fn f_n13_every_ring_is_inside_the_screen_and_on_its_capsule() {
        let _guard = lock();
        let modes = [Mode::Step, Mode::Brain, Mode::Punch, Mode::Pads, Mode::Keys];
        let mut seen = 0;
        for main in [Main::Instrument, Main::File, Main::Plugin, Main::Browser, Main::Sampling] {
            for mode in modes {
                for page in [LeftPage::Steps, LeftPage::Players, LeftPage::Bar] {
                    for (has_second, lane, inspect_step) in [(false, None, None), (true, None, None), (true, Some(0), None), (true, None, Some(0))] {
                        let ui = UiState { main, mode, left_page: page, has_second, lane, inspect_step, comp_count: 5, ..Default::default() };
                        let labels = [strip_labels(&ui, 0), strip_labels(&ui, 1)];
                        for it in nav::items(&ui) {
                            let (x, y, w, h, _) = nav::ring_box(&it);
                            assert!(x >= 0.0 && y >= 0.0 && x + w <= 480.0 && y + h <= 272.0, "{main:?} {mode:?} {:?}: ring {x},{y} {w}x{h}", it.kind);
                            if let nav::NavKind::Strip(n) = it.kind {
                                let (name, _) = labels[n as usize / 4][n as usize % 4];
                                let (cx, cw, ch) = theme::strip_capsule(n as usize % 4, name);
                                assert_eq!((x, y, w, h), (cx, 1.0, cw, ch), "the ring of button {n} is its capsule");
                            }
                            seen += 1;
                        }
                    }
                }
            }
        }
        assert!(seen > 500, "{seen} items checked");
    }
}
