//! The eight pieces that come with the machine (docs/features.md §F).
use octachrome::bench::Bench;
use octachrome_hw::Button;
use std::sync::atomic::Ordering::Relaxed;

fn pad_of(slot: usize) -> u8 {
    (0..16u8).find(|&p| octachrome::Controller::pad_to_step(p) as usize == slot).unwrap()
}

/// Open a piece from FILE and let it play for `ms`.
fn play(i: usize, ms: u64) -> Bench {
    let mut b = Bench::demo();
    b.press(Button::File);
    b.tap(pad_of(i));
    let from = b.now_ms();
    b.press(Button::Play);
    b.run_ms(ms);
    assert!(b.now_ms() >= from + ms);
    b
}

// F-F07: each of the eight pieces opens from FILE, plays for a while with sound all the way through, does not clip, and they are all
// about as loud as each other (so that none has to be turned up); every track that has steps makes notes.
#[test]
fn f_f07_the_eight_pieces_play() {
    let mut rows = Vec::new();
    let mut problems = Vec::new();
    for i in 0..8 {
        let name = octachrome::factory::NAMES[i];
        let b = play(i, 12_000);
        let (rms, peak) = (b.rms_db(2_000, 12_000), b.peak_db(0, 12_000));
        b.save_audio(&format!("piece-{}-{}", i + 1, name.replace(' ', "-"))); // with OCTACHROME_BENCH_DIR set: a WAV to listen to
        eprintln!("{i} {name:12} rms {rms:6.1} peak {peak:5.1} bpm {}", b.project().bpm);
        // a song starts with an intro that is quiet on purpose: its own test measures its sections
        if !b.project().song.on && rms < -30.0 {
            problems.push(format!("{name} is too quiet: {rms:.1} dB"));
        }
        if peak > -0.5 {
            problems.push(format!("{name} is at the limit: {peak:.1} dB"));
        }
        for from in (2_000..10_000).step_by(1_000) {
            if b.silent(from, from + 1_000) {
                problems.push(format!("{name} is silent at {from} ms"));
            }
        }
        for t in 0..8 {
            let tr = &b.project().tracks[t];
            let has_steps = tr.steps[..tr.length as usize].iter().any(|s| s.active);
            // a piece that is a whole song leaves tracks out of its intro on purpose: its own test plays all of it
            if !b.project().song.on && has_steps && b.instrument(t).as_deref() != Some("MidiOutBridge") && b.status().note_count[t].load(Relaxed) == 0 {
                problems.push(format!("{name}: track {t} never plays"));
            }
        }
        if !b.project().song.on {
            rows.push(rms); // a song starts with an intro that is sparse on purpose: its own test measures its sections
        }
    }
    let (lo, hi) = rows.iter().fold((0.0f32, -99.0f32), |(l, h), r| (l.min(*r), h.max(*r)));
    if hi - lo > 7.0 {
        problems.push(format!("the pieces differ by {:.1} dB in level", hi - lo));
    }
    assert!(problems.is_empty(), "{problems:#?}");
}

