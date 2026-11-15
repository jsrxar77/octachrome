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
            if has_steps && b.instrument(t).as_deref() != Some("MidiOutBridge") && b.status().note_count[t].load(Relaxed) == 0 {
                problems.push(format!("{name}: track {t} never plays"));
            }
        }
        rows.push(rms);
    }
    let (lo, hi) = rows.iter().fold((0.0f32, -99.0f32), |(l, h), r| (l.min(*r), h.max(*r)));
    if hi - lo > 7.0 {
        problems.push(format!("the pieces differ by {:.1} dB in level", hi - lo));
    }
    assert!(problems.is_empty(), "{problems:#?}");
}
