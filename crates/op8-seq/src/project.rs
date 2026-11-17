//! The editable model. Everything here is plain `Copy` data with no heap, so
//! the control thread and the audio thread can each hold a copy and stay in
//! lockstep by applying the same [`Command`]s.

use crate::{
    brain::Brain, rack::{Lfo, Players, TrackFx}, step::Step, step::StepComponent, BrainMode, Chord,
    ChordSlot, Scale, MAX_STEPS, PPQN, TRACKS,
};

/// How long one step is.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Division {
    ThirtySecond,
    #[default]
    Sixteenth,
    SixteenthTriplet,
    Eighth,
    Quarter,
}

impl Division {
    pub const ALL: [Division; 5] = [
        Division::ThirtySecond,
        Division::SixteenthTriplet,
        Division::Sixteenth,
        Division::Eighth,
        Division::Quarter,
    ];

    pub const fn ticks(self) -> u32 {
        match self {
            Division::ThirtySecond => PPQN / 8,
            Division::Sixteenth => PPQN / 4,
            Division::SixteenthTriplet => PPQN / 6,
            Division::Eighth => PPQN / 2,
            Division::Quarter => PPQN,
        }
    }

    pub const fn label(self) -> &'static str {
        match self {
            Division::ThirtySecond => "1/32",
            Division::Sixteenth => "1/16",
            Division::SixteenthTriplet => "1/16T",
            Division::Eighth => "1/8",
            Division::Quarter => "1/4",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Track {
    pub steps: [Step; MAX_STEPS],
    /// Polymetric length, 1..=16.
    pub length: u8,
    pub division: Division,
    pub brain_link: bool,
    pub muted: bool,
    /// A drum track: eight lanes, one per sound, several of which can sound in one step.
    pub kit: bool,
    /// An arpeggio or a chord over the note of each step.
    pub players: Players,
    /// The track's own effect.
    pub fx: TrackFx,
    /// The track's LFO.
    pub lfo: Lfo,
    /// The track's level in the mix, 0..=127 (the engine plays it as level / 127, never above 1).
    pub level: u8,
    /// Where the track sits between the speakers, 0..=127: 0 hard left, 64 centre, 127 hard right.
    pub pan: u8,
    /// How much of the track goes to the two sends, 0..=127 each: the delay (`send_a`) and the reverb (`send_b`), taken after the level and
    /// the pan. 0 is none.
    pub send_a: u8,
    pub send_b: u8,
}

/// A track's level when it is new: 80 % of the way up, what the engine used for every track before the mixer.
pub const DEFAULT_LEVEL: u8 = 102;
/// The centre of the pan.
pub const PAN_CENTRE: u8 = 64;

impl Default for Track {
    fn default() -> Self {
        Self {
            steps: [Step::default(); MAX_STEPS],
            length: 16,
            division: Division::Sixteenth,
            brain_link: false,
            muted: false,
            kit: false,
            players: Players::default(),
            fx: TrackFx::default(),
            lfo: Lfo::default(),
            level: DEFAULT_LEVEL,
            pan: PAN_CENTRE,
            send_a: 0,
            send_b: 0,
        }
    }
}

/// How many patterns each track has.
pub const PATTERNS: usize = 16;

/// What a track plays: its steps, how many of them, how fast they go and whether they are the lanes of a drum kit. The sound (the
/// instrument, the effect, the LFO, the mix) is the track's and does not change with the pattern.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Pattern {
    pub steps: [Step; MAX_STEPS],
    pub length: u8,
    pub division: Division,
    pub kit: bool,
}

impl Default for Pattern {
    fn default() -> Self {
        Self { steps: [Step::default(); MAX_STEPS], length: 16, division: Division::Sixteenth, kit: false }
    }
}

impl Pattern {
    /// The pattern a track has in it now.
    pub fn of(t: &Track) -> Self {
        Self { steps: t.steps, length: t.length, division: t.division, kit: t.kit }
    }
    /// Nothing on its steps (what a pattern is when it is new).
    pub fn is_empty(&self) -> bool {
        !self.steps[..self.length.clamp(1, MAX_STEPS as u8) as usize].iter().any(|s| s.active)
    }
}

impl Track {
    /// Put a pattern in the track's place.
    pub fn load(&mut self, p: &Pattern) {
        (self.steps, self.length, self.division, self.kit) = (p.steps, p.length, p.division, p.kit);
    }
    pub fn len(&self) -> usize {
        self.length.clamp(1, MAX_STEPS as u8) as usize
    }
    pub fn is_empty(&self) -> bool {
        !self.steps[..self.len()].iter().any(|s| s.active)
    }
}

/// The compressor on the master: six settings, each 0..=255 of its own range (how they sound is `op8-engine`'s `comp.rs`). Saved with the
/// project. The defaults do nothing: the threshold is at 0 dB, where nothing reaches it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MasterComp {
    pub threshold: u8,
    pub ratio: u8,
    pub attack: u8,
    pub release: u8,
    pub makeup: u8,
    pub mix: u8,
    /// Which track the compressor listens to: 0 is the mix itself, 1..=8 a track (its sound brings the whole mix down). Of 0..=255.
    pub side: u8,
}

