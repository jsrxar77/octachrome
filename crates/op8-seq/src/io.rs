//! Saving and loading a [`Project`] as bytes (the FILE button). A plain, versioned binary
//! format written by hand: no dependencies, and a file from a newer version is refused, not
//! misread. The instruments' own state is stored next to it by the `op8` crate.

use crate::{
    brain::{Brain, BrainMode, ChordKind, ChordSlot, Scale, MAX_PROGRESSION},
    project::{Division, Project, Track},
    step::{Step, StepComponent},
    MAX_STEPS, TRACKS,
};

pub const VERSION: u8 = 1;

struct W(Vec<u8>);
impl W {
    fn u8(&mut self, v: u8) {
        self.0.push(v);
    }
    fn i8(&mut self, v: i8) {
        self.0.push(v as u8);
    }
    fn f32(&mut self, v: f32) {
        self.0.extend_from_slice(&v.to_le_bytes());
    }
    fn u64(&mut self, v: u64) {
        self.0.extend_from_slice(&v.to_le_bytes());
    }
}

struct R<'a>(&'a [u8]);
impl R<'_> {
    fn u8(&mut self) -> Option<u8> {
        let (&b, rest) = self.0.split_first()?;
        self.0 = rest;
        Some(b)
    }
    fn i8(&mut self) -> Option<i8> {
        self.u8().map(|b| b as i8)
    }
    fn f32(&mut self) -> Option<f32> {
        let b = self.0.get(..4)?;
        let v = f32::from_le_bytes([b[0], b[1], b[2], b[3]]);
        self.0 = &self.0[4..];
        Some(v)
    }
    fn u64(&mut self) -> Option<u64> {
        let b = self.0.get(..8)?;
        let v = u64::from_le_bytes(b.try_into().ok()?);
        self.0 = &self.0[8..];
        Some(v)
    }
}

fn component(w: &mut W, c: &StepComponent) {
    use StepComponent::*;
    match *c {
        None => w.u8(0),
        Ratchet(n) => {
            w.u8(1);
            w.u8(n)
        }
        Chance(p) => {
            w.u8(2);
            w.u8(p)
        }
        Every { n, of } => {
            w.u8(3);
            w.u8(n);
            w.u8(of)
        }
        Velocity(v) => {
            w.u8(4);
            w.i8(v)
        }
        Transpose(v) => {
            w.u8(5);
            w.i8(v)
        }
        Octave(v) => {
            w.u8(6);
            w.i8(v)
        }
        Hold(n) => {
            w.u8(7);
            w.u8(n)
        }
        Delay(n) => {
            w.u8(8);
            w.u8(n)
        }
        RandomNote(n) => {
            w.u8(9);
            w.u8(n)
        }
        ParamLock { index, value } => {
            w.u8(10);
            w.u8(index);
            w.u8(value)
        }
    }
}

fn read_component(r: &mut R) -> Option<StepComponent> {
    use StepComponent::*;
    Some(match r.u8()? {
        0 => None,
        1 => Ratchet(r.u8()?),
        2 => Chance(r.u8()?),
        3 => Every { n: r.u8()?, of: r.u8()? },
        4 => Velocity(r.i8()?),
        5 => Transpose(r.i8()?),
        6 => Octave(r.i8()?),
        7 => Hold(r.u8()?),
        8 => Delay(r.u8()?),
        9 => RandomNote(r.u8()?),
        10 => ParamLock { index: r.u8()?, value: r.u8()? },
        _ => return Option::None,
    })
}

/// The project as bytes.
pub fn encode(p: &Project) -> Vec<u8> {
    let mut w = W(Vec::new());
    w.u8(VERSION);
    w.f32(p.bpm);
    w.f32(p.swing);
    w.u64(p.seed);
    let b = &p.brain;
    w.u8(b.key);
    w.u8(Scale::ALL.iter().position(|s| *s == b.scale).unwrap_or(0) as u8);
    w.u8(b.mode as u8);
    w.u8(b.len);
    for slot in &b.progression {
        w.u8(slot.degree);
        w.u8(slot.kind as u8);
        w.u8(slot.length);
    }
    for t in &p.tracks {
        w.u8(t.length);
        w.u8(Division::ALL.iter().position(|d| *d == t.division).unwrap_or(2) as u8);
        w.u8(t.brain_link as u8 | (t.muted as u8) << 1 | (t.kit as u8) << 2);
        for s in &t.steps {
            w.u8(s.active as u8);
            w.u8(s.note);
            w.u8(s.velocity);
            w.u8(s.gate);
            w.u8(s.lanes);
            w.u8(s.components.as_slice().len() as u8);
            for c in s.components.iter() {
                component(&mut w, c);
            }
        }
    }
    w.0
}

