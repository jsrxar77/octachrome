//! Punch-in FX (docs/features.md §P).
use op8::bench::Bench;
use op8_hw::Button;

// F-P01: PERFORM switches to Punch mode.
#[test]
fn f_p01_perform_switches_mode() {
    let mut b = Bench::demo();
    b.press(Button::Perform);
    assert_eq!(b.ui().mode, op8_ui::Mode::Punch);
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
    assert_eq!(b.ui().mode, op8_ui::Mode::Step);
    b.press(Button::Perform);
    b.press(Button::Perform);
    assert_eq!(b.ui().mode, op8_ui::Mode::Step, "PERFORM twice is back on STEP");
    b.press(Button::Chords);
    b.press(Button::Chords);
    assert_eq!(b.ui().mode, op8_ui::Mode::Step, "so is CHORDS twice");
    b.press(Button::NoteRepeat);
    b.press(Button::Settings);
    b.press(Button::Perform);
    assert_eq!(b.ui().mode, op8_ui::Mode::Punch, "from settings PERFORM enters Punch, it does not toggle out");
}
