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
    BrainMode, Chord, ChordKind, Command, Division, Scale, Step, StepComponent, MAX_COMPONENTS,
    TRACKS,
};
use op8_ui::{Main, Mode, UiState, COMPONENT_PRESETS};
use std::sync::atomic::Ordering::Relaxed;

pub const HOLD_MS: u64 = 300;
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

#[derive(Default)]
pub struct Controller {
    pub ui: UiState,
    pad_down_at: [Option<u64>; 16],
    /// The pad whose hold opened the inspector (its release must not toggle).
    inspector_pad: Option<u8>,
    knob_accum: [i32; 8],
    last_note_count: [u32; TRACKS],
    held: Held,
    punch_base: f32,
    /// Set when the encoder changes the theme; the app saves it and clears this.
    pub theme_changed: bool,
}

#[derive(Default, Clone, Copy)]
struct Held {
    shift: bool,
    mute: bool,
    erase: bool,
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
        match ev {
            ControlEvent::Button { button, pressed } => self.button(button, pressed, h),
            // In settings the pads and knobs do nothing; the 4D encoder picks the theme.
            ControlEvent::Pad { .. } | ControlEvent::Knob { .. } if self.ui.main == Main::Settings => {}
            ControlEvent::Encoder { delta } if self.ui.main == Main::Settings => {
                self.step_theme(delta)
            }
            ControlEvent::Pad { pad, event } => match self.ui.mode {
                Mode::Step => self.step_pad(pad, event, h, now_ms),
                Mode::Brain => self.brain_pad(pad, event, h),
                Mode::Punch => self.punch_pad(pad, event, h),
            },
            ControlEvent::Knob { index, delta } => self.knob(index as usize, delta, h),
            ControlEvent::KnobTouch { index, touched } => {
                if let Some(t) = self.ui.knob_touch.get_mut(index as usize) {
                    *t = touched;
                }
            }
            ControlEvent::Encoder { delta } => {
                if self.ui.inspect_step.is_some() {
                    let max = self
                        .inspected(h)
                        .map_or(0, |s| s.components.len().min(MAX_COMPONENTS - 1));
                    self.ui.comp_cursor =
                        (self.ui.comp_cursor as i32 + delta).clamp(0, max as i32) as usize;
                } else {
                    let step = if self.held.shift { 0.1 } else { 1.0 };
                    h.seq(Command::SetBpm(h.project.bpm + delta as f32 * step));
                }
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
        match b {
            Button::Shift => {
                self.held.shift = pressed;
                self.ui.shift = pressed;
            }
            Button::Mute => self.held.mute = pressed,
            Button::Erase => self.held.erase = pressed,
            _ if !pressed => {}
            Button::Group(_) if self.ui.main == Main::Settings => {}
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
                    self.ui.track = t;
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
            Button::Chords => self.set_mode(Mode::Brain, h),
            Button::Perform | Button::NoteRepeat => self.set_mode(Mode::Punch, h),
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
            Button::ChannelMidi => self.set_main(Main::Instrument, h),
            Button::JogLeft if self.ui.main == Main::Settings => self.step_theme(-1),
            Button::JogRight if self.ui.main == Main::Settings => self.step_theme(1),
            Button::Screen(5) => self.ui.right_page = 0,
            Button::Screen(6) => self.ui.right_page = 1,
            _ => {}
        }
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
        self.pad_down_at = [None; 16];
    }

    /// A pad mode button: choose the pad mode, and go back to composing.
    fn set_mode(&mut self, m: Mode, h: &mut EngineHandle) {
        self.release_pads(h);
        self.ui.main = Main::Instrument;
        self.ui.mode = m;
        self.ui.inspect_step = None;
        self.inspector_pad = None;
    }

    /// Change the main mode (docs/buttons.md §1). Entering settings lets go of
    /// the pads, which do nothing there.
    fn set_main(&mut self, m: Main, h: &mut EngineHandle) {
        if m == Main::Settings {
            self.release_pads(h);
        }
        self.ui.main = m;
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
        }
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
                    h.seq(Command::ToggleStep { track, step });
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
                    self.ui.live_chord_pad = Some(pad);
                    h.seq(Command::SetLiveChord(Some(chord)));
                }
            }
            PadEvent::Release if self.ui.live_chord_pad == Some(pad) => {
                self.ui.live_chord_pad = None;
                h.seq(Command::SetLiveChord(None));
            }
            _ => {}
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

    /// Turn raw knob delta into whole detents, keeping the remainder.
    fn detents(&mut self, k: usize, delta: i32) -> i32 {
        self.knob_accum[k] += delta;
        let n = self.knob_accum[k] / DETENT;
        self.knob_accum[k] -= n * DETENT;
        n
    }

    fn knob(&mut self, k: usize, delta: i32, h: &mut EngineHandle) {
        if k >= 4 {
            let index = self.ui.right_page * 4 + (k - 4);
            h.nudge_param(self.ui.track, index, delta as f32 / SWEEP);
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
                    0 => h.seq(Command::SetStepNote {
                        track,
                        step,
                        note: (s.note as i32 + n).clamp(0, 127) as u8,
                    }),
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
    pub fn tick(&mut self, h: &EngineHandle, now_ms: u64) {
        if self.ui.mode == Mode::Step && self.inspector_pad.is_none() {
            if let Some((pad, _)) = self
                .pad_down_at
                .iter()
                .enumerate()
                .filter_map(|(p, t)| t.map(|t| (p as u8, t)))
                .find(|(_, t)| now_ms.saturating_sub(*t) >= HOLD_MS)
            {
                self.inspector_pad = Some(pad);
                self.ui.inspect_step = Some(Self::pad_to_step(pad));
                self.ui.comp_cursor = 0;
            }
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
                    } else if track.steps[s].active {
                        out.set_pad(pad, color, 3);
                    } else {
                        out.set_pad(pad, color, 1);
                    }
                }
                Mode::Brain => {
                    let lit = self.ui.live_chord_pad == Some(pad);
                    let hue = colors[pad as usize % 8];
                    out.set_pad(pad, hue, if lit { 3 } else { 1 });
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
        out.set_button_color(Button::Play, NiColor::Green, if playing { 3 } else { 1 });
        out.set_button(Button::Stop, if playing { 0.2 } else { 1.0 });
        // The pad mode buttons only light while composing.
        let lit = |on: bool| if composing && on { 1.0 } else { 0.2 };
        out.set_button(Button::Step, lit(self.ui.mode == Mode::Step));
        out.set_button(Button::Chords, lit(self.ui.mode == Mode::Brain));
        out.set_button(Button::Perform, lit(self.ui.mode == Mode::Punch));
        out.set_button(Button::ChannelMidi, if composing { 1.0 } else { 0.2 });
        out.set_button(Button::Settings, if composing { 0.2 } else { 1.0 });
        out.set_button(Button::Shift, if self.held.shift { 1.0 } else { 0.2 });
        out.set_button(
            Button::Screen(5),
            if self.ui.right_page == 0 { 1.0 } else { 0.2 },
        );
        out.set_button(
            Button::Screen(6),
            if self.ui.right_page == 1 { 1.0 } else { 0.2 },
        );
        // Smart Strip shows the filter position.
        if let Some(x) = self.ui.strip {
            let i = (x * 24.0).round() as usize;
            out.set_strip(i, 0x7f);
        }
    }
}
