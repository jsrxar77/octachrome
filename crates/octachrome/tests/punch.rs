//! Punch-in FX (docs/features.md §P).
use octachrome::bench::Bench;
use octachrome_hw::Button;

// F-P01: PERFORM switches to Punch mode.
#[test]
fn f_p01_perform_switches_mode() {
    let mut b = Bench::demo();
    b.press(Button::Perform);
    assert_eq!(b.ui().mode, octachrome_ui::Mode::Punch);
}

// F-P02: a pad engages its effect while held and releases it on lift.
#[test]
fn f_p02_pad_engages_effect_while_held() {
    let mut b = Bench::demo();
    b.press(Button::Perform);
    b.strike(0); // LPF
    assert!(b.ui().punch.is_some(), "effect not engaged while held");
    b.pressure(0, 0.9);
    b.release(0);
    assert!(b.ui().punch.is_none(), "effect still engaged after release");
}

// F-P03: STEP leaves Punch, and pressing PERFORM again does too: there is always a way back.
#[test]
fn f_p03_there_is_always_a_way_back_to_step() {
    let mut b = Bench::demo();
    b.press(Button::Perform);
    b.press(Button::Step);
    assert_eq!(b.ui().mode, octachrome_ui::Mode::Step);
    b.press(Button::Perform);
    b.press(Button::Perform);
    assert_eq!(b.ui().mode, octachrome_ui::Mode::Step, "PERFORM twice is back on STEP");
    b.press(Button::Chords);
    b.press(Button::Chords);
    assert_eq!(b.ui().mode, octachrome_ui::Mode::Step, "so is CHORDS twice");
    b.press(Button::NoteRepeat);
    b.press(Button::Settings);
    b.press(Button::Perform);
    assert_eq!(b.ui().mode, octachrome_ui::Mode::Punch, "from settings PERFORM enters Punch, it does not toggle out");
}

use std::sync::atomic::Ordering::Relaxed;

// F-P04: PAD MODE: on a drum kit pads 1-8 play the eight sounds and the rest are free; on any other instrument the pads
// climb the Brain's scale from its key.
#[test]
fn f_p04_pad_mode_plays_the_pads() {
    let mut b = Bench::demo();
    b.press(Button::Group(0)); // the drums (a kit)
    b.press(Button::PadMode);
    assert_eq!(b.ui().mode, octachrome_ui::Mode::Pads);
    b.run_ms(100);
    let played = |b: &Bench| b.status().note_count[0].load(Relaxed);
    let before = played(&b);
    b.strike(1); // pad 2: the snare
    b.run_ms(100);
    b.release(1);
    assert_eq!(played(&b), before + 1, "a pad of the kit plays");
    assert!(b.peak(100, 300) > 0.01, "and it is heard, with the sequence stopped");
    b.strike(12); // a free pad
    b.run_ms(50);
    b.release(12);
    assert_eq!(played(&b), before + 1, "pads 9-16 are free on a kit");
    // a melodic track: the scale from the Brain's key (C major: C D E F G A B, then up an octave)
    b.press(Button::Group(2));
    let notes: Vec<u8> = (0..8).map(|p| octachrome_ui::pad_note(b.ui(), b.project(), p).unwrap()).collect();
    assert_eq!(notes, [48, 50, 52, 53, 55, 57, 59, 60]);
    // pressing PAD MODE again goes back to STEP
    b.press(Button::PadMode);
    assert_eq!(b.ui().mode, octachrome_ui::Mode::Step);
}

// F-P05: KEYBOARD: the pads are a chromatic keyboard from C3, and the 4D encoder (left and right) moves the octave.
#[test]
fn f_p05_keyboard_is_chromatic_and_has_octaves() {
    let mut b = Bench::demo();
    b.press(Button::Group(2)); // FmBass
    b.press(Button::Keyboard);
    assert_eq!(b.ui().mode, octachrome_ui::Mode::Keys);
    let note = |b: &Bench, pad: u8| octachrome_ui::pad_note(b.ui(), b.project(), pad).unwrap();
    assert_eq!((note(&b, 0), note(&b, 1), note(&b, 15)), (48, 49, 63));
    b.press(Button::JogRight);
    assert_eq!(note(&b, 0), 60, "one octave up");
    b.press(Button::JogLeft);
    b.press(Button::JogLeft);
    assert_eq!(note(&b, 0), 36, "two octaves down");
    // a held pad sounds, and letting go releases it
    let played = |b: &Bench| b.status().note_count[2].load(Relaxed);
    let before = played(&b);
    b.strike(3);
    b.run_ms(200);
    assert_eq!(played(&b), before + 1);
    assert!(b.peak(0, 300) > 0.01, "a key is heard");
    b.release(3);
    // in settings the keyboard does nothing
    b.press(Button::Settings);
    let n = played(&b);
    b.strike(3);
    b.run_ms(100);
    b.release(3);
    assert_eq!(played(&b), n, "settings: the pads do nothing");
}

// F-L08: in PAD MODE and KEYBOARD the pads that play are lit in the track colour, the held pad is white, and (on the keyboard) every C is brighter.
#[test]
fn f_l08_play_modes_light_the_pads() {
    let mut b = Bench::demo();
    b.press(Button::Group(2));
    b.press(Button::Keyboard);
    b.tick();
    assert_eq!(b.leds().pad(0) & 3, 3, "the C is bright");
    assert_eq!(b.leds().pad(1) & 3, 1, "the other keys dim");
    b.strike(1);
    b.tick();
    assert_eq!((b.leds().pad(1) >> 2, b.leds().pad(1) & 3), (octachrome_hw::NiColor::White as u8, 3), "a held key is white");
    b.release(1);
}
