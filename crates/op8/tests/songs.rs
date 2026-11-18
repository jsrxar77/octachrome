//! The eight pieces that come with the machine (docs/features.md §F).
use op8::bench::Bench;
use op8_hw::Button;
use std::sync::atomic::Ordering::Relaxed;

fn pad_of(slot: usize) -> u8 {
    (0..16u8).find(|&p| op8::Controller::pad_to_step(p) as usize == slot).unwrap()
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
        let name = op8::factory::NAMES[i];
        let b = play(i, 12_000);
        let (rms, peak) = (b.rms_db(2_000, 12_000), b.peak_db(0, 12_000));
        b.save_audio(&format!("piece-{}-{}", i + 1, name.replace(' ', "-"))); // with OP8_BENCH_DIR set: a WAV to listen to
        eprintln!("{i} {name:12} rms {rms:6.1} peak {peak:5.1} bpm {}", b.project().bpm);
        if rms < -30.0 {
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
            // house is a whole song whose intro leaves some tracks out on purpose: F-F08 plays all of it
            if i != 0 && has_steps && b.instrument(t).as_deref() != Some("MidiOutBridge") && b.status().note_count[t].load(Relaxed) == 0 {
                problems.push(format!("{name}: track {t} never plays"));
            }
        }
        if i != 0 {
            rows.push(rms); // house starts with a sparse intro on purpose: F-F08 measures its sections
        }
    }
    let (lo, hi) = rows.iter().fold((0.0f32, -99.0f32), |(l, h), r| (l.min(*r), h.max(*r)));
    if hi - lo > 7.0 {
        problems.push(format!("the pieces differ by {:.1} dB in level", hi - lo));
    }
    assert!(problems.is_empty(), "{problems:#?}");
}

// F-F08: house is a whole song: eleven scenes in a song that is followed from the start, between a minute and a half and two and a half
// minutes long; all seven instruments that sound take part; it never goes silent, does not clip, the sections differ in how loud they
// are (the drop is the fullest) and it goes round at its end.
#[test]
fn f_f08_house_is_a_whole_song() {
    let mut b = Bench::demo();
    b.press(Button::File);
    b.tap(pad_of(0));
    let p = b.project().clone();
    assert!(p.song.on, "the song is followed as soon as it loads");
    assert!(p.song.len >= 6, "at least six sections: {}", p.song.len);
    assert!(p.scenes.iter().filter(|s| s.used).count() >= 6);
    let bar_ms = 4.0 * 60_000.0 / p.bpm as f64;
    let starts: Vec<f64> = p.song.steps[..p.song.len as usize].iter().scan(0.0, |at, s| {
        let from = *at;
        *at += s.bars as f64 * bar_ms;
        Some(from)
    }).collect();
    let total = starts.last().unwrap() + p.song.steps[p.song.len as usize - 1].bars as f64 * bar_ms;
    assert!((90_000.0..=150_000.0).contains(&total), "it lasts {:.0} s", total / 1000.0);
    // the harmony changes between sections
    let scene_keys: Vec<_> = (0..p.song.len as usize).map(|i| p.scenes[p.song.steps[i].scene as usize].brain.progression[..4].to_vec()).collect();
    assert!(scene_keys.iter().any(|k| *k != scene_keys[0]), "not every section has the same chords");
    b.press(Button::Play);
    b.run_ms(total as u64 + 4_000);
    for t in 0..8 {
        if b.instrument(t).as_deref() != Some("MidiOutBridge") {
            assert!(b.status().note_count[t].load(Relaxed) > 0, "track {t} ({:?}) takes part", b.instrument(t));
        }
    }
    b.save_audio("house-song"); // with OP8_BENCH_DIR set: a WAV of the whole song to listen to
    let peaks: Vec<f32> = (0..p.song.len as usize).map(|i| b.peak_db(starts[i] as u64, (starts[i] + p.song.steps[i].bars as f64 * bar_ms) as u64)).collect();
    eprintln!("section peaks: {peaks:?}");
    assert!(b.peak_db(0, total as u64) < -0.5, "it does not clip: {peaks:?}");
    for from in (1_000..total as u64 - 1_000).step_by(1_000) {
        assert!(!b.silent(from, from + 1_000), "silent at {from} ms");
    }
    // sections
    let rms = |i: usize| b.rms_db(starts[i] as u64 + 1_000, (starts[i] + p.song.steps[i].bars as f64 * bar_ms) as u64 - 200);
    let levels: Vec<f32> = (0..p.song.len as usize).map(rms).collect();
    eprintln!("section levels: {levels:?}");
    // song steps: 0 intro, 1 groove, 2 main, 3 fill, 4 break, 5 and 6 build, 7 breath, 8 drop, 9 fill, 10 drop again, 11 outro
    assert!(levels[8] > levels[0] + 2.0, "the drop is fuller than the intro: {levels:?}");
    assert!(levels[8] > levels[4] + 1.0, "and than the break: {levels:?}");
    assert!(levels[8] > levels[7] + 3.0, "and than the bar of breath before it: {levels:?}");
    // and it goes round: four seconds past the end it is in the first section again
    assert_eq!(b.status().song_step.load(Relaxed), 0, "it goes back to the first section");
}

/// Every pattern of a track of the house project, as the steps it has when it plays (`None` if it is empty).
fn house_patterns(track: usize) -> Vec<Vec<op8_seq::Step>> {
    let mut p = op8::factory::project(0);
    (0..16u8)
        .filter_map(|i| {
            p.apply(&op8_seq::Command::SetPattern { track: track as u8, index: i, now: false });
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
    let velocities = |pat: &[op8_seq::Step]| {
        let v: Vec<u8> = pat.iter().filter(|s| s.active).map(|s| s.velocity).collect();
        v.iter().max().unwrap() - v.iter().min().unwrap()
    };
    let drums = house_patterns(0);
    assert!(drums.len() >= 8, "the drums have {} patterns", drums.len());
    assert!(drums.iter().filter(|p| velocities(p) >= 40).count() >= 6, "the drums have accents and ghost notes");
    for track in [2, 3, 4] {
        let pats = house_patterns(track);
        assert!(pats.len() >= 3, "track {track} has {} patterns", pats.len());
        assert!(pats.iter().all(|p| velocities(p) >= 10), "track {track} has steps of one velocity only");
        let shape = |p: &Vec<op8_seq::Step>| p.iter().map(|s| s.active).collect::<Vec<_>>();
        assert!(pats.iter().any(|p| shape(p) != shape(&pats[0])), "track {track} has one rhythm only");
    }
    // the sections do not repeat each other
    let p = op8::factory::project(0);
    let used: Vec<_> = (0..p.song.len as usize).map(|i| p.song.steps[i].scene as usize).collect();
    let distinct: std::collections::HashSet<_> = used.iter().map(|&i| (p.scenes[i].pattern, p.scenes[i].mute)).collect();
    assert!(distinct.len() >= 10, "{} different sections", distinct.len());
    // the second part of the build has more claps than the first
    let claps = |pat: &Vec<op8_seq::Step>| pat.iter().filter(|s| s.lanes & (1 << 5) != 0).count();
    assert!(claps(&drums[6]) > claps(&drums[5]), "the build speeds up");
}
