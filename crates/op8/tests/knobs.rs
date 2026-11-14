//! Encoders 5–8 and the touch overlay (docs/features.md §K).
use op8::bench::Bench;
use op8_cartridge::InstrumentCartridge;
use op8_hw::{Button, Screen};

// F-K01: encoder 5 moves parameter 1 of the selected track's instrument, in the direction it is turned.
#[test]
fn f_k01_knob5_moves_parameter() {
    let mut b = Bench::demo();
    b.press(Button::Group(1)); // FmBass
    let before = b.param(1, 1);
    b.knob(5, 200);
    let up = b.param(1, 1);
    assert!(up > before, "turning up did not raise the parameter ({before} → {up})");
    b.knob(5, -400);
    assert!(b.param(1, 1) < up, "turning down did not lower it");
}

// F-K02: with page 6 open, encoders 5–8 move parameters 5–8.
#[test]
fn f_k02_page_two_moves_second_bank() {
    let mut b = Bench::demo();
    b.press(Button::Group(1));
    b.press(Button::Screen(6));
    assert_eq!(b.ui().right_page, 1);
    let (p0, p4) = (b.param(1, 0), b.param(1, 4));
    b.knob(4, 300);
    assert_eq!(b.param(1, 0), p0, "page 1 parameter moved while page 2 is open");
    assert_ne!(b.param(1, 4), p4, "page 2 parameter did not move");
}

// F-K03: touching an encoder shows the overlay on the right screen.
#[test]
fn f_k03_touch_shows_overlay() {
    let mut b = Bench::demo();
    b.press(Button::Group(1));
    b.run_ms(100);
    let idle = b.screen_hash(Screen::Right);
    b.touch(5, true);
    assert!(b.ui().knob_touch[5]);
    let touched = b.screen_hash(Screen::Right);
    assert_ne!(idle, touched, "overlay did not appear");
    b.capture("f-k03-overlay");
    b.touch(5, false);
    assert!(!b.ui().knob_touch[5]);
}

// F-K04: every built-in instrument exposes at most eight parameters.
#[test]
fn f_k04_at_most_eight_parameters() {
    for name in op8_cartridges::builtin_names() {
        let c: Box<dyn InstrumentCartridge> = op8_cartridges::create(name).expect("registered");
        let n = c.parameters().len() + c.secondary_parameters().map_or(0, |s| s.len());
        assert!(n <= 8, "{name} exposes {n} parameters");
    }
}

// F-K09: on a drum track the step inspector says which sound the note plays, and the note knob chooses it.
#[test]
fn f_k09_inspector_names_the_drum_sound() {
    let mut b = Bench::demo();
    b.press(Button::Group(4)); // track E: drums
    b.press(Button::Screen(2)); // components: opens the inspector on step 1
    b.run_ms(100);
    let before = b.ui().step_sound.expect("a drum track names the sound");
    b.knob(0, 40); // one detent up: the next note, the next sound
    b.run_ms(100);
    let after = b.ui().step_sound.expect("still named");
    assert_ne!(before, after, "{before} -> {after}");
    // a track whose instrument is a plain pitch names nothing
    b.press(Button::Group(1));
    b.press(Button::Screen(2));
    b.run_ms(100);
    assert_eq!(b.ui().step_sound, None);
}
