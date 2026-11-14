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

// F-K09: a kit step has no note: the inspector's first cell is empty and its knob does nothing; the sound is a lane.
#[test]
fn f_k09_a_kit_step_has_no_note() {
    let mut b = Bench::demo();
    b.press(Button::Group(4)); // track E: drums, a kit
    b.press(Button::Screen(2)); // opens the inspector on step 1
    b.run_ms(100);
    let before = b.project().tracks[4].steps[0];
    b.knob(0, 40);
    b.run_ms(50);
    assert_eq!(b.project().tracks[4].steps[0], before, "the first knob changes nothing on a kit step");
    // a plain-pitch track still has its note knob
    b.press(Button::Group(1));
    b.press(Button::Screen(2));
    b.run_ms(50);
    let note = b.project().tracks[1].steps[0].note;
    b.knob(0, 40);
    b.run_ms(50);
    assert_eq!(b.project().tracks[1].steps[0].note, note + 1);
}

// F-K10: on a drum kit the pads are the steps of the lane chosen on the right (`sound`): a tap puts that sound
// on the step or takes it off, and leaves the other lanes of the step as they are.
#[test]
fn f_k10_a_tap_toggles_the_chosen_lane() {
    let mut b = Bench::demo();
    b.press(Button::Group(4));
    b.press(Button::Screen(6)); // shape: the second page, where `sound` is the last knob
    b.knob(7, 120); // 3 detents: kick -> snare (lane 1)
    let pad = (0..16u8)
        .find(|&p| b.project().tracks[4].steps[op8::Controller::pad_to_step(p) as usize].lanes == 0)
        .expect("an empty step");
    let step = op8::Controller::pad_to_step(pad) as usize;
    b.tap(pad);
    b.run_ms(50);
    assert_eq!(b.project().tracks[4].steps[step].lanes, 0b10, "the snare lane is on");
    // choose another sound, tap the same step: both lanes are on (two sounds in one step)
    b.knob(7, -120);
    b.tap(pad);
    b.run_ms(50);
    assert_eq!(b.project().tracks[4].steps[step].lanes, 0b11, "kick and snare together");
    // tapping a lane that is on takes only that one off
    b.tap(pad);
    b.run_ms(50);
    assert_eq!(b.project().tracks[4].steps[step].lanes, 0b10, "only the kick left");
}
