//! Screens (docs/features.md §D).
use op8::bench::Bench;
use op8_hw::{Button, Screen};

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
    assert_eq!(b.ui().mode, op8_ui::Mode::Step);
}

/// Centres of the four screen buttons (see `op8_ui::theme::BUTTON_CX`).
const CX: [usize; 4] = [24, 164, 304, 444];

/// Lit pixels in the strip (the top 20 rows) around one screen button.
fn strip_lit(b: &Bench, screen: Screen, col: usize) -> usize {
    let fb = b.screen(screen);
    let mut n = 0;
    for y in 0..20usize {
        for x in CX[col].saturating_sub(60)..(CX[col] + 60).min(480) {
            if fb.pixels()[y * 480 + x] != 0 {
                n += 1;
            }
        }
    }
    n
}

// F-D10: each screen names what its buttons do, in a capsule centred on the button; a button with no function shows nothing.
#[test]
fn f_d10_screens_name_their_buttons() {
    let mut b = Bench::demo();
    b.press(Button::Group(3)); // track D: macro, with two parameter pages
    b.run_ms(100);
    // Left: step, components; 3 and 4 do nothing.
    assert!(strip_lit(&b, Screen::Left, 0) > 20 && strip_lit(&b, Screen::Left, 1) > 20);
    assert_eq!(strip_lit(&b, Screen::Left, 2) + strip_lit(&b, Screen::Left, 3), 0);
    // Right: sound, shape; 7 and 8 do nothing.
    assert!(strip_lit(&b, Screen::Right, 0) > 20 && strip_lit(&b, Screen::Right, 1) > 20);
    assert_eq!(strip_lit(&b, Screen::Right, 2) + strip_lit(&b, Screen::Right, 3), 0);
    // The capsule is centred on its button: its top edge is lit at the centre and only there, not at the sides of the column.
    let edge = |b: &Bench, col: usize, dx: i32| b.screen(Screen::Right).pixels()[480 + (CX[col] as i32 + dx) as usize];
    assert_ne!(edge(&b, 1, 0), 0, "the capsule's top edge is at the button's centre");
    assert_eq!(edge(&b, 1, 55), 0, "and does not run past its text");
    // The page that is open has the bright outline (fg), the other the faint one.
    let t = op8_gfx::theme();
    assert_eq!(edge(&b, 0, 0), t.fg.0, "sound is open");
    assert_eq!(edge(&b, 1, 0), t.faint.0);
    b.press(Button::Screen(6));
    b.run_ms(100);
    assert_eq!(edge(&b, 1, 0), t.fg.0, "shape is open");
    assert_eq!(edge(&b, 0, 0), t.faint.0);
    // In settings the buttons do nothing, so nothing is named.
    b.press(Button::Settings);
    b.run_ms(100);
    for screen in [Screen::Left, Screen::Right] {
        let fb = b.screen(screen);
        let lit = (0..20usize).flat_map(|y| (120..360usize).map(move |x| (x, y))).filter(|&(x, y)| fb.pixels()[y * 480 + x] != 0).count();
        assert_eq!(lit, 0, "{screen:?}: no strip in settings");
    }
}

// F-D11: on the step grid an active step and an empty one are far apart in brightness.
#[test]
fn f_d11_active_and_empty_steps_are_far_apart() {
    let mut b = Bench::demo();
    b.press(Button::Group(0));
    b.run_ms(100);
    let lum = |b: &Bench, step: usize| {
        let (x, y) = (262 + (step % 4) * 46 + 20, 42 + (step / 4) * 46 + 20);
        let p = b.screen(Screen::Left).pixels()[y * 480 + x];
        let (r, g, bl) = ((p >> 11) & 31, (p >> 5) & 63, p & 31);
        (r as u32 * 8 + g as u32 * 4 + bl as u32 * 8) as f32
    };
    let steps: Vec<_> = (0..16).map(|s| (s, b.project().tracks[0].steps[s].active)).collect();
    let on = steps.iter().find(|s| s.1).expect("an active step").0;
    let off = steps.iter().find(|s| !s.1).expect("an empty step").0;
    assert!(lum(&b, on) > 5.0 * lum(&b, off).max(1.0), "active {} vs empty {}", lum(&b, on), lum(&b, off));
}
