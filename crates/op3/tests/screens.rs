//! Screens (docs/features.md §D).
use op3::bench::Bench;
use op3_hw::{Button, Screen};

// F-D01: both screens show content on the first frame.
#[test]
fn f_d01_both_screens_show_content() {
    let mut b = Bench::demo();
    b.run_ms(50);
    for s in [Screen::Left, Screen::Right] {
        assert!(b.lit_pixels(s) > 1000, "{s:?} has only {} lit pixels", b.lit_pixels(s));
    }
    b.capture("f-d01-start");
}

// F-D02: the background is pure black.
#[test]
fn f_d02_background_is_black() {
    let mut b = Bench::demo();
    b.run_ms(50);
    for s in [Screen::Left, Screen::Right] {
        let f = b.black_fraction(s);
        assert!(f > 0.4, "{s:?} is only {:.0}% black", f * 100.0);
    }
}

// F-D03: an idle scene keeps animating, but only a small part of the screen is
// reported as changed each frame (budget: 5 % of a full frame, averaged over
// one second). What the hardware then sends is worked out by its display
// thread (see docs/architecture.md §7).
#[test]
fn f_d03_idle_scene_sends_a_small_part() {
    let mut b = Bench::empty();
    b.run_ms(1000); // settle any first-frame animation
    let sent = b.pixels_sent();
    b.run_ms(1000);
    let per_frame = (b.pixels_sent() - sent) as f64 / 60.0;
    let full_frame = 2.0 * 480.0 * 272.0;
    let share = per_frame / full_frame * 100.0;
    assert!(share < 5.0, "idle scene sends {share:.1} % of a full frame per frame");
}

// F-D04: STEP and PERFORM look different. (Returning to STEP cannot be checked
// by comparing hashes: the screen keeps animating, so it never repeats exactly.)
#[test]
fn f_d04_mode_changes_screen() {
    let mut b = Bench::demo();
    b.run_ms(50);
    let step = b.screen_hash(Screen::Left);
    b.press(Button::Perform);
    b.run_ms(50);
    let perform = b.screen_hash(Screen::Left);
    assert_ne!(step, perform, "STEP and PERFORM look the same");
    b.press(Button::Step);
    assert_eq!(b.ui().mode, op3_ui::Mode::Step);
}
