//! PATTERN: the sixteen patterns of a track (docs/features.md, F-K25).
use op8::bench::Bench;
use op8_hw::{Button, Screen};
use op8_ui::Main;
use std::sync::atomic::Ordering::Relaxed;

fn pad_of(slot: usize) -> u8 {
    (0..16u8).find(|&p| op8::Controller::pad_to_step(p) as usize == slot).unwrap()
}

// F-K25: PATTERN opens both screens and is lit (again goes back); the sixteen pads are the sixteen patterns of the selected track, the one
// that is shown is the brightest, a pad that has something in it is lit more than an empty one; a tap shows that pattern (the track's steps
// are now its own, empty to begin with) and what is put on it stays in it; going back to the first one has what it had.
#[test]
fn f_k25_the_pads_are_the_patterns_of_the_track() {
    let mut b = Bench::demo();
    b.press(Button::Group(0)); // track A: the drums
    let first = b.project().tracks[0];
    assert!(!first.is_empty());
    b.press(Button::Pattern);
    assert_eq!(b.ui().main, Main::Pattern);
    b.run_ms(100);
    b.capture("f-k25-patterns");
    assert_eq!(b.project().current[0], 0);
    assert!(b.leds().pad(pad_of(0)) & 3 == 3, "the one that is shown is the brightest");
    assert!(b.leds().pad(pad_of(1)) & 3 == 1, "an empty one is dim");
    b.tap(pad_of(3));
    assert_eq!(b.project().current[0], 3);
    assert!(b.project().tracks[0].is_empty(), "a new pattern is empty");
    assert!(b.ui().pattern_note.contains("pattern 4"), "{}", b.ui().pattern_note);
    // put a step on it from the steps view, and go back
    b.press(Button::Pattern);
    assert_eq!(b.ui().main, Main::Instrument);
    b.press(Button::Step);
    b.tap(pad_of(5));
    assert!(!b.project().tracks[0].is_empty());
    b.press(Button::Pattern);
    b.tap(pad_of(0));
    assert_eq!(b.project().tracks[0], first, "the first is as it was");
    assert!(b.leds().pad(pad_of(3)) & 3 == 2, "the one with a step is lit more than an empty one");
    // the 4D reaches the pads too
    assert_eq!(op8_ui::nav_items(b.ui()).len(), 16);
    // another track has its own
    b.press(Button::Group(2));
    assert_eq!(b.project().current[2], 0);
    // saved with the project
    let q = op8_seq::io::decode(&op8_seq::io::encode(b.project())).unwrap();
    assert_eq!(q.pattern(0, 3), b.project().pattern(0, 3));
}

// F-K25: a tap while playing waits for the start of the track's next loop (the pattern is shown at once, what plays does not change
// until then), SHIFT + a tap plays it at once; an empty pattern is silent and going back brings the sound back.
#[test]
fn f_k25_a_pattern_waits_for_the_loop_and_shift_does_not() {
    let mut b = Bench::empty();
    b.app.handle.seq(op8_seq::Command::SetStep { track: 1, step: 0, value: op8_seq::Step::on(36) });
    b.press(Button::Group(1)); // the kick
    b.press(Button::Pattern);
    b.press(Button::Play);
    b.run_ms(300);
    assert_eq!(b.status().pattern[1].load(Relaxed), 0);
    b.tap(pad_of(1)); // not now
    assert_eq!(b.project().current[1], 1, "shown at once");
    assert_eq!(b.status().pattern[1].load(Relaxed), 0, "but it still plays the first until its loop ends");
    b.run_ms(2200);
    assert_eq!(b.status().pattern[1].load(Relaxed), 1, "from the next loop it plays the new one");
    let from = b.now_ms();
    b.run_ms(2200);
    assert!(b.silent(from + 200, b.now_ms()), "an empty pattern is silent");
    b.button_down(Button::Shift);
    b.tap(pad_of(0));
    b.button_up(Button::Shift);
    b.run_ms(60);
    assert_eq!(b.status().pattern[1].load(Relaxed), 0, "SHIFT: at once");
    let from = b.now_ms();
    b.run_ms(1500);
    assert!(b.peak_db(from, b.now_ms()) > -40.0, "and the kick is back: {}", b.peak_db(from, b.now_ms()));
    b.press(Button::Stop);
}

// F-K25: a pad held and another tapped copies the first over the second (and the held one is not shown); ERASE and a tap empties one.
#[test]
fn f_k25_copy_and_empty() {
    let mut b = Bench::demo();
    b.press(Button::Group(0));
    let first = b.project().pattern(0, 0).unwrap();
    b.press(Button::Pattern);
    b.strike(pad_of(0)); // held
    b.tap(pad_of(6));
    b.release(pad_of(0));
    assert_eq!(b.project().pattern(0, 6), Some(first), "copied");
    assert_eq!(b.project().current[0], 0, "and nothing was switched");
    assert!(b.ui().pattern_note.contains("copied"), "{}", b.ui().pattern_note);
    b.button_down(Button::Erase);
    b.tap(pad_of(6));
    b.button_up(Button::Erase);
    assert!(b.project().pattern(0, 6).unwrap().is_empty(), "emptied");
    assert_eq!(b.project().current[0], 0);
    b.run_ms(100);
    let _ = Screen::Left;
}
