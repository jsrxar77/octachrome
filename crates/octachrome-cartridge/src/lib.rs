//! The instrument "cartridge" contract.
//!
//! A cartridge is a synth, a sampler or a bridge to external gear. It sees
//! note events, a stereo buffer, four knobs and a framebuffer — and nothing
//! else. It never touches the sequencer or USB, so new instruments can be
//! written, compiled and added without changing either.
//!
//! Rules for implementors:
//!
//! * `process` runs on the real-time audio thread: **no allocation, no locks,
//!   no I/O, no syscalls**. Preallocate in the constructor.
//! * `process` **overwrites** both output slices (the host mixes tracks).
//! * Event `sample_offset`s are relative to the start of the block and sorted.
//! * Audio runs at [`SAMPLE_RATE`]; block sizes vary (64–128 on the Pi) up to
//!   [`MAX_BLOCK`].
//! * `draw_ui` runs on the UI thread at up to 60 FPS. Redraw the whole right
//!   screen every call; the host diffs frames and only sends what changed.

pub use octachrome_gfx::{FrameBuffer480x272, Rgb565};

/// Fixed engine sample rate.
pub const SAMPLE_RATE: f32 = 48_000.0;
/// Largest block the host will ever pass to `process`.
pub const MAX_BLOCK: usize = 1024;

/// Shown in the right screen's header.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InstrumentInfo {
    /// e.g. "KickSyn"
    pub name: String,
    /// e.g. "Drum", "Synth", "MIDI"
    pub category: String,
}

impl InstrumentInfo {
    pub fn new(name: impl Into<String>, category: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            category: category.into(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NoteKind {
    On,
    Off,
}

/// A note at a sample position inside the current block.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NoteEvent {
    /// Frames from the start of the block.
    pub sample_offset: u32,
    pub kind: NoteKind,
    /// MIDI note number, 60 = C4.
    pub note: u8,
    /// 0..=1. For `Off` this is release velocity (usually 0).
    pub velocity: f32,
}

impl Default for NoteEvent {
    fn default() -> Self {
        Self::off(0, 0)
    }
}

impl NoteEvent {
    pub const fn on(sample_offset: u32, note: u8, velocity: f32) -> Self {
        Self {
            sample_offset,
            kind: NoteKind::On,
            note,
            velocity,
        }
    }
    pub const fn off(sample_offset: u32, note: u8) -> Self {
        Self {
            sample_offset,
            kind: NoteKind::Off,
            note,
            velocity: 0.0,
        }
    }
}

/// How a normalised 0..=1 value is shown to the player.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ParamUnit {
    /// Plain percentage.
    Percent,
    /// Maps 0..=1 exponentially onto `min..=max` Hz.
    Hertz { min: f32, max: f32 },
    /// Maps 0..=1 exponentially onto `min..=max` milliseconds.
    Millis { min: f32, max: f32 },
    /// Maps linearly onto `min..=max` semitones.
    Semitones { min: f32, max: f32 },
    /// One of a fixed set of labelled choices.
    Choice(&'static [&'static str]),
}

/// One knob.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ParamDescriptor {
    /// Short label for a 120 px footer column (≤ 10 chars reads well).
    pub name: &'static str,
    pub default: f32,
    pub unit: ParamUnit,
}

impl ParamDescriptor {
    pub const fn new(name: &'static str, default: f32, unit: ParamUnit) -> Self {
        Self {
            name,
            default,
            unit,
        }
    }

    /// Value in the parameter's own unit.
    pub fn denormalize(&self, v: f32) -> f32 {
        let v = v.clamp(0.0, 1.0);
        match self.unit {
            ParamUnit::Percent => v * 100.0,
            ParamUnit::Hertz { min, max } | ParamUnit::Millis { min, max } => {
                min * (max / min).powf(v)
            }
            ParamUnit::Semitones { min, max } => min + (max - min) * v,
            ParamUnit::Choice(c) => choice_index(v, c.len()) as f32,
        }
    }

    /// Human-readable value for the footer.
    pub fn format(&self, v: f32) -> String {
        let d = self.denormalize(v);
        match self.unit {
            ParamUnit::Percent => format!("{d:.0}%"),
            ParamUnit::Hertz { .. } if d >= 1000.0 => format!("{:.1}k", d / 1000.0),
            ParamUnit::Hertz { .. } => format!("{d:.0}Hz"),
            ParamUnit::Millis { .. } if d >= 1000.0 => format!("{:.2}s", d / 1000.0),
            ParamUnit::Millis { .. } => format!("{d:.0}ms"),
            ParamUnit::Semitones { .. } => format!("{d:+.0}st"),
            ParamUnit::Choice(c) => c[d as usize].to_string(),
        }
    }
}

