//! The Brain: a master harmony track.
//!
//! The Brain holds a key, a scale and a progression of up to 8 chords, each
//! lasting some number of 1/16 steps. Tracks that are *linked* to the Brain
//! have their notes reinterpreted against the chord playing right now:
//!
//! 1. Notes are written relative to C. The note is shifted by the chord root
//!    (taking the shortest way, −5..+6 semitones, so the register stays put).
//! 2. Depending on [`BrainMode`], the result is snapped to the nearest chord
//!    tone or scale tone.
//!
//! So a C–E–G arpeggio on a linked track plays A–C–E under an A minor chord.
//! Holding a pad in Brain mode sets a *live* chord that overrides the
//! progression until released.

pub const NOTE_NAMES: [&str; 12] = [
    "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B",
];

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Scale {
    #[default]
    Major,
    Minor,
    Dorian,
    Phrygian,
    Lydian,
    Mixolydian,
    HarmonicMinor,
}

impl Scale {
    pub const ALL: [Scale; 7] = [
        Scale::Major,
        Scale::Minor,
        Scale::Dorian,
        Scale::Phrygian,
        Scale::Lydian,
        Scale::Mixolydian,
        Scale::HarmonicMinor,
    ];

    pub const fn intervals(self) -> [u8; 7] {
        match self {
            Scale::Major => [0, 2, 4, 5, 7, 9, 11],
            Scale::Minor => [0, 2, 3, 5, 7, 8, 10],
            Scale::Dorian => [0, 2, 3, 5, 7, 9, 10],
            Scale::Phrygian => [0, 1, 3, 5, 7, 8, 10],
            Scale::Lydian => [0, 2, 4, 6, 7, 9, 11],
            Scale::Mixolydian => [0, 2, 4, 5, 7, 9, 10],
            Scale::HarmonicMinor => [0, 2, 3, 5, 7, 8, 11],
        }
    }

    pub const fn name(self) -> &'static str {
        match self {
            Scale::Major => "MAJOR",
            Scale::Minor => "MINOR",
            Scale::Dorian => "DORIAN",
            Scale::Phrygian => "PHRYG",
            Scale::Lydian => "LYDIAN",
            Scale::Mixolydian => "MIXO",
            Scale::HarmonicMinor => "HARM MIN",
        }
    }

    /// Pitch-class mask of the scale in `key`.
    pub fn mask(self, key: u8) -> u16 {
        self.intervals()
            .iter()
            .fold(0, |m, &i| m | 1 << ((key + i) % 12))
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ChordKind {
    #[default]
    Triad,
    Seventh,
    Sus2,
    Sus4,
}

/// A chord as a root pitch class and a 12-bit pitch-class mask.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Chord {
    pub root: u8,
    pub mask: u16,
}

impl Default for Chord {
    fn default() -> Self {
        Chord::from_intervals(0, &[0, 4, 7])
    }
}

impl Chord {
    pub fn from_intervals(root: u8, intervals: &[u8]) -> Self {
        let root = root % 12;
        Self {
            root,
            mask: intervals.iter().fold(0, |m, &i| m | 1 << ((root + i) % 12)),
        }
    }

    /// Diatonic chord on scale `degree` (0 = I) built by stacking thirds.
    pub fn diatonic(key: u8, scale: Scale, degree: u8, kind: ChordKind) -> Self {
        let iv = scale.intervals();
        let d = degree as usize % 7;
        let pc = |k: usize| (key + iv[(d + k) % 7]) % 12;
        let root = pc(0);
        let tones: &[usize] = match kind {
            ChordKind::Triad => &[0, 2, 4],
            ChordKind::Seventh => &[0, 2, 4, 6],
            ChordKind::Sus2 => &[0, 1, 4],
            ChordKind::Sus4 => &[0, 3, 4],
        };
        let mask = tones.iter().fold(0u16, |m, &k| m | 1 << pc(k));
        Self { root, mask }
    }

    pub fn contains(&self, pitch_class: u8) -> bool {
        self.mask & (1 << (pitch_class % 12)) != 0
    }

    /// Mask rotated so the root is bit 0.
    fn relative(&self) -> u16 {
        let m = self.mask as u32;
        let r = self.root as u32;
        (((m >> r) | (m << (12 - r))) & 0xfff) as u16
    }

    /// Display name, e.g. "Am", "G7", "Cmaj7", "Bdim", "Dsus4".
    pub fn name(&self) -> String {
        let root = NOTE_NAMES[self.root as usize];
        let rel = self.relative();
        let has = |i: u8| rel & (1 << i) != 0;
        let quality = match (has(3), has(4), has(6), has(7), has(2), has(5)) {
            (false, true, _, true, _, _) => "",
            (true, false, _, true, _, _) => "m",
            (true, false, true, false, _, _) => "dim",
            (false, true, false, false, _, _) if has(8) => "aug",
            (false, false, _, true, true, false) => "sus2",
            (false, false, _, true, false, true) => "sus4",
            _ => "?",
        };
        let ext = if has(11) {
            if quality.is_empty() {
                "maj7"
            } else {
                "M7"
            }
        } else if has(10) {
            if quality == "dim" {
                "7b5"
            } else {
                "7"
            }
        } else {
            ""
        };
        let quality = if quality == "dim" && has(10) {
            "m"
        } else {
            quality
        };
        format!("{root}{quality}{ext}")
    }
}

/// One entry of the progression.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ChordSlot {
    /// Scale degree, 0 = I.
    pub degree: u8,
    pub kind: ChordKind,
    /// Duration in 1/16 steps (16 = one bar).
    pub length: u8,
}

