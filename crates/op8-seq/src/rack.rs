//! What a track carries besides its steps: the players (an arpeggio or a chord over the note of a step),
//! the track's effect and its LFO. Plain `Copy` data like the rest of the project; the sequencer reads
//! the players, the engine reads the effect and the LFO.

use crate::Rng;

// ---- players ---------------------------------------------------------------------------------

/// What a step does with its note.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PlayMode {
    /// The note as it is.
    #[default]
    Off,
    /// A chord on the note, all at once (or strummed).
    Chord,
    /// An arpeggio on the chord, low to high.
    Up,
    /// An arpeggio, high to low.
    Down,
    /// An arpeggio, up and then down.
    UpDown,
    /// An arpeggio in random order.
    Random,
}

impl PlayMode {
    pub const ALL: [PlayMode; 6] = [PlayMode::Off, PlayMode::Chord, PlayMode::Up, PlayMode::Down, PlayMode::UpDown, PlayMode::Random];
    pub const fn label(self) -> &'static str {
        match self {
            PlayMode::Off => "off",
            PlayMode::Chord => "chord",
            PlayMode::Up => "up",
            PlayMode::Down => "down",
            PlayMode::UpDown => "up-down",
            PlayMode::Random => "random",
        }
    }
    pub fn is_arp(self) -> bool {
        matches!(self, PlayMode::Up | PlayMode::Down | PlayMode::UpDown | PlayMode::Random)
    }
}

/// The chord a player builds on the note of a step.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ChordType {
    #[default]
    Major,
    Minor,
    Seventh,
    MinorSeventh,
    Sus2,
    Sus4,
    Power,
}

impl ChordType {
    pub const ALL: [ChordType; 7] = [
        ChordType::Major,
        ChordType::Minor,
        ChordType::Seventh,
        ChordType::MinorSeventh,
        ChordType::Sus2,
        ChordType::Sus4,
        ChordType::Power,
    ];
    pub const fn label(self) -> &'static str {
        match self {
            ChordType::Major => "major",
            ChordType::Minor => "minor",
            ChordType::Seventh => "7th",
            ChordType::MinorSeventh => "m7",
            ChordType::Sus2 => "sus2",
            ChordType::Sus4 => "sus4",
            ChordType::Power => "power",
        }
    }
    /// Semitones above the root.
    pub const fn intervals(self) -> &'static [u8] {
        match self {
            ChordType::Major => &[0, 4, 7],
            ChordType::Minor => &[0, 3, 7],
            ChordType::Seventh => &[0, 4, 7, 10],
            ChordType::MinorSeventh => &[0, 3, 7, 10],
            ChordType::Sus2 => &[0, 2, 7],
            ChordType::Sus4 => &[0, 5, 7],
            ChordType::Power => &[0, 7, 12],
        }
    }
}

/// How many notes an arpeggio plays in one step, by `speed` 0..=4.
pub const ARP_NOTES: [u32; 5] = [2, 3, 4, 6, 8];

/// The most notes a player makes from one note (four chord tones over three octaves).
pub const MAX_PLAYED: usize = 12;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Players {
    pub mode: PlayMode,
    pub chord: ChordType,
    /// An arpeggio: how many notes per step (see [`ARP_NOTES`]). A chord: how far apart the notes are strummed, 0 = together.
    pub speed: u8,
    /// How many octaves the chord is stacked over, 1..=3.
    pub octaves: u8,
}

impl Default for Players {
    fn default() -> Self {
        Self { mode: PlayMode::Off, chord: ChordType::Major, speed: 2, octaves: 1 }
    }
}

impl Players {
    /// The notes of the chord on `root`, low to high, stacked over `octaves`. Returns how many.
    pub fn chord_notes(&self, root: u8, out: &mut [u8; MAX_PLAYED]) -> usize {
        let mut n = 0;
        for oct in 0..self.octaves.clamp(1, 3) as u32 {
            for iv in self.chord.intervals() {
                let note = root as u32 + *iv as u32 + 12 * oct;
                if note <= 127 && n < MAX_PLAYED {
                    out[n] = note as u8;
                    n += 1;
                }
            }
        }
        n
    }

    /// The index into the chord's notes of the `k`-th arpeggio note (k counts on across steps), for `n` notes.
    pub fn arp_index(&self, k: u64, n: usize, rng: &mut Rng) -> usize {
        let n = n.max(1);
        let m = n as u64;
        match self.mode {
            PlayMode::Down => (m - 1 - k % m) as usize,
            PlayMode::UpDown if n > 1 => {
                let period = 2 * m - 2;
                let i = k % period;
                (if i < m { i } else { period - i }) as usize
            }
            PlayMode::Random => rng.below(n as u32) as usize,
            _ => (k % m) as usize,
        }
    }

    /// The notes an arpeggio plays in one step.
    pub fn arp_count(&self) -> u32 {
        ARP_NOTES[(self.speed as usize).min(ARP_NOTES.len() - 1)]
    }
}

// ---- the track's effect -----------------------------------------------------------------------

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum FxKind {
    #[default]
    Off,
    Delay,
    Reverb,
    Chorus,
    Filter,
    Drive,
}

