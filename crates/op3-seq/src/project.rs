//! The editable model. Everything here is plain `Copy` data with no heap, so
//! the control thread and the audio thread can each hold a copy and stay in
//! lockstep by applying the same [`Command`]s.

use crate::{
    brain::Brain, step::Step, step::StepComponent, BrainMode, Chord, ChordSlot, Scale, MAX_STEPS,
    PPQN, TRACKS,
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
}

impl Default for Track {
    fn default() -> Self {
        Self {
            steps: [Step::default(); MAX_STEPS],
            length: 16,
            division: Division::Sixteenth,
            brain_link: false,
            muted: false,
        }
    }
}

impl Track {
    pub fn len(&self) -> usize {
        self.length.clamp(1, MAX_STEPS as u8) as usize
    }
    pub fn is_empty(&self) -> bool {
        !self.steps[..self.len()].iter().any(|s| s.active)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Project {
    pub tracks: [Track; TRACKS],
    pub brain: Brain,
    pub bpm: f32,
    /// 0.5 = straight, up to 0.75 = heavy shuffle (applies to odd steps).
    pub swing: f32,
    /// Seed for every random component.
    pub seed: u64,
}

impl Default for Project {
    fn default() -> Self {
        Self {
            tracks: [Track::default(); TRACKS],
            brain: Brain::default(),
            bpm: 120.0,
            swing: 0.5,
            seed: 0x0003_5EED,
        }
    }
}

/// Every edit to a [`Project`]. Small and `Copy` so it can cross a lock-free
/// queue into the audio thread.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Command {
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
}

impl Project {
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
