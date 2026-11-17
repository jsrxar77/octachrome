//! Maps control-surface input onto engine commands and UI state, and derives
//! the LED state. Independent of the actual hardware: it only sees
//! [`ControlEvent`]s, so the same code drives the MK3 and the simulator.
//!
//! | control            | Step mode                    | Brain mode           | Punch FX mode       |
//! |--------------------|------------------------------|----------------------|---------------------|
//! | pad tap            | toggle step                  | live chord (hold)    | momentary FX (hold) |
//! | pad hold > 300 ms  | Step Components inspector    | –                    | –                   |
//! | pad pressure       | –                            | –                    | FX amount           |
//! | Group A–H          | select track                 | select track         | select track        |
//! | Shift + Group      | toggle Brain link            |                      |                     |
//! | Mute + Group       | mute track                   |                      |                     |
//! | Erase + pad        | clear step                   |                      |                     |
//! | encoders 1–4       | length · division · swing · BPM (inspector: note · vel · gate · component) | key · scale · mode · chords | – · – · – · BPM |
//! | encoders 5–8       | instrument parameters (Screen 5/6 buttons flip page)                    |||
//! | 4-D encoder        | BPM (inspector: component cursor)                                        |||
//! | Smart Strip        | master filter: left half low-pass, right half high-pass                  |||
//!
//! Main modes (docs/buttons.md §1): CHANNEL composes (all of the above);
//! SETTINGS takes both screens. In settings the pads, knobs and groups do
//! nothing, the 4D encoder and ◀ ▶ change the theme, and PLAY, STOP and RESTART
//! still work. SETTINGS again, CHANNEL, STEP, CHORDS or PERFORM go back.
//!
//! Step pads follow Maschine's step order: step 1 is the top-left pad.

use op8_engine::{EngineCommand, EngineHandle, PUNCH_SLOTS};
use op8_gfx::{set_theme, theme_id, ThemeId};
use op8_hw::{
    leds::{pad_to_hid, LedFrame},
    Button, ControlEvent, NiColor, PadEvent,
};
use op8_seq::{
    BrainMode, Chord, ChordKind, ChordType, Command, Division, FxKind, LfoShape, PlayMode, Scale, Step,
    StepComponent, TrackFx, LFO_TARGETS, MAX_COMPONENTS, TRACKS,
};
use op8_ui::{LeftPage, Main, Mode, NavKind, UiState, COMPONENT_PRESETS};
use std::sync::atomic::Ordering::Relaxed;

/// Where the player's presets are: next to the settings file, in `presets/`.
pub fn default_presets_dir() -> Option<std::path::PathBuf> {
    if let Some(p) = std::env::var_os("OP8_PRESETS").filter(|p| !p.is_empty()) {
        return Some(std::path::PathBuf::from(p));
    }
    crate::settings::default_path().and_then(|p| p.parent().map(|d| d.join("presets")))
}

pub const HOLD_MS: u64 = 300;
/// The velocity a pad plays and REC writes while FIXED VEL is on.
const FIXED_VELOCITY: u8 = 100;
/// SAMPLING: the beats the count-in counts before a recording.
const COUNT_IN_BEATS: u8 = 3;

/// A count-in that is going: when it began, what it will record and how many clicks have sounded.
struct CountIn {
    from_ms: u64,
    source: op8_engine::RecSource,
    beats_done: u8,
}
/// A button that has something to do is this bright (of 1.0): a very faint white.
const FAINT_WHITE: f32 = 0.10;
/// How long a knob of the instrument has to be still before its result is auditioned.
const AUDITION_PAUSE_MS: u64 = 250;
/// Knob units (1000 per turn) per discrete step: 25 detents per turn.
const DETENT: i32 = 40;
/// Knob units for a full 0→1 sweep of a continuous parameter.
const SWEEP: f32 = 800.0;

/// The pad and group LED colour of each track A–H in a theme, as entries of the
/// MK3's palette (16 hues; the order of the palette is still to be checked on
/// the unit, see `NiColor`). Glossy is the rainbow, matte its softer hues,
/// chrome one range from magenta to blue, then white.
pub fn track_led_colors(id: op8_gfx::ThemeId) -> [NiColor; TRACKS] {
    use op8_gfx::ThemeId::*;
    match id {
        Glossy => [
            NiColor::Red,
            NiColor::Orange,
            NiColor::Yellow,
            NiColor::Green,
            NiColor::Turquoise,
            NiColor::Blue,
            NiColor::Violet,
            NiColor::Magenta,
        ],
        Matte => [
            NiColor::Red,
            NiColor::LightOrange,
            NiColor::WarmYellow,
            NiColor::Mint,
            NiColor::Turquoise,
            NiColor::Blue,
            NiColor::Plum,
            NiColor::Fuchsia,
        ],
        Chrome => [
            NiColor::Fuchsia,
            NiColor::Magenta,
            NiColor::Purple,
            NiColor::Violet,
            NiColor::Plum,
            NiColor::Blue,
            NiColor::Cyan,
            NiColor::White,
        ],
    }
}

/// A file of the player's samples folder as it was: where, how big, when it changed.
type SampleStamp = (std::path::PathBuf, u64, std::time::SystemTime);

fn stamp_of(path: &std::path::Path) -> Option<SampleStamp> {
    let m = std::fs::metadata(path).ok()?;
    Some((path.to_path_buf(), m.len(), m.modified().unwrap_or(std::time::UNIX_EPOCH)))
}

/// The factory instruments in the order of the PLUG-IN pads (the top two rows), from the most guttural to the
/// most refined: the name shown, the name the registry knows, and whether it exists yet.
pub const FACTORY_INSTRUMENTS: [(&str, &str, bool); 8] = [
    ("drums", "drums", true),
    ("KickSyn", "KickSyn", true),
    ("FmBass", "FmBass", true),
    ("resonator", "resonator", true),
    ("sampler", "sampler", true),
    ("granular", "granular", true),
    ("macro", "macro", true),
    ("midi out", "MidiOutBridge", true),
];

/// The notes of a chord, ascending from its root in the octave below middle C (C3 = 48).
fn chord_notes(c: Chord) -> Vec<u8> {
    (0..12u8)
        .filter(|i| c.contains((c.root + i) % 12))
        .map(|i| 48 + c.root + i)
        .filter(|&n| n <= 127)
        .collect()
}

#[derive(Default)]
pub struct Controller {
    pub ui: UiState,
    /// Where the player's projects are saved and read, if anywhere (FILE does nothing for the user slots without it).
    pub projects_dir: Option<std::path::PathBuf>,
    /// Where SAMPLING saves what it records (`rec-1.wav` … `rec-8.wav`), if anywhere.
    pub samples_dir: Option<std::path::PathBuf>,
    /// What the player's folder looked like when its sounds were last put in the instruments: for each of the eight places the file, its
    /// size and when it changed. A place whose file is another one now is loaded again (see `rescan_samples`).
    sample_stamps: Vec<Option<SampleStamp>>,
    /// Where the player's presets are saved, one folder per instrument (`<name>/user-1.preset` …), if anywhere.
    pub presets_dir: Option<std::path::PathBuf>,
    /// How many samples of the take the drawing of its wave was made from.
    smp_drawn: usize,
    /// NAME: the file that is being named and the view it came from (it goes back there).
    name_target: Option<(std::path::PathBuf, Main)>,
    /// The notes that came in by MIDI and are down: (channel, note) and the track each one sounds on.
    midi_held: std::collections::HashMap<(u8, u8), u8>,
    /// PATTERN: the pad that is held (to copy it over another) and whether it has copied.
    pattern_held: Option<(u8, bool)>,
    /// MIDI clock in: the tempo and the position that the pulses say.
    clock: crate::clockin::ClockFollower,
    /// The parameter (0..=7) that `learn` is waiting for a control for, once a knob has been turned.
    learn_param: Option<u8>,
    /// The fractions of a step the compressor's knobs have turned and not yet applied.
    comp_accum: [f32; op8_seq::COMP_PARAMS],
    eq_accum: [f32; op8_seq::EQ_PARAMS],
    sends_accum: [f32; op8_seq::SENDS_PARAMS],
    /// SAMPLING: the count-in that is going before a recording starts.
    countin: Option<CountIn>,
    /// When the last taps of TAP were, to work out a tempo.
    taps: Vec<u64>,
    /// The time of the event being handled (ms).
    now: u64,
    /// The buttons that are down right now, for their lights.
    down: std::collections::HashSet<Button>,
    /// The notes of the chord that a CHORDS pad is sounding, to release them.
    chord_notes: Vec<u8>,
    /// The note each pad is holding in PAD MODE / KEYBOARD, to release it.
    held_notes: [Option<u8>; 16],
    /// A note to audition once the knob that asked for it has been still for a moment.
    audition_due: Option<u64>,
    /// When a knob of the instrument last moved.
    last_knob_ms: Option<u64>,
    /// The note the instrument's `sound` knob had at the last frame, to notice when it changes.
    last_paint: Option<u8>,
    pad_down_at: [Option<u64>; 16],
    /// The pad whose hold opened the inspector (its release must not toggle).
    inspector_pad: Option<u8>,
    knob_accum: [i32; 8],
    /// What is left over of a turn of a knob that moves a 0..=255 value in whole steps.
    knob_frac: [f32; 8],
    last_note_count: [u32; TRACKS],
    held: Held,
    punch_base: f32,
    /// Set when the encoder changes the theme; the app saves it and clears this.
    pub theme_changed: bool,
    /// What the player chose in SETTINGS (the devices, the buffer, the light; the theme is `op8_gfx::theme_id()`). Whoever starts the
    /// machine puts the saved settings here before the first frame.
    pub settings: crate::settings::Settings,
    /// Set when something of `settings` changed; the app saves it and clears this.
    pub settings_changed: bool,
    /// What SETTINGS asks of the hardware, for whoever can reach it (the main loop of the unit) to do and answer with
    /// [`Controller::devices_listed`].
    pub requests: Vec<Request>,
}

/// Something SETTINGS asks of the audio, the MIDI port or the surface: the controller does not hold them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Request {
    /// Look at the audio outputs and inputs and the MIDI ports, and what is open now.
    ListDevices,
    /// Use this audio output (`None`: automatic), and reopen the stream.
    AudioOut(Option<String>),
    AudioIn(Option<String>),
    /// Use this many frames in each audio block.
    Buffer(u32),
    /// Use this MIDI output port (`None`: automatic).
    MidiPort(Option<String>),
    /// Open this MIDI input (`None`: close it).
    MidiIn(Option<String>),
    /// Play a short note on the MIDI port, to hear whether what is plugged in answers.
    MidiTest,
    /// Silence everything on the MIDI port.
    MidiPanic,
}

/// What the hardware has, as it answers [`Request::ListDevices`] and the requests that change it.
#[derive(Clone, Debug, Default)]
pub struct DeviceList {
    pub outs: Vec<String>,
    pub ins: Vec<String>,
    pub ports: Vec<String>,
    pub midi_ins: Vec<String>,
    pub out_now: String,
    pub in_now: String,
    pub port_now: String,
    pub midi_in_now: String,
    /// What the audio stream says it has (`48 kHz · 2 ch · buffer 128`).
    pub stream_now: String,
    /// What the supply allows (`the host gives the MK3 480 mA: limited…`).
    pub supply: String,
    /// What to tell the player, if anything.
    pub note: Option<String>,
}

#[derive(Default, Clone, Copy)]
struct Held {
    shift: bool,
    mute: bool,
    erase: bool,
    /// SELECT is down: a pad of FILE or BROWSER chooses a place to name instead of opening it.
    select: bool,
}

impl Controller {
    pub fn new() -> Self {
        Self::default()
    }

    /// Steps are laid out in reading order (step 1 top-left), pads count from
    /// the bottom-left.
    pub fn pad_to_step(pad: u8) -> u8 {
        pad_to_hid(pad).unwrap_or(0)
    }

    pub fn handle(&mut self, ev: ControlEvent, h: &mut EngineHandle, now_ms: u64) {
        self.now = now_ms;
        match ev {
            ControlEvent::Button { button, pressed } => self.button(button, pressed, h),
            // In settings the pads and knobs do nothing; the 4D encoder picks the theme.
            ControlEvent::Pad { pad, event } if self.ui.main == Main::File => self.file_pad(pad, event, h),
            ControlEvent::Pad { pad, event } if self.ui.main == Main::Plugin => self.plugin_pad(pad, event, h),
            ControlEvent::Pad { pad, event } if self.ui.main == Main::Pattern => self.pattern_pad(pad, event, h),
            ControlEvent::Pad { pad, event } if self.ui.main == Main::Sampling => self.sampling_pad(pad, event, h),
            ControlEvent::Pad { pad, event } if self.ui.main == Main::Browser => self.browser_pad(pad, event, h),
            ControlEvent::Pad { pad, event } if self.ui.main == Main::Name => self.name_pad(pad, event, h),
            ControlEvent::Encoder { delta } if self.ui.main == Main::Name => self.name_turn(delta),
            // `learn` waits for a knob of the right screen of an instrument: it is the parameter that the control will turn
            ControlEvent::Knob { index, .. } if self.ui.settings.learn == 1 && self.ui.main == Main::Settings => self.learn_knob(index as usize),
            ControlEvent::Knob { index, delta } if self.ui.main == Main::Sampling => self.sampling_knob(index as usize, delta, h),
            ControlEvent::Knob { index, delta } if self.ui.main == Main::Mixer => self.mixer_page_knob(index as usize, delta, h),
            ControlEvent::Pad { .. } | ControlEvent::Knob { .. } if self.ui.main != Main::Instrument => {}
            ControlEvent::Encoder { delta } if self.ui.main == Main::Settings && self.ui.settings.page == 0 => {
                self.step_theme(delta)
            }
            // the 4D encoder moves the focus (or turns the value that is grabbed) wherever there are items to reach
            ControlEvent::Encoder { delta } if self.ui.held.is_none() && self.nav_active() => self.nav_turn(delta, h),
            ControlEvent::Pad { pad, event } => match self.ui.mode {
                Mode::Step => self.step_pad(pad, event, h, now_ms),
                Mode::Brain => self.brain_pad(pad, event, h),
                Mode::Punch => self.punch_pad(pad, event, h),
                Mode::Pads | Mode::Keys => self.play_pad(pad, event, h),
            },
            // VOLUME held: the eight knobs are the mix, one for each track (SHIFT: the pan)
            ControlEvent::Knob { index, delta } if self.ui.held == Some(op8_ui::Held::Volume) => self.mixer_knob(index as usize, delta, h),
            ControlEvent::Knob { index, delta } => self.knob(index as usize, delta, h, now_ms),
            ControlEvent::KnobTouch { index, touched } => {
                if let Some(t) = self.ui.knob_touch.get_mut(index as usize) {
                    *t = touched;
                }
            }
            ControlEvent::Encoder { delta } if self.ui.held.is_some() => match self.ui.held {
                Some(op8_ui::Held::Swing) => {
                    h.seq(Command::SetSwing(h.project.swing + delta as f32 * 0.005));
                }
                Some(op8_ui::Held::Volume) => {
                    h.send(EngineCommand::MasterVolume(h.master + delta as f32 * 0.01));
                }
                _ => {
                    let step = if self.held.shift { 0.1 } else { 1.0 };
                    h.seq(Command::SetBpm(h.project.bpm + delta as f32 * step));
                }
            },
            ControlEvent::Encoder { delta } => {
                if self.ui.inspect_step.is_some() {
                    let max = self
                        .inspected(h)
                        .map_or(0, |s| s.components.len().min(MAX_COMPONENTS - 1));
                    self.ui.comp_cursor =
                        (self.ui.comp_cursor as i32 + delta).clamp(0, max as i32) as usize;
                }
                // anything else: the tempo is only turned while TEMPO is held (above)
            }
            ControlEvent::Strip { position } => {
                self.ui.strip = position;
                let v = position.map_or(0.0, |x| x * 2.0 - 1.0);
                h.send(EngineCommand::MasterFilter(v));
            }
        }
    }

