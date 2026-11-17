//! The sequencer core. Pure, allocation-free on the hot path, and fully
//! deterministic: the same project, tempo and seed always produce the same
//! events at the same sample positions.
//!
//! Timing is tick based (96 PPQN). A track's playhead is a pure function of
//! the global tick — `(tick / ticks_per_step) % length` — so polymeter falls
//! out for free, and nothing can drift: tracks of length 3 and 4 line up again
//! after exactly 12 steps, forever.
//!
//! * [`step`] — steps and accumulable [`StepComponent`]s.
//! * [`brain`] — the harmony track: key, scale, chord progression, live chord.
//! * [`project`] — the editable model and the [`Command`]s that change it.
//! * [`sequencer`] — the clock that turns a project into sample-accurate
//!   [`NoteEvent`](op8_cartridge::NoteEvent)s.

pub mod brain;
pub mod fixed;
pub mod io;
pub mod project;
pub mod rack;
pub mod sequencer;
pub mod step;

pub use brain::{Brain, BrainMode, Chord, ChordKind, ChordSlot, Scale};
pub use project::{side_track, Command, Division, MasterComp, MasterEq, MasterSends, Project, Track, COMP_PARAMS, EQ_PARAMS, SENDS_PARAMS};
pub use rack::{ChordType, FxKind, Lfo, LfoShape, PlayMode, Players, TrackFx, LFO_TARGETS};
pub use sequencer::{BlockEvents, ParamEvent, Playhead, Sequencer, SyncEvent, SyncKind};
pub use step::{Components, ResolvedStep, Step, StepComponent};

pub const TRACKS: usize = 8;
/// Steps per track (polymeter: each track's length is 1..=16).
pub const MAX_STEPS: usize = 16;
/// Components stacked on one step.
pub const MAX_COMPONENTS: usize = 6;
/// Ticks per quarter note.
pub const PPQN: u32 = 96;

/// Small deterministic PRNG (xorshift64*). Seeded per track at play start.
#[derive(Clone, Copy, Debug)]
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Self(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1)
    }
    pub fn next_u32(&mut self) -> u32 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        (x.wrapping_mul(0x2545_F491_4F6C_DD1D) >> 32) as u32
    }
    /// 0..n
    pub fn below(&mut self, n: u32) -> u32 {
        if n == 0 {
            0
        } else {
            self.next_u32() % n
        }
    }
}