/// Index of a choice for a normalised value.
pub fn choice_index(v: f32, n: usize) -> usize {
    ((v.clamp(0.0, 1.0) * n as f32) as usize).min(n.saturating_sub(1))
}

/// The contract every instrument implements.
///
/// The six required methods are exactly the handover spec. The provided
/// methods are optional extras with safe defaults.
pub trait InstrumentCartridge: Send + Sync {
    /// Name and category shown in the right screen's top bar.
    fn info(&self) -> InstrumentInfo;

    /// Render one stereo block at 48 kHz. Overwrites both outputs.
    fn process(&mut self, output_l: &mut [f32], output_r: &mut [f32], events: &[NoteEvent]);

    /// The 4 main parameters (encoders 5–8).
    fn parameters(&self) -> &[ParamDescriptor; 4];
    /// Indices 0–3 are the main page, 4–7 the optional secondary page.
    fn set_parameter(&mut self, index: usize, normalized_value: f32);

    /// Draw the instrument's own UI into the right screen.
    fn draw_ui(&self, frame: &mut FrameBuffer480x272, accent_color: Rgb565);

    /// Presets and per-step parameter locks.
    fn save_state(&self) -> Vec<u8>;
    fn load_state(&mut self, data: &[u8]);

    // ---- optional extras --------------------------------------------------

    /// Optional second page of 4 parameters (indices 4–7 in `set_parameter`).
    fn secondary_parameters(&self) -> Option<&[ParamDescriptor; 4]> {
        None
    }

    /// Current normalised value of a parameter, if the cartridge tracks it.
    /// When `None` the host shows the last value it set.
    fn parameter_value(&self, _index: usize) -> Option<f32> {
        None
    }

    /// The note of the sound chosen with a knob, for an instrument that is a kit of
    /// sounds (drums): `KIT_BASE_NOTE` plus the lane. A track on such an instrument is
    /// programmed in lanes, and this is what the knobs audition.
    fn paint_note(&self) -> Option<u8> {
        None
    }

    /// Put a recorded sound (mono, 48 kHz) in one of the instrument's eight places for the player's own
    /// sounds, while it plays. Returns whether the instrument took it (only a sampler does).
    fn load_sample(&mut self, _place: usize, _data: Vec<f32>) -> bool {
        false
    }

    /// The pitch wheel of a keyboard: bend every note by `semitones` (positive up, 0 is centre) until it comes back to 0. An instrument
    /// with no pitch to bend (drums) ignores it.
    fn set_pitch_bend(&mut self, _semitones: f32) {}

    /// Silence every voice immediately (transport stop, panic).
    fn all_notes_off(&mut self) {}
}

/// Helpers for the small, versioned binary state blobs most cartridges need.
pub mod state {
    /// `[version, f32 LE × n]`
    pub fn encode_f32s(version: u8, values: &[f32]) -> Vec<u8> {
        let mut v = Vec::with_capacity(1 + values.len() * 4);
        v.push(version);
        for x in values {
            v.extend_from_slice(&x.to_le_bytes());
        }
        v
    }

    /// Decode into `out`; returns false (leaving `out` untouched) on a
    /// version mismatch or short blob.
    pub fn decode_f32s(version: u8, data: &[u8], out: &mut [f32]) -> bool {
        if data.first() != Some(&version) || data.len() < 1 + out.len() * 4 {
            return false;
        }
        for (i, o) in out.iter_mut().enumerate() {
            let b = &data[1 + i * 4..5 + i * 4];
            *o = f32::from_le_bytes([b[0], b[1], b[2], b[3]]);
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn param_formatting() {
        let cut = ParamDescriptor::new(
            "Cutoff",
            0.5,
            ParamUnit::Hertz {
                min: 20.0,
                max: 20000.0,
            },
        );
        assert_eq!(cut.format(0.0), "20Hz");
        assert_eq!(cut.format(1.0), "20.0k");
        let w = ParamDescriptor::new("Wave", 0.0, ParamUnit::Choice(&["SIN", "TRI", "SAW"]));
        assert_eq!(w.format(0.0), "SIN");
        assert_eq!(w.format(0.5), "TRI");
        assert_eq!(w.format(1.0), "SAW");
    }

    #[test]
    fn state_round_trip_and_version_check() {
        let blob = state::encode_f32s(3, &[0.25, 1.0]);
        let mut out = [0.0; 2];
        assert!(state::decode_f32s(3, &blob, &mut out));
        assert_eq!(out, [0.25, 1.0]);
        assert!(!state::decode_f32s(4, &blob, &mut out));
        assert!(!state::decode_f32s(3, &blob[..5], &mut out));
    }
}
