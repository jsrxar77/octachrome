//! Step sequencer (docs/features.md §S).
use op8::bench::Bench;
use op8_hw::{leds::button_slot, Button};

// F-S01: in STEP, tapping a pad toggles its step; the LED follows (level 3 on, 1 off).
#[test]
fn f_s01_tap_toggles_step_and_led() {
    let mut b = Bench::demo();
    b.press(Button::Group(1));
    b.press(Button::Step);
    let pad = 15; // step 3: off in the demo's kick track
    assert!(!b.project().tracks[1].steps[3].active, "precondition");
    assert_eq!(b.leds().pad(pad) & 3, 1, "inactive step should be dim");

    b.tap(pad);
    assert!(b.project().tracks[1].steps[3].active);
    assert_eq!(b.leds().pad(pad) & 3, 3, "active step should be bright");

    b.tap(pad);
    assert!(!b.project().tracks[1].steps[3].active);
}

// F-S02: holding a pad past 300 ms opens the inspector and does not toggle the step.
#[test]
fn f_s02_hold_opens_inspector_without_toggling() {
    let mut b = Bench::demo();
    b.press(Button::Group(1));
    let pad = 15;
    b.strike(pad);
    b.run_ms(350);
    assert_eq!(b.ui().inspect_step, Some(3), "inspector should be open on step 3");
    b.release(pad);
    assert!(!b.project().tracks[1].steps[3].active, "release after a hold must not toggle");
    b.capture("f-s02-inspector");
}

// F-S03: ERASE held + pad clears the step.
#[test]
fn f_s03_erase_clears_step() {
    let mut b = Bench::demo();
    b.press(Button::Group(1));
    assert!(b.project().tracks[1].steps[0].active, "precondition: step 1 is on");
    b.button_down(Button::Erase);
    b.tap(12); // pad 12 is step 1
    b.button_up(Button::Erase);
    assert!(!b.project().tracks[1].steps[0].active);
}

// F-S04: pads past the track length are dark.
#[test]
fn f_s04_pads_past_length_are_off() {
    let mut b = Bench::demo();
    b.press(Button::Group(2)); // track C (FmBass), length 7
    assert_eq!(b.project().tracks[2].length, 7, "precondition");
    assert_eq!(b.leds().pad(4), 0, "pad 4 is step 9, past length 7: must be off");
    assert_ne!(b.leds().pad(8) & 3, 0, "pad 8 is step 1, inside the track: must be lit");
}

// F-S05: a group selects its track; its LED is brighter than the others'.
#[test]
fn f_s05_group_selects_track() {
    let mut b = Bench::demo();
    b.press(Button::Group(4));
    assert_eq!(b.ui().track, 4);
    let lvl = |b: &Bench, g: u8| b.leds().slots[button_slot(Button::Group(g)).unwrap()] & 3;
    assert!(lvl(&b, 4) > lvl(&b, 0), "selected group should be brighter");
}

// F-S06: encoder 1 changes the track length in steps of one, clamped to 1..=16.
#[test]
fn f_s06_knob1_sets_length() {
    let mut b = Bench::demo();
    b.press(Button::Group(1));
    assert_eq!(b.project().tracks[1].length, 16, "precondition");
    b.knob(0, -200); // five detents
    assert_eq!(b.project().tracks[1].length, 11);
    b.knob(0, -20_000);
    assert_eq!(b.project().tracks[1].length, 1, "clamped at 1");
    b.knob(0, 20_000);
    assert_eq!(b.project().tracks[1].length, 16, "clamped at 16");
}

// F-S07: MUTE held + group toggles the track's mute.
#[test]
fn f_s07_mute_group_toggles_mute() {
    let mut b = Bench::demo();
    b.button_down(Button::Mute);
    b.press(Button::Group(7));
    b.button_up(Button::Mute);
    assert!(b.project().tracks[7].muted);
    b.button_down(Button::Mute);
    b.press(Button::Group(7));
    b.button_up(Button::Mute);
    assert!(!b.project().tracks[7].muted);
}

// F-S08: a muted track sends no new notes. Track H is the MIDI track.
#[test]
fn f_s08_muted_track_is_silent() {
    let mut b = Bench::demo();
    b.press(Button::Play);
    b.run_ms(1500);
    let before = b.midi_between(0, 1500).iter().filter(|m| m[0] & 0xf0 == 0x90).count();
    assert!(before > 0, "precondition: MIDI track should play notes before mute");

    b.button_down(Button::Mute);
    b.press(Button::Group(7));
    b.button_up(Button::Mute);
    let t = b.now_ms() + 50;
    b.run_ms(2500);
    let after = b.midi_between(t, b.now_ms()).iter().filter(|m| m[0] & 0xf0 == 0x90).count();
    assert_eq!(after, 0, "{after} note-ons after mute");
}
