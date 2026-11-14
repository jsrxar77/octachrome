//! Transport (docs/features.md §T).
use op8::bench::Bench;
use op8_hw::Button;
use std::sync::atomic::Ordering::Relaxed;

// F-T01: PLAY starts the sequencer and the sound; a second PLAY pauses it.
#[test]
fn f_t01_play_starts_and_pauses() {
    let mut b = Bench::demo();
    assert!(!b.status().playing.load(Relaxed));
    b.press(Button::Play);
    assert!(b.status().playing.load(Relaxed));
    b.run_ms(1000);
    assert!(b.rms_db(0, 1000) > -40.0, "no sound after PLAY: {} dB", b.rms_db(0, 1000));
    b.capture("f-t01-playing");

    b.press(Button::Play);
    assert!(!b.status().playing.load(Relaxed));
    b.run_ms(3000); // let the tails die out
    assert!(b.silent(3500, 4000), "still sounding after PAUSE: {} dB", b.peak_db(3500, 4000));
}

// F-T02: STOP stops the sequencer and the sound falls to silence.
#[test]
fn f_t02_stop_silences() {
    let mut b = Bench::demo();
    b.press(Button::Play);
    b.run_ms(1000);
    b.press(Button::Stop);
    assert!(!b.status().playing.load(Relaxed));
    b.run_ms(3000);
    let t = b.now_ms();
    assert!(b.silent(t - 500, t), "not silent after STOP: {} dB", b.peak_db(t - 500, t));
}

// F-T03: RESTART goes back to step 1 and keeps playing.
#[test]
fn f_t03_restart_returns_to_step_one() {
    let mut b = Bench::demo();
    b.press(Button::Play);
    b.run_ms(1300); // track A has moved on from step 1 by now
    assert!(b.status().playhead[0].load(Relaxed) > 0, "precondition: playhead should have moved");
    b.press(Button::Restart);
    b.tick();
    assert!(b.status().playing.load(Relaxed), "RESTART must keep playing");
    assert_eq!(b.status().playhead[0].load(Relaxed), 0, "RESTART must return to step 1");
}

// F-T04: tracks of different length drift apart (polymetry); the playhead
// never leaves its own length. Track B (index 1) has length 7.
#[test]
fn f_t04_polymeter_stays_in_length() {
    let mut b = Bench::demo();
    assert_eq!(b.project().tracks[1].length, 7, "precondition");
    b.press(Button::Play);
    let (mut max, mut seen_others) = (0u8, std::collections::BTreeSet::new());
    for _ in 0..(4 * 60) {
        b.tick();
        let p = b.status().playhead[1].load(Relaxed);
        max = max.max(p);
        seen_others.insert(p);
    }
    assert!(max <= 6, "playhead {max} is outside a length-7 track");
    assert_eq!(max, 6, "playhead never reached the last step");
    assert_eq!(seen_others.len(), 7, "expected all seven steps, got {seen_others:?}");
}