impl Default for ChordSlot {
    fn default() -> Self {
        Self {
            degree: 0,
            kind: ChordKind::Triad,
            length: 16,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum BrainMode {
    /// Shift by the chord root only.
    Transpose,
    /// Shift, then snap to the nearest chord tone (the default).
    #[default]
    Chord,
    /// Shift, then snap to the nearest tone of the key's scale.
    Scale,
}

pub const MAX_PROGRESSION: usize = 8;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Brain {
    pub key: u8,
    pub scale: Scale,
    pub mode: BrainMode,
    pub progression: [ChordSlot; MAX_PROGRESSION],
    /// Number of slots in use, 1..=8.
    pub len: u8,
    /// Chord held live from the pads; overrides the progression.
    pub live: Option<Chord>,
}

impl Default for Brain {
    fn default() -> Self {
        // I – vi – IV – V, one bar each.
        let mut progression = [ChordSlot::default(); MAX_PROGRESSION];
        for (slot, degree) in progression.iter_mut().zip([0u8, 5, 3, 4]) {
            slot.degree = degree;
        }
        Self {
            key: 0,
            scale: Scale::Major,
            mode: BrainMode::Chord,
            progression,
            len: 4,
            live: None,
        }
    }
}

impl Brain {
    fn slots(&self) -> &[ChordSlot] {
        &self.progression[..(self.len.clamp(1, MAX_PROGRESSION as u8)) as usize]
    }

    /// Total progression length in 1/16 steps.
    pub fn cycle_steps(&self) -> u64 {
        self.slots().iter().map(|s| s.length.max(1) as u64).sum()
    }

    /// Index of the progression slot playing at global 1/16 step `step16`.
    pub fn slot_at(&self, step16: u64) -> usize {
        let mut pos = step16 % self.cycle_steps();
        for (i, s) in self.slots().iter().enumerate() {
            let l = s.length.max(1) as u64;
            if pos < l {
                return i;
            }
            pos -= l;
        }
        0
    }

    /// The chord in effect at global 1/16 step `step16`.
    pub fn chord_at(&self, step16: u64) -> Chord {
        if let Some(c) = self.live {
            return c;
        }
        let s = self.progression[self.slot_at(step16)];
        Chord::diatonic(self.key, self.scale, s.degree, s.kind)
    }

    /// Reinterpret a C-relative note under `chord`.
    pub fn apply(&self, note: u8, chord: Chord) -> u8 {
        let shift = ((chord.root as i32 + 6) % 12) - 6; // -6..=5, shortest way
        let shift = if shift == -6 { 6 } else { shift };
        let shifted = (note as i32 + shift).clamp(0, 127);
        let mask = match self.mode {
            BrainMode::Transpose => return shifted as u8,
            BrainMode::Chord => chord.mask,
            BrainMode::Scale => self.scale.mask(self.key),
        };
        snap(shifted, mask)
    }
}

/// Nearest note whose pitch class is in `mask`; ties go down.
fn snap(note: i32, mask: u16) -> u8 {
    if mask == 0 {
        return note as u8;
    }
    for d in 0..12 {
        for cand in [note - d, note + d] {
            if (0..=127).contains(&cand) && mask & (1 << (cand % 12)) != 0 {
                return cand as u8;
            }
        }
    }
    note as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diatonic_chords_in_c_major() {
        let names: Vec<String> = (0..7)
            .map(|d| Chord::diatonic(0, Scale::Major, d, ChordKind::Triad).name())
            .collect();
        assert_eq!(names, ["C", "Dm", "Em", "F", "G", "Am", "Bdim"]);
        assert_eq!(
            Chord::diatonic(0, Scale::Major, 4, ChordKind::Seventh).name(),
            "G7"
        );
        assert_eq!(
            Chord::diatonic(0, Scale::Major, 0, ChordKind::Seventh).name(),
            "Cmaj7"
        );
        assert_eq!(
            Chord::diatonic(0, Scale::Major, 1, ChordKind::Seventh).name(),
            "Dm7"
        );
        assert_eq!(
            Chord::diatonic(0, Scale::Major, 0, ChordKind::Sus4).name(),
            "Csus4"
        );
        assert_eq!(
            Chord::diatonic(9, Scale::Minor, 0, ChordKind::Triad).name(),
            "Am"
        );
    }

    #[test]
    fn arpeggio_follows_chord() {
        let b = Brain::default();
        let am = Chord::diatonic(0, Scale::Major, 5, ChordKind::Triad);
        let out: Vec<u8> = [60u8, 64, 67].iter().map(|&n| b.apply(n, am)).collect();
        assert_eq!(out, [57, 60, 64]); // A3 C4 E4
        let g = Chord::diatonic(0, Scale::Major, 4, ChordKind::Triad);
        let out: Vec<u8> = [60u8, 64, 67].iter().map(|&n| b.apply(n, g)).collect();
        assert_eq!(out, [55, 59, 62]); // G3 B3 D4: the shortest shift is -5
    }

    #[test]
    fn progression_timing() {
        let b = Brain::default(); // I vi IV V, 16 steps each
        assert_eq!(b.chord_at(0).name(), "C");
        assert_eq!(b.chord_at(15).name(), "C");
        assert_eq!(b.chord_at(16).name(), "Am");
        assert_eq!(b.chord_at(48).name(), "G");
        assert_eq!(b.chord_at(64).name(), "C");
        let live = Brain {
            live: Some(Chord::from_intervals(2, &[0, 3, 7])),
            ..b
        };
        assert_eq!(live.chord_at(16).name(), "Dm");
    }

    #[test]
    fn scale_snap() {
        let b = Brain {
            mode: BrainMode::Scale,
            ..Brain::default()
        };
        // C# under C (no shift) snaps to C in C major.
        assert_eq!(b.apply(61, Chord::default()), 60);
    }
}
