//! Saving and loading a [`Project`] as bytes (the FILE button). A plain, versioned binary
//! format written by hand: no dependencies, and a file from a newer version is refused, not
//! misread. The instruments' own state is stored next to it by the `op8` crate.

use crate::{
    brain::{Brain, BrainMode, ChordKind, ChordSlot, Scale, MAX_PROGRESSION},
    project::{Division, Project, Track},
    rack::{ChordType, FxKind, Lfo, LfoShape, PlayMode, Players, TrackFx, LFO_TARGETS},
    step::{Step, StepComponent},
    MAX_STEPS, TRACKS,
};

/// Version 2 adds the players, the effect and the LFO of each track; a version 1 file still opens (those are then off).
/// Version 3 adds the level and the pan of each track; an older file still opens (the tracks are then at the default level, centred).
/// Version 4 adds the master compressor after the tracks; an older file still opens (it is then at its defaults, which do nothing).
/// Version 5 adds the master EQ after the compressor; an older file still opens (it is then flat).
pub const VERSION: u8 = 5;

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
        w.u8(PlayMode::ALL.iter().position(|m| *m == t.players.mode).unwrap_or(0) as u8);
        w.u8(ChordType::ALL.iter().position(|c| *c == t.players.chord).unwrap_or(0) as u8);
        w.u8(t.players.speed);
        w.u8(t.players.octaves);
        w.u8(FxKind::ALL.iter().position(|k| *k == t.fx.kind).unwrap_or(0) as u8);
        w.u8(t.fx.a);
        w.u8(t.fx.b);
        w.u8(t.fx.c);
        w.u8(LfoShape::ALL.iter().position(|k| *k == t.lfo.shape).unwrap_or(0) as u8);
        w.u8(t.lfo.rate);
        w.u8(t.lfo.depth);
        w.u8(t.lfo.target);
        w.u8(t.level);
        w.u8(t.pan);
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
    for i in 0..crate::COMP_PARAMS {
        w.u8(p.comp.get(i));
    }
    for i in 0..crate::EQ_PARAMS {
        w.u8(p.eq.get(i));
    }
    w.0
}

