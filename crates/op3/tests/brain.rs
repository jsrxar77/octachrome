//! Brain (docs/features.md §B).
use op3::bench::Bench;
use op3_hw::{Button, Screen};

// F-B01: CHORDS switches to Brain mode and the left screen changes.
#[test]
fn f_b01_chords_switches_mode() {
    let mut b = Bench::demo();
    let step_hash = b.screen_hash(Screen::Left);
    b.press(Button::Chords);
    assert_eq!(b.ui().mode, op3_ui::Mode::Brain);
    assert_ne!(b.screen_hash(Screen::Left), step_hash, "left screen did not change");
    b.capture("f-b01-brain");
}

// F-B02: holding a pad plays a live chord; releasing it clears the chord.
#[test]
fn f_b02_hold_plays_live_chord() {
    let mut b = Bench::demo();
    b.press(Button::Chords);
    b.strike(3);
    assert!(b.project().brain.live.is_some(), "no live chord while the pad is held");
    b.release(3);
    assert!(b.project().brain.live.is_none(), "live chord left behind after release");
}

// F-B03: SHIFT + group toggles whether the track follows the Brain.
#[test]
fn f_b03_shift_group_toggles_brain_link() {
    let mut b = Bench::demo();
    let before = b.project().tracks[1].brain_link;
    b.button_down(Button::Shift);
    b.press(Button::Group(1));
    b.button_up(Button::Shift);
    assert_eq!(b.project().tracks[1].brain_link, !before);
    b.button_down(Button::Shift);
    b.press(Button::Group(1));
    b.button_up(Button::Shift);
    assert_eq!(b.project().tracks[1].brain_link, before);
}

// F-B04: encoder 1 in Brain steps the key by one semitone, wrapping 11 → 0 → 11.
#[test]
fn f_b04_knob1_steps_key() {
    let mut b = Bench::demo();
    b.press(Button::Chords);
    let k0 = b.project().brain.key;
    b.knob(0, 40);
    assert_eq!(b.project().brain.key, (k0 + 1) % 12);
    b.knob(0, -40 * 2);
    assert_eq!(b.project().brain.key, (k0 + 11) % 12, "wrap below zero");
}
