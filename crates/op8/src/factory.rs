//! The eight projects that always ship (the top two rows of the FILE pads): eight short pieces in eight
//! styles, so that the first thing anyone hears is what the machine can do. Each is made only with the
//! factory instruments and their presets, on the standard tracks (A drums, B KickSyn, C FmBass, D resonator,
//! E sampler, F granular, G macro, H MIDI out), so it always loads.

use crate::projectfile::ProjectFile;
use op8_seq::{
    Brain, BrainMode, ChordKind, ChordSlot, ChordType, Division, FxKind, Lfo, LfoShape, PlayMode, Players, Project, Scale, Step,
    StepComponent as C, TrackFx,
};

pub const NAMES: [&str; 8] = ["house", "boom bap", "dub", "polyrhythm", "acid", "electro", "ambient", "breaks"];

/// The drum lanes (a kit step is a set of them).
const KICK: u8 = 1;
const SNARE: u8 = 1 << 1;
const CHAT: u8 = 1 << 2;
const OHAT: u8 = 1 << 3;
const TOM: u8 = 1 << 4;
const CLAP: u8 = 1 << 5;
const RIM: u8 = 1 << 6;
const CRASH: u8 = 1 << 7;

/// An instrument in its place, with one of its own presets applied (`""`: as it comes).
fn inst(name: &str, preset: &str, gain: f32) -> Option<(String, Vec<u8>)> {
    let mut c = op8_cartridges::create(name);
    let found = op8_cartridges::presets::factory(name).into_iter().find(|p| p.0 == preset);
    let state = match (c.as_mut(), found) {
        (Some(c), found) => {
            for (i, v) in found.iter().flat_map(|f| f.1.iter().enumerate()) {
                if let Some(v) = v {
                    c.set_parameter(i, *v);
                }
            }
            // the `level` knob is the last of every instrument but the kit (whose level is per sound)
            if name != "drums" {
                if let Some(level) = c.parameter_value(7) {
                    c.set_parameter(7, (level * gain).clamp(0.0, 1.0));
                }
            }
            c.save_state()
        }
        _ => Vec::new(),
    };
    Some((name.to_string(), state))
}

/// The instruments of a piece, by track: the standard layout, with a preset on each that plays and the level of each brought by `gain`
/// (the pieces are about as loud as each other).
fn instruments(presets: [&str; 8], gain: f32) -> Vec<Option<(String, Vec<u8>)>> {
    let names = [Some("drums"), Some("KickSyn"), Some("FmBass"), Some("resonator"), Some("sampler"), Some("granular"), Some("macro"), Some("MidiOutBridge")];
    names.iter().zip(presets).map(|(n, p)| n.and_then(|n| inst(n, p, gain))).collect()
}

fn put(steps: &mut [Step], list: &[(usize, u8)]) {
    for &(s, n) in list {
        steps[s] = Step::on(n);
    }
}

fn kit(steps: &mut [Step], lanes: u8, at: &[usize]) {
    for &s in at {
        steps[s].lanes |= lanes;
        steps[s].active = true;
    }
}

fn brain(key: u8, scale: Scale, kind: ChordKind, degrees: &[u8]) -> Brain {
    let mut b = Brain { key, scale, mode: BrainMode::Chord, ..Brain::default() };
    for (slot, &degree) in b.progression.iter_mut().zip(degrees) {
        *slot = ChordSlot { degree, kind, length: 16 };
    }
    b.len = degrees.len().clamp(1, 8) as u8;
    b
}

/// The track number of each standard instrument.
const A: usize = 0;
const B: usize = 1;
const CC: usize = 2;
const D: usize = 3;
const SAMP: usize = 4;
const MACRO: usize = 6;

