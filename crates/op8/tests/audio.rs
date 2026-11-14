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