/// Play the whole of piece `i` (a song) and check what every song of the machine must be: followed from the moment it loads, at least six
/// sections and six scenes, between a minute and a half and two and a half minutes long, other chords in some sections, every instrument
/// that sounds takes part, never silent for a second, never clipping, its fullest section 2 dB over its first, and round to the first
/// section at its end. Returns the bench, where each section starts (ms) and the loudness of each section.
fn whole_song(i: usize, name: &str) -> (Bench, Vec<f64>, Vec<f32>) {
    let mut b = Bench::demo();
    b.press(Button::File);
    b.tap(pad_of(i));
    let p = b.project().clone();
    assert!(p.song.on, "{name}: the song is followed as soon as it loads");
    assert!(p.song.len >= 6, "{name}: at least six sections: {}", p.song.len);
    assert!(p.scenes.iter().filter(|s| s.used).count() >= 6);
    let bar_ms = 4.0 * 60_000.0 / p.bpm as f64;
    let starts: Vec<f64> = p.song.steps[..p.song.len as usize].iter().scan(0.0, |at, s| {
        let from = *at;
        *at += s.bars as f64 * bar_ms;
        Some(from)
    }).collect();
    let total = starts.last().unwrap() + p.song.steps[p.song.len as usize - 1].bars as f64 * bar_ms;
    assert!((90_000.0..=150_000.0).contains(&total), "{name}: it lasts {:.0} s", total / 1000.0);
    // the harmony changes between sections
    let scene_keys: Vec<_> = (0..p.song.len as usize).map(|i| p.scenes[p.song.steps[i].scene as usize].brain.progression[..4].to_vec()).collect();
    assert!(scene_keys.iter().any(|k| *k != scene_keys[0]), "{name}: not every section has the same chords");
    b.press(Button::Play);
    b.run_ms(total as u64 + 4_000);
    b.save_audio(&format!("song-{}-{name}", i + 1)); // with OCTACHROME_BENCH_DIR set: a WAV of the whole song to listen to
    for t in 0..8 {
        if b.instrument(t).as_deref() != Some("MidiOutBridge") {
            assert!(b.status().note_count[t].load(Relaxed) > 0, "{name}: track {t} ({:?}) takes part", b.instrument(t));
        }
    }
    let peaks: Vec<f32> = (0..p.song.len as usize).map(|i| b.peak_db(starts[i] as u64, (starts[i] + p.song.steps[i].bars as f64 * bar_ms) as u64)).collect();
    eprintln!("{name} section peaks: {peaks:?}");
    assert!(b.peak_db(0, total as u64) < -0.5, "{name}: it does not clip: {peaks:?}");
    for from in (1_000..total as u64 - 1_000).step_by(1_000) {
        assert!(!b.silent(from, from + 1_000), "{name}: silent at {from} ms");
    }
    let rms = |i: usize| b.rms_db(starts[i] as u64 + 1_000, (starts[i] + p.song.steps[i].bars as f64 * bar_ms) as u64 - 200);
    let levels: Vec<f32> = (0..p.song.len as usize).map(rms).collect();
    eprintln!("{name} section levels: {levels:?}");
    let fullest = levels.iter().cloned().fold(f32::MIN, f32::max);
    assert!(fullest > levels[0] + 2.0, "{name}: its fullest section is fuller than its first: {levels:?}");
    // and it goes round: four seconds past the end it is in the first section again
    assert_eq!(b.status().song_step.load(Relaxed), 0, "{name}: it goes back to the first section");
    (b, starts, levels)
}

// F-F08: house is a whole song (the checks of every song, `whole_song`), with eleven scenes; the drop is fuller than the intro, the
// break and the bar of breath before it.
#[test]
fn f_f08_house_is_a_whole_song() {
    let (_, _, levels) = whole_song(0, "house");
    // song steps: 0 intro, 1 groove, 2 main, 3 fill, 4 break, 5 and 6 build, 7 breath, 8 drop, 9 fill, 10 drop again, 11 outro
    assert!(levels[8] > levels[0] + 2.0, "the drop is fuller than the intro: {levels:?}");
    assert!(levels[8] > levels[4] + 1.0, "and than the break: {levels:?}");
    assert!(levels[8] > levels[7] + 3.0, "and than the bar of breath before it: {levels:?}");
}

// F-F11: techno is a whole song (the checks of every song); the drop is fuller than the intro and the breakdown, and the arpeggio's filter
// opens: its patterns of the peak and the drop lock TIMBRE higher than the build begins, and the closed one locks nothing.
#[test]
fn f_f11_techno_is_a_whole_song() {
    let (_, _, levels) = whole_song(1, "techno");
    // song steps: 0 intro, 1 groove, 2 arpeggio, 3 peak, 4 breakdown, 5 and 6 build, 7 breath, 8 drop, 9 fill, 10 outro
    assert!(levels[8] > levels[0] + 2.0, "the drop is fuller than the intro: {levels:?}");
    assert!(levels[8] > levels[4] + 4.0, "and than the breakdown: {levels:?}");
    let mut p = octachrome::factory::project(1);
    let timbre = |p: &octachrome_seq::Project| -> Vec<u8> {
        p.tracks[6].steps.iter().flat_map(|s| s.components.iter().filter_map(|c| match c { octachrome_seq::StepComponent::ParamLock { index: 2, value } => Some(*value), _ => None })).collect()
    };
    let mut locks = Vec::new();
    for i in 0..3u8 {
        p.apply(&octachrome_seq::Command::SetPattern { track: 6, index: i, now: false });
        locks.push(timbre(&p));
    }
    assert!(locks[0].is_empty(), "the closed arpeggio locks nothing");
    assert!(locks[1].windows(2).all(|w| w[1] > w[0]), "the build opens it step by step: {:?}", locks[1]);
    assert!(locks[2].iter().all(|&v| v > locks[1][0]), "the drop is open");
}

