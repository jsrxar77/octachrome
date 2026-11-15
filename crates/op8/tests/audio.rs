//! Audio (docs/features.md §A).
use op8::bench::Bench;
use op8_hw::Button;

// F-A01: nothing sounds before PLAY.
#[test]
fn f_a01_silence_before_play() {
    let mut b = Bench::demo();
    b.run_ms(200);
    assert!(b.silent(0, 200), "sound before PLAY: {} dB", b.peak_db(0, 200));
}

// F-A02: the demo sounds: between 2 s and 7 s the mix is clearly audible.
#[test]
fn f_a02_demo_is_audible() {
    let mut b = Bench::demo();
    b.press(Button::Play);
    b.run_ms(7000);
    let rms = b.rms_db(2000, 7000);
    assert!(rms > -30.0, "demo too quiet: {rms:.1} dBFS RMS");
    b.save_audio("f-a02-demo");
}

// F-A03: no digital clipping in the demo.
#[test]
fn f_a03_no_clipping() {
    let mut b = Bench::demo();
    b.press(Button::Play);
    b.run_ms(7000);
    let peak = b.peak(0, 7000);
    assert!(peak < 0.99, "peak {peak:.3} is at or over full scale");
}

// F-A04: an empty pattern plays silence (no noise floor, no DC).
#[test]
fn f_a04_empty_project_is_silent() {
    let mut b = Bench::empty();
    b.press(Button::Play);
    b.run_ms(1000);
    assert!(b.silent(0, 1000), "empty project makes sound: {} dB", b.peak_db(0, 1000));
}

// F-A05: the same script gives exactly the same audio.
#[test]
fn f_a05_same_script_same_audio() {
    let run = || {
        let mut b = Bench::demo();
        b.press(Button::Play);
        b.run_ms(2000);
        (b.audio_l, b.audio_r)
    };
    let (a, c) = (run(), run());
    assert!(a.0 == c.0 && a.1 == c.1, "two runs differ");
}

// F-A11: choosing a sound can be heard at once, without playing the sequence: turning a knob of the instrument
// (or `sound`) and tapping a step all play it.
#[test]
fn f_a11_choices_are_auditioned() {
    use op8_hw::Button;
    // a knob turn on the drum track
    let mut b = Bench::demo();
    b.press(Button::Group(0));
    b.run_ms(300);
    let quiet = b.peak(0, 300);
    b.press(Button::Screen(6));
    b.knob(7, 120);
    b.run_ms(400);
    assert!(b.peak(300, 700) > quiet + 0.01, "turning `sound` plays the sound: {} vs {}", b.peak(300, 700), quiet);
    // a tap on a step
    let mut b = Bench::demo();
    b.press(Button::Group(0));
    b.run_ms(100);
    b.tap(0);
    b.run_ms(300);
    assert!(b.peak(100, 700) > 0.01, "tapping a step plays its sound");
}

// F-A12: a choice is auditioned once, not over and over: the `sound` knob plays each new sound once, and a continuous
// knob plays once, when it has been still for a moment.
#[test]
fn f_a12_auditions_do_not_repeat() {
    use op8_hw::Button;
    use std::sync::atomic::Ordering::Relaxed;
    let mut b = Bench::demo();
    b.press(Button::Group(0)); // the drums
    b.press(Button::Screen(6)); // shape: the page with `sound` and body, level, pan
    b.run_ms(300);
    let played = |b: &Bench| b.status().note_count[0].load(Relaxed);
    let before = played(&b);
    // turn `sound` through three sounds, one detent at a time: one note each, no more
    for _ in 0..3 {
        b.knob(7, 100); // 100/800 of the range: a bit more than one sound (1/8) per turn
        b.run_ms(60);
    }
    b.run_ms(400);
    let after_sound = played(&b);
    assert!(after_sound - before >= 1 && after_sound - before <= 3, "{} notes for three sounds", after_sound - before);
    // a continuous knob turned for a while plays nothing while it moves, and once after
    for _ in 0..20 {
        b.knob(5, 10);
        b.run_ms(20);
    }
    assert_eq!(played(&b), after_sound, "silent while the knob is moving");
    b.run_ms(500);
    assert_eq!(played(&b), after_sound + 1, "one note once it stops");
}
