//! The eight projects that always ship (the top two rows of the FILE pads): eight short pieces in eight
//! styles, so that the first thing anyone hears is what the machine can do. Each is made only with the
//! factory instruments and their presets, on the standard tracks (A drums, B KickSyn, C FmBass, D resonator,
//! E sampler, F granular, G macro, H MIDI out), so it always loads.

use crate::projectfile::ProjectFile;
use octachrome_seq::{
    Brain, BrainMode, ChordKind, ChordSlot, ChordType, Command, FxKind, Lfo, LfoShape, PlayMode, Players, Project, Scale, Step,
    StepComponent as C, TrackFx,
};

pub const NAMES: [&str; 8] = ["house", "techno", "garage", "dnb", "trance", "amapiano", "ambient", "lo-fi"];

/// The drum lanes (a kit step is a set of them).
const KICK: u8 = 1;
const SNARE: u8 = 1 << 1;
const CHAT: u8 = 1 << 2;
const OHAT: u8 = 1 << 3;
const TOM: u8 = 1 << 4;
const CLAP: u8 = 1 << 5;
const RIM: u8 = 1 << 6;
const CRASH: u8 = 1 << 7;

/// An instrument in its place, with one of its own presets applied (`""`: as it comes). `extra` sets knobs after the preset (a level
/// given there is kept as it is, not scaled by `gain`); `kit` voices the sounds of a drum kit: (sound, tune, decay, tone, snap, body, level, pan).
fn inst_with(name: &str, preset: &str, gain: f32, extra: &[(usize, f32)], kit: &[(usize, [f32; 7])]) -> Option<(String, Vec<u8>)> {
    let mut c = octachrome_cartridges::create(name);
    let found = octachrome_cartridges::presets::factory(name).into_iter().find(|p| p.0 == preset);
    let state = match (c.as_mut(), found) {
        (Some(c), found) => {
            for (i, v) in found.iter().flat_map(|f| f.1.iter().enumerate()) {
                if let Some(v) = v {
                    c.set_parameter(i, *v);
                }
            }
            for &(i, v) in extra {
                c.set_parameter(i, v);
            }
            for &(sound, values) in kit {
                c.set_parameter(7, (sound as f32 + 0.5) / 8.0); // the knobs edit the sound that is chosen
                for (i, v) in values.iter().enumerate() {
                    c.set_parameter(i, *v);
                }
            }
            if !kit.is_empty() {
                c.set_parameter(7, 0.5 / 8.0);
            }
            // the `level` knob is the last of every instrument but the kit (whose level is per sound)
            if name != "drums" && !extra.iter().any(|e| e.0 == 7) {
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

fn inst(name: &str, preset: &str, gain: f32) -> Option<(String, Vec<u8>)> {
    inst_with(name, preset, gain, &[], &[])
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

/// Show pattern `index` of `track` (a new one is empty), to fill it.
fn pattern(p: &mut Project, track: usize, index: u8) {
    p.apply(&Command::SetPattern { track: track as u8, index, now: false });
}

/// Store a scene: the pattern each track shows, the tracks that are muted and the harmony.
fn scene(p: &mut Project, index: u8, patterns: [u8; 8], muted: &[usize], harmony: Brain) {
    for (t, &i) in patterns.iter().enumerate() {
        pattern(p, t, i);
        p.tracks[t].muted = muted.contains(&t);
    }
    p.brain = harmony;
    p.apply(&Command::StoreScene { index });
}

/// Notes on a track's current pattern: (step, note, velocity, gate %). The accents and the short ghost notes are what make it groove.
fn notes(steps: &mut [Step], list: &[(usize, u8, u8, u8)]) {
    for &(s, n, velocity, gate) in list {
        steps[s] = Step::on(n);
        steps[s].velocity = velocity;
        steps[s].gate = gate;
    }
}

/// Drum steps on a kit pattern: (step, lanes, velocity).
fn hits(steps: &mut [Step], list: &[(usize, u8, u8)]) {
    for &(s, lanes, velocity) in list {
        steps[s].lanes |= lanes;
        steps[s].active = true;
        steps[s].velocity = velocity;
    }
}

/// A rising lock on a step (the build): parameter `index` of the instrument goes to `value`/255 on that step.
fn lock(step: &mut Step, index: u8, value: u8) {
    step.components.push(C::ParamLock { index, value });
}

/// House as a whole song of about two minutes (67 bars at 122): intro, groove, main, a fill, break, a two-part build, a bar of breath,
/// the drop, a fill, the drop again with other chords, and the outro. It is made as a deep Chicago house is: the kick on every beat and
/// nothing else on it, a bass that lives between the kicks (short ghost notes, accents on the fifth and the octave), stabs on the
/// off-beats whose rhythm changes from bar to bar, hats with accents and ghost notes, voice chops of different pitches, and a build that
/// rises (a clap roll, stabs that speed up, locks that open the pad and the cloud) into a bar with no drums. Every pattern has 16 steps:
/// what changes from bar to bar inside it is done with `Every`. What is in the steps when it loads is pattern 0 of each track (the
/// groove); the rest are more patterns and eleven scenes that choose which ones play, which tracks are heard and the chords.
fn house_song(p: &mut Project) {
    const F: usize = 5;
    const G: usize = MACRO;
    let own = p.brain;
    let every = |n: u8| C::Every { n, of: 2 };
    // ---- A, the drums (a kit): the lanes, with the velocity of each step ----
    // 0 groove: offbeat open hats, a clap on 2 and 4, closed hats with accents and ghost notes
    pattern(p, A, 0);
    p.tracks[A].kit = true;
    hits(&mut p.tracks[A].steps, &[(0, CHAT, 80), (1, CHAT, 40), (2, OHAT, 95), (3, CHAT, 65), (4, CLAP, 105), (5, CHAT, 40), (6, OHAT, 90), (7, CHAT, 70)]);
    hits(&mut p.tracks[A].steps, &[(8, CHAT, 80), (9, CHAT, 40), (10, OHAT, 95), (11, CHAT, 60), (12, CLAP, 105), (13, CHAT, 40), (14, OHAT, 90), (15, CHAT, 75)]);
    // 1 intro: only hats, half as many, and the last one open
    pattern(p, A, 1);
    p.tracks[A].kit = true;
    hits(&mut p.tracks[A].steps, &[(0, CHAT, 70), (2, CHAT, 40), (4, CHAT, 65), (6, CHAT, 40), (8, CHAT, 70), (10, CHAT, 40), (12, CHAT, 65), (14, OHAT, 60)]);
    // 2 main: the groove with a rim in place of a ghost hat, and a rim and a hat every second bar
    pattern(p, A, 2);
    p.tracks[A].kit = true;
    hits(&mut p.tracks[A].steps, &[(0, CHAT, 85), (1, CHAT, 40), (2, OHAT, 95), (3, CHAT, 65), (4, CLAP, 108), (5, CHAT, 40), (6, OHAT, 90), (7, CHAT, 70)]);
    hits(&mut p.tracks[A].steps, &[(8, CHAT, 85), (9, RIM, 55), (10, OHAT, 95), (11, CHAT, 60), (12, CLAP, 108), (13, CHAT, 40), (14, OHAT, 90), (15, CHAT | RIM, 70)]);
    p.tracks[A].steps[15].components.push(every(2));
    // 3 fill (one bar): the groove for half a bar, then a snare run that grows into the next section
    pattern(p, A, 3);
    p.tracks[A].kit = true;
    hits(&mut p.tracks[A].steps, &[(0, CHAT, 80), (2, OHAT, 90), (4, CLAP, 100), (6, CHAT, 60), (8, SNARE, 85), (9, SNARE, 60), (10, SNARE, 75)]);
    hits(&mut p.tracks[A].steps, &[(11, SNARE, 85), (12, SNARE, 95), (13, TOM, 95), (14, SNARE, 105), (15, CLAP | SNARE, 122)]);
    // 4 break: no clap, a few hats and rims
    pattern(p, A, 4);
    p.tracks[A].kit = true;
    hits(&mut p.tracks[A].steps, &[(0, CHAT, 60), (4, CHAT, 50), (7, RIM, 60), (8, CHAT, 60), (10, OHAT, 45), (12, CHAT, 50), (15, RIM, 55)]);
    // 5 build, part one: claps that get louder on the beats, open hats that get louder between them
    pattern(p, A, 5);
    p.tracks[A].kit = true;
    hits(&mut p.tracks[A].steps, &[(0, CLAP, 40), (4, CLAP, 55), (8, CLAP, 70), (12, CLAP, 90), (2, OHAT, 50), (6, OHAT, 60), (10, OHAT, 70), (14, OHAT, 85)]);
    // 6 build, part two: a clap on every sixteenth, from soft to loud
    pattern(p, A, 6);
    p.tracks[A].kit = true;
    for s in 0..16 {
        hits(&mut p.tracks[A].steps, &[(s, CLAP, 35 + 6 * s as u8)]);
    }
    // 7 drop: a crash on the first step, every sixteenth, open hats and a clap with their full velocity
    pattern(p, A, 7);
    p.tracks[A].kit = true;
    hits(&mut p.tracks[A].steps, &[(0, CRASH | CHAT, 110), (1, CHAT, 50), (2, OHAT, 100), (3, CHAT, 70), (4, CLAP, 112), (5, CHAT, 50), (6, OHAT, 95), (7, CHAT, 80)]);
    hits(&mut p.tracks[A].steps, &[(8, CHAT, 88), (9, CHAT, 50), (10, OHAT, 100), (11, CHAT, 70), (12, CLAP, 112), (13, CHAT, 50), (14, OHAT, 95), (15, CHAT, 80)]);
    // 8 drop again: no crash, a tom and a ghost clap that come every second bar
    pattern(p, A, 8);
    p.tracks[A].kit = true;
    hits(&mut p.tracks[A].steps, &[(0, CHAT, 92), (1, CHAT, 50), (2, OHAT, 100), (3, CHAT, 70), (4, CLAP, 112), (5, CHAT, 50), (6, OHAT, 95), (7, CHAT | RIM, 80)]);
    hits(&mut p.tracks[A].steps, &[(8, CHAT, 88), (9, CHAT, 50), (10, OHAT, 100), (11, CLAP, 45), (12, CLAP, 112), (13, CHAT, 50), (14, OHAT, 95), (15, TOM, 78)]);
    p.tracks[A].steps[11].components.push(every(1));
    p.tracks[A].steps[15].components.push(every(2));
    // ---- B, the kick: always on the beat ----
    pattern(p, B, 0);
    notes(&mut p.tracks[B].steps, &[(0, 36, 115, 50), (4, 36, 105, 50), (8, 36, 110, 50), (12, 36, 105, 50)]);
    pattern(p, B, 1); // half time, for the first part of the build
    notes(&mut p.tracks[B].steps, &[(0, 36, 115, 50), (8, 36, 110, 50)]);
    pattern(p, B, 2); // a roll, for the second
    notes(&mut p.tracks[B].steps, &[(0, 36, 100, 50), (4, 36, 105, 50), (8, 36, 110, 50), (10, 36, 100, 50), (12, 36, 110, 50), (13, 36, 105, 50), (14, 36, 115, 50), (15, 36, 120, 50)]);
    // ---- C, the bass: never on a kick (steps 0, 4, 8 and 12); the fifth and the octave are the accents, the short ones are ghosts ----
    pattern(p, CC, 0);
    notes(&mut p.tracks[CC].steps, &[(2, 36, 100, 40), (3, 36, 60, 25), (6, 43, 85, 40), (10, 36, 100, 40), (11, 36, 60, 25), (14, 48, 80, 35)]);
    notes(&mut p.tracks[CC].steps, &[(7, 36, 70, 25), (15, 43, 70, 25)]);
    p.tracks[CC].steps[7].components.push(every(1));
    p.tracks[CC].steps[15].components.push(every(2));
    pattern(p, CC, 1); // busier, for the first drop
    notes(&mut p.tracks[CC].steps, &[(2, 36, 105, 40), (3, 36, 65, 25), (5, 36, 70, 25), (6, 43, 90, 40), (7, 36, 70, 25), (10, 36, 105, 40)]);
    notes(&mut p.tracks[CC].steps, &[(11, 43, 80, 30), (13, 36, 70, 25), (14, 48, 90, 35), (15, 36, 70, 20)]);
    pattern(p, CC, 2); // another line, for the second
    notes(&mut p.tracks[CC].steps, &[(2, 36, 100, 45), (5, 48, 85, 30), (6, 36, 95, 40), (9, 43, 80, 35), (10, 36, 100, 40), (13, 48, 85, 30), (14, 36, 95, 40), (15, 43, 70, 20)]);
    p.tracks[CC].steps[15].components.push(every(2));
    // ---- G, the organ: the chord of the song (a minor seventh on the chord that sounds). Stabs on the off-beats, a different rhythm in
    // every second bar; long chords for the break; a roll that speeds up for the build ----
    pattern(p, G, 0); // main: stabs
    notes(&mut p.tracks[G].steps, &[(3, 60, 100, 45), (6, 60, 80, 40), (11, 60, 100, 45), (14, 60, 75, 40), (10, 60, 70, 35), (9, 60, 65, 35)]);
    p.tracks[G].steps[10].components.push(every(1));
    p.tracks[G].steps[9].components.push(every(2));
    pattern(p, G, 1); // break: two long chords
    notes(&mut p.tracks[G].steps, &[(0, 60, 85, 100), (8, 60, 80, 100)]);
    p.tracks[G].steps[0].components.push(C::Hold(6));
    p.tracks[G].steps[8].components.push(C::Hold(6));
    pattern(p, G, 2); // drop: dense stabs, one of them an octave up
    notes(&mut p.tracks[G].steps, &[(2, 60, 95, 45), (3, 60, 70, 35), (6, 60, 100, 45), (7, 60, 75, 35), (10, 60, 95, 45), (11, 60, 100, 45), (14, 60, 95, 45), (15, 60, 70, 35)]);
    p.tracks[G].steps[14].components.push(C::Octave(1));
    p.tracks[G].steps[7].components.push(C::Chance(60));
    p.tracks[G].steps[15].components.push(every(2));
    pattern(p, G, 3); // build: stabs that come faster and louder
    notes(&mut p.tracks[G].steps, &[(0, 60, 50, 40), (3, 60, 60, 40), (6, 60, 70, 40), (9, 60, 80, 40), (12, 60, 95, 40), (14, 60, 105, 40), (15, 60, 120, 40)]);
    // ---- E, the pad (the sampler's pad sound, played as a chord): one long chord in every bar; two that breathe in the break ----
    pattern(p, SAMP, 0);
    notes(&mut p.tracks[SAMP].steps, &[(0, 48, 80, 100)]);
    p.tracks[SAMP].steps[0].components.push(C::Hold(15));
    pattern(p, SAMP, 1);
    notes(&mut p.tracks[SAMP].steps, &[(0, 48, 85, 100), (8, 55, 75, 100)]);
    p.tracks[SAMP].steps[0].components.push(C::Hold(7));
    p.tracks[SAMP].steps[8].components.push(C::Hold(7));
    // ---- D, the hook (a plucked string): a short phrase on the notes of the chord, answered in the second bar ----
    pattern(p, D, 0);
    notes(&mut p.tracks[D].steps, &[(0, 72, 95, 60), (3, 67, 80, 50), (6, 64, 85, 50), (8, 69, 90, 60), (11, 72, 85, 50), (14, 67, 80, 50)]);
    notes(&mut p.tracks[D].steps, &[(1, 76, 70, 40), (13, 64, 70, 40)]);
    p.tracks[D].steps[3].components.push(every(1));
    p.tracks[D].steps[6].components.push(every(2));
    p.tracks[D].steps[1].components.push(every(2));
    p.tracks[D].steps[13].components.push(every(1));
    pattern(p, D, 1); // break: slow, high and sparse
    notes(&mut p.tracks[D].steps, &[(0, 76, 80, 80), (6, 72, 70, 60), (10, 76, 75, 60), (14, 79, 65, 60)]);
    p.tracks[D].steps[14].components.push(every(2));
    pattern(p, D, 2); // drop: the phrase twice as busy
    notes(&mut p.tracks[D].steps, &[(0, 72, 100, 50), (2, 76, 75, 40), (3, 67, 85, 45), (6, 64, 90, 45), (8, 69, 95, 50), (10, 72, 80, 40), (11, 76, 90, 45), (14, 67, 85, 45), (15, 72, 70, 35)]);
    p.tracks[D].steps[2].components.push(every(1));
    p.tracks[D].steps[10].components.push(every(2));
    p.tracks[D].steps[15].components.push(C::Ratchet(2));
    // ---- F, the cloud (the granular): low and dark, a long grain; two of them three quarters of a bar long, so that it drifts; the same, denser ----
    pattern(p, F, 0);
    p.tracks[F].brain_link = true;
    p.tracks[F].steps[0] = Step::on(48).with(C::Hold(14));
    p.tracks[F].fx = TrackFx { kind: FxKind::Reverb, a: 180, b: 170, c: 90 };
    pattern(p, F, 1);
    p.tracks[F].length = 12;
    put(&mut p.tracks[F].steps, &[(0, 43), (6, 48)]);
    p.tracks[F].steps[0].components.push(C::Hold(5));
    p.tracks[F].steps[6].components.push(C::Hold(5));
    pattern(p, F, 2);
    p.tracks[F].length = 12;
    put(&mut p.tracks[F].steps, &[(0, 43), (6, 48)]);
    p.tracks[F].steps[0].components.push(C::Hold(5));
    p.tracks[F].steps[6].components.push(C::Hold(5));
    lock(&mut p.tracks[F].steps[0], 3, 200); // DENSITY
    lock(&mut p.tracks[F].steps[6], 3, 250);
    // ---- the scenes: the patterns of A to H, the tracks that are silent and the chords. Every chord is a minor seventh (the organ and the pad
    // stack one on the note that sounds), so the harmony moves between Am7, Dm7 and Em7 ----
    let h = |degrees: &[u8]| brain(9, Scale::Minor, ChordKind::Seventh, degrees);
    let vamp = h(&[0, 3]); // Am7 and Dm7, a bar each: the chord of Chicago
    let song = h(&[0, 3, 0, 4]); // and with the Em7 that pulls back to the Am7
    //                      A  B  C  D  E  F  G  H
    scene(p, 0, [1, 0, 0, 0, 0, 0, 0, 0], &[CC, D, G, 7], vamp); // intro: hats, kick, pad and the cloud
    scene(p, 1, [0, 0, 0, 0, 0, 0, 0, 0], &[D, G, F, 7], vamp); // groove: the bass comes in
    scene(p, 2, [2, 0, 0, 0, 0, 0, 0, 0], &[F, 7], song); // main: the organ and the hook
    scene(p, 3, [3, 0, 0, 0, 0, 0, 0, 0], &[D, G, F, 7], vamp); // a fill: one bar of snares, without the organ and the hook
    scene(p, 4, [4, 0, 0, 1, 1, 1, 1, 0], &[B, CC, 7], h(&[3, 4])); // break: no kick, no bass; long chords, Dm7 and Em7
    scene(p, 5, [5, 1, 0, 1, 1, 1, 3, 0], &[CC, 7], h(&[4])); // build, one: half-time kick, claps, on one chord
    scene(p, 6, [6, 2, 0, 1, 1, 2, 3, 0], &[CC, 7], h(&[4])); // build, two: a kick roll, claps on every sixteenth, organ that speeds up
    scene(p, 7, [0, 0, 0, 0, 0, 0, 0, 0], &[A, B, CC, D, G, 7], h(&[0])); // a bar of breath: only the pad and the cloud ring
    scene(p, 8, [7, 0, 1, 2, 0, 1, 2, 0], &[7], song); // drop: everything, a busier bass
    scene(p, 9, [8, 0, 2, 2, 0, 1, 2, 0], &[7], song); // the drop again: another bass line
    scene(p, 10, [1, 0, 0, 0, 0, 0, 0, 0], &[CC, D, G, 7], vamp); // outro: it thins out
    // ---- the song, and back to the groove as the piece loads ----
    let steps = [(0, 8), (1, 8), (2, 8), (3, 1), (4, 8), (5, 4), (6, 4), (7, 1), (8, 8), (3, 1), (9, 8), (10, 8)];
    for (i, (scene, bars)) in steps.into_iter().enumerate() {
        p.apply(&Command::SetSongStep { index: i as u8, scene, bars });
    }
    p.apply(&Command::SetSongOn(true));
    for t in 0..8 {
        pattern(p, t, 0);
        p.tracks[t].muted = false;
    }
    p.brain = own;
}

/// Techno as a whole melodic techno song of about two and a half minutes (74 bars at 124), in D minor: intro, groove, the arpeggio comes
/// in, a first peak, a breakdown without the kick, a build in two parts, a bar of breath, the drop, a fill and the outro. It is made as the
/// genre is: a kick on every beat, tuned to the fifth of the key, and air that follows the chord; a rolling bass on the three sixteenths between the kicks; a
/// sixteenth arpeggio (the macro's analog voice) whose filter stays closed at first and opens in the build; a hook on a plucked string
/// that only comes in the breakdown and the drop; a pad on open chords (sus2, which fit every chord of the key); hats with accents, a dry
/// clap, rims on a 3-3-2 and snare rolls. The chords are Dm, Bb, F and C: the arpeggio and the hook play single notes that the Brain
/// puts on the chord, so every chord sounds as it should.
fn techno_song(p: &mut Project) {
    const F: usize = 5;
    const G: usize = MACRO;
    p.bpm = 124.0;
    p.swing = 0.5;
    p.brain = brain(2, Scale::Minor, ChordKind::Triad, &[0]);
    let every = |n: u8| C::Every { n, of: 2 };
    // ---- who plays what ----
    p.tracks[CC].brain_link = true;
    p.tracks[CC].lfo = Lfo { shape: LfoShape::Triangle, rate: 40, depth: 70, target: 3 }; // the bass's tone moves slowly (INDEX)
    p.tracks[D].brain_link = true;
    p.tracks[D].fx = TrackFx { kind: FxKind::Delay, a: 170, b: 120, c: 110 };
    let pad = &mut p.tracks[SAMP];
    pad.brain_link = true;
    pad.players = Players { mode: PlayMode::Chord, chord: ChordType::Sus2, speed: 0, octaves: 1 };
    pad.fx = TrackFx { kind: FxKind::Reverb, a: 200, b: 170, c: 90 };
    p.tracks[F].brain_link = true;
    p.tracks[F].fx = TrackFx { kind: FxKind::Reverb, a: 190, b: 120, c: 100 };
    p.tracks[G].brain_link = true;
    p.tracks[G].fx = TrackFx { kind: FxKind::Delay, a: 150, b: 100, c: 90 };
    // ---- A, the kit ----
    let main_hats = |p: &mut Project| {
        for s in 0..16 {
            let v = [70, 42, 85, 58][s % 4];
            hits(&mut p.tracks[A].steps, &[(s, if s % 4 == 2 { OHAT } else { CHAT }, v)]);
        }
    };
    pattern(p, A, 0); // groove: hats, a clap on 2 and 4, rims on a 3-3-2
    p.tracks[A].kit = true;
    main_hats(p);
    hits(&mut p.tracks[A].steps, &[(4, CLAP, 100), (12, CLAP, 100), (3, RIM, 60), (11, RIM, 55), (15, RIM, 50)]);
    p.tracks[A].steps[15].components.push(every(2));
    pattern(p, A, 1); // intro: only quiet hats
    p.tracks[A].kit = true;
    for s in 0..16 {
        hits(&mut p.tracks[A].steps, &[(s, if s % 4 == 2 { OHAT } else { CHAT }, [45, 30, 60, 38][s % 4])]);
    }
    pattern(p, A, 2); // breakdown: a shaker of ghost hats and two rims
    p.tracks[A].kit = true;
    for s in 0..16 {
        hits(&mut p.tracks[A].steps, &[(s, CHAT, [40, 25, 35, 28][s % 4])]);
        if s % 2 == 1 {
            p.tracks[A].steps[s].components.push(C::Chance(70));
        }
    }
    hits(&mut p.tracks[A].steps, &[(6, RIM, 45), (14, RIM, 40)]);
    pattern(p, A, 3); // build, one: a snare on every beat, louder each time, and the hats
    p.tracks[A].kit = true;
    main_hats(p);
    hits(&mut p.tracks[A].steps, &[(0, SNARE, 60), (4, SNARE, 75), (8, SNARE, 90), (12, SNARE, 105)]);
    pattern(p, A, 4); // build, two: a snare on every sixteenth, from soft to loud
    p.tracks[A].kit = true;
    for s in 0..16 {
        hits(&mut p.tracks[A].steps, &[(s, SNARE, 40 + 5 * s as u8)]);
    }
    hits(&mut p.tracks[A].steps, &[(2, OHAT, 80), (6, OHAT, 85), (10, OHAT, 90), (14, OHAT, 100)]);
    pattern(p, A, 5); // drop: a crash, the groove, a tom that answers every second bar
    p.tracks[A].kit = true;
    main_hats(p);
    hits(&mut p.tracks[A].steps, &[(0, CRASH, 110), (4, CLAP, 105), (12, CLAP, 105), (3, RIM, 65), (6, RIM, 55), (11, RIM, 60), (13, TOM, 60)]);
    p.tracks[A].steps[13].components.push(every(2));
    pattern(p, A, 6); // fill: half a bar of groove, then toms and snares
    p.tracks[A].kit = true;
    hits(&mut p.tracks[A].steps, &[(0, CHAT, 70), (2, OHAT, 85), (4, CLAP, 100), (6, OHAT, 85), (8, TOM, 80), (10, TOM, 85), (12, SNARE, 95), (13, SNARE, 100), (14, SNARE, 110), (15, SNARE, 120)]);
    // ---- B, the kick: on every beat ----
    pattern(p, B, 0);
    notes(&mut p.tracks[B].steps, &[(0, 36, 120, 50), (4, 36, 112, 50), (8, 36, 116, 50), (12, 36, 112, 50)]);
    // ---- C, the bass: the three sixteenths between the kicks, the middle one strongest ----
    pattern(p, CC, 0);
    for s in (0..16).filter(|s| s % 4 != 0) {
        notes(&mut p.tracks[CC].steps, &[(s, 36, [0, 72, 100, 82][s % 4], 30)]);
    }
    pattern(p, CC, 1); // drop: the same, with the octave and the fifth in places
    for s in (0..16).filter(|s| s % 4 != 0) {
        notes(&mut p.tracks[CC].steps, &[(s, 36, [0, 75, 105, 85][s % 4], 30)]);
    }
    notes(&mut p.tracks[CC].steps, &[(7, 48, 85, 25), (14, 43, 100, 30), (15, 48, 80, 25)]);
    p.tracks[CC].steps[7].components.push(every(2));
    p.tracks[CC].steps[15].components.push(every(1));
    // ---- G, the arpeggio: sixteen notes that the Brain puts on the chord, accents every three ----
    let arp = [60, 67, 72, 64, 67, 72, 76, 67, 60, 67, 72, 64, 79, 76, 72, 67];
    let arp_steps = |p: &mut Project| {
        for (s, &n) in arp.iter().enumerate() {
            notes(&mut p.tracks[G].steps, &[(s, n, if s % 3 == 0 { 110 } else { 72 }, 45)]);
        }
    };
    pattern(p, G, 0); // its filter as the knob leaves it: closed, dull
    arp_steps(p);
    pattern(p, G, 1); // the build: the filter opens step by step (TIMBRE locks)
    arp_steps(p);
    for s in 0..16 {
        lock(&mut p.tracks[G].steps[s], 2, 80 + 10 * s as u8);
    }
    pattern(p, G, 2); // the peak and the drop: open
    arp_steps(p);
    for s in 0..16 {
        lock(&mut p.tracks[G].steps[s], 2, if s % 3 == 0 { 235 } else { 200 });
    }
    // ---- D, the hook: a long phrase on a plucked string, for the breakdown and the drop ----
    pattern(p, D, 0); // drop
    notes(&mut p.tracks[D].steps, &[(0, 79, 100, 90), (3, 76, 85, 60), (6, 72, 90, 80), (8, 74, 80, 50), (10, 76, 95, 80), (13, 79, 85, 50), (15, 81, 70, 40)]);
    p.tracks[D].steps[0].components.push(C::Hold(2));
    p.tracks[D].steps[10].components.push(C::Hold(2));
    p.tracks[D].steps[15].components.push(every(2));
    pattern(p, D, 1); // breakdown: slower, the same shape
    notes(&mut p.tracks[D].steps, &[(0, 76, 90, 100), (6, 79, 80, 70), (8, 72, 85, 100), (14, 74, 70, 60)]);
    p.tracks[D].steps[0].components.push(C::Hold(5));
    p.tracks[D].steps[8].components.push(C::Hold(5));
    // ---- E, the pad: an open chord held for the bar; in the build, on every eighth (a gate) and louder ----
    pattern(p, SAMP, 0);
    notes(&mut p.tracks[SAMP].steps, &[(0, 48, 85, 100)]);
    p.tracks[SAMP].steps[0].components.push(C::Hold(15));
    pattern(p, SAMP, 1);
    for s in (0..16).step_by(2) {
        notes(&mut p.tracks[SAMP].steps, &[(s, 48, 60 + 4 * s as u8, 40)]);
    }
    // ---- F, the air: a soft tone on the chord, held, in a room; denser in the build ----
    pattern(p, F, 0);
    notes(&mut p.tracks[F].steps, &[(0, 60, 100, 100)]);
    p.tracks[F].steps[0].components.push(C::Hold(15));
    pattern(p, F, 1);
    notes(&mut p.tracks[F].steps, &[(0, 60, 100, 100), (8, 67, 110, 100)]);
    p.tracks[F].steps[0].components.push(C::Hold(7));
    p.tracks[F].steps[8].components.push(C::Hold(7));
    lock(&mut p.tracks[F].steps[0], 3, 170); // DENSITY
    lock(&mut p.tracks[F].steps[8], 3, 240);
    // ---- the scenes ----
    let h = |degrees: &[u8]| brain(2, Scale::Minor, ChordKind::Triad, degrees);
    let full = h(&[0, 5, 2, 6]); // Dm, Bb, F, C
    //                      A  B  C  D  E  F  G  H
    scene(p, 0, [1, 0, 0, 0, 0, 0, 0, 0], &[CC, D, SAMP, G, 7], h(&[0])); // intro: hats, kick and the air
    scene(p, 1, [0, 0, 0, 0, 0, 0, 0, 0], &[D, SAMP, G, 7], h(&[0])); // groove: the bass rolls in, the clap and the rims
    scene(p, 2, [0, 0, 0, 0, 0, 0, 0, 0], &[D, SAMP, 7], h(&[0, 5])); // the arpeggio comes in, closed
    scene(p, 3, [0, 0, 1, 0, 0, 0, 2, 0], &[D, 7], full); // a first peak: the arpeggio open, the pad, all the chords
    scene(p, 4, [2, 0, 0, 1, 0, 0, 0, 0], &[B, CC, 7], full); // breakdown: no kick, no bass; the pad, the air, the hook and the arpeggio closed
    scene(p, 5, [3, 0, 0, 1, 1, 1, 1, 0], &[B, CC, 7], h(&[5, 6])); // build, one: snares, the pad on eighths, the filter opens
    scene(p, 6, [4, 0, 0, 1, 1, 1, 1, 0], &[B, CC, 7], h(&[6])); // build, two: a snare roll on the last chord
    scene(p, 7, [0, 0, 0, 1, 0, 0, 0, 0], &[A, B, CC, F, G, 7], h(&[0])); // a bar of breath: the pad and the hook ring
    scene(p, 8, [5, 0, 1, 0, 0, 0, 2, 0], &[7], full); // drop: everything
    scene(p, 9, [6, 0, 1, 0, 0, 0, 2, 0], &[7], full); // a fill
    scene(p, 10, [0, 0, 0, 0, 0, 0, 0, 0], &[D, SAMP, 7], h(&[0])); // outro: the arpeggio closed, it thins out
    let steps = [(0, 8), (1, 8), (2, 8), (3, 8), (4, 8), (5, 4), (6, 4), (7, 1), (8, 16), (9, 1), (10, 8)];
    for (i, (scene, bars)) in steps.into_iter().enumerate() {
        p.apply(&Command::SetSongStep { index: i as u8, scene, bars });
    }
    p.apply(&Command::SetSongOn(true));
    for t in 0..8 {
        pattern(p, t, 0);
        p.tracks[t].muted = false;
    }
    p.brain = h(&[0]);
}

/// The song steps of a piece, and the song followed from the start, the tracks back on their first pattern with nothing muted and the
/// harmony of the first section as the piece loads.
fn finish(p: &mut Project, steps: &[(u8, u8)], first: Brain) {
    for (i, &(scene, bars)) in steps.iter().enumerate() {
        p.apply(&Command::SetSongStep { index: i as u8, scene, bars });
    }
    p.apply(&Command::SetSongOn(true));
    for t in 0..8 {
        pattern(p, t, 0);
        p.tracks[t].muted = false;
    }
    p.brain = first;
}

/// A Brain whose chords each last `bars` bars.
fn slow_brain(key: u8, scale: Scale, kind: ChordKind, degrees: &[u8], bars: u8) -> Brain {
    let mut b = brain(key, scale, kind, degrees);
    for slot in b.progression.iter_mut() {
        slot.length = 16 * bars;
    }
    b
}

/// Garage as a UK garage (2-step) song of about two minutes (68 bars at 132), in F minor, with a heavy shuffle. The kick leaves the
/// third beat (one, and the "and" of three: the skip of two-step), a snare with a clap on two and four, hats on the off-beats with ghost
/// sixteenths that the swing drags; a bass that bounces between the root and its octave; organ stabs (the macro's phase distortion) on a
/// minor seventh; chopped vocals (the sampler's voice, single notes that the Brain puts on the chord); plucks on the off-beats in the
/// last hook, and a breath from the granular's bells. The chords are Fm7, Bbm7 and Cm7.
fn garage_song(p: &mut Project) {
    const F: usize = 5;
    const G: usize = MACRO;
    p.bpm = 132.0;
    p.swing = 0.62;
    let h = |d: &[u8]| brain(5, Scale::Minor, ChordKind::Seventh, d);
    let every = |n: u8| C::Every { n, of: 2 };
    p.tracks[CC].brain_link = true;
    p.tracks[D].brain_link = true;
    p.tracks[D].fx = TrackFx { kind: FxKind::Delay, a: 120, b: 90, c: 90 };
    p.tracks[SAMP].brain_link = true;
    p.tracks[SAMP].fx = TrackFx { kind: FxKind::Delay, a: 100, b: 80, c: 100 };
    p.tracks[F].brain_link = true;
    p.tracks[F].fx = TrackFx { kind: FxKind::Reverb, a: 190, b: 150, c: 100 };
    let organ = &mut p.tracks[G];
    organ.brain_link = true;
    organ.players = Players { mode: PlayMode::Chord, chord: ChordType::MinorSeventh, speed: 0, octaves: 1 };
    organ.fx = TrackFx { kind: FxKind::Reverb, a: 120, b: 110, c: 70 };
    // ---- A, the kit ----
    let shuffle = |p: &mut Project, open: bool| {
        hits(&mut p.tracks[A].steps, &[(2, CHAT, 85), (6, CHAT, 75), (10, CHAT, 85), (14, if open { OHAT } else { CHAT }, 80)]);
        hits(&mut p.tracks[A].steps, &[(3, CHAT, 40), (7, CHAT, 45), (11, CHAT, 40), (15, CHAT, 50), (1, CHAT, 30)]);
        p.tracks[A].steps[1].components.push(C::Chance(60));
    };
    pattern(p, A, 0); // verse: two-step
    p.tracks[A].kit = true;
    shuffle(p, false);
    hits(&mut p.tracks[A].steps, &[(4, SNARE | CLAP, 110), (12, SNARE | CLAP, 110), (9, RIM, 50)]);
    p.tracks[A].steps[9].components.push(every(2));
    pattern(p, A, 1); // intro: only the hats
    p.tracks[A].kit = true;
    hits(&mut p.tracks[A].steps, &[(2, CHAT, 60), (6, CHAT, 55), (10, CHAT, 60), (14, CHAT, 55), (3, CHAT, 28), (7, CHAT, 30), (11, CHAT, 28), (15, CHAT, 32)]);
    pattern(p, A, 2); // hook: an open hat, rims that answer
    p.tracks[A].kit = true;
    shuffle(p, true);
    hits(&mut p.tracks[A].steps, &[(4, SNARE | CLAP, 112), (12, SNARE | CLAP, 112), (7, RIM, 55), (13, RIM, 45), (9, RIM, 50)]);
    p.tracks[A].steps[13].components.push(every(1));
    pattern(p, A, 3); // break: a few rims and hats
    p.tracks[A].kit = true;
    hits(&mut p.tracks[A].steps, &[(2, CHAT, 45), (10, CHAT, 45), (7, RIM, 45), (15, RIM, 40)]);
    pattern(p, A, 4); // build: snares on every sixteenth, louder
    p.tracks[A].kit = true;
    for s in 0..16 {
        hits(&mut p.tracks[A].steps, &[(s, SNARE, 45 + 5 * s as u8)]);
    }
    pattern(p, A, 5); // the last hook: a crash and the hook's groove
    p.tracks[A].kit = true;
    shuffle(p, true);
    hits(&mut p.tracks[A].steps, &[(0, CRASH, 100), (4, SNARE | CLAP, 115), (12, SNARE | CLAP, 115), (7, RIM, 55), (9, RIM, 50), (13, TOM, 55)]);
    p.tracks[A].steps[13].components.push(every(2));
    // ---- B, the kick: one and the "and" of three; in the hook one more, every second bar ----
    pattern(p, B, 0);
    notes(&mut p.tracks[B].steps, &[(0, 36, 120, 50), (10, 36, 105, 50)]);
    pattern(p, B, 1);
    notes(&mut p.tracks[B].steps, &[(0, 36, 120, 50), (7, 36, 85, 50), (10, 36, 108, 50)]);
    p.tracks[B].steps[7].components.push(every(2));
    // ---- C, the bass: the root held, the octave and the seventh bouncing ----
    pattern(p, CC, 0);
    notes(&mut p.tracks[CC].steps, &[(0, 36, 110, 90), (3, 36, 75, 40), (6, 48, 95, 35), (10, 36, 105, 80), (13, 43, 85, 40), (14, 46, 70, 30)]);
    p.tracks[CC].steps[0].components.push(C::Hold(1));
    pattern(p, CC, 1); // the hook: busier
    notes(&mut p.tracks[CC].steps, &[(0, 36, 115, 70), (2, 48, 80, 30), (3, 36, 85, 40), (6, 48, 100, 35), (8, 36, 70, 30), (10, 36, 110, 70), (11, 48, 75, 30), (13, 43, 90, 40), (15, 46, 75, 30)]);
    // ---- G, the organ: stabs on a skipping rhythm; long chords for the break ----
    pattern(p, G, 0);
    notes(&mut p.tracks[G].steps, &[(2, 60, 95, 35), (5, 60, 80, 30), (10, 60, 95, 35), (13, 60, 85, 30), (15, 60, 65, 25)]);
    p.tracks[G].steps[15].components.push(every(2));
    pattern(p, G, 1);
    notes(&mut p.tracks[G].steps, &[(0, 60, 85, 100), (8, 60, 75, 100)]);
    p.tracks[G].steps[0].components.push(C::Hold(6));
    p.tracks[G].steps[8].components.push(C::Hold(6));
    // ---- E, the vocal chops: short notes on the chord, a stutter, a little late ones ----
    pattern(p, SAMP, 0);
    notes(&mut p.tracks[SAMP].steps, &[(2, 72, 100, 30), (3, 72, 75, 25), (6, 67, 90, 35), (9, 75, 80, 30), (11, 72, 95, 30), (14, 79, 85, 30)]);
    p.tracks[SAMP].steps[3].components.push(C::Ratchet(2));
    p.tracks[SAMP].steps[9].components.push(C::Delay(6));
    p.tracks[SAMP].steps[14].components.push(every(2));
    pattern(p, SAMP, 1); // the break: long voices
    notes(&mut p.tracks[SAMP].steps, &[(0, 72, 85, 100), (8, 79, 75, 100)]);
    p.tracks[SAMP].steps[0].components.push(C::Hold(6));
    p.tracks[SAMP].steps[8].components.push(C::Hold(6));
    // ---- D, plucks on the off-beats (the last hook) ----
    pattern(p, D, 0);
    notes(&mut p.tracks[D].steps, &[(2, 72, 80, 40), (6, 76, 70, 40), (10, 72, 80, 40), (14, 79, 75, 40), (11, 84, 60, 30)]);
    p.tracks[D].steps[11].components.push(every(2));
    // ---- F, a breath: the granular's bells, held ----
    pattern(p, F, 0);
    notes(&mut p.tracks[F].steps, &[(0, 60, 90, 100)]);
    p.tracks[F].steps[0].components.push(C::Hold(15));
    // ---- the scenes ----
    let verse = h(&[0, 4]);
    let hook = h(&[0, 3, 4, 3]);
    //                      A  B  C  D  E  F  G  H
    scene(p, 0, [1, 0, 0, 0, 0, 0, 0, 0], &[B, CC, D, SAMP, 7], verse); // intro: hats, the organ and the breath
    scene(p, 1, [0, 0, 0, 0, 0, 0, 0, 0], &[D, SAMP, F, 7], verse); // verse: two-step, the bass and the organ
    scene(p, 2, [2, 1, 1, 0, 0, 0, 0, 0], &[D, F, 7], hook); // hook: the vocal chops, a busier bass
    scene(p, 3, [0, 0, 0, 0, 0, 0, 0, 0], &[D, F, 7], verse); // the verse again, with the chops
    scene(p, 4, [3, 0, 0, 0, 1, 0, 1, 0], &[B, CC, D, 7], h(&[3, 4])); // break: no kick, no bass; long chords and voices
    scene(p, 5, [4, 0, 0, 0, 1, 0, 1, 0], &[B, CC, D, 7], h(&[4])); // build: a snare roll
    scene(p, 6, [5, 1, 1, 0, 0, 0, 0, 0], &[7], hook); // the last hook: everything
    scene(p, 7, [1, 0, 0, 0, 0, 0, 0, 0], &[CC, D, SAMP, 7], verse); // outro
    finish(p, &[(0, 8), (1, 8), (2, 8), (3, 8), (4, 8), (5, 4), (6, 16), (7, 8)], verse);
}

/// Dnb as a liquid drum & bass song of about two and a quarter minutes (97 bars at 174), in E minor. A two-step break (the kick on one
/// and the "and" of three, the snare on two and four with ghost notes between), hats on eighths and a shuffle under them; a reese bass
/// (the bass's growl with an LFO on its index) that holds the roots; long open pads (sus2) on the macro's analog voice; a melody on a
/// plucked string thrown into an echo; voices from the sampler in the break, and shimmer from the granular. The chords are Em, C, G
/// and D, as open fifths that fit them all.
fn dnb_song(p: &mut Project) {
    const F: usize = 5;
    const G: usize = MACRO;
    p.bpm = 174.0;
    p.swing = 0.5;
    let h = |d: &[u8]| brain(4, Scale::Minor, ChordKind::Triad, d);
    let every = |n: u8| C::Every { n, of: 2 };
    let bass = &mut p.tracks[CC];
    bass.brain_link = true;
    bass.lfo = Lfo { shape: LfoShape::Triangle, rate: 70, depth: 110, target: 3 };
    p.tracks[D].brain_link = true;
    p.tracks[D].fx = TrackFx { kind: FxKind::Delay, a: 170, b: 130, c: 120 };
    p.tracks[SAMP].brain_link = true;
    p.tracks[SAMP].fx = TrackFx { kind: FxKind::Reverb, a: 200, b: 160, c: 100 };
    p.tracks[F].brain_link = true;
    p.tracks[F].fx = TrackFx { kind: FxKind::Reverb, a: 210, b: 170, c: 110 };
    let pad = &mut p.tracks[G];
    pad.brain_link = true;
    pad.players = Players { mode: PlayMode::Chord, chord: ChordType::Sus2, speed: 0, octaves: 1 };
    pad.fx = TrackFx { kind: FxKind::Chorus, a: 80, b: 140, c: 128 };
    // ---- A, the kit ----
    pattern(p, A, 0); // the break
    p.tracks[A].kit = true;
    for s in (0..16).step_by(2) {
        hits(&mut p.tracks[A].steps, &[(s, CHAT, if s % 4 == 0 { 80 } else { 60 })]);
    }
    hits(&mut p.tracks[A].steps, &[(4, SNARE, 120), (12, SNARE, 118), (7, SNARE, 40), (9, SNARE, 35), (15, SNARE, 45), (13, CHAT, 35), (3, CHAT, 30)]);
    p.tracks[A].steps[9].components.push(every(2));
    pattern(p, A, 1); // intro: a ride of hats
    p.tracks[A].kit = true;
    for s in 0..16 {
        hits(&mut p.tracks[A].steps, &[(s, CHAT, [55, 25, 45, 30][s % 4])]);
    }
    pattern(p, A, 2); // the drop: the break with an open hat and a crash
    p.tracks[A].kit = true;
    for s in (0..16).step_by(2) {
        hits(&mut p.tracks[A].steps, &[(s, CHAT, if s % 4 == 0 { 85 } else { 62 })]);
    }
    hits(&mut p.tracks[A].steps, &[(0, CRASH, 95), (4, SNARE, 122), (12, SNARE, 120), (7, SNARE, 42), (10, OHAT, 70), (15, SNARE, 48), (13, CHAT, 35), (3, CHAT, 30)]);
    p.tracks[A].steps[0].components.push(C::Every { n: 1, of: 4 });
    pattern(p, A, 3); // build: snares on every sixteenth
    p.tracks[A].kit = true;
    for s in 0..16 {
        hits(&mut p.tracks[A].steps, &[(s, SNARE, 40 + 5 * s as u8)]);
    }
    pattern(p, A, 4); // break: a soft ride
    p.tracks[A].kit = true;
    for s in (0..16).step_by(2) {
        hits(&mut p.tracks[A].steps, &[(s, CHAT, 35)]);
    }
    hits(&mut p.tracks[A].steps, &[(12, RIM, 40)]);
    // ---- B, the kick: one and the "and" of three ----
    pattern(p, B, 0);
    notes(&mut p.tracks[B].steps, &[(0, 36, 120, 50), (10, 36, 110, 50)]);
    pattern(p, B, 1); // the drop: one more, every second bar
    notes(&mut p.tracks[B].steps, &[(0, 36, 120, 50), (10, 36, 110, 50), (11, 36, 80, 50)]);
    p.tracks[B].steps[11].components.push(every(2));
    // ---- C, the reese: the root held, a move at the end of the bar ----
    pattern(p, CC, 0);
    notes(&mut p.tracks[CC].steps, &[(0, 36, 110, 100), (10, 36, 100, 100), (14, 43, 90, 60)]);
    p.tracks[CC].steps[0].components.push(C::Hold(8));
    p.tracks[CC].steps[10].components.push(C::Hold(3));
    // ---- G, the pad: an open chord for the bar ----
    pattern(p, G, 0);
    notes(&mut p.tracks[G].steps, &[(0, 60, 85, 100)]);
    p.tracks[G].steps[0].components.push(C::Hold(15));
    // ---- D, the melody: a phrase that answers itself every second bar ----
    pattern(p, D, 0);
    notes(&mut p.tracks[D].steps, &[(0, 79, 95, 60), (3, 76, 80, 50), (6, 72, 85, 60), (10, 74, 75, 50), (12, 76, 90, 60), (14, 79, 70, 40)]);
    p.tracks[D].steps[3].components.push(every(1));
    p.tracks[D].steps[10].components.push(every(2));
    pattern(p, D, 1); // the intro and the break: slow
    notes(&mut p.tracks[D].steps, &[(0, 76, 80, 100), (8, 79, 70, 100)]);
    p.tracks[D].steps[0].components.push(C::Hold(5));
    p.tracks[D].steps[8].components.push(C::Hold(5));
    // ---- E, voices: two long notes ----
    pattern(p, SAMP, 0);
    notes(&mut p.tracks[SAMP].steps, &[(0, 72, 80, 100), (8, 76, 70, 100)]);
    p.tracks[SAMP].steps[0].components.push(C::Hold(7));
    p.tracks[SAMP].steps[8].components.push(C::Hold(7));
    // ---- F, shimmer: the granular's bells, high, held ----
    pattern(p, F, 0);
    notes(&mut p.tracks[F].steps, &[(0, 72, 85, 100)]);
    p.tracks[F].steps[0].components.push(C::Hold(15));
    // ---- the scenes ----
    let prog = h(&[0, 5, 2, 6]); // Em, C, G, D
    //                      A  B  C  D  E  F  G  H
    scene(p, 0, [1, 0, 0, 1, 0, 0, 0, 0], &[B, CC, SAMP, 7], prog); // intro: the pad, the shimmer, a ride and the slow melody
    scene(p, 1, [0, 0, 0, 1, 0, 0, 0, 0], &[CC, SAMP, 7], prog); // the break rolls in, without the bass
    scene(p, 2, [0, 0, 0, 0, 0, 0, 0, 0], &[A, B, CC, D, SAMP, 7], prog); // a bar of breath: the pad and the shimmer
    scene(p, 3, [2, 1, 0, 0, 0, 0, 0, 0], &[SAMP, 7], prog); // the drop: the reese, the melody
    scene(p, 4, [4, 0, 0, 1, 0, 0, 0, 0], &[B, CC, 7], prog); // break: the voices, the slow melody, a soft ride
    scene(p, 5, [3, 0, 0, 1, 0, 0, 0, 0], &[B, CC, 7], h(&[5, 6])); // build: a snare roll
    scene(p, 6, [2, 1, 0, 0, 0, 0, 0, 0], &[7], prog); // the drop again, with the voices
    scene(p, 7, [1, 0, 0, 1, 0, 0, 0, 0], &[B, CC, SAMP, 7], prog); // outro
    finish(p, &[(0, 16), (1, 8), (2, 1), (3, 24), (4, 16), (5, 8), (6, 16), (7, 8)], prog);
}

/// Trance as an uplifting trance song of about two and a half minutes (81 bars at 138), in A minor: Am, F, C and G, the progression of
/// the anthems. A kick on every beat, a galloping bass (the off-beat and the sixteenth after it), a sixteenth arpeggio on a plucked
/// string, a lead on the macro's swarm (a voice of many detuned saws, the closest the machine has to a supersaw) played in fifths and
/// octaves, a pad that is gated on every sixteenth (the trance gate) on the sampler's bell, and air from the granular's bells. A long
/// breakdown where the lead is heard for the first time, slowly, over the pad, a build with a snare that goes from eighths to sixteenths, a bar of breath and
/// the drop. Every melody is single notes that the Brain puts on the chord; the pad plays open chords (sus2) that fit all four.
fn trance_song(p: &mut Project) {
    const F: usize = 5;
    const G: usize = MACRO;
    p.bpm = 138.0;
    p.swing = 0.5;
    let h = |d: &[u8]| brain(9, Scale::Minor, ChordKind::Triad, d);
    let every = |n: u8| C::Every { n, of: 2 };
    p.tracks[CC].brain_link = true;
    p.tracks[D].brain_link = true;
    p.tracks[D].fx = TrackFx { kind: FxKind::Delay, a: 160, b: 110, c: 100 };
    let pad = &mut p.tracks[SAMP];
    pad.brain_link = true;
    pad.players = Players { mode: PlayMode::Chord, chord: ChordType::Sus2, speed: 0, octaves: 1 };
    pad.fx = TrackFx { kind: FxKind::Reverb, a: 200, b: 170, c: 100 };
    p.tracks[F].brain_link = true;
    p.tracks[F].fx = TrackFx { kind: FxKind::Reverb, a: 210, b: 170, c: 110 };
    let lead = &mut p.tracks[G];
    lead.brain_link = true;
    lead.players = Players { mode: PlayMode::Chord, chord: ChordType::Power, speed: 0, octaves: 1 };
    lead.fx = TrackFx { kind: FxKind::Delay, a: 170, b: 120, c: 110 };
    // ---- A, the kit ----
    let groove = |p: &mut Project| {
        for s in 0..16 {
            hits(&mut p.tracks[A].steps, &[(s, if s % 4 == 2 { OHAT } else { CHAT }, [60, 35, 90, 45][s % 4])]);
        }
        hits(&mut p.tracks[A].steps, &[(4, CLAP, 105), (12, CLAP, 105)]);
    };
    pattern(p, A, 0); // the groove
    p.tracks[A].kit = true;
    groove(p);
    pattern(p, A, 1); // intro: only the open hats
    p.tracks[A].kit = true;
    hits(&mut p.tracks[A].steps, &[(2, OHAT, 70), (6, OHAT, 70), (10, OHAT, 70), (14, OHAT, 75)]);
    pattern(p, A, 2); // breakdown: nothing but a soft ride on the beats
    p.tracks[A].kit = true;
    hits(&mut p.tracks[A].steps, &[(0, CHAT, 30), (4, CHAT, 30), (8, CHAT, 30), (12, CHAT, 30)]);
    pattern(p, A, 3); // build: a snare on every eighth, louder
    p.tracks[A].kit = true;
    for s in (0..16).step_by(2) {
        hits(&mut p.tracks[A].steps, &[(s, SNARE, 50 + 7 * s as u8)]);
    }
    pattern(p, A, 4); // build, end: on every sixteenth, with a stutter at the end
    p.tracks[A].kit = true;
    for s in 0..16 {
        hits(&mut p.tracks[A].steps, &[(s, SNARE, 60 + 4 * s as u8)]);
    }
    p.tracks[A].steps[14].components.push(C::Ratchet(2));
    p.tracks[A].steps[15].components.push(C::Ratchet(2));
    pattern(p, A, 5); // drop: a crash
    p.tracks[A].kit = true;
    groove(p);
    hits(&mut p.tracks[A].steps, &[(0, CRASH, 110)]);
    p.tracks[A].steps[0].components.push(C::Every { n: 1, of: 4 });
    // ---- B, the kick ----
    pattern(p, B, 0);
    notes(&mut p.tracks[B].steps, &[(0, 36, 120, 50), (4, 36, 115, 50), (8, 36, 118, 50), (12, 36, 115, 50)]);
    // ---- C, the bass: the gallop ----
    pattern(p, CC, 0);
    for s in [2, 3, 6, 7, 10, 11, 14, 15] {
        notes(&mut p.tracks[CC].steps, &[(s, 36, if s % 2 == 0 { 105 } else { 75 }, 35)]);
    }
    notes(&mut p.tracks[CC].steps, &[(15, 48, 80, 30)]);
    p.tracks[CC].steps[15].components.push(every(2));
    // ---- D, the arpeggio: sixteenths on the chord ----
    let arp = [60, 64, 67, 72, 76, 72, 67, 64, 60, 64, 67, 72, 79, 76, 72, 67];
    pattern(p, D, 0);
    for (s, &n) in arp.iter().enumerate() {
        notes(&mut p.tracks[D].steps, &[(s, n, if s % 4 == 0 { 100 } else { 70 }, 40)]);
    }
    // ---- G, the lead: the melody of the drop, and slow in the breakdown ----
    pattern(p, G, 0); // drop
    notes(&mut p.tracks[G].steps, &[(0, 72, 110, 60), (3, 76, 95, 50), (6, 79, 100, 60), (8, 76, 90, 40), (10, 72, 100, 60), (13, 74, 90, 50), (14, 76, 85, 40)]);
    p.tracks[G].steps[13].components.push(every(2));
    p.tracks[G].steps[14].components.push(every(1));
    pattern(p, G, 1); // breakdown: long notes
    notes(&mut p.tracks[G].steps, &[(0, 76, 85, 100), (8, 72, 80, 100), (12, 74, 70, 80)]);
    p.tracks[G].steps[0].components.push(C::Hold(6));
    p.tracks[G].steps[8].components.push(C::Hold(2));
    // ---- E, the pad: held for the bar; gated on every sixteenth ----
    pattern(p, SAMP, 0);
    notes(&mut p.tracks[SAMP].steps, &[(0, 48, 80, 100)]);
    p.tracks[SAMP].steps[0].components.push(C::Hold(15));
    pattern(p, SAMP, 1);
    for s in 0..16 {
        notes(&mut p.tracks[SAMP].steps, &[(s, 48, if s % 2 == 0 { 85 } else { 55 }, 45)]);
    }
    // ---- F, air: the bells, high, held ----
    pattern(p, F, 0);
    notes(&mut p.tracks[F].steps, &[(0, 72, 85, 100)]);
    p.tracks[F].steps[0].components.push(C::Hold(15));
    // ---- the scenes ----
    let prog = h(&[0, 5, 2, 6]); // Am, F, C, G
    //                      A  B  C  D  E  F  G  H
    scene(p, 0, [1, 0, 0, 0, 0, 0, 0, 0], &[CC, D, SAMP, F, G, 7], h(&[0])); // intro: kick and open hats
    scene(p, 1, [0, 0, 0, 0, 0, 0, 0, 0], &[SAMP, F, G, 7], prog); // the bass gallops in with the arpeggio, the chords move
    scene(p, 2, [0, 0, 0, 0, 0, 0, 0, 0], &[F, G, 7], prog); // main: the pad held; the lead is kept for the breakdown and the drop
    scene(p, 3, [2, 0, 0, 0, 0, 0, 1, 0], &[B, CC, D, 7], prog); // breakdown: the pad, the air and the lead, slow
    scene(p, 4, [3, 0, 0, 0, 1, 0, 1, 0], &[B, CC, 7], prog); // build: the snare on eighths, the arpeggio back
    scene(p, 5, [4, 0, 0, 0, 1, 0, 1, 0], &[B, CC, 7], h(&[6])); // build, end: sixteenths, on G
    scene(p, 6, [0, 0, 0, 0, 0, 0, 1, 0], &[A, B, CC, D, 7], h(&[0])); // a bar of breath
    scene(p, 7, [5, 0, 0, 0, 1, 0, 0, 0], &[7], prog); // drop: everything
    scene(p, 8, [1, 0, 0, 0, 0, 0, 0, 0], &[SAMP, G, 7], prog); // outro
    finish(p, &[(0, 8), (1, 8), (2, 16), (3, 16), (4, 4), (5, 4), (6, 1), (7, 16), (8, 8)], h(&[0]));
}

/// Ambient as a slow piece of about two and a quarter minutes (40 bars at 70, six sections) in F Lydian, the major key with a raised fourth that
/// sounds suspended. The chords last two bars each (F, G, D, C), as open chords (sus2, which fit them all): a pad on the macro's analog
/// voice with a chorus, a cloud from the granular's bells whose position drifts (an LFO), slow notes on the resonator's harp thrown into an echo,
/// a low glass line on the sampler, and a sub that holds the roots. In the middle a heartbeat (a soft kick every other bar) and a few
/// ticks of the kit come and go.
fn ambient_song(p: &mut Project) {
    const F: usize = 5;
    const G: usize = MACRO;
    p.bpm = 70.0;
    p.swing = 0.5;
    let h = |d: &[u8]| slow_brain(5, Scale::Lydian, ChordKind::Triad, d, 2);
    let every = |n: u8, of: u8| C::Every { n, of };
    p.tracks[CC].brain_link = true;
    p.tracks[D].brain_link = true;
    p.tracks[D].fx = TrackFx { kind: FxKind::Delay, a: 200, b: 160, c: 130 };
    p.tracks[SAMP].brain_link = true;
    p.tracks[SAMP].fx = TrackFx { kind: FxKind::Reverb, a: 220, b: 180, c: 120 };
    let cloud = &mut p.tracks[F];
    cloud.brain_link = true;
    cloud.lfo = Lfo { shape: LfoShape::Sine, rate: 30, depth: 120, target: 3 };
    cloud.fx = TrackFx { kind: FxKind::Reverb, a: 230, b: 190, c: 130 };
    let pad = &mut p.tracks[G];
    pad.brain_link = true;
    pad.players = Players { mode: PlayMode::Chord, chord: ChordType::Sus2, speed: 2, octaves: 2 };
    pad.fx = TrackFx { kind: FxKind::Chorus, a: 70, b: 140, c: 128 };
    // ---- G, the pad: a chord held for the bar, strummed slowly ----
    pattern(p, G, 0);
    notes(&mut p.tracks[G].steps, &[(0, 60, 75, 100)]);
    p.tracks[G].steps[0].components.push(C::Hold(15));
    // ---- F, the cloud: held ----
    pattern(p, F, 0);
    notes(&mut p.tracks[F].steps, &[(0, 72, 80, 100)]);
    p.tracks[F].steps[0].components.push(C::Hold(15));
    // ---- D, the harp: a few notes, not every bar ----
    pattern(p, D, 0);
    notes(&mut p.tracks[D].steps, &[(0, 79, 70, 100), (6, 76, 55, 100), (10, 84, 60, 100), (14, 72, 50, 100)]);
    p.tracks[D].steps[6].components.push(C::Chance(60));
    p.tracks[D].steps[10].components.push(every(2, 2));
    p.tracks[D].steps[14].components.push(every(1, 3));
    // ---- E, the glass: a low line, two notes ----
    pattern(p, SAMP, 0);
    notes(&mut p.tracks[SAMP].steps, &[(0, 60, 70, 100), (8, 67, 60, 100)]);
    p.tracks[SAMP].steps[8].components.push(every(2, 2));
    // ---- C, the sub: the root held ----
    pattern(p, CC, 0);
    notes(&mut p.tracks[CC].steps, &[(0, 36, 80, 100)]);
    p.tracks[CC].steps[0].components.push(C::Hold(15));
    // ---- B, the heartbeat: a soft kick, two close together, every other bar ----
    pattern(p, B, 0);
    notes(&mut p.tracks[B].steps, &[(0, 36, 90, 50), (3, 36, 60, 50)]);
    p.tracks[B].steps[0].components.push(every(1, 2));
    p.tracks[B].steps[3].components.push(every(1, 2));
    // ---- A, ticks: a few soft hats and rims ----
    pattern(p, A, 0);
    p.tracks[A].kit = true;
    hits(&mut p.tracks[A].steps, &[(2, CHAT, 35), (7, RIM, 40), (10, CHAT, 30), (13, CHAT, 25), (15, RIM, 30)]);
    p.tracks[A].steps[13].components.push(C::Chance(50));
    p.tracks[A].steps[15].components.push(every(2, 2));
    // ---- the scenes: the chords last two bars each ----
    let prog = h(&[0, 1, 5, 4]); // F, G, D, C
    //                      A  B  C  D  E  F  G  H
    scene(p, 0, [0, 0, 0, 0, 0, 0, 0, 0], &[A, B, CC, D, SAMP, 7], h(&[0, 1])); // dawn: the pad and the cloud
    scene(p, 1, [0, 0, 0, 0, 0, 0, 0, 0], &[A, B, SAMP, 7], prog); // bloom: the bells and the sub
    scene(p, 2, [0, 0, 0, 0, 0, 0, 0, 0], &[7], prog); // pulse: the glass, the heartbeat and the ticks
    scene(p, 3, [0, 0, 0, 0, 0, 0, 0, 0], &[A, B, 7], h(&[5, 4, 0, 1])); // drift: the heartbeat goes, other chords
    scene(p, 4, [0, 0, 0, 0, 0, 0, 0, 0], &[A, B, CC, SAMP, 7], h(&[0, 1])); // dusk: back to the pad, the cloud and the harp
    scene(p, 5, [0, 0, 0, 0, 0, 0, 0, 0], &[A, B, 7], h(&[1, 5])); // glow: the glass and the sub stay, without the heartbeat
    finish(p, &[(0, 8), (1, 8), (2, 8), (3, 6), (5, 6), (4, 4)], h(&[0, 1]));
}

/// Lo-fi as a lo-fi hip hop song of about a minute and three quarters (36 bars at 80), in D minor, with a lazy swing. A dusty kit (a
/// soft kick that lands late in the bar, a dull snare with ghost notes, hats that drag); an electric piano (the macro's `fm`) on minor
/// sevenths that comes in a little after the beat (a delay on its steps), the way a hand plays; a warm sub; a guitar-like phrase on a
/// plucked string; a sung "ah" from the sampler an octave down, and crackle from the granular's glitch. The chords are Dm7, Am7, Gm7
/// and Am7 (Gm7 and Am7 in the break, Dm7 alone at the end).
fn lofi_song(p: &mut Project) {
    const F: usize = 5;
    const G: usize = MACRO;
    p.bpm = 80.0;
    p.swing = 0.62;
    let h = |d: &[u8]| brain(2, Scale::Minor, ChordKind::Seventh, d);
    let every = |n: u8| C::Every { n, of: 2 };
    p.tracks[CC].brain_link = true;
    p.tracks[D].brain_link = true;
    p.tracks[D].fx = TrackFx { kind: FxKind::Delay, a: 140, b: 90, c: 90 };
    p.tracks[SAMP].brain_link = true;
    p.tracks[SAMP].fx = TrackFx { kind: FxKind::Reverb, a: 180, b: 150, c: 110 };
    p.tracks[F].fx = TrackFx { kind: FxKind::Filter, a: 90, b: 60, c: 128 };
    let keys = &mut p.tracks[G];
    keys.brain_link = true;
    keys.players = Players { mode: PlayMode::Chord, chord: ChordType::MinorSeventh, speed: 1, octaves: 1 };
    keys.fx = TrackFx { kind: FxKind::Chorus, a: 60, b: 120, c: 110 };
    // ---- A, the kit ----
    pattern(p, A, 0); // the beat
    p.tracks[A].kit = true;
    hits(&mut p.tracks[A].steps, &[(0, CHAT, 60), (2, CHAT, 45), (4, SNARE, 105), (6, CHAT, 55), (8, CHAT, 60), (10, CHAT, 45), (12, SNARE, 100), (14, CHAT, 55)]);
    hits(&mut p.tracks[A].steps, &[(7, SNARE, 30), (15, SNARE, 35), (11, CHAT, 30), (3, RIM, 35)]);
    p.tracks[A].steps[15].components.push(every(2));
    p.tracks[A].steps[3].components.push(C::Chance(50));
    pattern(p, A, 1); // the second half: an open hat
    p.tracks[A].kit = true;
    hits(&mut p.tracks[A].steps, &[(0, CHAT, 60), (2, CHAT, 45), (4, SNARE, 108), (6, CHAT, 55), (8, CHAT, 60), (10, OHAT, 50), (12, SNARE, 102), (14, CHAT, 55), (9, SNARE, 30), (15, RIM, 40)]);
    // ---- B, the kick: one, a late one, and the "and" of three ----
    pattern(p, B, 0);
    notes(&mut p.tracks[B].steps, &[(0, 36, 115, 50), (7, 36, 80, 50), (10, 36, 100, 50)]);
    p.tracks[B].steps[7].components.push(every(2));
    // ---- C, the sub ----
    pattern(p, CC, 0);
    notes(&mut p.tracks[CC].steps, &[(0, 36, 100, 100), (10, 36, 85, 80)]);
    p.tracks[CC].steps[0].components.push(C::Hold(7));
    p.tracks[CC].steps[10].components.push(C::Hold(3));
    // ---- G, the keys: a chord on one, and again a little after the snare, both late ----
    pattern(p, G, 0);
    notes(&mut p.tracks[G].steps, &[(0, 60, 85, 100), (6, 60, 65, 60), (11, 60, 75, 80)]);
    p.tracks[G].steps[0].components.push(C::Hold(4));
    p.tracks[G].steps[0].components.push(C::Delay(4));
    p.tracks[G].steps[6].components.push(C::Delay(6));
    p.tracks[G].steps[11].components.push(C::Hold(3));
    p.tracks[G].steps[11].components.push(C::Delay(3));
    // ---- D, the guitar: a phrase in the second half of the bar ----
    pattern(p, D, 0);
    notes(&mut p.tracks[D].steps, &[(8, 74, 85, 60), (9, 72, 70, 50), (11, 69, 80, 60), (14, 67, 75, 80)]);
    p.tracks[D].steps[9].components.push(C::Delay(4));
    p.tracks[D].steps[14].components.push(every(2));
    // ---- E, the "ah": one long note ----
    pattern(p, SAMP, 0);
    notes(&mut p.tracks[SAMP].steps, &[(0, 72, 75, 100)]);
    p.tracks[SAMP].steps[0].components.push(C::Hold(11));
    p.tracks[SAMP].steps[0].components.push(every(1));
    // ---- F, crackle: the glitch, small and quiet, all through ----
    pattern(p, F, 0);
    notes(&mut p.tracks[F].steps, &[(0, 60, 70, 100)]);
    p.tracks[F].steps[0].components.push(C::Hold(15));
    // ---- the scenes ----
    let prog = h(&[0, 4, 3, 4]); // Dm7, Am7, Gm7, Am7
    //                      A  B  C  D  E  F  G  H
    scene(p, 0, [0, 0, 0, 0, 0, 0, 0, 0], &[A, B, CC, D, SAMP, 7], prog); // intro: the keys and the crackle
    scene(p, 1, [0, 0, 0, 0, 0, 0, 0, 0], &[D, SAMP, 7], prog); // A: the beat and the sub
    scene(p, 2, [1, 0, 0, 0, 0, 0, 0, 0], &[7], prog); // B: the guitar and the "ah"
    scene(p, 3, [0, 0, 0, 0, 0, 0, 0, 0], &[A, B, CC, 7], h(&[3, 4])); // break: no beat, Gm7 and Am7
    scene(p, 4, [1, 0, 0, 0, 0, 0, 0, 0], &[SAMP, 7], prog); // A again, with the guitar
    scene(p, 5, [0, 0, 0, 0, 0, 0, 0, 0], &[A, B, CC, D, SAMP, 7], h(&[0])); // outro: on Dm7
    finish(p, &[(0, 4), (1, 8), (2, 8), (3, 4), (4, 8), (5, 4)], prog);
}

/// Amapiano as a song of about two and a half minutes (68 bars at 113), in F minor, with a light swing. Its voice is the log drum: the
/// KickSyn tuned low with a long decay, playing a melodic, syncopated line on the notes of the chord. A soft kick on every beat (the
/// kit's own), shakers on every sixteenth with accents, rims and toms that talk to each other, a clap on two and four; a jazzy piano (the
/// macro's `fm`) on minor sevenths that comps across the bar; a marimba (the resonator's nylon string, short) with a phrase; a deep sub under
/// the chord changes, a sung "ah" and a cloud from the granular's bells. The chords are Fm7, Bbm7, Fm7 and Cm7.
fn amapiano_song(p: &mut Project) {
    const F: usize = 5;
    const G: usize = MACRO;
    p.bpm = 113.0;
    p.swing = 0.56;
    let h = |d: &[u8]| brain(5, Scale::Minor, ChordKind::Seventh, d);
    let every = |n: u8| C::Every { n, of: 2 };
    p.tracks[B].brain_link = true; // the log drum plays the chord
    p.tracks[CC].brain_link = true;
    p.tracks[D].brain_link = true;
    p.tracks[D].fx = TrackFx { kind: FxKind::Delay, a: 130, b: 90, c: 90 };
    p.tracks[SAMP].brain_link = true;
    p.tracks[SAMP].fx = TrackFx { kind: FxKind::Reverb, a: 190, b: 150, c: 100 };
    p.tracks[F].brain_link = true;
    p.tracks[F].fx = TrackFx { kind: FxKind::Reverb, a: 200, b: 160, c: 110 };
    let piano = &mut p.tracks[G];
    piano.brain_link = true;
    piano.players = Players { mode: PlayMode::Chord, chord: ChordType::MinorSeventh, speed: 1, octaves: 1 };
    piano.fx = TrackFx { kind: FxKind::Reverb, a: 140, b: 120, c: 80 };
    // ---- A, the kit: its own kick on every beat, shakers, rims, toms, a clap ----
    let shakers = |p: &mut Project| {
        for s in 0..16 {
            hits(&mut p.tracks[A].steps, &[(s, CHAT, [50, 30, 65, 38][s % 4])]);
        }
    };
    pattern(p, A, 0); // the groove
    p.tracks[A].kit = true;
    shakers(p);
    hits(&mut p.tracks[A].steps, &[(0, KICK, 90), (4, KICK | CLAP, 95), (8, KICK, 88), (12, KICK | CLAP, 95), (3, RIM, 60), (6, RIM, 50), (11, TOM, 55), (14, RIM, 55)]);
    p.tracks[A].steps[11].components.push(every(2));
    pattern(p, A, 1); // intro: shakers and the soft kick
    p.tracks[A].kit = true;
    shakers(p);
    hits(&mut p.tracks[A].steps, &[(0, KICK, 80), (4, KICK, 75), (8, KICK, 80), (12, KICK, 75)]);
    pattern(p, A, 2); // the drop: more rims and toms
    p.tracks[A].kit = true;
    shakers(p);
    hits(&mut p.tracks[A].steps, &[(0, KICK, 92), (4, KICK | CLAP, 98), (8, KICK, 90), (12, KICK | CLAP, 98), (3, RIM, 65), (6, RIM, 55), (7, TOM, 50), (10, RIM, 60), (11, TOM, 60), (14, RIM, 60), (15, TOM, 55)]);
    p.tracks[A].steps[7].components.push(every(1));
    p.tracks[A].steps[15].components.push(every(2));
    pattern(p, A, 3); // break: only shakers
    p.tracks[A].kit = true;
    shakers(p);
    pattern(p, A, 4); // build: rims on every sixteenth, louder
    p.tracks[A].kit = true;
    for s in 0..16 {
        hits(&mut p.tracks[A].steps, &[(s, RIM, 40 + 5 * s as u8)]);
    }
    // ---- B, the log drum: a syncopated line on the chord ----
    pattern(p, B, 0);
    notes(&mut p.tracks[B].steps, &[(0, 36, 110, 60), (3, 36, 85, 50), (6, 43, 100, 60), (8, 36, 70, 40), (10, 48, 105, 60), (13, 43, 90, 50), (14, 46, 80, 40)]);
    p.tracks[B].steps[8].components.push(every(2));
    p.tracks[B].steps[14].components.push(every(1));
    pattern(p, B, 1); // the drop: busier, with a roll
    notes(&mut p.tracks[B].steps, &[(0, 36, 115, 60), (2, 36, 70, 40), (3, 36, 90, 50), (6, 43, 105, 60), (9, 48, 85, 50), (10, 48, 110, 60), (12, 43, 80, 40), (13, 43, 95, 50), (15, 36, 85, 40)]);
    p.tracks[B].steps[15].components.push(C::Ratchet(2));
    // ---- C, the sub: under the chord ----
    pattern(p, CC, 0);
    notes(&mut p.tracks[CC].steps, &[(0, 36, 90, 100)]);
    p.tracks[CC].steps[0].components.push(C::Hold(11));
    // ---- G, the piano: chords across the bar ----
    pattern(p, G, 0);
    notes(&mut p.tracks[G].steps, &[(0, 60, 85, 70), (3, 60, 70, 50), (7, 60, 80, 60), (10, 60, 75, 50), (14, 60, 70, 50)]);
    p.tracks[G].steps[3].components.push(every(2));
    p.tracks[G].steps[14].components.push(every(1));
    pattern(p, G, 1); // break: long chords
    notes(&mut p.tracks[G].steps, &[(0, 60, 80, 100), (8, 60, 70, 100)]);
    p.tracks[G].steps[0].components.push(C::Hold(6));
    p.tracks[G].steps[8].components.push(C::Hold(6));
    // ---- D, the marimba: a phrase ----
    pattern(p, D, 0);
    notes(&mut p.tracks[D].steps, &[(1, 72, 80, 40), (4, 76, 70, 40), (6, 79, 85, 40), (9, 76, 70, 40), (11, 72, 80, 40), (12, 74, 60, 30), (14, 79, 75, 40)]);
    p.tracks[D].steps[12].components.push(every(2));
    // ---- E, the "ah" ----
    pattern(p, SAMP, 0);
    notes(&mut p.tracks[SAMP].steps, &[(0, 72, 80, 100), (8, 79, 70, 100)]);
    p.tracks[SAMP].steps[0].components.push(C::Hold(6));
    p.tracks[SAMP].steps[8].components.push(C::Hold(6));
    p.tracks[SAMP].steps[8].components.push(every(2));
    // ---- F, a cloud ----
    pattern(p, F, 0);
    notes(&mut p.tracks[F].steps, &[(0, 60, 85, 100)]);
    p.tracks[F].steps[0].components.push(C::Hold(15));
    // ---- the scenes ----
    let prog = h(&[0, 3, 0, 4]); // Fm7, Bbm7, Fm7, Cm7
    //                      A  B  C  D  E  F  G  H
    scene(p, 0, [1, 0, 0, 0, 0, 0, 0, 0], &[B, CC, D, SAMP, 7], prog); // intro: shakers, the soft kick, the piano and the cloud
    scene(p, 1, [0, 0, 0, 0, 0, 0, 0, 0], &[CC, D, SAMP, F, 7], prog); // groove: the log drum comes in
    scene(p, 2, [0, 0, 0, 0, 0, 0, 0, 0], &[SAMP, F, 7], prog); // main: the sub and the marimba
    scene(p, 3, [3, 0, 0, 0, 0, 0, 1, 0], &[CC, D, 7], prog); // break: the log drum alone with long chords and the "ah"
    scene(p, 4, [4, 0, 0, 0, 0, 0, 1, 0], &[B, CC, D, 7], h(&[4])); // build: rims
    scene(p, 5, [2, 1, 0, 0, 0, 0, 0, 0], &[7], prog); // drop: everything, the log drum busier
    scene(p, 6, [1, 0, 0, 0, 0, 0, 0, 0], &[CC, D, SAMP, 7], prog); // outro
    finish(p, &[(0, 8), (1, 8), (2, 16), (3, 8), (4, 4), (5, 16), (6, 8)], prog);
}

pub fn project(i: usize) -> Project {
    let mut p = Project::default();
    match i {
        // 0 · house: a whole song (see `house_song`); here the settings of the tracks, the patterns are made there
        0 => {
            p.bpm = 122.0;
            p.swing = 0.54;
            p.brain = brain(9, Scale::Minor, ChordKind::Seventh, &[0, 3]);
            p.tracks[CC].brain_link = true;
            p.tracks[D].brain_link = true; // the hook: notes on the chord that sounds
            p.tracks[D].fx = TrackFx { kind: FxKind::Delay, a: 150, b: 90, c: 100 };
            let organ = &mut p.tracks[MACRO];
            organ.brain_link = true;
            organ.players = Players { mode: PlayMode::Chord, chord: ChordType::MinorSeventh, speed: 0, octaves: 1 };
            organ.fx = TrackFx { kind: FxKind::Delay, a: 150, b: 90, c: 90 };
            let pad = &mut p.tracks[SAMP];
            pad.brain_link = true;
            pad.players = Players { mode: PlayMode::Chord, chord: ChordType::MinorSeventh, speed: 0, octaves: 1 };
            pad.fx = TrackFx { kind: FxKind::Reverb, a: 170, b: 150, c: 80 };
            house_song(&mut p);
        }
        // 1 · techno: a melodic techno song (see `techno_song`)
        1 => techno_song(&mut p),
        2 => garage_song(&mut p),
        3 => dnb_song(&mut p),
        4 => trance_song(&mut p),
        5 => amapiano_song(&mut p),
        6 => ambient_song(&mut p),
        _ => lofi_song(&mut p),
    }
    p
}

/// The presets of each piece, by track A to H (`""`: as it comes), and how much of the level of its instruments it keeps.
const PRESETS: [([&str; 8], f32); 8] = [
    (["", "punch", "round", "nylon", "bell", "bells", "", ""], 0.55),
    (["", "punch", "pluck", "harp", "bell", "bells", "", ""], 0.7),
    (["", "tight", "slide", "nylon", "voice", "bells", "", ""], 0.7),
    (["", "punch", "growl", "harp", "voice", "bells", "", ""], 0.7),
    (["", "hard", "round", "nylon", "bell", "bells", "swarm", ""], 0.7),
    (["", "deep", "sub", "nylon", "voice", "bells", "", ""], 0.7),
    (["", "soft", "sub", "harp", "glass", "bells", "", ""], 0.8),
    (["", "soft", "sub", "nylon", "voice", "glitch", "", ""], 0.7),
];

/// A new, empty project: the standard instruments as they come and nothing in the steps (a tap on an empty place of FILE).
pub fn blank() -> ProjectFile {
    let mut project = Project::default();
    project.bpm = 120.0;
    ProjectFile { project, instruments: instruments([""; 8], 1.0) }
}

/// What each piece changes in its instruments after the preset, by track: knobs (index, value; a level given here is kept as it is, not
/// scaled by the piece's gain), and the sounds of its kit (sound, tune, decay, tone, snap, body, level, pan). They were set by rendering
/// every track of the piece alone and measuring it (docs/instruments.md §9).
pub type Knobs = &'static [(usize, &'static [(usize, f32)])];
pub type Kit = &'static [(usize, [f32; 7])];

/// House: the hook is a soft string, the chords an electric piano (the macro's `fm`), the pad the bell sound played slowly, the cloud the
/// bells, low and quiet; the kit has the clap and the snare forward and the hats back.
const HOUSE_KNOBS: Knobs = &[
    (3, &[(7, 1.0)]),
    (4, &[(4, 0.25), (5, 0.60), (7, 0.14)]),
    (5, &[(2, 0.60), (4, 0.25), (7, 0.18)]), // longer grains (a cleaner tone); TUNE an octave down (a fraction of one would leave the key)
    (6, &[(0, 0.4375), (2, 0.30), (7, 0.60)]), // the macro as its "fm" engine: an electric piano
];
const HOUSE_KIT: Kit = &[
    (1, [0.5, 0.40, 0.45, 0.55, 0.60, 0.95, 0.5]),
    (2, [0.5, 0.12, 0.42, 0.5, 0.5, 0.42, 0.38]),
    (3, [0.5, 0.30, 0.42, 0.5, 0.5, 0.44, 0.62]),
    (5, [0.5, 0.38, 0.55, 0.5, 0.55, 1.0, 0.5]),
    (6, [0.5, 0.12, 0.45, 0.5, 0.5, 0.65, 0.45]),
    (7, [0.5, 0.80, 0.50, 0.5, 0.5, 0.35, 0.6]),
];
/// Techno: the arpeggio is the macro's `va filter` (TIMBRE is its filter: the build opens it), short; the pad the bell sound played slowly;
/// the air the granular's bells an octave down; the kit has a dry clap, a snare for the rolls and quiet hats.
const TECHNO_KNOBS: Knobs = &[
    (2, &[(7, 0.85)]),
    (3, &[(2, 0.45), (7, 0.80)]),
    (4, &[(4, 0.45), (5, 0.70), (7, 0.16)]),
    (5, &[(2, 0.60), (4, 0.25), (7, 0.30)]), // the bells an octave down, long grains: air on the chord (low, its grains muddy the kick)
    (6, &[(0, 0.0208), (2, 0.30), (4, 0.35), (7, 0.55)]),
];
const TECHNO_KIT: Kit = &[
    (1, [0.5, 0.35, 0.50, 0.65, 0.50, 0.80, 0.5]),
    (2, [0.5, 0.10, 0.45, 0.5, 0.5, 0.30, 0.40]),
    (3, [0.5, 0.25, 0.45, 0.5, 0.5, 0.32, 0.60]),
    (4, [0.5, 0.35, 0.45, 0.5, 0.5, 0.55, 0.55]),
    (5, [0.5, 0.30, 0.50, 0.6, 0.45, 1.0, 0.5]),
    (6, [0.5, 0.10, 0.50, 0.5, 0.5, 0.55, 0.40]),
    (7, [0.5, 0.85, 0.45, 0.5, 0.5, 0.35, 0.5]),
];
/// Garage: the organ is the macro's analog voice, short (its phase distortion was shrill, 3.6 kHz); the voice chops quiet; the breath the
/// granular's bells an octave down; the kick up to sit over the bass; the kit crisp.
const GARAGE_KNOBS: Knobs = &[
    (1, &[(7, 0.75)]),
    (2, &[(7, 0.38)]),
    (3, &[(7, 1.0)]),
    (4, &[(7, 0.20)]),
    (5, &[(2, 0.6), (4, 0.25), (7, 0.25)]),
    (6, &[(0, 0.0208), (2, 0.50), (4, 0.30), (7, 0.50)]),
];
const GARAGE_KIT: Kit = &[
    (1, [0.55, 0.30, 0.55, 0.70, 0.45, 0.80, 0.5]),
    (2, [0.5, 0.08, 0.55, 0.5, 0.5, 0.38, 0.40]),
    (3, [0.5, 0.25, 0.55, 0.5, 0.5, 0.35, 0.60]),
    (4, [0.5, 0.30, 0.50, 0.5, 0.5, 0.50, 0.60]),
    (5, [0.5, 0.30, 0.55, 0.6, 0.5, 0.80, 0.5]),
    (6, [0.5, 0.10, 0.55, 0.5, 0.5, 0.55, 0.40]),
    (7, [0.5, 0.80, 0.50, 0.5, 0.5, 0.30, 0.5]),
];
/// Dnb: the pad is the macro's analog voice held long; the reese the bass's growl; the shimmer the granular's bells, long grains, quiet.
const DNB_KNOBS: Knobs = &[
    (2, &[(7, 0.30)]), // the reese is loud by nature (it was at -14 dB): down under the kick
    (3, &[(7, 0.6)]),
    (4, &[(4, 0.40), (5, 0.70), (7, 1.0)]),
    (5, &[(2, 0.6), (7, 0.20)]),
    (6, &[(0, 0.0208), (2, 0.45), (4, 0.90), (7, 0.45)]),
];
const DNB_KIT: Kit = &[
    (1, [0.55, 0.35, 0.55, 0.80, 0.55, 0.90, 0.5]),
    (2, [0.5, 0.08, 0.50, 0.5, 0.5, 0.35, 0.42]),
    (3, [0.5, 0.25, 0.50, 0.5, 0.5, 0.35, 0.58]),
    (6, [0.5, 0.10, 0.50, 0.5, 0.5, 0.50, 0.45]),
    (7, [0.5, 0.85, 0.45, 0.5, 0.5, 0.30, 0.5]),
];
/// Trance: the lead is the macro's swarm; the gated pad the bell with a short attack; the air the granular's bells, quiet.
const TRANCE_KNOBS: Knobs = &[
    (2, &[(7, 0.40)]),
    (3, &[(7, 0.65)]),
    (4, &[(4, 0.05), (5, 0.30), (7, 0.07)]), // a gate on every sixteenth is loud (it was at -16 dB)
    (5, &[(2, 0.6), (7, 0.18)]),
    (6, &[(4, 0.55), (7, 0.70)]), // the lead leads
];
const TRANCE_KIT: Kit = &[
    (1, [0.5, 0.35, 0.55, 0.70, 0.5, 0.85, 0.5]),
    (2, [0.5, 0.10, 0.50, 0.5, 0.5, 0.32, 0.42]),
    (3, [0.5, 0.30, 0.50, 0.5, 0.5, 0.40, 0.58]),
    (5, [0.5, 0.35, 0.50, 0.6, 0.5, 0.95, 0.5]),
    (7, [0.5, 0.90, 0.45, 0.5, 0.5, 0.35, 0.5]),
];
/// Amapiano: the log drum is the KickSyn tuned low with a long decay and little punch; the piano the macro's `fm`; the marimba the
/// resonator's nylon string, short (its bells were shrill, its muted string too quiet); the cloud the granular's bells; the kit's own kick soft and round, the shakers quiet.
const AMAPIANO_KNOBS: Knobs = &[
    (1, &[(0, 0.5624), (1, 0.62), (2, 0.15), (3, 0.30), (4, 0.10), (5, 0.25), (7, 0.35)]), // TUNE 65.4 Hz: note 36 is a C2, so the log drum plays the chord's notes
    (2, &[(7, 0.20)]),
    (3, &[(4, 0.30), (7, 1.0)]),
    (4, &[(7, 0.40)]),
    (5, &[(2, 0.6), (7, 0.22)]),
    (6, &[(0, 0.4375), (2, 0.30), (7, 0.55)]),
];
const AMAPIANO_KIT: Kit = &[
    (0, [0.45, 0.40, 0.40, 0.3, 0.6, 0.60, 0.5]),
    (2, [0.5, 0.08, 0.50, 0.5, 0.5, 0.32, 0.42]),
    (4, [0.5, 0.35, 0.45, 0.5, 0.5, 0.55, 0.60]),
    (5, [0.5, 0.30, 0.55, 0.6, 0.5, 0.75, 0.5]),
    (6, [0.5, 0.12, 0.55, 0.5, 0.5, 0.55, 0.38]),
];
/// Ambient: the pad is the macro's analog voice held long; the bells the resonator's harp, long (its bells ring on partials outside the key); the glass slow; the cloud long grains an
/// octave down; the heartbeat the KickSyn soft and long.
const AMBIENT_KNOBS: Knobs = &[
    (1, &[(1, 0.60), (2, 0.20), (7, 0.55)]),
    (2, &[(7, 0.25)]),
    (3, &[(2, 0.40), (7, 0.55)]),
    (4, &[(4, 0.45), (5, 0.80), (7, 0.35)]),
    (5, &[(2, 0.7), (4, 0.25), (7, 0.35)]),
    (6, &[(0, 0.0208), (2, 0.35), (4, 0.95), (7, 0.45)]),
];
const AMBIENT_KIT: Kit = &[(2, [0.5, 0.10, 0.40, 0.5, 0.5, 0.35, 0.40]), (6, [0.5, 0.15, 0.40, 0.5, 0.5, 0.45, 0.60])];
/// Lo-fi: the keys are the macro's `fm`, soft; the "ah" an octave down; the crackle the granular's glitch, small and quiet; the kit dull.
const LOFI_KNOBS: Knobs = &[
    (1, &[(7, 0.9)]),
    (2, &[(7, 0.25)]),
    (3, &[(7, 1.0)]),
    (4, &[(3, 0.25), (7, 0.40)]),
    (5, &[(2, 0.1), (7, 0.08)]),
    (6, &[(0, 0.4375), (2, 0.25), (7, 0.60)]),
];
const LOFI_KIT: Kit = &[
    (1, [0.4167, 0.30, 0.25, 0.40, 0.65, 0.95, 0.5]), // the snare on D, the key
    (2, [0.5, 0.08, 0.30, 0.5, 0.5, 0.40, 0.42]),
    (3, [0.5, 0.25, 0.30, 0.5, 0.5, 0.40, 0.58]),
    (6, [0.5, 0.10, 0.35, 0.5, 0.5, 0.55, 0.45]),
];
const KNOBS: [Knobs; 8] = [HOUSE_KNOBS, TECHNO_KNOBS, GARAGE_KNOBS, DNB_KNOBS, TRANCE_KNOBS, AMAPIANO_KNOBS, AMBIENT_KNOBS, LOFI_KNOBS];
const KITS: [Kit; 8] = [HOUSE_KIT, TECHNO_KIT, GARAGE_KIT, DNB_KIT, TRANCE_KIT, AMAPIANO_KIT, AMBIENT_KIT, LOFI_KIT];

/// What piece `i` puts on its instruments: the preset of each track and the share of the level it keeps, its knobs and its kit (the
/// twin carries a copy, checked by `tests/twin_pieces.rs`).
pub fn voicing(i: usize) -> ([&'static str; 8], f32, Knobs, Kit) {
    let i = i.min(7);
    (PRESETS[i].0, PRESETS[i].1, KNOBS[i], KITS[i])
}

pub fn file(i: usize) -> ProjectFile {
    let i = i.min(7);
    let (names, gain) = (PRESETS[i].0, PRESETS[i].1);
    let mut instruments = instruments(names, gain);
    if !KITS[i].is_empty() {
        instruments[0] = inst_with("drums", names[0], gain, &[], KITS[i]);
    }
    for &(t, extra) in KNOBS[i] {
        let name = ["drums", "KickSyn", "FmBass", "resonator", "sampler", "granular", "macro", "MidiOutBridge"][t];
        instruments[t] = inst_with(name, names[t], gain, extra, &[]);
    }
    ProjectFile { project: project(i), instruments }
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