/// A project from bytes, or `None` if they are not a project of a version this build knows.
pub fn decode(data: &[u8]) -> Option<Project> {
    let mut r = R(data);
    let version = r.u8()?;
    if !(1..=VERSION).contains(&version) {
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
        if version >= 2 {
            tr.players = Players {
                mode: *PlayMode::ALL.get(r.u8()? as usize)?,
                chord: *ChordType::ALL.get(r.u8()? as usize)?,
                speed: r.u8()?.min(4),
                octaves: r.u8()?.clamp(1, 3),
            };
            tr.fx = TrackFx { kind: *FxKind::ALL.get(r.u8()? as usize)?, a: r.u8()?, b: r.u8()?, c: r.u8()? };
            tr.lfo = Lfo {
                shape: *LfoShape::ALL.get(r.u8()? as usize)?,
                rate: r.u8()?,
                depth: r.u8()?,
                target: r.u8()?.min(LFO_TARGETS - 1),
            };
        }
        if version >= 3 {
            tr.level = r.u8()?.min(127);
            tr.pan = r.u8()?.min(127);
        }
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
    if version >= 4 {
        for i in 0..crate::COMP_PARAMS {
            let v = r.u8()?;
            p.comp.set(i, v);
        }
    }
    if version >= 5 {
        for i in 0..crate::EQ_PARAMS {
            let v = r.u8()?;
            p.eq.set(i, v);
        }
    }
    Some(p)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The bytes of a version 3 file for `p`: the current ones without the compressor at the end.
    fn as_version_3(p: &Project) -> Vec<u8> {
        let mut v = encode(p);
        v.truncate(v.len() - crate::COMP_PARAMS - crate::EQ_PARAMS);
        v[0] = 3;
        v
    }

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
        p.tracks[1].players = Players { mode: PlayMode::UpDown, chord: ChordType::MinorSeventh, speed: 3, octaves: 2 };
        p.tracks[1].fx = TrackFx { kind: FxKind::Delay, a: 10, b: 200, c: 77 };
        p.tracks[1].lfo = Lfo { shape: LfoShape::Saw, rate: 33, depth: 250, target: 4 };
        p.tracks[2].level = 31;
        p.tracks[2].pan = 120;
        p.comp = crate::MasterComp { threshold: 90, ratio: 200, attack: 10, release: 250, makeup: 77, mix: 128 };
        p.eq = crate::MasterEq { low: 10, mid: 200, freq: 33, high: 250 };
        let bytes = encode(&p);
        let q = decode(&bytes).expect("decodes");
        assert_eq!(q, p);
    }

    // F-F01
    #[test]
    fn a_version_1_file_still_opens_with_the_extras_off() {
        let mut p = Project::default();
        p.bpm = 101.0;
        p.tracks[3].steps[2] = Step::on(50);
        let v3 = as_version_3(&p);
        // no components in this project, so each track is 3 header bytes, 14 extras (version 2's 12 and the level and pan) and 16 steps of 6 bytes
        let head = v3.len() - TRACKS * (3 + 14 + MAX_STEPS * 6);
        let mut v1 = v3[..head].to_vec();
        v1[0] = 1;
        for t in 0..TRACKS {
            let at = head + t * (3 + 14 + MAX_STEPS * 6);
            v1.extend_from_slice(&v3[at..at + 3]);
            v1.extend_from_slice(&v3[at + 17..at + 17 + MAX_STEPS * 6]);
        }
        assert_eq!(decode(&v1).expect("a version 1 file opens"), p);
    }

    // F-F01
    #[test]
    fn a_version_2_file_still_opens_with_the_mix_at_its_defaults() {
        let mut p = Project::default();
        p.tracks[3].steps[2] = Step::on(50);
        p.tracks[4].lfo = Lfo { shape: LfoShape::Saw, rate: 33, depth: 250, target: 4 };
        let v3 = as_version_3(&p);
        let per_track = 3 + 14 + MAX_STEPS * 6;
        let head = v3.len() - TRACKS * per_track;
        let mut v2 = v3[..head].to_vec();
        v2[0] = 2;
        for t in 0..TRACKS {
            let at = head + t * per_track;
            v2.extend_from_slice(&v3[at..at + 15]); // the header and version 2's twelve extras
            v2.extend_from_slice(&v3[at + 17..at + per_track]); // not the level and the pan
        }
        assert_eq!(decode(&v2).expect("a version 2 file opens"), p, "the mix is at its defaults, as the project made it");
    }

    // F-F01
    #[test]
    fn a_version_3_file_still_opens_with_the_compressor_doing_nothing() {
        let mut p = Project::default();
        p.tracks[1].level = 40;
        let q = decode(&as_version_3(&p)).expect("a version 3 file opens");
        assert_eq!(q, p);
        assert_eq!(q.comp, crate::MasterComp::default());
    }

    // F-F01
    #[test]
    fn a_version_4_file_still_opens_with_the_eq_flat() {
        let mut p = Project::default();
        p.comp.threshold = 70;
        let mut v = encode(&p);
        v.truncate(v.len() - crate::EQ_PARAMS);
        v[0] = 4;
        let q = decode(&v).expect("a version 4 file opens");
        assert_eq!(q, p);
        assert_eq!(q.eq, crate::MasterEq::default());
    }

    // F-F01
    #[test]
    fn garbage_and_other_versions_are_refused() {
        assert!(decode(&[]).is_none());
        assert!(decode(&[VERSION + 1, 0, 0]).is_none());
        assert!(decode(&[0, 0, 0]).is_none());
        let bytes = encode(&Project::default());
        assert!(decode(&bytes[..bytes.len() - 3]).is_none(), "a truncated file is refused");
        let mut bad = bytes.clone();
        bad[0] = 200;
        assert!(decode(&bad).is_none());
    }
}