/// How many settings the master compressor has.
pub const COMP_PARAMS: usize = 7;

/// The track the compressor listens to for a side-chain setting (`None` is the mix itself): 0..=255 is cut in nine.
pub fn side_track(v: u8) -> Option<usize> {
    match v as usize * 9 / 256 {
        0 => None,
        n => Some(n - 1),
    }
}

impl Default for MasterComp {
    fn default() -> Self {
        Self { threshold: 255, ratio: 102, attack: 128, release: 128, makeup: 0, mix: 255, side: 0 }
    }
}

impl MasterComp {
    /// Setting `i` (threshold, ratio, attack, release, makeup, mix, side-chain), 0..=255.
    pub fn get(&self, i: usize) -> u8 {
        [self.threshold, self.ratio, self.attack, self.release, self.makeup, self.mix, self.side].get(i).copied().unwrap_or(0)
    }
    pub fn set(&mut self, i: usize, v: u8) {
        match i {
            0 => self.threshold = v,
            1 => self.ratio = v,
            2 => self.attack = v,
            3 => self.release = v,
            4 => self.makeup = v,
            5 => self.mix = v,
            6 => self.side = v,
            _ => {}
        }
    }
}

/// The two effects the tracks are sent to: a delay in step with the tempo and a reverb. Four settings, each 0..=255 of its own range (what
/// they mean is `op8-engine`'s `sends.rs`). Saved with the project.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MasterSends {
    /// The delay's time, as one of six note values.
    pub time: u8,
    /// How much of the delay comes round again.
    pub feedback: u8,
    /// The size of the reverb's room (how long it rings).
    pub size: u8,
    /// How bright the reverb is (high: bright; low: the highs die first).
    pub tone: u8,
    /// Where each send is taken from: from 128 up it is before the level and the pan of the track (pre-fader), under it after them.
    pub tap_a: u8,
    pub tap_b: u8,
}

/// How many settings the sends have.
pub const SENDS_PARAMS: usize = 6;

impl MasterSends {
    /// Whether the send to bus `bus` (0 delay, 1 reverb) is taken before the track's level and pan.
    pub fn is_pre(&self, bus: usize) -> bool {
        (if bus == 0 { self.tap_a } else { self.tap_b }) >= 128
    }
}

impl Default for MasterSends {
    fn default() -> Self {
        Self { time: 100, feedback: 100, size: 150, tone: 128, tap_a: 0, tap_b: 0 }
    }
}

impl MasterSends {
    /// Setting `i` (time, feedback, size, tone, delay tap, reverb tap), 0..=255.
    pub fn get(&self, i: usize) -> u8 {
        [self.time, self.feedback, self.size, self.tone, self.tap_a, self.tap_b].get(i).copied().unwrap_or(0)
    }
    pub fn set(&mut self, i: usize, v: u8) {
        match i {
            0 => self.time = v,
            1 => self.feedback = v,
            2 => self.size = v,
            3 => self.tone = v,
            4 => self.tap_a = v,
            5 => self.tap_b = v,
            _ => {}
        }
    }
}

/// The EQ on the master: the gain of the low and high shelves, the gain and the frequency of the middle band, each 0..=255 of its own
/// range (what they mean is `op8-engine`'s `eq.rs`). Saved with the project. 128 is no gain at all (and the middle of the frequency), so
/// the defaults do nothing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MasterEq {
    pub low: u8,
    pub mid: u8,
    pub freq: u8,
    pub high: u8,
}

/// How many settings the master EQ has.
pub const EQ_PARAMS: usize = 4;

impl Default for MasterEq {
    fn default() -> Self {
        Self { low: 128, mid: 128, freq: 128, high: 128 }
    }
}

