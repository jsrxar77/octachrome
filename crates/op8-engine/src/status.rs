use op8_seq::{Chord, TRACKS};
use std::sync::{
    atomic::{AtomicBool, AtomicU16, AtomicU32, AtomicU8, Ordering::Relaxed},
    Mutex,
};

/// What the audio thread publishes for the UI and LEDs. Relaxed atomics: a
/// frame being one block stale is fine.
#[derive(Default)]
pub struct EngineStatus {
    pub playing: AtomicBool,
    /// The tick the sequencer has reached (96 to the quarter note, counted from the start).
    pub tick: AtomicU32,
    pub playhead: [AtomicU8; TRACKS],
    /// Which pattern of each track is playing.
    pub pattern: [AtomicU8; TRACKS],
    /// The mutes the song's scene sets while it plays: bits 0 to 7 the tracks, bit 8 set when the song is the one that decides.
    pub song_mute: AtomicU16,
    /// The step of the song that plays, 255 when the song is not being followed.
    pub song_step: AtomicU8,
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
    /// The master compressor, for its picture: the deepest reduction (dB, 0 or less) and the loudest input (dB) of the last block, as f32 bits.
    pub comp_reduction: AtomicU32,
    pub comp_input: AtomicU32,
    /// SAMPLING: true while the engine is recording, how many samples are in `take`, and the
    /// loudest sample of the last block (f32 bits).
    pub recording: AtomicBool,
    /// SAMPLING, armed: waiting for a sound louder than the threshold; `rec_peak` then follows the level of the source.
    pub armed: AtomicBool,
    pub rec_len: AtomicU32,
    pub rec_peak: AtomicU32,
    /// What was recorded (mono, 48 kHz). Sized once, so the audio thread never allocates; only the
    /// audio thread writes it while `recording`, and only the control thread reads it when not.
    pub take: Mutex<Vec<f32>>,
}

impl EngineStatus {
    /// Whether track `i` is muted as it sounds: the scene of a song that is playing decides, else the track's own mute (`own`).
    pub fn muted(&self, i: usize, own: bool) -> bool {
        let m = self.song_mute.load(Relaxed);
        if m & 0x100 != 0 { m >> i & 1 == 1 } else { own }
    }

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
    /// How far the master compressor is bringing the sound down, in dB (0 or less).
    pub fn comp_reduction(&self) -> f32 {
        f32::from_bits(self.comp_reduction.load(Relaxed))
    }
    /// The loudest the sound going into the master compressor was in the last block, in dB (-90 for silence).
    pub fn comp_input(&self) -> f32 {
        f32::from_bits(self.comp_input.load(Relaxed))
    }
    pub fn load(&self) -> f32 {
        f32::from_bits(self.load.load(Relaxed))
    }
}
