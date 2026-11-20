//! The demo build (`cargo test -p octachrome --features demo`): F-X01. Without the feature this file is empty.
#![cfg(feature = "demo")]
use octachrome::bench::Bench;
use octachrome_hw::Button;
use octachrome_ui::Main;
use std::sync::atomic::Ordering::Relaxed;

fn pad_of(slot: usize) -> u8 {
    (0..16u8).find(|&p| octachrome::Controller::pad_to_step(p) as usize == slot).unwrap()
}

// F-X01: in the demo a song is chosen in FILE and heard with PLAY and STOP, the knobs of an instrument turn, and nothing else works
#[test]
fn f_x01_the_demo_plays_and_turns_an_instrument_and_nothing_else() {
    let mut b = Bench::demo();
    // choosing a song works
    b.press(Button::File);
    assert_eq!(b.ui().main, Main::File);
    b.tap(pad_of(1));
    b.run_ms(100);
    assert_eq!(b.ui().main, Main::Instrument);
    // and it plays and stops
    b.press(Button::Play);
    b.run_ms(300);
    assert!(b.status().playing.load(Relaxed), "PLAY plays");
    b.press(Button::Stop);
    b.run_ms(100);
    assert!(!b.status().playing.load(Relaxed), "STOP stops");
    // an instrument knob turns
    let before = b.project().clone();
    b.knob(4, 40);
    b.run_ms(100);
    // the rest does nothing: the steps, the other modes, the left screen's knobs
    let steps = b.project().tracks[0].steps;
    b.tap(pad_of(5));
    assert_eq!(b.project().tracks[0].steps, steps, "a pad does not write a step");
    for button in [Button::Mixer, Button::Arranger, Button::Sampling, Button::Browser, Button::Settings, Button::Scene, Button::Pattern, Button::Rec, Button::Erase, Button::Shift] {
        b.press(button);
        assert_eq!(b.ui().main, Main::Instrument, "{button:?} does nothing");
    }
    b.knob(0, 40);
    assert_eq!(b.project().bpm, before.bpm, "the tempo knob does nothing");
    // the lights say the same: only what works is lit
    for dark in [Button::Mixer, Button::Rec, Button::Erase, Button::Shift, Button::Settings, Button::Arranger] {
        let slot = octachrome_hw::leds::button_slot(dark).unwrap();
        assert_eq!(b.leds().slots[slot], 0, "{dark:?} is dark in the demo");
    }
    for lit in [Button::Play, Button::Stop, Button::File] {
        let slot = octachrome_hw::leds::button_slot(lit).unwrap();
        assert_ne!(b.leds().slots[slot], 0, "{lit:?} is lit in the demo");
    }
    // nothing is saved
    b.press(Button::File);
    b.tap(pad_of(9));
    assert_eq!(b.ui().file_note, octachrome::DEMO_NOTE);
}
