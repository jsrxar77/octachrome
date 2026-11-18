//! Brain (docs/features.md §B).
use octachrome::bench::Bench;
use octachrome_hw::{Button, Screen};

// F-B01: CHORDS switches to Brain mode and the left screen changes.
#[test]
fn f_b01_chords_switches_mode() {
    let mut b = Bench::demo();
    let step_hash = b.screen_hash(Screen::Left);
    b.press(Button::Chords);
    assert_eq!(b.ui().mode, octachrome_ui::Mode::Brain);
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
    let before = b.project().tracks[2].brain_link;
    b.button_down(Button::Shift);
    b.press(Button::Group(2));
    b.button_up(Button::Shift);
    assert_eq!(b.project().tracks[2].brain_link, !before);
    b.button_down(Button::Shift);
    b.press(Button::Group(2));
    b.button_up(Button::Shift);
    assert_eq!(b.project().tracks[2].brain_link, before);
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

// F-B07: a CHORDS pad sounds the chord on the selected track's instrument while it is held (and still sets the live
// chord for the tracks linked to the Brain); a drum kit has no chords, so it stays silent.
#[test]
fn f_b07_a_chord_pad_sounds_the_chord() {
    use octachrome_hw::Button;
    use std::sync::atomic::Ordering::Relaxed;
    let mut b = Bench::demo();
    b.press(Button::Group(2)); // FmBass
    b.press(Button::Chords);
    let played = |b: &Bench, t: usize| b.status().note_count[t].load(Relaxed);
    let before = played(&b, 2);
    b.strike(0); // I, a triad: three notes
    b.run_ms(200);
    assert_eq!(played(&b, 2) - before, 3, "the three notes of the triad");
    assert!(b.peak(0, 300) > 0.01, "and it is heard with the sequence stopped");
    assert!(b.project().brain.live.is_some(), "the live chord is still set for the linked tracks");
    b.release(0);
    assert!(b.project().brain.live.is_none());
    // a seventh chord has four notes
    let before = played(&b, 2);
    b.strike(8);
    b.run_ms(50);
    b.release(8);
    assert_eq!(played(&b, 2) - before, 4);
    // the drum kit: no sound, but the live chord is still set
    b.press(Button::Group(0));
    let before = played(&b, 0);
    b.strike(0);
    b.run_ms(50);
    assert!(b.project().brain.live.is_some());
    b.release(0);
    assert_eq!(played(&b, 0), before, "a kit does not play chords");
}