/// Every pattern of a track of the house project, as the steps it has when it plays (`None` if it is empty).
fn house_patterns(track: usize) -> Vec<Vec<octachrome_seq::Step>> {
    let mut p = octachrome::factory::project(0);
    (0..16u8)
        .filter_map(|i| {
            p.apply(&octachrome_seq::Command::SetPattern { track: track as u8, index: i, now: false });
            let t = &p.tracks[track];
            let steps = t.steps[..t.length as usize].to_vec();
            steps.iter().any(|s| s.active).then_some(steps)
        })
        .collect()
}

// F-F10: house is composed, not just filled in: the bass never sits on a kick, the hats and the stabs have accents and ghost notes (their
// velocities differ), the bass and the stabs and the voice have more than one rhythm, no two sections have the same patterns, and the
// build speeds up (more claps in its second part than in its first).
#[test]
fn f_f10_house_is_composed() {
    let kicks: std::collections::HashSet<usize> = house_patterns(1).iter().flat_map(|p| (0..p.len()).filter(|&s| p[s].active)).collect();
    for (n, pat) in house_patterns(2).iter().enumerate() {
        for s in (0..pat.len()).filter(|&s| pat[s].active) {
            assert!(!(s % 4 == 0 && kicks.contains(&s)), "bass pattern {n} plays on the kick at step {s}");
        }
    }
    let velocities = |pat: &[octachrome_seq::Step]| {
        let v: Vec<u8> = pat.iter().filter(|s| s.active).map(|s| s.velocity).collect();
        v.iter().max().unwrap() - v.iter().min().unwrap()
    };
    let drums = house_patterns(0);
    assert!(drums.len() >= 8, "the drums have {} patterns", drums.len());
    assert!(drums.iter().filter(|p| velocities(p) >= 40).count() >= 6, "the drums have accents and ghost notes");
    for track in [2, 3, 6] {
        let pats = house_patterns(track);
        assert!(pats.len() >= 3, "track {track} has {} patterns", pats.len());
        assert!(pats.iter().filter(|p| velocities(p) >= 10).count() >= 2, "track {track} has steps of one velocity only");
        let shape = |p: &Vec<octachrome_seq::Step>| p.iter().map(|s| s.active).collect::<Vec<_>>();
        assert!(pats.iter().any(|p| shape(p) != shape(&pats[0])), "track {track} has one rhythm only");
    }
    // the sections do not repeat each other
    let p = octachrome::factory::project(0);
    let used: Vec<_> = (0..p.song.len as usize).map(|i| p.song.steps[i].scene as usize).collect();
    let distinct: std::collections::HashSet<_> = used.iter().map(|&i| (p.scenes[i].pattern, p.scenes[i].mute)).collect();
    assert!(distinct.len() >= 10, "{} different sections", distinct.len());
    // the second part of the build has more claps than the first
    let claps = |pat: &Vec<octachrome_seq::Step>| pat.iter().filter(|s| s.lanes & (1 << 5) != 0).count();
    assert!(claps(&drums[6]) > claps(&drums[5]), "the build speeds up");
}

/// Every pattern of a track of factory piece `piece`, as the steps it has when it plays (empty ones left out).
fn patterns_of(piece: usize, track: usize) -> Vec<Vec<octachrome_seq::Step>> {
    let mut p = octachrome::factory::project(piece);
    (0..16u8)
        .filter_map(|i| {
            p.apply(&octachrome_seq::Command::SetPattern { track: track as u8, index: i, now: false });
            let t = &p.tracks[track];
            let steps = t.steps[..t.length as usize].to_vec();
            steps.iter().any(|s| s.active).then_some(steps)
        })
        .collect()
}

fn lanes_at(pat: &[octachrome_seq::Step], lanes: u8) -> Vec<usize> {
    (0..pat.len()).filter(|&s| pat[s].lanes & lanes != 0).collect()
}

// F-F13: garage is a whole song (the checks of every song); it is two-step: the kick never lands on two or four (steps 4 and 12), the
// snare does, and the swing is heavy.
#[test]
fn f_f13_garage_is_a_whole_two_step_song() {
    let (_, _, levels) = whole_song(2, "garage");
    assert!(levels[6] > levels[0] + 4.0, "the last hook is fuller than the intro: {levels:?}");
    for pat in patterns_of(2, 1) {
        assert!(!pat[4].active && !pat[12].active, "the kick skips two and four");
    }
    assert!(lanes_at(&patterns_of(2, 0)[0], 1 << 1).contains(&4), "a snare on two");
    assert!(octachrome::factory::project(2).swing >= 0.6);
}

