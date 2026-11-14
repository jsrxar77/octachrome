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
    assert_eq!(b.project().tracks[2].length, 7, "precondition");
    b.press(Button::Play);
    let (mut max, mut seen_others) = (0u8, std::collections::BTreeSet::new());
    for _ in 0..(4 * 60) {
        b.tick();
        let p = b.status().playhead[2].load(Relaxed);
        max = max.max(p);
        seen_others.insert(p);
    }
    assert!(max <= 6, "playhead {max} is outside a length-7 track");
    assert_eq!(max, 6, "playhead never reached the last step");
    assert_eq!(seen_others.len(), 7, "expected all seven steps, got {seen_others:?}");
}

// F-T05: TAP works the tempo out from the taps (the average gap of the last ones), and a pause of more than two seconds starts over.
#[test]
fn f_t05_tap_sets_the_tempo() {
    let mut b = Bench::empty();
    let mut at = Vec::new();
    for _ in 0..4 {
        at.push(b.now_ms());
        b.press(Button::Tap);
        b.run_ms(500);
    }
    let want = 60_000.0 / ((at[3] - at[0]) as f32 / 3.0);
    assert!((b.project().bpm - want).abs() < 2.0, "four taps {:?} ms apart are {want} bpm: {}", at, b.project().bpm);
    // after a long pause the first tap only starts counting
    b.run_ms(3000);
    let bpm = b.project().bpm;
    let first = b.now_ms();
    b.press(Button::Tap);
    assert_eq!(b.project().bpm, bpm, "one tap does not change the tempo");
    b.run_ms(600);
    let second = b.now_ms();
    b.press(Button::Tap);
    let want = 60_000.0 / (second - first) as f32;
    assert!((b.project().bpm - want).abs() < 3.0, "two taps {} ms apart are {want} bpm: {}", second - first, b.project().bpm);
}

// F-T06: SHIFT + TAP turns the metronome on and off; while it is on it clicks on every beat of a running sequencer, and it is silent when off.
#[test]
fn f_t06_shift_tap_is_the_metronome() {
    let mut b = Bench::empty();
    b.press(Button::Play);
    b.run_ms(700);
    assert!(b.silent(0, 700), "an empty pattern is silent without the metronome");
    b.button_down(Button::Shift);
    b.press(Button::Tap);
    b.button_up(Button::Shift);
    assert!(b.ui().metronome);
    b.run_ms(1000);
    assert!(b.peak(800, 1700) > 0.02, "the metronome clicks");
    b.button_down(Button::Shift);
    b.press(Button::Tap);
    b.button_up(Button::Shift);
    assert!(!b.ui().metronome);
    b.run_ms(400);
    assert!(b.silent(1900, 2100), "and stops");
}