/// A project from bytes, or `None` if they are not a project of a version this build knows.
pub fn decode(data: &[u8]) -> Option<Project> {
    let mut r = R(data);
    if r.u8()? != VERSION {
        return None;
    }
    let mut p = Project::default();
    p.bpm = r.f32()?.clamp(20.0, 300.0);
    p.swing = r.f32()?.clamp(0.5, 0.75);
    p.seed = r.u64()?;
    let key = r.u8()?;
    let scale = *Scale::ALL.get(r.u8()? as usize)?;
    let mode = match r.u8()? {
        0 => BrainMode::Transpose,
        1 => BrainMode::Chord,
        2 => BrainMode::Scale,
        _ => return None,
    };
    let len = r.u8()?;
    let mut progression = [ChordSlot::default(); MAX_PROGRESSION];
    for slot in progression.iter_mut() {
        slot.degree = r.u8()?;
        slot.kind = match r.u8()? {
            0 => ChordKind::Triad,
            1 => ChordKind::Seventh,
            2 => ChordKind::Sus2,
            3 => ChordKind::Sus4,
            _ => return None,
        };
        slot.length = r.u8()?;
    }
    p.brain = Brain { key: key % 12, scale, mode, progression, len: len.clamp(1, MAX_PROGRESSION as u8), live: None };
    for t in p.tracks.iter_mut().take(TRACKS) {
        let mut tr = Track::default();
        tr.length = r.u8()?.clamp(1, MAX_STEPS as u8);
        tr.division = *Division::ALL.get(r.u8()? as usize)?;
        let flags = r.u8()?;
        (tr.brain_link, tr.muted, tr.kit) = (flags & 1 != 0, flags & 2 != 0, flags & 4 != 0);
        for s in tr.steps.iter_mut() {
            let mut st = Step::default();
            st.active = r.u8()? != 0;
            st.note = r.u8()?.min(127);
            st.velocity = r.u8()?.clamp(1, 127);
            st.gate = r.u8()?.clamp(1, 100);
            st.lanes = r.u8()?;
            for _ in 0..r.u8()? {
                let c = read_component(&mut r)?;
                st.components.push(c);
            }
            *s = st;
        }
        *t = tr;
    }
    Some(p)
}

#[cfg(test)]
mod tests {
    use super::*;

    // F-F01
    #[test]
    fn a_project_round_trips_through_bytes() {
        let mut p = Project::default();
        p.bpm = 97.5;
        p.swing = 0.6;
        p.brain.key = 7;
        p.brain.scale = Scale::Dorian;
        p.tracks[2].kit = true;
        p.tracks[2].muted = true;
        p.tracks[2].length = 11;
        p.tracks[2].division = Division::Eighth;
        p.tracks[2].steps[3] = Step::kit(0b1010_0001)
            .with(StepComponent::Ratchet(3))
            .with(StepComponent::Every { n: 2, of: 4 })
            .with(StepComponent::Velocity(-20))
            .with(StepComponent::ParamLock { index: 5, value: 200 });
        p.tracks[5].steps[0] = Step::on(61).with(StepComponent::Octave(-1));
        let bytes = encode(&p);
        let q = decode(&bytes).expect("decodes");
        assert_eq!(q, p);
    }

    // F-F01
    #[test]
    fn garbage_and_other_versions_are_refused() {
        assert!(decode(&[]).is_none());
        assert!(decode(&[VERSION + 1, 0, 0]).is_none());
        let bytes = encode(&Project::default());
        assert!(decode(&bytes[..bytes.len() - 3]).is_none(), "a truncated file is refused");
        let mut bad = bytes.clone();
        bad[0] = 200;
        assert!(decode(&bad).is_none());
    }
}
