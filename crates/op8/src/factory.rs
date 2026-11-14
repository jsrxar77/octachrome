//! The eight projects that always ship (the top two rows of the FILE pads): each one shows one
//! idea and is made only with the factory instruments, on the standard tracks (A drums, B KickSyn,
//! C FmBass, D resonator, E MIDI out, F macro), so it always loads.

use crate::projectfile::ProjectFile;
use op8_seq::{Project, Step, StepComponent};

pub const NAMES: [&str; 8] = ["demo", "four floor", "backbeat", "bassline", "pluck", "lead", "polymeter", "blank"];

/// The instruments of the standard tracks, with their default state.
fn instruments() -> Vec<Option<(String, Vec<u8>)>> {
    ["drums", "KickSyn", "FmBass", "resonator", "MidiOutBridge", "macro"]
        .iter()
        .map(|n| Some((n.to_string(), Vec::new())))
        .chain([None, None])
        .collect()
}

pub fn project(i: usize) -> Project {
    let mut p = Project::default();
    match i {
        0 => return crate::demo::project(),
        1 => {
            // A hat over a kick on every beat
            p.bpm = 124.0;
            p.tracks[0].kit = true;
            for s in [2, 6, 10, 14] {
                p.tracks[0].steps[s] = Step::kit(1 << 2);
            }
            p.tracks[0].steps[15] = Step::kit(1 << 3);
            for s in [0, 4, 8, 12] {
                p.tracks[1].steps[s] = Step::on(36);
            }
        }
        2 => {
            // The kit alone: a backbeat with a clap on the snare and a tom to turn it round
            p.bpm = 96.0;
            let k = &mut p.tracks[0];
            k.kit = true;
            k.steps[0] = Step::kit(1 << 0);
            k.steps[10] = Step::kit(1 << 0);
            for s in [4, 12] {
                k.steps[s] = Step::kit(1 << 1 | 1 << 5);
            }
            for s in (0..16).step_by(2) {
                k.steps[s].lanes |= 1 << 2;
                k.steps[s].active = true;
            }
            k.steps[15] = Step::kit(1 << 4);
        }
        3 => {
            // A seven-step bass line that follows the Brain
            p.bpm = 110.0;
            let b = &mut p.tracks[2];
            b.length = 7;
            b.brain_link = true;
            b.steps[0] = Step::on(36);
            b.steps[2] = Step::on(48).with(StepComponent::Ratchet(2));
            b.steps[3] = Step::on(43);
            b.steps[5] = Step::on(40).with(StepComponent::Octave(1));
            for s in [0, 8] {
                p.tracks[1].steps[s] = Step::on(36);
            }
        }
        4 => {
            // Plucked strings over the chords
            p.bpm = 84.0;
            let r = &mut p.tracks[3];
            r.length = 8;
            r.brain_link = true;
            for (s, n) in [(0, 60), (2, 64), (3, 67), (5, 72), (6, 67)] {
                r.steps[s] = Step::on(n);
            }
        }
        5 => {
            // A slow lead line
            p.bpm = 118.0;
            let m = &mut p.tracks[5];
            m.brain_link = true;
            m.steps[0] = Step::on(60);
            m.steps[6] = Step::on(67).with(StepComponent::Hold(2));
            m.steps[10] = Step::on(64).with(StepComponent::Octave(1));
        }
        6 => {
            // Three lengths drifting apart: 5, 7 and 3 steps
            p.bpm = 100.0;
            p.tracks[1].length = 5;
            p.tracks[1].steps[0] = Step::on(36);
            p.tracks[3].length = 7;
            p.tracks[3].steps[0] = Step::on(60);
            p.tracks[3].steps[3] = Step::on(67);
            p.tracks[0].kit = true;
            p.tracks[0].length = 3;
            p.tracks[0].steps[0] = Step::kit(1 << 2);
        }
        _ => p.bpm = 120.0, // blank: the instruments, an empty pattern
    }
    p
}

pub fn file(i: usize) -> ProjectFile {
    ProjectFile { project: project(i), instruments: instruments() }
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