    fn inspected<'a>(&self, h: &'a EngineHandle) -> Option<&'a Step> {
        self.ui
            .inspect_step
            .map(|s| &h.project.tracks[self.ui.track].steps[s as usize])
    }

    fn button(&mut self, b: Button, pressed: bool, h: &mut EngineHandle) {
        if pressed {
            self.down.insert(b);
        } else {
            self.down.remove(&b);
        }
        // TEMPO, SWING and VOLUME: while one is held the 4D encoder turns it
        let held = match b {
            Button::Tempo => Some(op8_ui::Held::Tempo),
            Button::Swing => Some(op8_ui::Held::Swing),
            Button::Volume => Some(op8_ui::Held::Volume),
            _ => None,
        };
        if let Some(hd) = held {
            self.ui.held = if pressed && self.ui.main == Main::Instrument { Some(hd) } else { None };
        }
        match b {
            Button::Shift => {
                self.held.shift = pressed;
                self.ui.shift = pressed;
            }
            Button::Mute => self.held.mute = pressed,
            Button::Erase => self.held.erase = pressed,
            Button::Select => self.held.select = pressed,
            _ if !pressed => {}
            Button::Group(_) if self.ui.main != Main::Instrument => {}
            Button::Group(g) => {
                let t = g as usize;
                if self.held.shift {
                    h.seq(Command::SetBrainLink {
                        track: g,
                        linked: !h.project.tracks[t].brain_link,
                    });
                } else if self.held.mute {
                    h.seq(Command::SetMute {
                        track: g,
                        muted: !h.project.tracks[t].muted,
                    });
                } else {
                    if matches!(self.ui.mode, Mode::Pads | Mode::Keys | Mode::Brain) {
                        // held notes belong to the track they were played on
                        self.release_pads(h);
                    }
                    self.ui.track = t;
                    // a track shows its first page on the right (button 5) and, in STEP, its steps on the left (button 1)
                    self.ui.right_page = 0;
                    if self.ui.mode == Mode::Step {
                        self.ui.left_page = LeftPage::Steps;
                    }
                    self.ui.inspect_step = None;
                    self.inspector_pad = None;
                }
            }
            Button::Play => h.toggle_play(),
            Button::Stop => {
                h.send(EngineCommand::Stop);
            }
            Button::Restart => {
                h.send(EngineCommand::Play);
            }
            Button::Step => self.set_mode(Mode::Step, h),
            // Pressing the button of the mode you are in leaves it for STEP, so
            // there is always a way back to the sequencer.
            Button::PadMode => self.toggle_mode(Mode::Pads, h),
            Button::Keyboard => self.toggle_mode(Mode::Keys, h),
            Button::Chords => self.toggle_mode(Mode::Brain, h),
            Button::Perform | Button::NoteRepeat => self.toggle_mode(Mode::Punch, h),
            // Main modes: SETTINGS toggles the settings view (both screens);
            // CHANNEL goes back to composing.
            Button::Settings => {
                let to = if self.ui.main == Main::Settings {
                    Main::Instrument
                } else {
                    Main::Settings
                };
                self.set_main(to, h);
            }
            // MIXER toggles the mixer view (both screens), the way SETTINGS toggles its own
            Button::Mixer => {
                let to = if self.ui.main == Main::Mixer { Main::Instrument } else { Main::Mixer };
                self.set_main(to, h);
            }
            Button::Rec if self.ui.main == Main::Instrument => self.ui.rec = !self.ui.rec,
            Button::Rec if self.ui.main == Main::Sampling => self.toggle_record(h),
            Button::Tap if self.ui.main == Main::Sampling => {
                let on = !self.ui.smp.count_in;
                self.ui.smp.count_in = on;
                self.ui.smp.note = if on { "count-in on: REC counts three beats" } else { "count-in off: REC starts at once" }.into();
            }
            Button::Browser => {
                let to = if self.ui.main == Main::Browser { Main::Instrument } else { Main::Browser };
                self.set_main(to, h);
            }
            Button::Sampling => {
                let to = if self.ui.main == Main::Sampling { Main::Instrument } else { Main::Sampling };
                self.set_main(to, h);
            }
            Button::Tap if self.ui.main == Main::Instrument && self.held.shift => {
                self.ui.metronome = !self.ui.metronome;
                h.send(EngineCommand::Metronome(self.ui.metronome));
            }
            Button::Tap if self.ui.main == Main::Instrument => self.tap_tempo(h),
            Button::FixedVel if self.ui.main == Main::Instrument => self.ui.fixed_vel = !self.ui.fixed_vel,
            Button::PlugIn => {
                let to = if self.ui.main == Main::Plugin { Main::Instrument } else { Main::Plugin };
                self.set_main(to, h);
            }
            Button::Pattern => {
                let to = if self.ui.main == Main::Pattern { Main::Instrument } else { Main::Pattern };
                self.set_main(to, h);
            }
            Button::File => {
                let to = if self.ui.main == Main::File { Main::Instrument } else { Main::File };
                self.set_main(to, h);
            }
            Button::ChannelMidi => self.set_main(Main::Instrument, h),
            Button::JogLeft if self.ui.main == Main::Instrument && self.ui.mode == Mode::Keys && self.ui.lane.is_none() => self.shift_octave(-1, h),
            Button::JogRight if self.ui.main == Main::Instrument && self.ui.mode == Mode::Keys && self.ui.lane.is_none() => self.shift_octave(1, h),
            Button::JogLeft if self.nav_active() => self.nav_dir(-1, 0, h),
            Button::JogRight if self.nav_active() => self.nav_dir(1, 0, h),
            Button::JogUp if self.nav_active() => self.nav_dir(0, -1, h),
            Button::JogDown if self.nav_active() => self.nav_dir(0, 1, h),
            Button::JogPush if self.nav_active() || (self.ui.main == Main::Instrument && self.ui.inspect_step.is_some()) => self.nav_push(h),
            Button::JogLeft if self.ui.main == Main::Name => self.ui.naming.move_cursor(-1),
            Button::JogRight if self.ui.main == Main::Name => self.ui.naming.move_cursor(1),
            Button::JogPush if self.ui.main == Main::Name => self.finish_naming(h),
            Button::JogLeft if self.ui.main == Main::Settings => self.step_theme(-1),
            Button::JogRight if self.ui.main == Main::Settings => self.step_theme(1),
            // Screen buttons: 1 = step view, 2 = components of the first step, 5 = sound, 6 = shape.
            // the four pages of settings are the buttons over the left screen
            Button::Screen(n @ 1..=4) if self.ui.main == Main::Settings => self.set_settings_page(n - 1),
            // in the mixer: 1 and 2 choose the four channels on the left, 5 and 6 the page of the compressor on the right
            Button::Screen(n @ (1 | 2)) if self.ui.main == Main::Mixer => self.ui.mixer.group = n - 1,
            // 3 and 4 are the sends: lit, the knobs under the left screen are how much each channel sends (again: back to the levels)
            Button::Screen(n @ (3 | 4)) if self.ui.main == Main::Mixer => {
                self.ui.mixer.send = if self.ui.mixer.send == n - 2 { 0 } else { n - 2 };
            }
            Button::Screen(n @ (5..=8)) if self.ui.main == Main::Mixer => {
                // `sends` again is its second page, where each send is taken from, and again the first
                self.ui.mixer.page = match (n, self.ui.mixer.page) {
                    (8, 3) => 4,
                    (8, _) => 3,
                    _ => n - 5,
                };
                self.ui.grabbed = false;
                // the focus is not left on a value the new page does not have
                if let NavKind::Value(k) = self.ui.focus {
                    if k >= 4 && (k as usize - 4) >= op8_ui::mixer_page_values(self.ui.mixer.page) {
                        self.ui.focus = NavKind::Value(4);
                    }
                }
            }
            Button::Screen(1) if self.ui.main == Main::Instrument => self.set_mode(Mode::Step, h),
            Button::Screen(3) if self.ui.main == Main::Instrument => self.open_left(LeftPage::Players, h),
            Button::Screen(4) if self.ui.main == Main::Instrument => self.open_left(LeftPage::Bar, h),
            Button::Screen(7) if self.ui.main == Main::Instrument => self.ui.right_page = 2,
            Button::Screen(8) if self.ui.main == Main::Instrument => self.ui.right_page = 3,
            Button::Screen(2) if self.ui.main == Main::Instrument => {
                self.set_mode(Mode::Step, h);
                self.ui.inspect_step = Some(0);
                self.ui.comp_cursor = 0;
                self.ui.focus = NavKind::Comp(0);
            }
            Button::Screen(5) if self.ui.main == Main::Instrument => self.ui.right_page = 0,
            Button::Screen(6) if self.ui.main == Main::Instrument => self.ui.right_page = 1,
            _ => {}
        }
    }

    /// Whether this step shows as on: on a kit, whether the chosen lane is on in it.
    fn lane_on(&self, kit: bool, step: &Step) -> bool {
        match (kit, self.ui.lane) {
            (true, Some(l)) => step.lanes & (1 << l) != 0,
            _ => step.active,
        }
    }

    /// Look at the player's folder: which of the eight user slots hold a project.
    fn refresh_files(&mut self) {
        self.ui.confirm = None;
        self.ui.files = (0..16)
            .map(|i| {
                if i < 8 {
                    (crate::factory::NAMES[i].to_string(), true)
                } else {
                    let path = self.projects_dir.as_ref().map(|d| crate::projectfile::user_path(d, i - 8));
                    let has = path.as_ref().is_some_and(|p| p.is_file());
                    let name = path.as_deref().and_then(crate::projectfile::read_name).unwrap_or_else(|| format!("user {}", i - 7));
                    (if has { name } else { String::new() }, has)
                }
            })
            .collect();
    }

    /// The sixteen places of PLUG-IN and which one the selected track has now.
    /// SAMPLING is open on the audio input: the program opens the input stream only then.
    pub fn wants_input(&self) -> bool {
        self.ui.main == Main::Sampling && self.ui.smp.source == 0
    }

    // ---- navigation with the 4D encoder (docs/design.md §4) ----------------------------------------------------------

    /// A view with items to reach is open: the 4D encoder and its arrows move the focus in it.
    fn nav_active(&self) -> bool {
        !op8_ui::nav_items(&self.ui).is_empty()
    }

    /// The pad that is cell `slot` of the sixteen-place views and of the grid.
    fn slot_pad(slot: usize) -> u8 {
        (0..16u8).find(|&p| Self::pad_to_step(p) as usize == slot).unwrap_or(0)
    }

    /// Turn the 4D encoder: along the list, or the value that is grabbed.
    fn nav_turn(&mut self, delta: i32, h: &mut EngineHandle) {
        // an effect of PERFORM is sounding: the wheel is its intensity (what the pressure of the pad is); the arrows still move
        if self.ui.main == Main::Instrument && self.ui.mode == Mode::Punch && matches!(self.ui.focus, NavKind::Cell(_)) {
            if let Some((slot, v)) = self.ui.punch {
                let v = (v + delta as f32 * 0.04).clamp(0.0, 1.0);
                self.ui.punch = Some((slot, v));
                h.send(EngineCommand::PunchIntensity(v));
                return;
            }
        }
        if self.ui.grabbed {
            if let NavKind::Comp(_) = self.ui.focus {
                self.knob(3, delta * DETENT, h, self.now);
            }
            if let NavKind::Value(k) = self.ui.focus {
                if self.ui.main == Main::Sampling {
                    self.sampling_knob(k as usize, delta * DETENT, h);
                } else if self.ui.main == Main::Mixer {
                    self.mixer_page_knob(k as usize, delta * DETENT, h);
                } else {
                    self.knob(k as usize, delta * DETENT, h, self.now);
                }
            }
            return;
        }
        let items = op8_ui::nav_items(&self.ui);
        let at = op8_ui::nav_step(items.len(), op8_ui::nav_index(&self.ui), delta);
        if let Some(it) = items.get(at) {
            self.ui.focus = it.kind;
        }
        self.sync_comp_cursor();
    }

    /// In the step inspector the focus on a component is the component cursor (what knob 4 edits).
    fn sync_comp_cursor(&mut self) {
        if let (NavKind::Comp(i), Some(_)) = (self.ui.focus, self.ui.inspect_step) {
            self.ui.comp_cursor = i as usize;
        }
    }

    /// The arrows of the 4D encoder: across the screens (a grabbed value is let go first).
    fn nav_dir(&mut self, dx: i32, dy: i32, _h: &mut EngineHandle) {
        self.ui.grabbed = false;
        let items = op8_ui::nav_items(&self.ui);
        let at = op8_ui::nav_move_dir(&items, op8_ui::nav_index(&self.ui), dx, dy);
        if let Some(it) = items.get(at) {
            self.ui.focus = it.kind;
        }
        self.sync_comp_cursor();
    }

    /// Push the 4D encoder: do what tapping the item would (a place or a pad is the pad itself, a screen button is that button, a track
    /// is its group button; SHIFT does what SHIFT does to them), or grab or let go of a value.
    fn nav_push(&mut self, h: &mut EngineHandle) {
        let Some(item) = op8_ui::nav_focused(&self.ui) else { return };
        match item.kind {
            NavKind::Value(_) => self.ui.grabbed = !self.ui.grabbed,
            NavKind::Pick(n) => self.settings_pick(n as usize, h),
            // a component of the inspector: grab it (turning changes its kind, as knob 4 does) or let go
            NavKind::Comp(i) => {
                self.ui.comp_cursor = i as usize;
                self.ui.grabbed = !self.ui.grabbed;
            }
            NavKind::Sound(i) => {
                // the kit's sound is parameter 7 (what the knob `sound` sets): choose it and hear it
                h.send(EngineCommand::SetParam { track: self.ui.track as u8, index: 7, value: (i as f32 + 0.5) / 8.0 });
                h.send(EngineCommand::Audition { track: self.ui.track as u8, note: op8_seq::step::KIT_BASE_NOTE + i, velocity: 0.9 });
            }
            NavKind::Strip(n) => {
                self.button(Button::Screen(n + 1), true, h);
                self.button(Button::Screen(n + 1), false, h);
            }
            NavKind::Track(t) => {
                self.button(Button::Group(t), true, h);
                self.button(Button::Group(t), false, h);
            }
            NavKind::Cell(c) => {
                let pad = Self::slot_pad(c as usize);
                if self.ui.main == Main::Instrument {
                    // SHIFT + push on a step opens its inspector (what holding the pad does)
                    if self.held.shift && self.ui.mode == Mode::Step {
                        self.ui.inspect_step = Some(c);
                        self.ui.comp_cursor = 0;
                        self.ui.focus = NavKind::Comp(0);
                        self.ui.left_page = LeftPage::Steps;
                        return;
                    }
                    // an effect of PERFORM is engaged and kept (the wheel is then its intensity); a second push lets it go
                    if self.ui.mode == Mode::Punch {
                        let slot = op8_ui::nav_punch_slot(c as usize);
                        if self.ui.punch.map(|(s, _)| s) == Some(slot) {
                            self.ui.punch = None;
                            h.send(EngineCommand::Punch(None, 0.0));
                        } else {
                            self.ui.punch = Some((slot, 0.5));
                            h.send(EngineCommand::Punch(Some(PUNCH_SLOTS[slot].0), 0.5));
                        }
                        return;
                    }
                    let now = self.now;
                    for ev in [PadEvent::Press, PadEvent::Release] {
                        match self.ui.mode {
                            Mode::Step => self.step_pad(pad, ev, h, now),
                            Mode::Brain => self.brain_pad(pad, ev, h),
                            Mode::Punch => self.punch_pad(pad, ev, h),
                            Mode::Pads | Mode::Keys => self.play_pad(pad, ev, h),
                        }
                    }
                    return;
                }
                for ev in [PadEvent::Press, PadEvent::Release] {
                    match self.ui.main {
                        Main::File => self.file_pad(pad, ev, h),
                        Main::Plugin => self.plugin_pad(pad, ev, h),
                        Main::Pattern => self.pattern_pad(pad, ev, h),
                        Main::Browser => self.browser_pad(pad, ev, h),
                        Main::Sampling => self.sampling_pad(pad, ev, h),
                        _ => {}
                    }
                }
            }
        }
    }

    /// The name of the instrument on the selected track.
    fn instrument_name(&self, h: &EngineHandle) -> Option<String> {
        h.slots[self.ui.track].as_ref().and_then(|c| c.lock().ok().map(|c| c.info().name))
    }

    /// The file of a preset of an instrument: `slot` is the pad (8..16 are the player's).
    fn preset_file(&self, name: &str, slot: usize) -> Option<std::path::PathBuf> {
        let safe: String = name.chars().filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_').collect();
        Some(self.presets_dir.as_ref()?.join(safe).join(format!("user-{}.preset", slot - 7)))
    }

    /// The sixteen presets of the selected track's instrument: the top eight come with it (`presets.txt`), the bottom eight are the player's files.
    fn refresh_browser(&mut self, h: &EngineHandle) {
        self.ui.confirm = None;
        let name = self.instrument_name(h);
        self.ui.browse_inst = name.clone().unwrap_or_else(|| "empty track".into());
        let factory = name.as_deref().map(op8_cartridges::presets::factory).unwrap_or_default();
        self.ui.browse = (0..16usize)
            .map(|i| {
                if i < 8 {
                    return factory.get(i).map_or((String::new(), false), |p| (p.0.clone(), true));
                }
                let file = name.as_ref().and_then(|n| self.preset_file(n, i));
                let has = file.as_ref().is_some_and(|p| p.is_file());
                (file.as_deref().and_then(crate::projectfile::read_name).unwrap_or_else(|| format!("user {}", i - 7)), has)
            })
            .collect();
    }

    /// The pads of BROWSER: a tap loads the preset into the track's instrument and plays a note; SHIFT + tap on the
    /// player's eight saves the instrument's knobs into it.
    fn browser_pad(&mut self, pad: u8, ev: PadEvent, h: &mut EngineHandle) {
        if !matches!(ev, PadEvent::Press | PadEvent::Strike(_)) || self.pad_down_at[pad as usize].is_some() {
            if ev == PadEvent::Release {
                self.pad_down_at[pad as usize] = None;
            }
            return;
        }
        self.pad_down_at[pad as usize] = Some(0);
        let slot = Self::pad_to_step(pad) as usize;
        self.ui.browse_last = Some(slot);
        if self.held.select {
            let file = self.instrument_name(h).and_then(|n| if slot >= 8 { self.preset_file(&n, slot) } else { None });
            self.begin_naming(slot, file, "preset", Main::Browser, h);
            return;
        }
        if self.held.erase {
            let file = self.instrument_name(h).and_then(|n| if slot >= 8 { self.preset_file(&n, slot) } else { None });
            let (note, done) = self.erase_place(slot, file, "preset");
            self.ui.browse_note = note;
            if done {
                self.refresh_browser(h);
            }
            return;
        }
        self.ui.confirm = None;
        let track = self.ui.track;
        let (Some(name), Some(cart)) = (self.instrument_name(h), h.slots[track].clone()) else {
            self.ui.browse_note = "this track has no instrument (PLUG-IN)".into();
            return;
        };
        if slot < 8 {
            if self.held.shift {
                self.ui.browse_note = "the presets that come with it stay as they are".into();
                return;
            }
            let Some((preset, knobs)) = op8_cartridges::presets::factory(&name).into_iter().nth(slot) else {
                self.ui.browse_note = "empty: this instrument has fewer presets".into();
                return;
            };
            for (i, v) in knobs.into_iter().enumerate() {
                if let Some(value) = v {
                    h.send(EngineCommand::SetParam { track: track as u8, index: i as u8, value });
                }
            }
            self.ui.browse_note = format!("loaded {preset}");
            let note = self.audition_note(h);
            self.audition(h, note, 0.9);
            return;
        }
        let Some(file) = self.preset_file(&name, slot) else {
            self.ui.browse_note = "nowhere to save".into();
            return;
        };
        if self.held.shift {
            let Some(state) = cart.lock().ok().map(|c| c.save_state()) else { return };
            let mut bytes = b"OP8S\x01".to_vec();
            bytes.extend_from_slice(&state);
            let tmp = file.with_extension("tmp");
            let wrote = file
                .parent()
                .map_or(Ok(()), std::fs::create_dir_all)
                .and_then(|_| std::fs::write(&tmp, &bytes))
                .and_then(|_| std::fs::rename(&tmp, &file));
            self.ui.browse_note = match wrote {
                Ok(()) => {
                    if crate::projectfile::read_name(&file).is_none() {
                        crate::projectfile::write_name(&file, &crate::projectfile::today());
                    }
                    format!("saved in user {}", slot - 7)
                }
                Err(e) => format!("could not save: {e}"),
            };
            self.refresh_browser(h);
            return;
        }
        let Some(bytes) = std::fs::read(&file).ok().filter(|b| b.starts_with(b"OP8S\x01")) else {
            self.ui.browse_note = "empty: SHIFT + tap saves here".into();
            return;
        };
        // the knobs of the instrument take the preset's values; the engine is told so that it keeps them as its own
        let values: Vec<Option<f32>> = match cart.lock() {
            Ok(mut c) => {
                c.load_state(&bytes[5..]);
                (0..8).map(|i| c.parameter_value(i)).collect()
            }
            Err(_) => return,
        };
        for (i, v) in values.into_iter().enumerate() {
            if let Some(value) = v {
                h.send(EngineCommand::SetParam { track: track as u8, index: i as u8, value });
            }
        }
        self.ui.browse_note = format!("loaded user {}", slot - 7);
        let note = self.audition_note(h);
        self.audition(h, note, 0.9);
    }

    /// Remember what the samples folder looks like now, without loading anything: the instruments were just made from it (or the player's
    /// folder was just chosen). Whatever changes in it after this is what `rescan_samples` puts in.
    pub fn watch_samples(&mut self) {
        let found = self.samples_dir.as_deref().map(op8_cartridges::sampler::scan_folder).unwrap_or_default();
        self.sample_stamps = (0..8).map(|k| found.get(k).and_then(|f| f.as_ref()).and_then(|f| stamp_of(&f.path))).collect();
    }

    /// Look at the samples folder again and put in the instruments the sounds that are new or changed since the last look: the player can
    /// copy WAV files in while OP-8 runs. A place whose file is gone becomes empty. Returns how many places were filled or emptied.
    fn rescan_samples(&mut self, h: &mut EngineHandle) -> usize {
        let Some(dir) = self.samples_dir.clone() else { return 0 };
        let found = op8_cartridges::sampler::scan_folder(&dir);
        self.sample_stamps.resize(8, None);
        let mut changed = 0;
        for (k, f) in found.into_iter().enumerate().take(8) {
            let stamp = f.as_ref().and_then(|f| stamp_of(&f.path));
            if stamp == self.sample_stamps[k] {
                continue;
            }
            let data = f.map_or_else(Vec::new, |f| f.sample.data().to_vec());
            for c in h.slots.iter().flatten() {
                if let Ok(mut c) = c.lock() {
                    c.load_sample(k, data.clone());
                }
            }
            self.sample_stamps[k] = stamp;
            changed += 1;
        }
        if changed > 0 {
            self.refresh_samples();
        }
        changed
    }

    /// Which of the eight places of the player's own have a recording file.
    fn refresh_samples(&mut self) {
        let dir = self.samples_dir.clone();
        for k in 0..8 {
            let file = dir.as_ref().map(|d| d.join(op8_cartridges::sampler::recording_file(k)));
            self.ui.smp.saved[k] = file.as_ref().is_some_and(|f| f.is_file());
            self.ui.smp.names[k] = file.filter(|f| f.is_file()).and_then(|f| crate::projectfile::read_name(&f)).unwrap_or_default();
        }
    }

    /// REC in SAMPLING: start recording from the chosen source, or stop.
    fn toggle_record(&mut self, h: &mut EngineHandle) {
        if self.countin.take().is_some() {
            // REC during the count-in cancels it
            self.ui.smp.counting = 0;
            return;
        }
        if self.ui.smp.recording || self.ui.smp.armed {
            h.send(EngineCommand::Record(None));
            self.ui.smp.recording = false;
            self.ui.smp.armed = false;
            return;
        }
        let source = if self.ui.smp.source == 0 { op8_engine::RecSource::Input } else { op8_engine::RecSource::Resample };
        // SHIFT + REC waits for a sound; with the count-in on, REC counts three beats first; otherwise it starts at once
        let arm = self.held.shift;
        if !arm && self.ui.smp.count_in {
            self.countin = Some(CountIn { from_ms: self.now, source, beats_done: 0 });
            self.ui.smp.counting = COUNT_IN_BEATS;
            self.ui.smp.note = String::new();
            return;
        }
        if arm {
            h.send(EngineCommand::RecordWhenLoud(source, op8_ui::SAMPLING_ARM_THRESHOLD));
        } else {
            h.send(EngineCommand::Record(Some(source)));
        }
        // the take starts again: the part that is kept is all of it
        let s = &mut self.ui.smp;
        (s.recording, s.len, s.start, s.end, s.peaks, s.note) = (!arm, 0, 0.0, 1.0, Vec::new(), String::new());
        s.armed = arm;
        self.smp_drawn = 0;
    }

    /// Every frame in SAMPLING: what the engine has recorded, how loud it is, and the wave to draw.
    fn sampling_tick(&mut self, h: &mut EngineHandle, now_ms: u64) {
        if let Some(c) = &mut self.countin {
            // a beat of the project's tempo each: a click at the start of every one, and the recording after the last
            let beat_ms = (60_000.0 / h.project.bpm.max(20.0)) as u64;
            let beat = ((now_ms.saturating_sub(c.from_ms)) / beat_ms.max(1)) as u8;
            if beat >= COUNT_IN_BEATS {
                let source = c.source;
                self.countin = None;
                self.ui.smp.counting = 0;
                h.send(EngineCommand::Record(Some(source)));
                let s = &mut self.ui.smp;
                (s.recording, s.len, s.start, s.end, s.peaks, s.note) = (true, 0, 0.0, 1.0, Vec::new(), String::new());
                self.smp_drawn = 0;
            } else {
                if beat >= c.beats_done {
                    // the last click of the count is the high one
                    h.send(EngineCommand::Click(beat + 1 == COUNT_IN_BEATS));
                    c.beats_done = beat + 1;
                }
                self.ui.smp.counting = COUNT_IN_BEATS - beat;
            }
        }
        let st = &h.status;
        let s = &mut self.ui.smp;
        s.phase = (now_ms % 2000) as f32 / 2000.0;
        let was = s.recording;
        s.recording = st.recording.load(Relaxed);
        s.armed = st.armed.load(Relaxed);
        s.len = st.rec_len.load(Relaxed) as usize;
        let peak = st.rec_peak();
        s.level = if s.recording || s.armed { peak.max(s.level * 0.9) } else { s.level * 0.9 };
        if was && !s.recording && s.len > 0 {
            s.note = if s.len >= op8_engine::MAX_RECORD { "ten seconds: that is all it keeps".into() } else { String::new() };
        }
        if s.len != self.smp_drawn {
            if let Ok(take) = st.take.try_lock() {
                s.peaks = op8_ui::peaks_of(&take[..s.len.min(take.len())]);
                self.smp_drawn = s.len;
            }
        }
    }

    /// The knobs under the right screen: source, start, end, fade.
    fn sampling_knob(&mut self, k: usize, delta: i32, h: &mut EngineHandle) {
        if !(4..8).contains(&k) {
            return;
        }
        let d = delta as f32 / SWEEP;
        let n = if k == 4 { self.detents(k, delta) } else { 0 };
        let s = &mut self.ui.smp;
        match k {
            4 if !s.recording && !s.armed && s.counting == 0 => s.source = (s.source as i32 + n).clamp(0, 1) as u8,
            5 => s.start = (s.start + d).clamp(0.0, (s.end - 0.01).max(0.0)),
            6 => s.end = (s.end + d).clamp((s.start + 0.01).min(1.0), 1.0),
            7 => s.fade = (s.fade + d).clamp(0.0, 1.0),
            _ => {}
        }
        if k == 5 || k == 6 {
            // a part that is playing follows the cut (the engine does nothing if none is)
            let (a, b) = self.ui.smp.window();
            h.send(EngineCommand::PreviewWindow(a as u32, b as u32));
        }
    }

    /// The part of the take that is kept, faded at both ends and brought to the same level as every sample.
    fn kept(&self, h: &EngineHandle) -> Option<Vec<f32>> {
        let s = &self.ui.smp;
        let (a, b) = s.window();
        if b <= a {
            return None;
        }
        let take = h.status.take.try_lock().ok()?;
        let mut data: Vec<f32> = take[a..b].to_vec();
        drop(take);
        let fade = s.fade_samples().max(48).min(data.len() / 2);
        for i in 0..fade {
            let g = i as f32 / fade as f32;
            data[i] *= g;
            let j = data.len() - 1 - i;
            data[j] *= g;
        }
        let peak = data.iter().fold(0.0f32, |m, x| m.max(x.abs()));
        if peak > 1e-6 {
            let g = 0.9 / peak;
            data.iter_mut().for_each(|x| *x *= g);
        }
        Some(data)
    }

    /// The pads of SAMPLING: the top two rows save the kept part into a place, the bottom two listen to it.
    fn sampling_pad(&mut self, pad: u8, ev: PadEvent, h: &mut EngineHandle) {
        if !matches!(ev, PadEvent::Press | PadEvent::Strike(_)) {
            return;
        }
        if self.ui.smp.recording || self.ui.smp.armed || self.ui.smp.counting > 0 {
            self.ui.smp.note = "stop recording first (REC)".into();
            return;
        }
        let slot = Self::pad_to_step(pad) as usize;
        // SELECT held: name the recording that is in that place (it has none of its own to name before it is saved)
        if self.held.select && slot < 8 {
            let path = self.samples_dir.as_ref().map(|d| d.join(op8_cartridges::sampler::recording_file(slot)));
            self.begin_naming(slot + 8, path, "recording", Main::Sampling, h);
            return;
        }
        let Some(data) = self.kept(h) else {
            self.ui.smp.note = "nothing recorded yet (REC)".into();
            return;
        };
        if slot >= 8 {
            let (a, b) = self.ui.smp.window();
            h.send(EngineCommand::Preview(Some((a as u32, b as u32))));
            return;
        }
        let Some(dir) = self.samples_dir.clone() else {
            self.ui.smp.note = "no folder for samples".into();
            return;
        };
        let file = dir.join(op8_cartridges::sampler::recording_file(slot));
        let tmp = file.with_extension("wav.tmp");
        let wrote = std::fs::create_dir_all(&dir)
            .and_then(|_| std::fs::write(&tmp, op8_cartridges::wav::encode_16(&data, op8_cartridge::SAMPLE_RATE as u32)))
            .and_then(|_| std::fs::rename(&tmp, &file));
        if let Err(e) = wrote {
            self.ui.smp.note = format!("could not save: {e}");
            return;
        }
        // every sampler that is playing gets it at once
        for c in h.slots.iter().flatten() {
            if let Ok(mut c) = c.lock() {
                c.load_sample(slot, data.clone());
            }
        }
        // what was just written is what the instruments have: a later look at the folder does not load it again
        self.sample_stamps.resize(8, None);
        self.sample_stamps[slot] = stamp_of(&file);
        self.refresh_samples();
        self.ui.smp.note = format!("saved in user {}", slot + 1);
    }

    fn refresh_plugs(&mut self, h: &EngineHandle) {
        self.ui.plugs = (0..16)
            .map(|i| match FACTORY_INSTRUMENTS.get(i) {
                Some((name, _, built)) => (name.to_string(), *built),
                None => (String::new(), false), // the places for installed instruments start empty
            })
            .collect();
        let now = h.slots[self.ui.track].as_ref().and_then(|c| c.lock().ok().map(|c| c.info().name));
        self.ui.plug_current = now.and_then(|n| FACTORY_INSTRUMENTS.iter().position(|f| f.1 == n));
    }

    /// PLUG-IN: a pad puts that instrument on the selected track. The MIDI out is one bridge that owns the MIDI
    /// queue, so choosing it moves the bridge from the track that has it.
    fn plugin_pad(&mut self, pad: u8, ev: PadEvent, h: &mut EngineHandle) {
        if !matches!(ev, PadEvent::Press | PadEvent::Strike(_)) || self.pad_down_at[pad as usize].is_some() {
            if ev == PadEvent::Release {
                self.pad_down_at[pad as usize] = None;
            }
            return;
        }
        self.pad_down_at[pad as usize] = Some(0);
        let slot = Self::pad_to_step(pad) as usize;
        if let Err(note) = self.put_instrument(self.ui.track, slot, h) {
            self.ui.plug_note = note;
            return;
        }
        self.ui.right_page = 0;
        self.ui.plug_note = String::new();
        self.set_main(Main::Instrument, h);
        // the pad is let go in composing: that release must not toggle a step
        self.pad_down_at[pad as usize] = None;
    }

    /// Put the instrument of place `slot` of PLUG-IN on `track` (what a pad of PLUG-IN does, and a program change). The MIDI out is one
    /// bridge that owns the MIDI queue, so choosing it moves the bridge from the track that has it. The reason, if it cannot.
    fn put_instrument(&mut self, track: usize, slot: usize, h: &mut EngineHandle) -> Result<(), String> {
        let Some(&(label, name, built)) = FACTORY_INSTRUMENTS.get(slot) else {
            return Err("empty: installed instruments will go here".into());
        };
        if !built {
            return Err(format!("{label} is not built yet"));
        }
        let current = h.slots[track].as_ref().and_then(|c| c.lock().ok().map(|c| c.info().name));
        if current.as_deref() != Some(name) {
            if name == "MidiOutBridge" {
                // move the running bridge here
                let from = (0..TRACKS).find(|&t| h.slots[t].as_ref().is_some_and(|c| c.lock().is_ok_and(|c| c.info().name == name)));
                let Some(from) = from else {
                    return Err("no MIDI out is running".into());
                };
                let bridge = h.slots[from].take();
                h.send(EngineCommand::LoadCartridge { track: from as u8, cartridge: None });
                h.slots[track] = bridge.clone();
                h.send(EngineCommand::LoadCartridge { track: track as u8, cartridge: bridge });
            } else if let Some(c) = op8_cartridges::create(name) {
                h.load_cartridge(track, c);
            }
            // a drum kit is programmed in lanes: the track follows the instrument
            if name != "drums" && h.project.tracks[track].kit {
                h.seq(Command::SetKit { track: track as u8, kit: false });
            }
        }
        Ok(())
    }

    /// PATTERN: a tap on a pad shows that pattern of the selected track and has it play from the start of the track's next loop (SHIFT:
    /// at once); a pad held and another tapped copies the first over the second; ERASE and a tap empties it.
    fn pattern_pad(&mut self, pad: u8, ev: PadEvent, h: &mut EngineHandle) {
        let (track, slot) = (self.ui.track as u8, Self::pad_to_step(pad));
        match ev {
            PadEvent::Press | PadEvent::Strike(_) => {
                if self.pad_down_at[pad as usize].is_some() {
                    return;
                }
                self.pad_down_at[pad as usize] = Some(0);
                if self.held.erase {
                    h.seq(Command::ClearPattern { track, index: slot });
                    self.ui.pattern_note = format!("pattern {} is empty", slot + 1);
                } else if let Some((first, _)) = self.pattern_held.filter(|(first, _)| *first != pad) {
                    let from = Self::pad_to_step(first);
                    h.seq(Command::CopyPattern { track, from, to: slot });
                    self.ui.pattern_note = format!("pattern {} copied over {}", from + 1, slot + 1);
                    self.pattern_held = Some((first, true));
                } else if self.pattern_held.is_none() {
                    self.pattern_held = Some((pad, false));
                }
            }
            PadEvent::Release => {
                let was_down = self.pad_down_at[pad as usize].take().is_some();
                if let Some((first, copied)) = self.pattern_held {
                    if first == pad {
                        self.pattern_held = None;
                        if was_down && !copied {
                            let now = self.held.shift;
                            h.seq(Command::SetPattern { track, index: slot, now });
                            self.ui.pattern_note = format!("pattern {}{}", slot + 1, if now { " plays now" } else { ": from the next loop" });
                            self.ui.focus = NavKind::Cell(slot);
                        }
                    }
                }
            }
            PadEvent::Pressure(_) => {}
        }
    }

    /// FILE: a pad opens the project in its slot; SHIFT + a pad of the player's slots saves into it.
    fn file_pad(&mut self, pad: u8, ev: PadEvent, h: &mut EngineHandle) {
        if !matches!(ev, PadEvent::Press | PadEvent::Strike(_)) || self.pad_down_at[pad as usize].is_some() {
            if ev == PadEvent::Release {
                self.pad_down_at[pad as usize] = None;
            }
            return;
        }
        self.pad_down_at[pad as usize] = Some(0);
        let slot = Self::pad_to_step(pad) as usize; // reading order: the top two rows are the factory projects
        self.ui.file_last = Some(slot);
        if self.held.select {
            let path = (slot >= 8).then(|| self.projects_dir.as_ref().map(|d| crate::projectfile::user_path(d, slot - 8))).flatten();
            self.begin_naming(slot, path, "project", Main::File, h);
            return;
        }
        if self.held.erase {
            let path = (slot >= 8).then(|| self.projects_dir.as_ref().map(|d| crate::projectfile::user_path(d, slot - 8))).flatten();
            let (note, done) = self.erase_place(slot, path, "project");
            self.ui.file_note = note;
            if done {
                self.refresh_files();
            }
            return;
        }
        self.ui.confirm = None;
        if self.held.shift {
            if slot < 8 {
                self.ui.file_note = "the factory projects stay as they are".into();
                return;
            }
            let Some(dir) = &self.projects_dir else {
                self.ui.file_note = "nowhere to save".into();
                return;
            };
            let f = crate::projectfile::ProjectFile::capture(h);
            let path = crate::projectfile::user_path(dir, slot - 8);
            self.ui.file_note = match crate::projectfile::save(&path, &f) {
                Ok(()) => {
                    // named by the day and the tempo, unless the player has named it already (the name is a text file next to it)
                    if crate::projectfile::read_name(&path).is_none() {
                        crate::projectfile::write_name(&path, &format!("{} {:.0}", crate::projectfile::today(), h.project.bpm));
                    }
                    format!("saved in user {}", slot - 7)
                }
                Err(e) => format!("could not save: {e}"),
            };
            self.refresh_files();
            return;
        }
        let path = (slot >= 8).then(|| self.projects_dir.as_ref().map(|d| crate::projectfile::user_path(d, slot - 8))).flatten();
        let file = if slot < 8 {
            Some(crate::factory::file(slot))
        } else {
            path.as_ref().and_then(|p| crate::projectfile::load(p))
        };
        match file {
            Some(f) => {
                f.apply(h);
                self.ui.file_note = String::new();
                self.set_main(Main::Instrument, h);
                self.ui.mode = Mode::Step;
                self.pad_down_at[pad as usize] = None; // see below: the release comes in composing
                self.show_the_music(h);
            }
            // a place with a file that cannot be read is left alone; a place with nothing in it opens a new, empty project
            None if path.as_ref().is_some_and(|p| p.exists()) => self.ui.file_note = "damaged: not loaded".into(),
            None => {
                crate::factory::blank().apply(h);
                self.ui.file_note = "a new empty project: SHIFT + tap saves it".into();
                self.set_main(Main::Instrument, h);
                self.ui.mode = Mode::Step;
                self.pad_down_at[pad as usize] = None;
            }
        }
    }

    /// ERASE held and a pad of FILE or BROWSER: the player's own places are erased, but only on the second touch of the same one (the
    /// first marks it, in red, and says so; any other touch lets it be). Returns what to say and whether something was erased.
    fn erase_place(&mut self, slot: usize, path: Option<std::path::PathBuf>, what: &str) -> (String, bool) {
        if slot < 8 {
            self.ui.confirm = None;
            return (format!("the {what}s that come with it stay as they are"), false);
        }
        let Some(path) = path.filter(|p| p.is_file()) else {
            self.ui.confirm = None;
            return ("nothing to erase there".into(), false);
        };
        if self.ui.confirm == Some(slot) {
            self.ui.confirm = None;
            let name = crate::projectfile::read_name(&path).unwrap_or_else(|| format!("user {}", slot - 7));
            return if crate::projectfile::erase(&path) { (format!("erased {name}"), true) } else { ("could not erase it".into(), false) };
        }
        self.ui.confirm = Some(slot);
        let name = crate::projectfile::read_name(&path).unwrap_or_else(|| format!("user {}", slot - 7));
        (format!("erase {name}? touch it again"), false)
    }

    /// After a project opens, put the screens on something: the first track that has steps, its first page, and on a kit the sound
    /// that has steps (so that what is drawn is not an empty lane).
    fn show_the_music(&mut self, h: &mut EngineHandle) {
        let tracks = h.project.tracks;
        let has = |t: usize| tracks[t].steps[..tracks[t].length as usize].iter().any(|s| s.active);
        if let Some(t) = (0..TRACKS).find(|&t| has(t)) {
            self.ui.track = t;
            if tracks[t].kit {
                let lane = (0..8u8).find(|&l| tracks[t].steps[..tracks[t].length as usize].iter().any(|s| s.lanes & (1 << l) != 0));
                if let Some(l) = lane {
                    h.send(EngineCommand::SetParam { track: t as u8, index: 7, value: (l as f32 + 0.5) / 8.0 });
                }
            }
        }
        self.ui.right_page = 0;
        self.ui.left_page = LeftPage::Steps;
        self.ui.inspect_step = None;
    }

    /// The note a tap on a step places on this track's instrument, if it chooses a sound by a knob.
    fn paint_note(&self, h: &EngineHandle) -> Option<u8> {
        let c = h.slots[self.ui.track].as_ref()?;
        let c = c.try_lock().ok()?;
        c.paint_note()
    }

    /// The note to play when a knob is turned: the chosen sound, else the track's first step, else a C.
    fn audition_note(&self, h: &EngineHandle) -> u8 {
        self.paint_note(h).unwrap_or_else(|| {
            h.project.tracks[self.ui.track].steps.iter().find(|s| s.active).map_or(48, |s| s.note)
        })
    }

    /// Play a note on the selected track now.
    fn audition(&mut self, h: &mut EngineHandle, note: u8, velocity: f32) {
        h.send(EngineCommand::Audition { track: self.ui.track as u8, note, velocity });
    }

    /// Let go of whatever a pad is holding: an engaged effect, a live chord.
    fn release_pads(&mut self, h: &mut EngineHandle) {
        if self.ui.mode == Mode::Punch && self.ui.punch.is_some() {
            h.send(EngineCommand::Punch(None, 0.0));
            self.ui.punch = None;
        }
        if self.ui.mode == Mode::Brain && self.ui.live_chord_pad.take().is_some() {
            h.seq(Command::SetLiveChord(None));
        }
        self.release_chord(h);
        for pad in 0..16 {
            if let Some(note) = self.held_notes[pad].take() {
                h.send(EngineCommand::LiveNote { track: self.ui.track as u8, note, velocity: 0.0, on: false });
            }
        }
        self.ui.pads_down = [false; 16];
        self.pad_down_at = [None; 16];
    }

    /// PAD MODE and KEYBOARD: a pad plays its note on the track's instrument while it is held (no recording yet).
    fn play_pad(&mut self, pad: u8, ev: PadEvent, h: &mut EngineHandle) {
        let p = pad as usize;
        let track = self.ui.track as u8;
        match ev {
            PadEvent::Press | PadEvent::Strike(_) if self.held_notes[p].is_none() => {
                let Some(note) = op8_ui::pad_note(&self.ui, &h.project, pad) else { return };
                let velocity = self.pad_velocity(if let PadEvent::Strike(v) = ev { v } else { 0.8 });
                h.send(EngineCommand::LiveNote { track, note, velocity, on: true });
                self.held_notes[p] = Some(note);
                self.ui.pads_down[p] = true;
                self.record(h, note, velocity);
            }
            PadEvent::Release => {
                if let Some(note) = self.held_notes[p].take() {
                    h.send(EngineCommand::LiveNote { track, note, velocity: 0.0, on: false });
                }
                self.ui.pads_down[p] = false;
            }
            _ => {}
        }
    }

    /// REC: write a note played by hand into the selected track, at the step nearest to now. On a drum
    /// kit the sound's lane is switched on in that step (so several sounds can share it); anywhere
    /// else the step takes the note and the velocity, keeping its gate and components. Does nothing
    /// unless REC is armed and the sequencer is running.
    fn record(&mut self, h: &mut EngineHandle, note: u8, velocity: f32) {
        use std::sync::atomic::Ordering::Relaxed;
        if !self.ui.rec || !h.status.playing.load(Relaxed) {
            return;
        }
        let t = self.ui.track;
        let len = h.project.tracks[t].len();
        let mut step = h.status.playhead[t].load(Relaxed) as usize % len;
        // nearest: late in the step it belongs to the next one; down: the step sounding; up: the next
        let late = h.status.phase[t].load(Relaxed) >= 128;
        if self.ui.quantize == 2 || (self.ui.quantize == 0 && late) {
            step = (step + 1) % len;
        }
        let track = t as u8;
        let mut v = h.project.tracks[t].steps[step];
        if self.ui.lane.is_some() {
            if !h.project.tracks[t].kit {
                h.seq(Command::SetKit { track, kit: true });
                v = h.project.tracks[t].steps[step];
            }
            v.lanes |= 1 << ((note - op8_seq::step::KIT_BASE_NOTE) % 8);
        } else {
            v.note = note;
        }
        v.active = true;
        v.velocity = self.rec_velocity(velocity);
        h.seq(Command::SetStep { track, step: step as u8, value: v });
    }

    /// The velocity written by REC: the strike's, or 100 with FIXED VEL.
    fn rec_velocity(&self, velocity: f32) -> u8 {
        if self.ui.fixed_vel {
            return FIXED_VELOCITY;
        }
        (velocity * 127.0).round().clamp(1.0, 127.0) as u8
    }

    /// The velocity a pad plays at: the strike's, or the fixed one.
    fn pad_velocity(&self, strike: f32) -> f32 {
        if self.ui.fixed_vel {
            FIXED_VELOCITY as f32 / 127.0
        } else {
            strike.clamp(0.1, 1.0)
        }
    }

    /// TAP: the tempo from the last taps (the first tap after a pause only starts counting).
    fn tap_tempo(&mut self, h: &mut EngineHandle) {
        if self.taps.last().is_some_and(|&t| self.now.saturating_sub(t) > 2000) {
            self.taps.clear();
        }
        self.taps.push(self.now);
        if self.taps.len() > 5 {
            self.taps.remove(0);
        }
        if self.taps.len() >= 2 {
            let span = (self.taps[self.taps.len() - 1] - self.taps[0]) as f32 / (self.taps.len() - 1) as f32;
            h.seq(Command::SetBpm(60_000.0 / span));
        }
    }

    /// KEYBOARD: move the pads an octave (the 4D encoder, left and right).
    fn shift_octave(&mut self, by: i32, h: &mut EngineHandle) {
        self.release_pads(h);
        self.ui.octave = (self.ui.octave + by).clamp(-2, 4);
    }

    /// Like `set_mode`, but the button of the mode already active goes back to STEP.
    fn toggle_mode(&mut self, m: Mode, h: &mut EngineHandle) {
        let to = if self.ui.main == Main::Instrument && self.ui.mode == m { Mode::Step } else { m };
        self.set_mode(to, h);
    }

    /// Screen buttons 3 and 4: the step view, on that page of the left screen; the same button again goes back to the steps.
    fn open_left(&mut self, page: LeftPage, h: &mut EngineHandle) {
        let open = self.ui.main == Main::Instrument
            && self.ui.mode == Mode::Step
            && self.ui.inspect_step.is_none()
            && self.ui.left_page == page;
        self.set_mode(Mode::Step, h);
        if !open {
            self.ui.left_page = page;
        }
    }

    /// A pad mode button: choose the pad mode, and go back to composing.
    fn set_mode(&mut self, m: Mode, h: &mut EngineHandle) {
        self.release_pads(h);
        self.ui.main = Main::Instrument;
        self.ui.left_page = LeftPage::Steps;
        self.ui.mode = m;
        self.ui.inspect_step = None;
        self.inspector_pad = None;
        // PERFORM is played on its sixteen effects: the focus starts on one (unless it already is on a pad)
        if m == Mode::Punch && !matches!(self.ui.focus, NavKind::Cell(_)) {
            self.ui.focus = NavKind::Cell(0);
        }
    }

    /// Change the main mode (docs/buttons.md §1). Entering settings lets go of
    /// the pads, which do nothing there.
    fn set_main(&mut self, m: Main, h: &mut EngineHandle) {
        if m != Main::Instrument {
            self.release_pads(h);
        }
        if m == Main::File {
            self.refresh_files();
            self.ui.file_note.clear();
        }
        // the player may have copied WAV files into the folder: the sounds that are new are put in the instruments that play
        let news = if matches!(m, Main::Plugin | Main::Sampling) && self.ui.main != m { self.rescan_samples(h) } else { 0 };
        let heard = |n: usize| if n == 0 { String::new() } else { format!("loaded {n} {} from the folder", if n == 1 { "sound" } else { "sounds" }) };
        if m == Main::Settings {
            // the pages that show devices look at them when settings opens
            self.ui.settings.note.clear();
            self.sync_settings_view();
            if matches!(self.ui.settings.page, 1 | 2) {
                self.requests.push(Request::ListDevices);
            }
        }
        if m == Main::Plugin {
            self.refresh_plugs(h);
            self.ui.plug_note = heard(news);
        }
        if self.ui.main == Main::Sampling && m != Main::Sampling {
            // leaving: whatever was recording or playing stops (the take stays for when it comes back)
            h.send(EngineCommand::Record(None));
            h.send(EngineCommand::Preview(None));
        }
        if m == Main::Pattern {
            self.ui.pattern_note.clear();
            self.pattern_held = None;
        }
        if m == Main::Browser {
            self.refresh_browser(h);
            self.ui.browse_note.clear();
            self.ui.browse_last = None;
        }
        if m == Main::Sampling {
            self.refresh_samples();
            self.ui.smp.note = heard(news);
        }
        self.ui.main = m;
        // the focus starts on something sensible: where you were, or the first place
        self.ui.grabbed = false;
        self.ui.focus = match m {
            Main::File => NavKind::Cell(self.ui.file_last.unwrap_or(0) as u8),
            Main::Plugin => NavKind::Cell(self.ui.plug_current.unwrap_or(0) as u8),
            Main::Pattern => NavKind::Cell(h.project.current[self.ui.track]),
            // composing: on the track you are working on
            Main::Instrument => NavKind::Track(self.ui.track as u8),
            Main::Settings => NavKind::Pick(0),
            _ => NavKind::Cell(0),
        };
        self.ui.inspect_step = None;
        self.inspector_pad = None;
    }

    /// Move to the next (`d > 0`) or previous theme, wrapping. The theme
    /// applies at once; `theme_changed` tells the app to save it.
    fn step_theme(&mut self, d: i32) {
        let all = ThemeId::ALL;
        let now = all.iter().position(|t| *t == theme_id()).unwrap_or(0) as i32;
        let next = all[(now + d).rem_euclid(all.len() as i32) as usize];
        if next != theme_id() {
            set_theme(next);
            self.theme_changed = true;
            self.settings_changed = true;
        }
    }

    /// Open a page of settings (0 theme, 1 audio, 2 midi, 3 light). The pages that show devices ask for the lists.
    fn set_settings_page(&mut self, page: u8) {
        self.ui.settings.page = page.min(3);
        self.ui.settings.note.clear();
        self.ui.grabbed = false;
        self.ui.focus = NavKind::Pick(0);
        self.sync_settings_view();
        if matches!(page, 1 | 2) {
            self.requests.push(Request::ListDevices);
        }
    }

    /// The choices that are kept, as the pages show them: the entry of each list that is chosen (0 is automatic), the block, the steps of
    /// the light.
    fn sync_settings_view(&mut self) {
        let (s, v) = (&self.settings, &mut self.ui.settings);
        let at = |list: &[String], want: &Option<String>| want.as_ref().and_then(|w| list.iter().position(|n| n.contains(w.as_str()))).map_or(0, |i| i + 1);
        v.out_sel = at(&v.outs, &s.audio_out);
        v.in_sel = at(&v.ins, &s.audio_in);
        v.port_sel = at(&v.ports, &s.midi_out);
        v.midi_in_sel = at(&v.midi_ins, &s.midi_in);
        v.midi_by_channel = s.midi_by_channel;
        v.buffer = s.buffer.unwrap_or(128);
        v.screen = s.screen;
        v.pads = s.pads;
        v.midi_clock = s.midi_clock;
        v.midi_clock_in = s.midi_clock_in;
        v.learned = s.midi_map.len();
    }

    /// What the hardware answered: the lists and what is open.
    pub fn devices_listed(&mut self, d: DeviceList) {
        let v = &mut self.ui.settings;
        (v.outs, v.ins, v.ports, v.midi_ins) = (d.outs, d.ins, d.ports, d.midi_ins);
        (v.out_now, v.in_now, v.port_now, v.midi_in_now, v.stream_now) = (d.out_now, d.in_now, d.port_now, d.midi_in_now, d.stream_now);
        if !d.supply.is_empty() {
            v.supply = d.supply;
        }
        if let Some(note) = d.note {
            v.note = note;
        }
        self.sync_settings_view();
    }

    /// Push on a choice of the open page of settings: use that device, that block, that step of the light, or do what the button says.
    fn settings_pick(&mut self, n: usize, h: &mut EngineHandle) {
        use crate::settings::BUFFERS;
        let name = |list: &[String], row: usize| (row > 0).then(|| list.get(row - 1).cloned()).flatten();
        match (self.ui.settings.page, n) {
            (1, 0..=7) => {
                self.settings.audio_out = name(&self.ui.settings.outs, n);
                self.requests.push(Request::AudioOut(self.settings.audio_out.clone()));
                self.ui.settings.note = "opening it…".into();
            }
            (1, 8..=11) => {
                self.settings.audio_in = name(&self.ui.settings.ins, n - 8);
                self.requests.push(Request::AudioIn(self.settings.audio_in.clone()));
                self.ui.settings.note = "it opens when SAMPLING needs it".into();
            }
            (1, 16..=18) => {
                let b = BUFFERS[n - 16];
                self.settings.buffer = Some(b);
                self.requests.push(Request::Buffer(b));
                self.ui.settings.note = "reopening the audio…".into();
            }
            (2, 0..=7) => {
                self.settings.midi_out = name(&self.ui.settings.ports, n);
                self.requests.push(Request::MidiPort(self.settings.midi_out.clone()));
                self.ui.settings.note = "opening it…".into();
            }
            (2, 8..=11) => {
                self.settings.midi_in = name(&self.ui.settings.midi_ins, n - 8);
                self.requests.push(Request::MidiIn(self.settings.midi_in.clone()));
                self.ui.settings.note = "opening it…".into();
            }
            (2, 12) => self.requests.push(Request::MidiTest),
            (2, 13) => self.requests.push(Request::MidiPanic),
            (2, 16) => {
                self.settings.midi_clock_in = !self.settings.midi_clock_in;
                self.clock = Default::default();
                self.ui.settings.note = if self.settings.midi_clock_in { "the tempo, start and stop come from the input" } else { "the tempo is its own" }.into();
            }
            (2, 17) => {
                // learn: a knob of the right screen, then a control; again, to give up
                self.learn_param = None;
                self.ui.settings.learn = if self.ui.settings.learn == 0 { 1 } else { 0 };
                self.ui.settings.note = if self.ui.settings.learn == 1 { "turn a knob of the right screen of an instrument".into() } else { String::new() };
            }
            (2, 18) => {
                let n = self.settings.midi_map.len();
                self.settings.midi_map.clear();
                self.ui.settings.note = format!("forgot {n} control{}", if n == 1 { "" } else { "s" });
            }
            (2, 15) => {
                self.settings.midi_by_channel = !self.settings.midi_by_channel;
                self.ui.settings.note = if self.settings.midi_by_channel { "channels 1 to 8 play A to H" } else { "notes play the selected track" }.into();
            }
            (2, 14) => {
                self.settings.midi_clock = !self.settings.midi_clock;
                h.send(EngineCommand::MidiClock(self.settings.midi_clock));
                self.ui.settings.note = if self.settings.midi_clock { "the clock goes out on the port" } else { "no clock on the port" }.into();
            }
            (3, 0..=3) => self.settings.screen = n as u8 + 1,
            (3, 4..=6) => self.settings.pads = n as u8 - 3,
            _ => return,
        }
        if !matches!((self.ui.settings.page, n), (2, 12 | 13 | 17)) {
            self.settings_changed = true;
        }
        self.sync_settings_view();
    }

    fn step_pad(&mut self, pad: u8, ev: PadEvent, h: &mut EngineHandle, now: u64) {
        let p = pad as usize;
        let track = self.ui.track as u8;
        let step = Self::pad_to_step(pad);
        match ev {
            PadEvent::Press | PadEvent::Strike(_) => {
                if self.pad_down_at[p].is_none() {
                    self.pad_down_at[p] = Some(now);
                    if self.held.erase {
                        h.seq(Command::SetStep {
                            track,
                            step,
                            value: Step::default(),
                        });
                    }
                }
            }
            PadEvent::Release => {
                let was_down = self.pad_down_at[p].take().is_some();
                if self.inspector_pad == Some(pad) {
                    self.inspector_pad = None;
                    self.ui.inspect_step = None;
                } else if was_down && !self.held.erase && self.inspector_pad.is_none() {
                    match self.paint_note(h) {
                        // A kit (drums): the pads are the steps of the lane chosen on the right.
                        // A tap puts that sound on the step, or takes it off, leaving the other lanes.
                        Some(n) => {
                            let lane = (n - op8_seq::step::KIT_BASE_NOTE) % 8;
                            if !h.project.tracks[self.ui.track].kit {
                                h.seq(Command::SetKit { track, kit: true });
                            }
                            h.seq(Command::ToggleLane { track, step, lane });
                            let s = h.project.tracks[self.ui.track].steps[step as usize];
                            if s.lanes & (1 << lane) != 0 {
                                self.audition(h, n, s.velocity as f32 / 127.0);
                            }
                        }
                        None => {
                            h.seq(Command::ToggleStep { track, step });
                        }
                    }
                }
            }
            PadEvent::Pressure(_) => {}
        }
    }

    fn brain_pad(&mut self, pad: u8, ev: PadEvent, h: &mut EngineHandle) {
        let b = h.project.brain;
        let chord = match pad {
            0..=6 => Chord::diatonic(b.key, b.scale, pad, ChordKind::Triad),
            7 => Chord::diatonic(b.key, b.scale, 0, ChordKind::Sus4),
            8..=14 => Chord::diatonic(b.key, b.scale, pad - 8, ChordKind::Seventh),
            _ => Chord::diatonic(b.key, b.scale, 4, ChordKind::Sus2),
        };
        match ev {
            PadEvent::Press | PadEvent::Strike(_) => {
                if self.ui.live_chord_pad != Some(pad) {
                    self.release_chord(h);
                    self.ui.live_chord_pad = Some(pad);
                    h.seq(Command::SetLiveChord(Some(chord)));
                    // The chord itself sounds on the selected track's instrument, as long as the pad is
                    // held (a drum kit has no chords: its sounds are not notes).
                    if self.ui.lane.is_none() {
                        let velocity = self.pad_velocity(if let PadEvent::Strike(v) = ev { v } else { 0.8 });
                        let track = self.ui.track as u8;
                        self.chord_notes = chord_notes(chord);
                        for &note in &self.chord_notes {
                            h.send(EngineCommand::LiveNote { track, note, velocity, on: true });
                        }
                    }
                }
            }
            PadEvent::Release if self.ui.live_chord_pad == Some(pad) => {
                self.ui.live_chord_pad = None;
                h.seq(Command::SetLiveChord(None));
                self.release_chord(h);
            }
            _ => {}
        }
    }

    /// Let go of the notes of the chord that is sounding.
    fn release_chord(&mut self, h: &mut EngineHandle) {
        let track = self.ui.track as u8;
        for note in self.chord_notes.drain(..) {
            h.send(EngineCommand::LiveNote { track, note, velocity: 0.0, on: false });
        }
    }

    fn punch_pad(&mut self, pad: u8, ev: PadEvent, h: &mut EngineHandle) {
        let slot = pad as usize;
        let fx = PUNCH_SLOTS[slot].0;
        match ev {
            PadEvent::Press => {
                if self.ui.punch.map(|(s, _)| s) != Some(slot) {
                    self.punch_base = 0.5;
                    self.ui.punch = Some((slot, 0.5));
                    h.send(EngineCommand::Punch(Some(fx), 0.5));
                }
            }
            PadEvent::Strike(v) => {
                self.punch_base = v;
                self.ui.punch = Some((slot, v));
                h.send(EngineCommand::Punch(Some(fx), v));
            }
            PadEvent::Pressure(p) => {
                if self.ui.punch.map(|(s, _)| s) == Some(slot) {
                    let v = (self.punch_base * 0.5).max(p);
                    self.ui.punch = Some((slot, v));
                    h.send(EngineCommand::PunchIntensity(v));
                }
            }
            PadEvent::Release => {
                if self.ui.punch.map(|(s, _)| s) == Some(slot) {
                    self.ui.punch = None;
                    h.send(EngineCommand::Punch(None, 0.0));
                }
            }
        }
    }

    /// Turn a knob that moves a 0..=255 value: whole steps, with the remainder kept for the next turn.
    fn nudge_u8(&mut self, k: usize, cur: u8, delta: i32) -> u8 {
        self.knob_frac[k] += delta as f32 * 255.0 / SWEEP;
        let whole = self.knob_frac[k].trunc();
        self.knob_frac[k] -= whole;
        (cur as f32 + whole).clamp(0.0, 255.0) as u8
    }

    /// The knobs a new effect starts from (knob positions that sound right at once).
    fn fx_defaults(kind: FxKind) -> TrackFx {
        let (a, b, c) = match kind {
            FxKind::Off => (128, 110, 90),
            FxKind::Delay => (178, 110, 100),
            FxKind::Reverb => (150, 150, 90),
            FxKind::Chorus => (70, 140, 128),
            FxKind::Filter => (80, 60, 255),
            FxKind::Drive => (100, 170, 200),
        };
        TrackFx { kind, a, b, c }
    }

    /// Button 7, knobs 5–8: the effect of the track.
    fn fx_knob(&mut self, k: usize, delta: i32, h: &mut EngineHandle) {
        let track = self.ui.track as u8;
        let mut fx = h.project.tracks[self.ui.track].fx;
        if k == 4 {
            let n = self.detents(k, delta);
            if n == 0 {
                return;
            }
            let i = FxKind::ALL.iter().position(|x| *x == fx.kind).unwrap_or(0) as i32;
            let kind = FxKind::ALL[(i + n).clamp(0, FxKind::ALL.len() as i32 - 1) as usize];
            if kind != fx.kind {
                fx = Self::fx_defaults(kind);
            }
        } else if fx.kind == FxKind::Off {
            return;
        } else {
            match k {
                5 => fx.a = self.nudge_u8(k, fx.a, delta),
                6 => fx.b = self.nudge_u8(k, fx.b, delta),
                _ => fx.c = self.nudge_u8(k, fx.c, delta),
            }
        }
        h.seq(Command::SetFx { track, fx });
    }

    /// Button 8, knobs 5–8: the LFO of the track.
    fn lfo_knob(&mut self, k: usize, delta: i32, h: &mut EngineHandle) {
        let track = self.ui.track as u8;
        let mut lfo = h.project.tracks[self.ui.track].lfo;
        match k {
            4 => {
                let n = self.detents(k, delta);
                let i = LfoShape::ALL.iter().position(|x| *x == lfo.shape).unwrap_or(0) as i32;
                lfo.shape = LfoShape::ALL[(i + n).clamp(0, LfoShape::ALL.len() as i32 - 1) as usize];
            }
            5 => lfo.rate = self.nudge_u8(k, lfo.rate, delta),
            6 => lfo.depth = self.nudge_u8(k, lfo.depth, delta),
            _ => {
                let n = self.detents(k, delta);
                lfo.target = (lfo.target as i32 + n).clamp(0, LFO_TARGETS as i32 - 1) as u8;
            }
        }
        h.seq(Command::SetLfo { track, lfo });
    }

    /// Buttons 3 and 4, knobs 1–4: the players, or the bar.
    fn page_knob(&mut self, k: usize, delta: i32, h: &mut EngineHandle) {
        let track = self.ui.track as u8;
        let t = h.project.tracks[self.ui.track];
        let n = self.detents(k, delta);
        if n == 0 {
            return;
        }
        match self.ui.left_page {
            LeftPage::Players if !t.kit => {
                let mut pl = t.players;
                match k {
                    0 => {
                        let i = PlayMode::ALL.iter().position(|x| *x == pl.mode).unwrap_or(0) as i32;
                        let was = pl.mode;
                        pl.mode = PlayMode::ALL[(i + n).clamp(0, PlayMode::ALL.len() as i32 - 1) as usize];
                        // a chord starts all together; an arpeggio starts at four notes a step
                        if pl.mode == PlayMode::Chord && was != PlayMode::Chord {
                            pl.speed = 0;
                        } else if pl.mode.is_arp() && !was.is_arp() {
                            pl.speed = 2;
                        }
                    }
                    1 => {
                        let i = ChordType::ALL.iter().position(|x| *x == pl.chord).unwrap_or(0) as i32;
                        pl.chord = ChordType::ALL[(i + n).clamp(0, ChordType::ALL.len() as i32 - 1) as usize];
                    }
                    2 => pl.speed = (pl.speed as i32 + n).clamp(0, 4) as u8,
                    _ => pl.octaves = (pl.octaves as i32 + n).clamp(1, 3) as u8,
                }
                h.seq(Command::SetPlayers { track, players: pl });
            }
            LeftPage::Bar => match k {
                0 => {
                    h.seq(Command::SetLength { track, length: (t.length as i32 + n).clamp(1, 16) as u8 });
                }
                1 => {
                    let i = Division::ALL.iter().position(|d| *d == t.division).unwrap_or(2) as i32;
                    let division = Division::ALL[(i + n).clamp(0, Division::ALL.len() as i32 - 1) as usize];
                    h.seq(Command::SetDivision { track, division });
                }
                2 => {
                    h.seq(Command::ShiftTrack { track, by: n.clamp(-8, 8) as i8 });
                }
                _ => self.ui.quantize = (self.ui.quantize as i32 + n).clamp(0, 2) as usize,
            },
            _ => {}
        }
    }

    /// Turn raw knob delta into whole detents, keeping the remainder.
    /// The mix: while VOLUME is held, knob `k` is the level of track `k` (A to H), or with SHIFT its pan. A turn of one detent is
    /// four steps of 127 (a pan of one detent is the same, so the centre is reached in sixteen).
    fn mixer_knob(&mut self, k: usize, delta: i32, h: &mut EngineHandle) {
        let n = self.detents(k, delta);
        if n == 0 || k >= TRACKS {
            return;
        }
        let t = h.project.tracks[k];
        if self.held.shift {
            let pan = (t.pan as i32 + n * 4).clamp(0, 127) as u8;
            h.seq(Command::SetPan { track: k as u8, pan });
        } else {
            let level = (t.level as i32 + n * 4).clamp(0, 127) as u8;
            h.seq(Command::SetLevel { track: k as u8, level });
        }
    }

    /// The knobs in MIXER: the four under the left screen are the levels of the four channels that are shown (with SHIFT their pans),
    /// the four under the right screen are the values of the compressor's page.
    fn mixer_page_knob(&mut self, k: usize, delta: i32, h: &mut EngineHandle) {
        if k < 4 {
            let track = self.ui.mixer.group as usize * 4 + k;
            // SHIFT is always the pan; otherwise the level, or the send that is lit
            if self.ui.mixer.send > 0 && !self.held.shift {
                let n = self.detents(k, delta);
                if n != 0 {
                    let t = h.project.tracks[track];
                    let bus = self.ui.mixer.send - 1;
                    let now = if bus == 0 { t.send_a } else { t.send_b };
                    h.seq(Command::SetSend { track: track as u8, bus, value: (now as i32 + n * 4).clamp(0, 127) as u8 });
                }
            } else {
                self.mixer_knob(track, delta, h);
            }
        } else if let Some(i) = op8_ui::mixer_sends_index(self.ui.mixer.page, k) {
            self.sends_accum[i] += delta as f32 / SWEEP * 255.0;
            let whole = self.sends_accum[i].trunc();
            self.sends_accum[i] -= whole;
            if whole != 0.0 {
                let v = (h.project.sends.get(i) as f32 + whole).clamp(0.0, 255.0) as u8;
                h.seq(Command::SetMasterSends { index: i as u8, value: v });
            }
        } else if let Some(i) = op8_ui::mixer_eq_index(self.ui.mixer.page, k) {
            // the same for the EQ: a sweep of the knob is the whole range of 255
            self.eq_accum[i] += delta as f32 / SWEEP * 255.0;
            let whole = self.eq_accum[i].trunc();
            self.eq_accum[i] -= whole;
            if whole != 0.0 {
                let v = (h.project.eq.get(i) as f32 + whole).clamp(0.0, 255.0) as u8;
                h.seq(Command::SetMasterEq { index: i as u8, value: v });
            }
        } else if let Some(i) = op8_ui::mixer_comp_index(self.ui.mixer.page, k) {
            // a sweep of the knob (800) is the whole range of 255
            let d = delta as f32 / SWEEP * 255.0;
            self.comp_accum[i] += d;
            let whole = self.comp_accum[i].trunc();
            self.comp_accum[i] -= whole;
            if whole != 0.0 {
                let v = (h.project.comp.get(i) as f32 + whole).clamp(0.0, 255.0) as u8;
                h.seq(Command::SetMasterComp { index: i as u8, value: v });
            }
        }
    }

    /// SELECT held and a pad of FILE or BROWSER: name that place. Only the player's own places that have something in them can be named.
    fn begin_naming(&mut self, slot: usize, path: Option<std::path::PathBuf>, what: &str, from: Main, h: &mut EngineHandle) {
        let say = |s: &mut Self, text: String| {
            match from {
                Main::File => s.ui.file_note = text,
                Main::Sampling => s.ui.smp.note = text,
                _ => s.ui.browse_note = text,
            }
        };
        if slot < 8 {
            return say(self, format!("the {what}s that come with it keep their names"));
        }
        let Some(path) = path.filter(|p| p.is_file()) else {
            return say(self, format!("nothing there to name: save a {what} first"));
        };
        let current = crate::projectfile::read_name(&path).unwrap_or_default();
        self.ui.naming = op8_ui::NameView::new(format!("{what} user {}", slot - 7), &current);
        self.name_target = Some((path, from));
        self.set_main(Main::Name, h);
    }

    /// A message from the MIDI input. Notes play the selected track, or the track of their channel (1 to 8 are A to H) when the player chose
    /// that; they are written by REC like the pads' (on the selected track). Three controllers mix the track they are sent to: 7 is its
    /// level, 10 its pan and 91 its send to the reverb. 120 and 123 let go of every note that is held.
    pub fn midi_in(&mut self, msg: &[u8], h: &mut EngineHandle) {
        self.midi_in_at(msg, self.now * 1000, h);
    }

    /// A message that came in at `at_us` (microseconds from any fixed moment: what the clock measures is the time between them).
    pub fn midi_in_at(&mut self, msg: &[u8], at_us: u64, h: &mut EngineHandle) {
        let Some(&status) = msg.first() else { return };
        if matches!(status, 0xF2 | 0xF8 | 0xFA | 0xFB | 0xFC) {
            return self.clock_in(msg, at_us, h);
        }
        let (kind, channel) = (status & 0xF0, status & 0x0F);
        let target = if self.settings.midi_by_channel {
            if channel as usize >= TRACKS {
                return;
            }
            channel as usize
        } else {
            self.ui.track
        };
        let data = |i: usize| msg.get(i).copied().unwrap_or(0) & 0x7F;
        match kind {
            0x90 if data(2) > 0 => {
                let (note, velocity) = (data(1), self.pad_velocity(data(2) as f32 / 127.0));
                // a note that is already down is let go before it sounds again
                if let Some(old) = self.midi_held.insert((channel, note), target as u8) {
                    h.send(EngineCommand::LiveNote { track: old, note, velocity: 0.0, on: false });
                }
                h.send(EngineCommand::LiveNote { track: target as u8, note, velocity, on: true });
                if target == self.ui.track {
                    self.record(h, note, velocity);
                }
            }
            0x80 | 0x90 => {
                let note = data(1);
                if let Some(track) = self.midi_held.remove(&(channel, note)) {
                    h.send(EngineCommand::LiveNote { track, note, velocity: 0.0, on: false });
                }
            }
            // a program change picks the instrument of that place of PLUG-IN for the track (0..=7 are the eight that come with it); a place
            // with nothing in it, or the instrument that is already there, changes nothing
            0xC0 => {
                if self.put_instrument(target, data(1) as usize, h).is_ok() && target == self.ui.track {
                    self.refresh_plugs(h);
                }
            }
            // the pitch wheel: 14 bits, the middle (8192) is no bend, and the ends are two semitones
            0xE0 => {
                let raw = data(1) as i32 | (data(2) as i32) << 7;
                let semitones = (raw - 8192) as f32 / 8192.0 * 2.0;
                h.send(EngineCommand::PitchBend { track: target as u8, semitones });
            }
            0xB0 if self.learned_control(data(1), data(2), target, h) => {}
            0xB0 => match data(1) {
                7 => {
                    h.seq(Command::SetLevel { track: target as u8, level: data(2) });
                }
                10 => {
                    h.seq(Command::SetPan { track: target as u8, pan: data(2) });
                }
                91 => {
                    h.seq(Command::SetSend { track: target as u8, bus: 1, value: data(2) });
                }
                120 | 123 => {
                    for ((_, note), track) in self.midi_held.drain() {
                        h.send(EngineCommand::LiveNote { track, note, velocity: 0.0, on: false });
                    }
                }
                _ => {}
            },
            _ => {}
        }
    }

    /// `learn`, waiting for a knob: the right screen of the instrument has two pages of four parameters; the knob that turned is the one
    /// the next control will turn.
    fn learn_knob(&mut self, k: usize) {
        if k < 4 {
            self.ui.settings.note = "turn a knob under the right screen".into();
            return;
        }
        let page = op8_ui::right_page(&self.ui);
        if page > 1 {
            self.ui.settings.note = "that page of the instrument has no parameters: use the first or the second".into();
            return;
        }
        self.learn_param = Some((page * 4 + (k - 4)) as u8);
        self.ui.settings.learn = 2;
        self.ui.settings.note = format!("page {} knob {}: now move a control", page + 1, k - 3);
    }

    /// A controller that comes in: the one `learn` is waiting for, or one that was learned and turns its parameter.
    fn learned_control(&mut self, cc: u8, value: u8, target: usize, h: &mut EngineHandle) -> bool {
        if self.ui.settings.learn == 2 && !matches!(cc, 120..=127) {
            let Some(param) = self.learn_param.take() else { return true };
            self.settings.midi_map.retain(|&(c, _)| c != cc);
            self.settings.midi_map.push((cc, param));
            self.settings_changed = true;
            self.ui.settings.learn = 0;
            self.ui.settings.note = format!("control {cc} turns page {} knob {}", param / 4 + 1, param % 4 + 1);
            self.sync_settings_view();
            return true;
        }
        if let Some(&(_, param)) = self.settings.midi_map.iter().find(|&&(c, _)| c == cc) {
            h.send(EngineCommand::SetParam { track: target as u8, index: param, value: value as f32 / 127.0 });
            return true;
        }
        false
    }

    /// MIDI clock in: pulses, start, continue, stop and the song position pointer, when the clock in is on.
    fn clock_in(&mut self, msg: &[u8], at_us: u64, h: &mut EngineHandle) {
        if !self.settings.midi_clock_in {
            return;
        }
        match msg[0] {
            0xF8 => {
                let measured = self.clock.pulse(at_us);
                let running = self.clock.running && h.is_playing();
                let expected = running.then(|| self.clock.expected_tick());
                // the tempo is looked at every sixteenth (6 pulses), so the commands that go to the sequencer are few
                let Some(measured) = measured else { return };
                let due = expected.map_or(true, |e| (e / crate::clockin::TICKS_PER_PULSE) % 6 == 0);
                if !due {
                    return;
                }
                let want = match expected {
                    Some(e) => {
                        let at = h.status.tick.load(std::sync::atomic::Ordering::Relaxed) as u64;
                        // far off (a start that came while it was somewhere else, a tempo that jumped): go to where the clock says
                        if (e as i64 - at as i64).abs() > crate::clockin::JUMP_TICKS {
                            h.send(EngineCommand::PlayFrom(e));
                        }
                        crate::clockin::ClockFollower::tempo(measured, e, at)
                    }
                    None => measured,
                };
                let want = (want * 10.0).round() / 10.0;
                if (want - h.project.bpm).abs() >= 0.1 {
                    h.seq(Command::SetBpm(want));
                }
            }
            0xFA => {
                self.clock.start(0);
                h.send(EngineCommand::Play);
            }
            0xFB => {
                let from = self.clock.position;
                self.clock.start(from);
                h.send(if from == 0 { EngineCommand::Play } else { EngineCommand::PlayFrom(from) });
            }
            0xFC => {
                self.clock.stop();
                h.send(EngineCommand::Stop);
            }
            0xF2 if msg.len() == 3 => self.clock.set_position((msg[1] & 0x7F) as u16 | ((msg[2] & 0x7F) as u16) << 7),
            _ => {}
        }
    }

    /// The 4D encoder in NAME: the letter under the cursor (SHIFT: ten at a time).
    fn name_turn(&mut self, delta: i32) {
        self.ui.naming.turn(delta * if self.held.shift { 10 } else { 1 });
    }

    /// The pads in NAME: the twelve of the top three rows are the places of the name; the last four are `clear`, `delete`, `insert` and
    /// `save`.
    fn name_pad(&mut self, pad: u8, ev: PadEvent, h: &mut EngineHandle) {
        if !matches!(ev, PadEvent::Press | PadEvent::Strike(_)) || self.pad_down_at[pad as usize].is_some() {
            if ev == PadEvent::Release {
                self.pad_down_at[pad as usize] = None;
            }
            return;
        }
        self.pad_down_at[pad as usize] = Some(0);
        match Self::pad_to_step(pad) as usize {
            c @ 0..=11 => self.ui.naming.cursor = c,
            12 => self.ui.naming.clear(),
            13 => self.ui.naming.delete(),
            14 => self.ui.naming.insert(),
            _ => self.finish_naming(h),
        }
    }

    /// Save the name next to its file (a name with nothing in it takes the name away, and the place is `user N` again) and go back.
    fn finish_naming(&mut self, h: &mut EngineHandle) {
        let Some((path, from)) = self.name_target.take() else {
            return self.set_main(Main::Instrument, h);
        };
        let text = self.ui.naming.text();
        let what = if text.is_empty() {
            let _ = std::fs::remove_file(crate::projectfile::name_path(&path));
            "the name is taken away".to_string()
        } else {
            crate::projectfile::write_name(&path, &text);
            format!("named {text}")
        };
        self.set_main(from, h);
        match from {
            Main::File => {
                self.refresh_files();
                self.ui.file_note = what;
            }
            Main::Sampling => {
                self.refresh_samples();
                self.ui.smp.note = what;
            }
            _ => {
                self.refresh_browser(h);
                self.ui.browse_note = what;
            }
        }
    }

    fn detents(&mut self, k: usize, delta: i32) -> i32 {
        self.knob_accum[k] += delta;
        let n = self.knob_accum[k] / DETENT;
        self.knob_accum[k] -= n * DETENT;
        n
    }

    fn knob(&mut self, k: usize, delta: i32, h: &mut EngineHandle, now_ms: u64) {
        if k >= 4 {
            match op8_ui::right_page(&self.ui) {
                2 => self.fx_knob(k, delta, h),
                3 => self.lfo_knob(k, delta, h),
                page => {
                    h.nudge_param(self.ui.track, page * 4 + (k - 4), delta as f32 / SWEEP);
                }
            }
            // Hear what the knob did, once, when it has been still for a moment (see `tick`).
            self.last_knob_ms = Some(now_ms);
            self.audition_due = Some(now_ms + AUDITION_PAUSE_MS);
            return;
        }
        let track = self.ui.track as u8;
        let t = h.project.tracks[self.ui.track];
        if self.ui.mode == Mode::Step {
            if let Some(step) = self.ui.inspect_step {
                let s = t.steps[step as usize];
                let n = self.detents(k, delta);
                if n == 0 {
                    return;
                }
                match k {
                    // a drum kit has no note on its steps (the sound is a lane): this knob does nothing
                    0 if t.kit => false,
                    0 => {
                        let note = (s.note as i32 + n).clamp(0, 127) as u8;
                        h.seq(Command::SetStepNote { track, step, note });
                        // Hear the note (on a drum track, the sound) as it changes.
                        self.audition(h, note, s.velocity as f32 / 127.0);
                        true
                    }
                    1 => h.seq(Command::SetStepVelocity {
                        track,
                        step,
                        velocity: (s.velocity as i32 + n * 4).clamp(1, 127) as u8,
                    }),
                    2 => h.seq(Command::SetStepGate {
                        track,
                        step,
                        gate: (s.gate as i32 + n * 5).clamp(1, 100) as u8,
                    }),
                    _ => {
                        let cur = s
                            .components
                            .as_slice()
                            .get(self.ui.comp_cursor)
                            .copied()
                            .unwrap_or(StepComponent::None);
                        let i = COMPONENT_PRESETS
                            .iter()
                            .position(|c| *c == cur)
                            .unwrap_or(0) as i32;
                        let next = COMPONENT_PRESETS
                            [(i + n).rem_euclid(COMPONENT_PRESETS.len() as i32) as usize];
                        let index = self.ui.comp_cursor as u8;
                        if self.ui.comp_cursor < s.components.len() {
                            if next == StepComponent::None {
                                h.seq(Command::RemoveComponent { track, step, index })
                            } else {
                                h.seq(Command::SetComponent {
                                    track,
                                    step,
                                    index,
                                    component: next,
                                })
                            }
                        } else if next != StepComponent::None {
                            h.seq(Command::AddComponent {
                                track,
                                step,
                                component: next,
                            })
                        } else {
                            true
                        }
                    }
                };
                return;
            }
        }
        if self.ui.mode == Mode::Step && self.ui.left_page != LeftPage::Steps {
            return self.page_knob(k, delta, h);
        }
        match (self.ui.mode, k) {
            (Mode::Brain, 0) => {
                let n = self.detents(k, delta);
                if n != 0 {
                    h.seq(Command::SetKey(
                        (h.project.brain.key as i32 + n).rem_euclid(12) as u8,
                    ));
                }
            }
            (Mode::Brain, 1) => {
                let n = self.detents(k, delta);
                if n != 0 {
                    let i = Scale::ALL
                        .iter()
                        .position(|s| *s == h.project.brain.scale)
                        .unwrap_or(0) as i32;
                    h.seq(Command::SetScale(
                        Scale::ALL[(i + n).rem_euclid(Scale::ALL.len() as i32) as usize],
                    ));
                }
            }
            (Mode::Brain, 2) => {
                let n = self.detents(k, delta);
                if n != 0 {
                    let modes = [BrainMode::Transpose, BrainMode::Chord, BrainMode::Scale];
                    let i = modes
                        .iter()
                        .position(|m| *m == h.project.brain.mode)
                        .unwrap_or(1) as i32;
                    h.seq(Command::SetBrainMode(modes[(i + n).rem_euclid(3) as usize]));
                }
            }
            (Mode::Brain, 3) => {
                let n = self.detents(k, delta);
                if n != 0 {
                    h.seq(Command::SetProgressionLength(
                        (h.project.brain.len as i32 + n).clamp(1, 8) as u8,
                    ));
                }
            }
            (Mode::Keys, 0) => {
                let n = self.detents(k, delta);
                if n != 0 {
                    self.shift_octave(n.signum(), h);
                }
            }
            (Mode::Step, 0) => {
                let n = self.detents(k, delta);
                if n != 0 {
                    h.seq(Command::SetLength {
                        track,
                        length: (t.length as i32 + n).clamp(1, 16) as u8,
                    });
                }
            }
            (Mode::Step, 1) => {
                let n = self.detents(k, delta);
                if n != 0 {
                    let i = Division::ALL
                        .iter()
                        .position(|d| *d == t.division)
                        .unwrap_or(2) as i32;
                    let d =
                        Division::ALL[(i + n).clamp(0, Division::ALL.len() as i32 - 1) as usize];
                    h.seq(Command::SetDivision { track, division: d });
                }
            }
            (Mode::Step, 2) => {
                h.seq(Command::SetSwing(
                    h.project.swing + delta as f32 / (SWEEP * 4.0),
                ));
            }
            (_, 3) => {
                let n = self.detents(k, delta);
                if n != 0 {
                    let step = if self.held.shift { 0.1 } else { 1.0 };
                    h.seq(Command::SetBpm(h.project.bpm + n as f32 * step));
                }
            }
            _ => {}
        }
    }

    /// Once per UI frame: promote long holds to the inspector, age the
    /// activity LEDs.
    pub fn tick(&mut self, h: &mut EngineHandle, now_ms: u64) {
        self.ui.master = h.master;
        if self.ui.main == Main::Mixer {
            let m = &mut self.ui.mixer;
            m.comp = h.project.comp;
            m.eq = h.project.eq;
            m.sends = h.project.sends;
            m.reduction = h.status.comp_reduction();
            m.input = h.status.comp_input();
        }
        if self.ui.main != Main::Sampling && self.countin.take().is_some() {
            self.ui.smp.counting = 0; // leaving SAMPLING cancels a count-in
        }
        if self.ui.main == Main::Sampling {
            self.sampling_tick(h, now_ms);
        }
        self.ui.metronome = h.metronome;
        {
            let t = &h.project.tracks[self.ui.track];
            (self.ui.players, self.ui.track_fx, self.ui.lfo, self.ui.kit) = (t.players, t.fx, t.lfo, t.kit);
            if t.lfo.target != 1 {
                // the name of the knob the LFO moves, from the instrument (the last name is kept if the audio thread holds it)
                let i = t.lfo.param();
                if let Some(c) = h.slots[self.ui.track].as_ref().and_then(|c| c.try_lock().ok()) {
                    self.ui.lfo_target = match i {
                        None => "nothing".into(),
                        Some(i) if i < 4 => c.parameters()[i].name.to_lowercase(),
                        Some(i) => c.secondary_parameters().map_or(format!("knob {}", i + 1), |s| s[i - 4].name.to_lowercase()),
                    };
                } else if i.is_none() {
                    self.ui.lfo_target = "nothing".into();
                }
            } else {
                self.ui.lfo_target = "volume".into();
            }
        }
        // Does this track's instrument have a second page? (If the audio thread holds the lock this frame, keep the last answer.)
        if let Some(c) = h.slots[self.ui.track].as_ref() {
            if let Ok(c) = c.try_lock() {
                self.ui.has_second = c.secondary_parameters().is_some();
            }
        } else {
            self.ui.has_second = false;
        }
        self.ui.lane = self.paint_note(h).map(|n| (n - op8_seq::step::KIT_BASE_NOTE) as usize % 8);
        // The `sound` knob chose another sound: hear it once, at once. Anything else a knob
        // did is heard once, when it has been still for a moment.
        let paint = self.paint_note(h);
        let recent = self.last_knob_ms.is_some_and(|t| now_ms.saturating_sub(t) < 500);
        if paint != self.last_paint {
            if let (Some(n), true) = (paint, recent) {
                h.send(EngineCommand::Audition { track: self.ui.track as u8, note: n, velocity: 0.9 });
                self.audition_due = None;
            }
            self.last_paint = paint;
        }
        if self.audition_due.is_some_and(|d| now_ms >= d) {
            self.audition_due = None;
            let note = self.audition_note(h);
            h.send(EngineCommand::Audition { track: self.ui.track as u8, note, velocity: 0.9 });
        }
        if self.ui.mode == Mode::Step && self.inspector_pad.is_none() {
            if let Some((pad, _)) = self
                .pad_down_at
                .iter()
                .enumerate()
                .filter_map(|(p, t)| t.map(|t| (p as u8, t)))
                .find(|(_, t)| now_ms.saturating_sub(*t) >= HOLD_MS)
            {
                self.inspector_pad = Some(pad);
                self.ui.left_page = LeftPage::Steps;
                self.ui.inspect_step = Some(Self::pad_to_step(pad));
                self.ui.comp_cursor = 0;
                self.ui.focus = NavKind::Comp(0);
            }
        }
        // what the 4D's list depends on: how many components the inspected step has; and when the inspector is closed the focus
        // is not on one of them any more
        self.ui.comp_count = self.inspected(h).map_or(0, |s| s.components.len());
        self.ui.mix = std::array::from_fn(|i| (h.project.tracks[i].level, h.project.tracks[i].pan));
        if let (NavKind::Comp(i), Some(_)) = (self.ui.focus, self.ui.inspect_step) {
            // a component taken away leaves the focus on the `+` at the end of the stack
            let last = self.ui.comp_count.min(MAX_COMPONENTS - 1);
            if i as usize > last {
                self.ui.focus = NavKind::Comp(last as u8);
                self.ui.comp_cursor = last;
            }
        }
        if self.ui.inspect_step.is_none() && matches!(self.ui.focus, NavKind::Comp(_)) {
            self.ui.focus = NavKind::Cell(0);
            self.ui.grabbed = false;
        }
        for t in 0..TRACKS {
            let n = h.status.note_count[t].load(Relaxed);
            if n != self.last_note_count[t] {
                self.last_note_count[t] = n;
                self.ui.activity[t] = 6;
            } else {
                self.ui.activity[t] = self.ui.activity[t].saturating_sub(1);
            }
        }
    }

    pub fn leds(&self, h: &EngineHandle, out: &mut LedFrame) {
        out.clear();
        let tr = self.ui.track;
        let colors = track_led_colors(op8_gfx::theme_id());
        let color = colors[tr];
        let playing = h.status.playing.load(Relaxed);
        let track = &h.project.tracks[tr];
        let composing = self.ui.main == Main::Instrument;
        // In settings the pads do nothing, so they stay dark.
        if self.ui.main == Main::File {
            // the factory projects, the player's that exist, and the empty slots that can be saved into
            for pad in 0..16u8 {
                let slot = Self::pad_to_step(pad) as usize;
                let level = match self.ui.files.get(slot) {
                    Some((_, true)) if slot < 8 => 2,
                    Some((_, true)) => 3,
                    _ => 1,
                };
                out.set_pad(pad, NiColor::White, level);
            }
        }
        if self.ui.main == Main::Name {
            // the twelve places of the name (the one with the cursor brighter) and the four actions
            for pad in 0..16u8 {
                let slot = Self::pad_to_step(pad) as usize;
                let level = if slot < 12 { if slot == self.ui.naming.cursor { 3 } else { 1 } } else { 2 };
                out.set_pad(pad, NiColor::White, level);
            }
        }
        if self.ui.main == Main::Browser {
            for pad in 0..16u8 {
                let slot = Self::pad_to_step(pad) as usize;
                match self.ui.browse.get(slot) {
                    Some((_, true)) if self.ui.browse_last == Some(slot) => out.set_pad(pad, NiColor::White, 3),
                    Some((_, true)) => out.set_pad(pad, NiColor::White, 2),
                    _ if slot >= 8 => out.set_pad(pad, NiColor::White, 1),
                    _ => {}
                }
            }
        }
        if self.ui.main == Main::Sampling {
            for pad in 0..16u8 {
                let slot = Self::pad_to_step(pad) as usize;
                let level = if slot < 8 { if self.ui.smp.saved[slot] { 3 } else { 1 } } else if self.ui.smp.len > 0 && !self.ui.smp.recording { 2 } else { 0 };
                if level > 0 {
                    out.set_pad(pad, NiColor::White, level);
                }
            }
        }
        if self.ui.main == Main::Pattern {
            let now_pat = h.status.pattern[tr].load(Relaxed) as usize;
            for pad in 0..16u8 {
                let slot = Self::pad_to_step(pad) as usize;
                let has = h.project.pattern(tr, slot).is_some_and(|p| !p.is_empty());
                if slot == h.project.current[tr] as usize {
                    out.set_pad(pad, NiColor::White, 3);
                } else if playing && slot == now_pat {
                    out.set_pad(pad, color, 2);
                } else {
                    out.set_pad(pad, NiColor::White, if has { 2 } else { 1 });
                }
            }
        }
        if self.ui.main == Main::Plugin {
            for pad in 0..16u8 {
                let slot = Self::pad_to_step(pad) as usize;
                match self.ui.plugs.get(slot) {
                    Some((_, true)) if self.ui.plug_current == Some(slot) => out.set_pad(pad, NiColor::White, 3),
                    Some((_, true)) => out.set_pad(pad, NiColor::White, 2),
                    _ => {}
                }
            }
        }
        for pad in (0..16u8).filter(|_| composing) {
            match self.ui.mode {
                Mode::Step => {
                    let s = Self::pad_to_step(pad) as usize;
                    let ph = h.status.playhead[tr].load(Relaxed) as usize;
                    if self.ui.inspect_step == Some(s as u8) {
                        out.set_pad(pad, NiColor::White, 3);
                    } else if playing && s == ph {
                        out.set_pad(pad, NiColor::White, 2);
                    } else if s >= track.len() {
                        out.set_pad(pad, NiColor::Off, 0);
                    } else if self.lane_on(track.kit, &track.steps[s]) {
                        out.set_pad(pad, color, 3);
                    } else {
                        // Off is a dim white, not a dim colour: it stays apart from
                        // an active pad even when the theme or the supply caps the levels.
                        out.set_pad(pad, NiColor::White, 1);
                    }
                }
                Mode::Brain => {
                    let lit = self.ui.live_chord_pad == Some(pad);
                    let hue = colors[pad as usize % 8];
                    out.set_pad(pad, hue, if lit { 3 } else { 1 });
                }
                Mode::Pads | Mode::Keys => {
                    match op8_ui::pad_note(&self.ui, &h.project, pad) {
                        Some(_) if self.ui.pads_down[pad as usize] => out.set_pad(pad, NiColor::White, 3),
                        // C is marked on the keyboard so the player can find their way
                        Some(n) if self.ui.mode == Mode::Keys && n % 12 == 0 => out.set_pad(pad, color, 3),
                        Some(_) => out.set_pad(pad, color, 1),
                        None => out.set_pad(pad, NiColor::Off, 0),
                    }
                }
                Mode::Punch => {
                    let lit = self.ui.punch.map(|(s, _)| s) == Some(pad as usize);
                    out.set_pad(
                        pad,
                        if lit { NiColor::White } else { color },
                        if lit { 3 } else { 1 },
                    );
                }
            }
        }
        for t in 0..TRACKS {
            let level = if !composing {
                1
            } else if self.ui.activity[t] > 0 || t == tr {
                3
            } else if h.project.tracks[t].muted {
                0
            } else {
                1
            };
            out.set_button_color(Button::Group(t as u8), colors[t], level);
        }
        // Every other button, from one table (see `lights`): dark, faint white, or strong white.
        let ctx = crate::lights::Context { playing, shift: self.held.shift, mute: self.held.mute, erase: self.held.erase, down: &self.down };
        for b in crate::lights::all() {
            let l = crate::lights::light(&self.ui, &ctx, b);
            if op8_hw::leds::button_is_colour(b) {
                let colour = if b == Button::Play { NiColor::Green } else if b == Button::Rec { NiColor::Red } else { NiColor::White };
                out.set_button_color(b, colour, match l { crate::lights::Light::Off => 0, crate::lights::Light::Dim => 1, crate::lights::Light::Full => 3 });
            } else {
                out.set_button(b, match l { crate::lights::Light::Off => 0.0, crate::lights::Light::Dim => FAINT_WHITE, crate::lights::Light::Full => 1.0 });
            }
        }
        // Smart Strip shows the filter position.
        if let Some(x) = self.ui.strip {
            let i = (x * 24.0).round() as usize;
            out.set_strip(i, 0x7f);
        }
    }
}