impl FxKind {
    pub const ALL: [FxKind; 6] = [FxKind::Off, FxKind::Delay, FxKind::Reverb, FxKind::Chorus, FxKind::Filter, FxKind::Drive];
    pub const fn label(self) -> &'static str {
        match self {
            FxKind::Off => "off",
            FxKind::Delay => "delay",
            FxKind::Reverb => "reverb",
            FxKind::Chorus => "chorus",
            FxKind::Filter => "filter",
            FxKind::Drive => "drive",
        }
    }
    /// The names of the three knobs of this effect (knobs 6–8).
    pub const fn knobs(self) -> [&'static str; 3] {
        match self {
            FxKind::Off => ["", "", ""],
            FxKind::Delay => ["time", "feedback", "mix"],
            FxKind::Reverb => ["size", "tone", "mix"],
            FxKind::Chorus => ["rate", "depth", "mix"],
            FxKind::Filter => ["sweep", "resonance", "mix"],
            FxKind::Drive => ["drive", "tone", "mix"],
        }
    }
}

/// The effect of one track: what it is and its three knobs, each 0..=255.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TrackFx {
    pub kind: FxKind,
    pub a: u8,
    pub b: u8,
    pub c: u8,
}

impl Default for TrackFx {
    fn default() -> Self {
        Self { kind: FxKind::Off, a: 128, b: 110, c: 90 }
    }
}

impl TrackFx {
    pub fn is_on(&self) -> bool {
        self.kind != FxKind::Off
    }
    /// The three knobs as 0..=1.
    pub fn knobs(&self) -> [f32; 3] {
        [self.a as f32 / 255.0, self.b as f32 / 255.0, self.c as f32 / 255.0]
    }
}

// ---- the track's LFO --------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum LfoShape {
    #[default]
    Sine,
    Triangle,
    Square,
    Saw,
    /// A new random value every cycle, held.
    Random,
}

impl LfoShape {
    pub const ALL: [LfoShape; 5] = [LfoShape::Sine, LfoShape::Triangle, LfoShape::Square, LfoShape::Saw, LfoShape::Random];
    pub const fn label(self) -> &'static str {
        match self {
            LfoShape::Sine => "sine",
            LfoShape::Triangle => "triangle",
            LfoShape::Square => "square",
            LfoShape::Saw => "saw",
            LfoShape::Random => "random",
        }
    }
    /// The wave at `phase` 0..1, in -1..=1 (Random is held by the caller).
    pub fn at(self, phase: f32) -> f32 {
        match self {
            LfoShape::Sine => (phase * std::f32::consts::TAU).sin(),
            LfoShape::Triangle => 1.0 - 4.0 * (phase - 0.5).abs(),
            LfoShape::Square => {
                if phase < 0.5 {
                    1.0
                } else {
                    -1.0
                }
            }
            LfoShape::Saw => 1.0 - 2.0 * phase,
            LfoShape::Random => 0.0,
        }
    }
}

/// What the LFO moves: 0 nothing, 1 the track's volume, 2..=9 the instrument's knobs 1–8.
pub const LFO_TARGETS: u8 = 10;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Lfo {
    pub shape: LfoShape,
    /// 0..=255, from one cycle in 20 seconds to 20 per second.
    pub rate: u8,
    /// 0..=255: how far it swings around the knob's own value.
    pub depth: u8,
    /// See [`LFO_TARGETS`].
    pub target: u8,
}

impl Default for Lfo {
    fn default() -> Self {
        Self { shape: LfoShape::Sine, rate: 110, depth: 128, target: 0 }
    }
}

impl Lfo {
    pub fn is_on(&self) -> bool {
        self.target != 0 && self.depth > 0
    }
    pub fn hz(&self) -> f32 {
        0.05 * 400f32.powf(self.rate as f32 / 255.0)
    }
    pub fn depth_f(&self) -> f32 {
        self.depth as f32 / 255.0
    }
    /// Which knob of the instrument it moves, if it moves one.
    pub fn param(&self) -> Option<usize> {
        (self.target >= 2).then(|| (self.target - 2).min(7) as usize)
    }
    pub fn moves_volume(&self) -> bool {
        self.target == 1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // F-S13
    #[test]
    fn a_chord_stacks_over_the_octaves_and_stays_in_range() {
        let p = Players { mode: PlayMode::Chord, chord: ChordType::Seventh, speed: 0, octaves: 2 };
        let mut out = [0u8; MAX_PLAYED];
        let n = p.chord_notes(60, &mut out);
        assert_eq!(&out[..n], &[60, 64, 67, 70, 72, 76, 79, 82]);
        let n = p.chord_notes(120, &mut out);
        assert!(out[..n].iter().all(|&x| x <= 127));
    }

    // F-S13
    #[test]
    fn arpeggio_orders() {
        let mut rng = Rng::new(1);
        let mut idx = |mode, n: usize| -> Vec<usize> {
            let p = Players { mode, ..Default::default() };
            (0..8).map(|k| p.arp_index(k, n, &mut rng)).collect()
        };
        assert_eq!(idx(PlayMode::Up, 3), [0, 1, 2, 0, 1, 2, 0, 1]);
        assert_eq!(idx(PlayMode::Down, 3), [2, 1, 0, 2, 1, 0, 2, 1]);
        assert_eq!(idx(PlayMode::UpDown, 3), [0, 1, 2, 1, 0, 1, 2, 1]);
        assert!(idx(PlayMode::Random, 3).iter().all(|&i| i < 3));
    }
}
