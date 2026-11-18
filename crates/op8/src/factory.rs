//! The eight projects that always ship (the top two rows of the FILE pads): eight short pieces in eight
//! styles, so that the first thing anyone hears is what the machine can do. Each is made only with the
//! factory instruments and their presets, on the standard tracks (A drums, B KickSyn, C FmBass, D resonator,
//! E sampler, F granular, G macro, H MIDI out), so it always loads.

use crate::projectfile::ProjectFile;
use op8_seq::{
    Brain, BrainMode, ChordKind, ChordSlot, ChordType, Command, Division, FxKind, Lfo, LfoShape, PlayMode, Players, Project, Scale, Step,
    StepComponent as C, TrackFx,
};

pub const NAMES: [&str; 8] = ["house", "techno", "dub", "polyrhythm", "acid", "electro", "ambient", "breaks"];

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
    let mut c = op8_cartridges::create(name);
    let found = op8_cartridges::presets::factory(name).into_iter().find(|p| p.0 == preset);
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
    (["", "punch", "round", "nylon", "bell", "bells", "", ""], 0.55),
    (["", "punch", "pluck", "harp", "bell", "bells", "", ""], 0.7),
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

/// What each piece changes in its instruments after the preset, by track: knobs (index, value; a level given here is kept as it is, not
/// scaled by the piece's gain), and the sounds of its kit (sound, tune, decay, tone, snap, body, level, pan). They were set by rendering
/// every track of the piece alone and measuring it (docs/instruments.md §9).
type Knobs = &'static [(usize, &'static [(usize, f32)])];
type Kit = &'static [(usize, [f32; 7])];

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
const KNOBS: [Knobs; 8] = [HOUSE_KNOBS, TECHNO_KNOBS, &[], &[], &[], &[], &[], &[]];
const KITS: [Kit; 8] = [HOUSE_KIT, TECHNO_KIT, &[], &[], &[], &[], &[], &[]];

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