pub fn project(i: usize) -> Project {
    let mut p = Project::default();
    match i {
        // 0 · house: a kick on every beat, offbeat open hats and a clap, a round bass between the kicks, stabs that follow the chords
        0 => {
            p.bpm = 124.0;
            p.swing = 0.54;
            p.brain = brain(9, Scale::Minor, ChordKind::Seventh, &[0, 5, 2, 6]);
            p.tracks[A].kit = true;
            kit(&mut p.tracks[A].steps, OHAT, &[2, 6, 10, 14]);
            kit(&mut p.tracks[A].steps, CLAP, &[4, 12]);
            kit(&mut p.tracks[A].steps, CHAT, &[0, 8, 15]);
            put(&mut p.tracks[B].steps, &[(0, 36), (4, 36), (8, 36), (12, 36)]);
            p.tracks[CC].brain_link = true;
            put(&mut p.tracks[CC].steps, &[(2, 36), (6, 36), (10, 36), (13, 43)]);
            for s in [2, 6, 10, 13] {
                p.tracks[CC].steps[s].gate = 40;
            }
            let stab = &mut p.tracks[D];
            stab.brain_link = true;
            put(&mut stab.steps, &[(3, 60), (6, 60), (11, 60), (14, 60)]);
            stab.players = Players { mode: PlayMode::Chord, chord: ChordType::MinorSeventh, speed: 0, octaves: 1 };
            stab.fx = TrackFx { kind: FxKind::Delay, a: 160, b: 100, c: 110 };
            let pad = &mut p.tracks[MACRO];
            pad.brain_link = true;
            pad.steps[0] = Step::on(60).with(C::Hold(14));
            pad.fx = TrackFx { kind: FxKind::Reverb, a: 150, b: 150, c: 70 };
        }
        // 1 · boom bap: a lazy swing, a sub bass under a soft kick, a voice sample and chords on the off beats
        1 => {
            p.bpm = 90.0;
            p.swing = 0.60;
            p.brain = brain(0, Scale::Dorian, ChordKind::Seventh, &[0, 3, 0, 4]);
            p.tracks[A].kit = true;
            kit(&mut p.tracks[A].steps, SNARE, &[4, 12]);
            kit(&mut p.tracks[A].steps, CHAT, &[0, 2, 4, 6, 8, 10, 12, 14]);
            kit(&mut p.tracks[A].steps, RIM, &[15]);
            p.tracks[A].steps[15].components.push(C::Chance(50));
            put(&mut p.tracks[B].steps, &[(0, 36), (7, 36), (10, 36)]);
            p.tracks[CC].brain_link = true;
            put(&mut p.tracks[CC].steps, &[(0, 36), (7, 36), (10, 38)]);
            for s in [0, 7, 10] {
                p.tracks[CC].steps[s].gate = 75;
            }
            let keys = &mut p.tracks[D];
            keys.brain_link = true;
            put(&mut keys.steps, &[(3, 60), (11, 60)]);
            keys.players = Players { mode: PlayMode::Chord, chord: ChordType::MinorSeventh, speed: 2, octaves: 1 };
            keys.fx = TrackFx { kind: FxKind::Reverb, a: 140, b: 130, c: 70 };
            put(&mut p.tracks[SAMP].steps, &[(0, 60), (11, 67)]);
            p.tracks[SAMP].steps[11].components.push(C::Chance(70));
        }
        // 2 · dub: slow and wide: one deep kick, a sub bass that breathes, bells thrown into an echo, a pad in a big room
        2 => {
            p.bpm = 70.0;
            p.swing = 0.5;
            p.brain = brain(9, Scale::Minor, ChordKind::Triad, &[0, 0, 0, 3]);
            p.tracks[A].kit = true;
            kit(&mut p.tracks[A].steps, RIM, &[12]);
            kit(&mut p.tracks[A].steps, CHAT, &[6, 14]);
            p.tracks[A].fx = TrackFx { kind: FxKind::Delay, a: 200, b: 140, c: 120 };
            put(&mut p.tracks[B].steps, &[(0, 36), (10, 36)]);
            let bass = &mut p.tracks[CC];
            bass.brain_link = true;
            bass.steps[0] = Step::on(36).with(C::Hold(2));
            bass.steps[7] = Step::on(36);
            bass.steps[10] = Step::on(39).with(C::Hold(1));
            bass.lfo = Lfo { shape: LfoShape::Triangle, rate: 55, depth: 150, target: 3 };
            let bells = &mut p.tracks[D];
            bells.brain_link = true;
            put(&mut bells.steps, &[(4, 72), (11, 67)]);
            bells.steps[11].components.push(C::Chance(70));
            bells.fx = TrackFx { kind: FxKind::Delay, a: 190, b: 150, c: 130 };
            let pad = &mut p.tracks[MACRO];
            pad.brain_link = true;
            pad.steps[0] = Step::on(57).with(C::Hold(15));
            pad.fx = TrackFx { kind: FxKind::Reverb, a: 200, b: 170, c: 110 };
        }
        // 3 · polyrhythm: five lengths that never line up (3, 5, 7, 9 and 11 steps), over a chord that moves slowly
        3 => {
            p.bpm = 96.0;
            p.swing = 0.5;
            p.brain = brain(0, Scale::Major, ChordKind::Triad, &[0, 4, 5, 3]);
            p.tracks[A].kit = true;
            p.tracks[A].length = 3;
            kit(&mut p.tracks[A].steps, CHAT, &[0]);
            p.tracks[B].length = 5;
            p.tracks[B].steps[0] = Step::on(36);
            let harp = &mut p.tracks[D];
            harp.length = 7;
            harp.brain_link = true;
            put(&mut harp.steps, &[(0, 60), (2, 64), (4, 67)]);
            let strings = &mut p.tracks[MACRO];
            strings.length = 9;
            strings.brain_link = true;
            strings.steps[0] = Step::on(55).with(C::Hold(6));
            strings.fx = TrackFx { kind: FxKind::Reverb, a: 170, b: 150, c: 90 };
            let bell = &mut p.tracks[SAMP];
            bell.length = 11;
            bell.brain_link = true;
            put(&mut bell.steps, &[(0, 72), (6, 67)]);
            bell.fx = TrackFx { kind: FxKind::Delay, a: 170, b: 110, c: 100 };
        }
        // 4 · acid: a bass line that opens and closes by itself, hats and a clap, a drive on the bass
        4 => {
            p.bpm = 130.0;
            p.swing = 0.5;
            p.brain = brain(9, Scale::Minor, ChordKind::Triad, &[0, 0, 0, 6]);
            p.tracks[A].kit = true;
            kit(&mut p.tracks[A].steps, CHAT, &[2, 6, 10, 14]);
            kit(&mut p.tracks[A].steps, CLAP, &[4, 12]);
            kit(&mut p.tracks[A].steps, CRASH, &[0]);
            p.tracks[A].steps[0].components.push(C::Every { n: 1, of: 4 });
            put(&mut p.tracks[B].steps, &[(0, 36), (4, 36), (8, 36), (12, 36)]);
            let bass = &mut p.tracks[CC];
            bass.brain_link = true;
            put(&mut bass.steps, &[(0, 36), (2, 36), (3, 48), (5, 36), (6, 36), (8, 36), (10, 39), (11, 36), (13, 36), (14, 43)]);
            bass.steps[6] = Step::on(36).with(C::Octave(1));
            bass.steps[11] = Step::on(36).with(C::Ratchet(2));
            bass.steps[14].components.push(C::Velocity(-30));
            bass.fx = TrackFx { kind: FxKind::Drive, a: 110, b: 170, c: 190 };
            bass.lfo = Lfo { shape: LfoShape::Triangle, rate: 80, depth: 200, target: 3 };
        }
        // 5 · electro: a dry kit, a hard bass, and an arpeggio that runs over the chords with an echo behind it
        5 => {
            p.bpm = 110.0;
            p.swing = 0.5;
            p.brain = brain(4, Scale::Minor, ChordKind::Triad, &[0, 5, 6, 2]);
            p.tracks[A].kit = true;
            kit(&mut p.tracks[A].steps, SNARE, &[4, 12]);
            kit(&mut p.tracks[A].steps, CLAP, &[12]);
            kit(&mut p.tracks[A].steps, CHAT, &[0, 2, 4, 6, 8, 10, 12, 14]);
            put(&mut p.tracks[B].steps, &[(0, 36), (3, 36), (8, 36), (11, 36)]);
            let bass = &mut p.tracks[CC];
            bass.brain_link = true;
            put(&mut bass.steps, &[(0, 36), (3, 36), (6, 48), (8, 36), (11, 41)]);
            let lead = &mut p.tracks[MACRO];
            lead.brain_link = true;
            lead.division = Division::Eighth;
            put(&mut lead.steps, &[(0, 60), (3, 64), (6, 67), (9, 72)]);
            lead.players = Players { mode: PlayMode::UpDown, chord: ChordType::Minor, speed: 2, octaves: 2 };
            lead.fx = TrackFx { kind: FxKind::Delay, a: 178, b: 120, c: 110 };
        }
        // 6 · ambient: nothing hits. Long sounds: a glass line, a pad that holds through a loop and follows the chords, a chorus on the strings
        6 => {
            p.bpm = 60.0;
            p.swing = 0.5;
            p.brain = brain(5, Scale::Lydian, ChordKind::Seventh, &[0, 1, 0, 5]);
            let glass = &mut p.tracks[D];
            glass.brain_link = true;
            put(&mut glass.steps, &[(0, 72), (5, 67), (9, 76), (13, 71)]);
            glass.steps[9].components.push(C::Chance(70));
            glass.fx = TrackFx { kind: FxKind::Reverb, a: 210, b: 175, c: 130 };
            let strings = &mut p.tracks[MACRO];
            strings.brain_link = true;
            strings.steps[0] = Step::on(60).with(C::Hold(15));
            strings.fx = TrackFx { kind: FxKind::Chorus, a: 70, b: 140, c: 128 };
            let pad = &mut p.tracks[SAMP];
            pad.brain_link = true;
            pad.steps[0] = Step::on(48).with(C::Hold(15));
            pad.fx = TrackFx { kind: FxKind::Reverb, a: 190, b: 160, c: 100 };
        }
        // 7 · breaks: fast and unsteady: a kit that never plays the same bar twice, a growling bass, glitches and a ratchet or two
        _ => {
            p.bpm = 140.0;
            p.swing = 0.52;
            p.brain = brain(2, Scale::Phrygian, ChordKind::Triad, &[0, 0, 1, 0]);
            p.tracks[A].kit = true;
            kit(&mut p.tracks[A].steps, SNARE, &[4, 13]);
            p.tracks[A].steps[13].components.push(C::Ratchet(2));
            kit(&mut p.tracks[A].steps, CHAT, &[2, 6, 8, 12, 14]);
            for s in [2, 6, 14] {
                p.tracks[A].steps[s].components.push(C::Chance(70));
            }
            kit(&mut p.tracks[A].steps, CLAP, &[12]);
            p.tracks[A].steps[12].components.push(C::Chance(40));
            kit(&mut p.tracks[A].steps, TOM, &[15]);
            p.tracks[A].steps[15].components.push(C::Every { n: 2, of: 2 });
            put(&mut p.tracks[B].steps, &[(0, 36), (10, 36)]);
            p.tracks[B].steps[3] = Step::on(36).with(C::Chance(50));
            let bass = &mut p.tracks[CC];
            bass.brain_link = true;
            put(&mut bass.steps, &[(0, 36), (6, 36), (7, 39), (11, 36)]);
            bass.steps[7].components.push(C::Every { n: 1, of: 2 });
            let noise = &mut p.tracks[D];
            noise.brain_link = true;
            put(&mut noise.steps, &[(1, 60), (9, 64), (14, 67)]);
            for s in [1, 9, 14] {
                noise.steps[s].components.push(C::RandomNote(7));
                noise.steps[s].components.push(C::Chance(60));
            }
            let tick = &mut p.tracks[SAMP];
            put(&mut tick.steps, &[(1, 60), (5, 60), (9, 60), (11, 60), (14, 60)]);
            for (s, c) in [(1, C::Ratchet(3)), (5, C::Chance(60)), (9, C::Ratchet(4)), (11, C::Chance(50)), (14, C::Ratchet(2))] {
                tick.steps[s].components.push(c);
            }
            tick.fx = TrackFx { kind: FxKind::Delay, a: 150, b: 90, c: 90 };
        }
    }
    p
}