// F-F14: dnb is a whole song (the checks of every song); it runs at drum & bass tempo (170 or more) on a two-step break: the snare on
// two and four, the kick on one and the "and" of three.
#[test]
fn f_f14_dnb_is_a_whole_song() {
    let (_, _, levels) = whole_song(3, "dnb");
    assert!(levels[3] > levels[0] + 4.0, "the drop is fuller than the intro: {levels:?}");
    assert!(levels[3] > levels[4] + 2.0, "and than the break: {levels:?}");
    assert!(octachrome::factory::project(3).bpm >= 170.0);
    let snares = lanes_at(&patterns_of(3, 0)[0], 1 << 1);
    assert!(snares.contains(&4) && snares.contains(&12), "the snare on two and four: {snares:?}");
    let kick = &patterns_of(3, 1)[0];
    assert!(kick[0].active && kick[10].active);
}

// F-F15: trance is a whole song (the checks of every song); its bass gallops between the kicks (never on one of them), its pad has a
// gated pattern (a note on every sixteenth), and the drop is fuller than the breakdown.
#[test]
fn f_f15_trance_is_a_whole_song() {
    let (_, _, levels) = whole_song(4, "trance");
    assert!(levels[7] > levels[3] + 4.0, "the drop is fuller than the breakdown: {levels:?}");
    for pat in patterns_of(4, 2) {
        for s in [0, 4, 8, 12] {
            assert!(!pat[s].active, "the bass leaves the kick alone");
        }
    }
    assert!(patterns_of(4, 4).iter().any(|p| p.iter().all(|s| s.active)), "a gated pad");
}

// F-F16: amapiano is a whole song (the checks of every song); its log drum (the KickSyn) follows the chord and plays a line of at least
// three notes, and the kit's own kick is on every beat.
#[test]
fn f_f16_amapiano_is_a_whole_song() {
    let (_, _, levels) = whole_song(5, "amapiano");
    assert!(levels[5] > levels[0] + 4.0, "the drop is fuller than the intro: {levels:?}");
    let p = octachrome::factory::project(5);
    assert!(p.tracks[1].brain_link, "the log drum follows the chord");
    let mut notes: Vec<u8> = patterns_of(5, 1).iter().flat_map(|p| p.iter().filter(|s| s.active).map(|s| s.note).collect::<Vec<_>>()).collect();
    notes.sort();
    notes.dedup();
    assert!(notes.len() >= 3, "a line, not one note: {notes:?}");
    assert_eq!(lanes_at(&patterns_of(5, 0)[0], 1), vec![0, 4, 8, 12], "the soft kick on every beat");
}

// F-F17: ambient is a whole song (the checks of every song); nothing hits hard: no snare, clap or crash anywhere, a kick at most twice
// a bar, and every chord lasts two bars.
#[test]
fn f_f17_ambient_is_a_whole_song() {
    whole_song(6, "ambient");
    for pat in patterns_of(6, 0) {
        assert!(lanes_at(&pat, (1 << 1) | (1 << 5) | (1 << 7)).is_empty(), "no snare, clap or crash");
    }
    for pat in patterns_of(6, 1) {
        assert!(pat.iter().filter(|s| s.active).count() <= 2, "a heartbeat, not a beat");
    }
    let p = octachrome::factory::project(6);
    assert!(p.scenes.iter().filter(|s| s.used).all(|s| s.brain.progression[..s.brain.len as usize].iter().all(|c| c.length == 32)));
}

// F-F18: lo-fi is a whole song (the checks of every song) with a heavy swing, and its keys come in after the beat, as a hand plays
// (a delay on their steps).
#[test]
fn f_f18_lofi_is_a_whole_song() {
    let (_, _, levels) = whole_song(7, "lo-fi");
    assert!(levels[1] > levels[0] + 4.0, "the beat makes it fuller than the intro: {levels:?}");
    let p = octachrome::factory::project(7);
    assert!(p.swing >= 0.6);
    let keys = &patterns_of(7, 6)[0];
    assert!(keys.iter().filter(|s| s.active).all(|s| s.components.iter().any(|c| matches!(c, octachrome_seq::StepComponent::Delay(_)))), "every chord a little late");
}
