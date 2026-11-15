use op8_seq::{Chord, TRACKS};
use std::sync::{
    atomic::{AtomicBool, AtomicU32, AtomicU8, Ordering::Relaxed},
    Mutex,
};

/// What the audio thread publishes for the UI and LEDs. Relaxed atomics: a
/// frame being one block stale is fine.
#[derive(Default)]
pub struct EngineStatus {
    pub playing: AtomicBool,
    pub playhead: [AtomicU8; TRACKS],
    /// How far into its step each track is, 0..=255 (the fraction of the step that has passed).
    pub phase: [AtomicU8; TRACKS],
    /// Incremented on every note-on, per track (drives activity LEDs).
    pub note_count: [AtomicU32; TRACKS],
    /// Peak level per track as f32 bits, decaying.
    pub peak: [AtomicU32; TRACKS],
    pub chord_root: AtomicU8,
    pub chord_mask: AtomicU32,
    pub chord_slot: AtomicU8,
    /// Fraction of the block period spent rendering, as f32 bits.
    pub load: AtomicU32,
    pub dropped_events: AtomicU32,
    /// SAMPLING: true while the engine is recording, how many samples are in `take`, and the
    /// loudest sample of the last block (f32 bits).
    pub recording: AtomicBool,
    pub rec_len: AtomicU32,
    pub rec_peak: AtomicU32,
    /// What was recorded (mono, 48 kHz). Sized once, so the audio thread never allocates; only the
    /// audio thread writes it while `recording`, and only the control thread reads it when not.
    pub take: Mutex<Vec<f32>>,
}

impl EngineStatus {
    pub fn chord(&self) -> Chord {
        Chord {
            root: self.chord_root.load(Relaxed),
            mask: self.chord_mask.load(Relaxed) as u16,
        }
    }
    pub fn peak(&self, t: usize) -> f32 {
        f32::from_bits(self.peak[t].load(Relaxed))
    }
    pub fn rec_peak(&self) -> f32 {
        f32::from_bits(self.rec_peak.load(Relaxed))
    }
    pub fn load(&self) -> f32 {
        f32::from_bits(self.load.load(Relaxed))
    }
}