/// The presets of each piece, by track A to H (`""`: as it comes), and how much of the level of its instruments it keeps.
const PRESETS: [([&str; 8], f32); 8] = [
    (["", "punch", "round", "steel", "", "shimmer", "chords", ""], 0.85),
    (["", "soft", "sub", "nylon", "voice", "rumble", "", ""], 1.0),
    (["", "boom", "sub", "bell", "", "dust", "swarm", ""], 0.8),
    (["", "soft", "", "harp", "bell", "bells", "strings", ""], 1.0),
    (["", "tight", "acid", "", "", "swarm", "", ""], 0.5),
    (["", "click", "growl", "", "", "glitch", "chip", ""], 0.65),
    (["", "", "", "glass", "pad", "cloud", "strings", ""], 1.0),
    (["", "hard", "growl", "muted", "tick", "glitch", "", ""], 1.0),
];

/// A new, empty project: the standard instruments as they come and nothing in the steps (a tap on an empty place of FILE).
pub fn blank() -> ProjectFile {
    let mut project = Project::default();
    project.bpm = 120.0;
    ProjectFile { project, instruments: instruments([""; 8], 1.0) }
}

pub fn file(i: usize) -> ProjectFile {
    let i = i.min(7);
    ProjectFile { project: project(i), instruments: instruments(PRESETS[i].0, PRESETS[i].1) }
}

#[cfg(test)]
mod tests {
    use super::*;

    // F-F03
    #[test]
    fn the_eight_factory_projects_load_and_differ() {
        let mut seen = Vec::new();
        for i in 0..8 {
            let f = file(i);
            assert!(ProjectFile::decode(&f.encode()).is_some(), "{} round-trips", NAMES[i]);
            assert!(!seen.contains(&f.project), "{} differs from the others", NAMES[i]);
            seen.push(f.project);
        }
    }
}