impl MasterEq {
    /// Setting `i` (low, mid, freq, high), 0..=255.
    pub fn get(&self, i: usize) -> u8 {
        [self.low, self.mid, self.freq, self.high].get(i).copied().unwrap_or(0)
    }
    pub fn set(&mut self, i: usize, v: u8) {
        match i {
            0 => self.low = v,
            1 => self.mid = v,
            2 => self.freq = v,
            3 => self.high = v,
            _ => {}
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Project {
    pub tracks: [Track; TRACKS],
    pub brain: Brain,
    pub bpm: f32,
    /// 0.5 = straight, up to 0.75 = heavy shuffle (applies to odd steps).
    pub swing: f32,
    /// Seed for every random component.
    pub seed: u64,
    /// The compressor on the master.
    pub comp: MasterComp,
    /// The EQ on the master.
    pub eq: MasterEq,
    /// The delay and the reverb the tracks are sent to.
    pub sends: MasterSends,
    /// The patterns of each track. The one that is shown and edited, `current`, is in the track itself (its steps, length, division and
    /// kit); the others are here, and the place of `current` here is always an empty one.
    pub stored: Box<[[Pattern; PATTERNS]; TRACKS]>,
    /// Which pattern of each track is shown and edited.
    pub current: [u8; TRACKS],
}

impl Default for Project {
    fn default() -> Self {
        Self {
            tracks: [Track::default(); TRACKS],
            brain: Brain::default(),
            bpm: 120.0,
            swing: 0.5,
            seed: 0x0003_5EED,
            comp: MasterComp::default(),
            eq: MasterEq::default(),
            sends: MasterSends::default(),
            stored: Box::new([[Pattern::default(); PATTERNS]; TRACKS]),
            current: [0; TRACKS],
        }
    }
}

/// Every edit to a [`Project`]. Small and `Copy` so it can cross a lock-free
/// queue into the audio thread.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Command {
    /// Make a track a drum kit (eight lanes). Notes already on its steps become lanes by pitch class.
    SetKit {
        track: u8,
        kit: bool,
    },
    /// Switch one lane of a kit step on or off.
    ToggleLane {
        track: u8,
        step: u8,
        lane: u8,
    },
    ToggleStep {
        track: u8,
        step: u8,
    },
    SetStep {
        track: u8,
        step: u8,
        value: Step,
    },
    SetStepNote {
        track: u8,
        step: u8,
        note: u8,
    },
    SetStepVelocity {
        track: u8,
        step: u8,
        velocity: u8,
    },
    SetStepGate {
        track: u8,
        step: u8,
        gate: u8,
    },
    AddComponent {
        track: u8,
        step: u8,
        component: StepComponent,
    },
    /// Replace component `index` (no-op when out of range).
    SetComponent {
        track: u8,
        step: u8,
        index: u8,
        component: StepComponent,
    },
    RemoveComponent {
        track: u8,
        step: u8,
        index: u8,
    },
    ClearComponents {
        track: u8,
        step: u8,
    },
    ClearTrack {
        track: u8,
    },
    SetLength {
        track: u8,
        length: u8,
    },
    SetDivision {
        track: u8,
        division: Division,
    },
    /// Turn the steps of the track round inside its length: positive moves them later, negative earlier.
    ShiftTrack {
        track: u8,
        by: i8,
    },
    SetPlayers {
        track: u8,
        players: Players,
    },
    SetFx {
        track: u8,
        fx: TrackFx,
    },
    SetLfo {
        track: u8,
        lfo: Lfo,
    },
    SetLevel {
        track: u8,
        level: u8,
    },
    /// How much of a track goes to a send (`bus` 0 the delay, 1 the reverb), 0..=127.
    SetSend {
        track: u8,
        bus: u8,
        value: u8,
    },
    /// One setting of the sends' effects (`MasterSends::set`).
    SetMasterSends {
        index: u8,
        value: u8,
    },
    /// One setting of the master EQ (`MasterEq::set`).
    SetMasterEq {
        index: u8,
        value: u8,
    },
    /// One setting of the master compressor (`MasterComp::set`).
    SetMasterComp {
        index: u8,
        value: u8,
    },
    SetPan {
        track: u8,
        pan: u8,
    },
    SetBrainLink {
        track: u8,
        linked: bool,
    },
    SetMute {
        track: u8,
        muted: bool,
    },
    SetBpm(f32),
    SetSwing(f32),
    SetKey(u8),
    SetScale(Scale),
    SetBrainMode(BrainMode),
    SetProgressionSlot {
        index: u8,
        slot: ChordSlot,
    },
    SetProgressionLength(u8),
    SetLiveChord(Option<Chord>),
    /// Show and edit another pattern of a track. What plays changes when the sequencer says (at the start of the track's next loop, or
    /// at once with `now`; the project only changes what is shown).
    SetPattern { track: u8, index: u8, now: bool },
    /// Copy a pattern of a track over another one.
    CopyPattern { track: u8, from: u8, to: u8 },
    /// Empty a pattern of a track.
    ClearPattern { track: u8, index: u8 },
}

impl Project {
    /// What pattern `index` of a track holds (the track's own steps if it is the one shown).
    pub fn pattern(&self, track: usize, index: usize) -> Option<Pattern> {
        let t = self.tracks.get(track)?;
        if index >= PATTERNS {
            return None;
        }
        Some(if index == self.current[track] as usize { Pattern::of(t) } else { self.stored[track][index] })
    }

    fn set_pattern(&mut self, track: usize, index: usize, p: Pattern) {
        if index == self.current[track] as usize {
            self.tracks[track].load(&p);
        } else {
            self.stored[track][index] = p;
        }
    }

    fn step_mut(&mut self, track: u8, step: u8) -> Option<&mut Step> {
        self.tracks
            .get_mut(track as usize)?
            .steps
            .get_mut(step as usize)
    }

    /// Apply an edit. Out-of-range indices are ignored, never panic.
    pub fn apply(&mut self, cmd: &Command) {
        use Command::*;
        match *cmd {
            SetPattern { track, index, .. } => {
                let (t, i) = (track as usize, index as usize);
                if t < TRACKS && i < PATTERNS && i != self.current[t] as usize {
                    // what is shown goes to its place, and the one asked for comes out of its own
                    self.stored[t][self.current[t] as usize] = Pattern::of(&self.tracks[t]);
                    let p = std::mem::take(&mut self.stored[t][i]);
                    self.tracks[t].load(&p);
                    self.current[t] = index;
                }
            }
            CopyPattern { track, from, to } => {
                let t = track as usize;
                if let Some(p) = self.pattern(t, from as usize) {
                    if (to as usize) < PATTERNS {
                        self.set_pattern(t, to as usize, p);
                    }
                }
            }
            ClearPattern { track, index } => {
                let t = track as usize;
                if t < TRACKS && (index as usize) < PATTERNS {
                    self.set_pattern(t, index as usize, Pattern::default());
                }
            }
            SetKit { track, kit } => {
                if let Some(t) = self.tracks.get_mut(track as usize) {
                    if kit && !t.kit {
                        for s in t.steps.iter_mut().filter(|s| s.active && s.lanes == 0) {
                            s.lanes = 1 << ((s.note % 12) % 8);
                        }
                    }
                    if !kit && t.kit {
                        // leaving the kit: a step's first lane becomes its note
                        for s in t.steps.iter_mut().filter(|s| s.lanes != 0) {
                            s.note = crate::step::KIT_BASE_NOTE + s.lanes.trailing_zeros() as u8;
                            s.lanes = 0;
                        }
                    }
                    t.kit = kit;
                }
            }
            ToggleLane { track, step, lane } => {
                if let Some(s) = self.step_mut(track, step) {
                    s.lanes ^= 1 << (lane % 8);
                    s.active = s.lanes != 0;
                }
            }
            ToggleStep { track, step } => {
                if let Some(s) = self.step_mut(track, step) {
                    s.active = !s.active;
                }
            }
            SetStep { track, step, value } => {
                if let Some(s) = self.step_mut(track, step) {
                    *s = value;
                }
            }
            SetStepNote { track, step, note } => {
                if let Some(s) = self.step_mut(track, step) {
                    s.note = note.min(127);
                }
            }
            SetStepVelocity {
                track,
                step,
                velocity,
            } => {
                if let Some(s) = self.step_mut(track, step) {
                    s.velocity = velocity.clamp(1, 127);
                }
            }
            SetStepGate { track, step, gate } => {
                if let Some(s) = self.step_mut(track, step) {
                    s.gate = gate.clamp(1, 100);
                }
            }
            AddComponent {
                track,
                step,
                component,
            } => {
                if let Some(s) = self.step_mut(track, step) {
                    s.components.push(component);
                }
            }
            SetComponent {
                track,
                step,
                index,
                component,
            } => {
                if let Some(s) = self.step_mut(track, step) {
                    let i = index as usize;
                    if i < s.components.len() {
                        s.components.remove(i);
                        s.components.insert(i, component);
                    }
                }
            }
            RemoveComponent { track, step, index } => {
                if let Some(s) = self.step_mut(track, step) {
                    if (index as usize) < s.components.len() {
                        s.components.remove(index as usize);
                    }
                }
            }
            ClearComponents { track, step } => {
                if let Some(s) = self.step_mut(track, step) {
                    s.components.clear();
                }
            }
            ClearTrack { track } => {
                if let Some(t) = self.tracks.get_mut(track as usize) {
                    t.steps = [Step::default(); MAX_STEPS];
                }
            }
            SetLength { track, length } => {
                if let Some(t) = self.tracks.get_mut(track as usize) {
                    t.length = length.clamp(1, MAX_STEPS as u8);
                }
            }
            SetDivision { track, division } => {
                if let Some(t) = self.tracks.get_mut(track as usize) {
                    t.division = division;
                }
            }
            ShiftTrack { track, by } => {
                if let Some(t) = self.tracks.get_mut(track as usize) {
                    let len = t.len();
                    let by = (by as i32).rem_euclid(len as i32) as usize;
                    t.steps[..len].rotate_right(by);
                }
            }
            SetPlayers { track, players } => {
                if let Some(t) = self.tracks.get_mut(track as usize) {
                    t.players = players;
                }
            }
            SetFx { track, fx } => {
                if let Some(t) = self.tracks.get_mut(track as usize) {
                    t.fx = fx;
                }
            }
            SetLfo { track, lfo } => {
                if let Some(t) = self.tracks.get_mut(track as usize) {
                    t.lfo = Lfo { target: lfo.target.min(crate::rack::LFO_TARGETS - 1), ..lfo };
                }
            }
            SetMasterComp { index, value } => self.comp.set(index as usize, value),
            SetMasterEq { index, value } => self.eq.set(index as usize, value),
            SetMasterSends { index, value } => self.sends.set(index as usize, value),
            SetSend { track, bus, value } => {
                if let Some(t) = self.tracks.get_mut(track as usize) {
                    if bus == 0 {
                        t.send_a = value.min(127);
                    } else {
                        t.send_b = value.min(127);
                    }
                }
            }
            SetLevel { track, level } => {
                if let Some(t) = self.tracks.get_mut(track as usize) {
                    t.level = level.min(127);
                }
            }
            SetPan { track, pan } => {
                if let Some(t) = self.tracks.get_mut(track as usize) {
                    t.pan = pan.min(127);
                }
            }
            SetBrainLink { track, linked } => {
                if let Some(t) = self.tracks.get_mut(track as usize) {
                    t.brain_link = linked;
                }
            }
            SetMute { track, muted } => {
                if let Some(t) = self.tracks.get_mut(track as usize) {
                    t.muted = muted;
                }
            }
            SetBpm(b) => self.bpm = b.clamp(20.0, 300.0),
            SetSwing(s) => self.swing = s.clamp(0.5, 0.75),
            SetKey(k) => self.brain.key = k % 12,
            SetScale(s) => self.brain.scale = s,
            SetBrainMode(m) => self.brain.mode = m,
            SetProgressionSlot { index, slot } => {
                if let Some(s) = self.brain.progression.get_mut(index as usize) {
                    *s = slot;
                }
            }
            SetProgressionLength(n) => {
                self.brain.len = n.clamp(1, crate::brain::MAX_PROGRESSION as u8)
            }
            SetLiveChord(c) => self.brain.live = c,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn commands_are_bounds_checked() {
        let mut p = Project::default();
        p.apply(&Command::ToggleStep { track: 99, step: 0 });
        p.apply(&Command::ToggleStep { track: 0, step: 99 });
        p.apply(&Command::SetLength {
            track: 0,
            length: 0,
        });
        assert_eq!(p.tracks[0].length, 1);
        p.apply(&Command::SetLength {
            track: 0,
            length: 200,
        });
        assert_eq!(p.tracks[0].length, 16);
    }

    #[test]
    fn two_copies_stay_in_lockstep() {
        let cmds = [
            Command::ToggleStep { track: 1, step: 3 },
            Command::AddComponent {
                track: 1,
                step: 3,
                component: StepComponent::Ratchet(3),
            },
            Command::SetLength {
                track: 1,
                length: 5,
            },
            Command::SetBpm(97.0),
        ];
        let (mut a, mut b) = (Project::default(), Project::default());
        for c in &cmds {
            a.apply(c);
        }
        for c in &cmds {
            b.apply(c);
        }
        assert_eq!(a, b);
        assert!(a.tracks[1].steps[3].active);
    }
}
