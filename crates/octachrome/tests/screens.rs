//! Screens (docs/features.md §D).
use octachrome::bench::Bench;
use octachrome_hw::{Button, Screen};

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
    assert_eq!(b.ui().mode, octachrome_ui::Mode::Step);
}

/// Centres of the four screen buttons (see `octachrome_ui::theme::BUTTON_CX`).
const CX: [usize; 4] = [24, 164, 304, 444];

/// Lit pixels in the strip (the top 24 rows) around one screen button.
fn strip_lit(b: &Bench, screen: Screen, col: usize) -> usize {
    let fb = b.screen(screen);
    let mut n = 0;
    for y in 0..24usize {
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
    b.press(Button::Group(6)); // track G: macro, with two parameter pages
    b.run_ms(100);
    // Left: step, components, players, bar. Right: shape, sound, fx, mod.
    for col in 0..4 {
        assert!(strip_lit(&b, Screen::Left, col) > 20, "left button {}", col + 1);
        assert!(strip_lit(&b, Screen::Right, col) > 20, "right button {}", col + 5);
    }
    // The capsule is centred on its button: its top edge is lit at the centre and only there, not at the sides of the column.
    let edge = |b: &Bench, col: usize, dx: i32| b.screen(Screen::Right).pixels()[480 + (CX[col] as i32 + dx) as usize];
    assert_ne!(edge(&b, 1, 0), 0, "the capsule's top edge is at the button's centre");
    assert_eq!(edge(&b, 1, 55), 0, "and does not run past its text");
    // The page that is open has the bright outline (fg), the other the faint one.
    let t = octachrome_gfx::theme();
    assert_eq!(edge(&b, 0, 0), t.fg.0, "sound (button 5) is open");
    assert_eq!(edge(&b, 1, 0), t.faint.0);
    b.press(Button::Screen(6));
    b.run_ms(100);
    assert_eq!(edge(&b, 1, 0), t.fg.0, "shape (button 6) is open");
    assert_eq!(edge(&b, 0, 0), t.faint.0);
    // An instrument with one page has no shape button: that capsule is empty (the sampler-like case is the MIDI out).
    b.press(Button::Screen(5));
    b.press(Button::Group(7)); // track H: the MIDI out has no second page
    b.run_ms(100);
    assert_eq!(strip_lit(&b, Screen::Right, 1), 0, "no shape button on an instrument without a second page");
    assert!(strip_lit(&b, Screen::Right, 2) > 20 && strip_lit(&b, Screen::Right, 3) > 20, "but fx and mod are there");
    // In settings the four buttons over the left screen are its pages, and the right screen's buttons do nothing, so nothing is named there.
    b.press(Button::Settings);
    b.run_ms(100);
    assert!(strip_lit(&b, Screen::Left, 1) > 20 && strip_lit(&b, Screen::Left, 2) > 20, "the pages are named on the left");
    let fb = b.screen(Screen::Right);
    let lit = (0..24usize).flat_map(|y| (120..360usize).map(move |x| (x, y))).filter(|&(x, y)| fb.pixels()[y * 480 + x] != 0).count();
    assert_eq!(lit, 0, "no strip on the right screen of settings");
}

// F-D11: on the step grid an active step and an empty one are far apart in brightness.
#[test]
fn f_d11_active_and_empty_steps_are_far_apart() {
    let mut b = Bench::demo();
    b.press(Button::Group(1));
    b.run_ms(100);
    let lum = |b: &Bench, step: usize| {
        let (x, y) = (262 + (step % 4) * 42 + 18, 50 + (step / 4) * 42 + 18);
        let p = b.screen(Screen::Left).pixels()[y * 480 + x];
        let (r, g, bl) = ((p >> 11) & 31, (p >> 5) & 63, p & 31);
        (r as u32 * 8 + g as u32 * 4 + bl as u32 * 8) as f32
    };
    let steps: Vec<_> = (0..16).map(|s| (s, b.project().tracks[1].steps[s].active)).collect();
    let on = steps.iter().find(|s| s.1).expect("an active step").0;
    let off = steps.iter().find(|s| !s.1).expect("an empty step").0;
    assert!(lum(&b, on) > 5.0 * lum(&b, off).max(1.0), "active {} vs empty {}", lum(&b, on), lum(&b, off));
}

// F-D13: the text in a capsule is centred in it, by its ink, within a pixel (design.md §3).
#[test]
fn f_d13_capsule_text_is_centred() {
    let mut b = Bench::demo();
    b.press(Button::Group(6));
    b.run_ms(200);
    for (screen, cols) in [(Screen::Left, [0usize, 1]), (Screen::Right, [0, 1])] {
        let fb = b.screen(screen);
        for col in cols {
            // The capsule: where its top edge (row 1) is lit around the button.
            let xs: Vec<usize> = (CX[col].saturating_sub(70)..(CX[col] + 70).min(480)).filter(|&x| fb.pixels()[480 + x] != 0).collect();
            let (c0, c1) = (*xs.first().unwrap(), *xs.last().unwrap());
            // The ink: bright pixels inside, away from the 1 px outline and the rounded ends.
            let bright = |x: usize, y: usize| {
                let p = fb.pixels()[y * 480 + x];
                (p >> 11) + ((p >> 5) & 63) / 2 + (p & 31) > 12
            };
            let ink: Vec<(usize, usize)> = (4..21).flat_map(|y| (c0 + 8..c1 - 8).map(move |x| (x, y))).filter(|&(x, y)| bright(x, y)).collect();
            let (x0, x1) = (ink.iter().map(|p| p.0).min().unwrap(), ink.iter().map(|p| p.0).max().unwrap());
            let (y0, y1) = (ink.iter().map(|p| p.1).min().unwrap(), ink.iter().map(|p| p.1).max().unwrap());
            let (cx, cy) = ((c0 + c1) as f32 / 2.0, 12.0);
            assert!(((x0 + x1) as f32 / 2.0 - cx).abs() <= 1.0, "{screen:?} {col}: ink x {x0}..{x1} in capsule {c0}..{c1}");
            assert!(((y0 + y1) as f32 / 2.0 - cy).abs() <= 2.5, "{screen:?} {col}: ink y {y0}..{y1}");
        }
    }
}

// F-D14: the LED of each of the eight screen buttons agrees with what the screens say: no label, no light; a label, dim; the open page, full.
#[test]
fn f_d14_screen_button_leds_match_the_labels() {
    use octachrome_hw::leds::button_slot;
    let led = |b: &Bench, n: u8| b.leds().slots[button_slot(Button::Screen(n)).unwrap()];
    let mut b = Bench::demo();
    b.press(Button::Group(6)); // track G: macro, two pages
    b.tick();
    let labels = |b: &Bench| octachrome_ui::screen_buttons(b.ui());
    let all = labels(&b);
    assert!(all[0].is_some() && all[1].is_some() && all[4].is_some() && all[5].is_some(), "step, components, sound, shape");
    for n in 1..=8u8 {
        assert_eq!(led(&b, n) > 0, all[n as usize - 1].is_some(), "button {n}: lit exactly when it has a label");
    }
    assert!(led(&b, 5) > led(&b, 6) && led(&b, 1) > led(&b, 2), "the open page (1 and 5) is brighter than the other");
    b.press(Button::Screen(6));
    b.tick();
    assert!(led(&b, 6) > led(&b, 5), "shape (button 6) is open now");
    // A track whose instrument has one page (track E, the MIDI bridge) has no `shape`: no label, and the button is dark.
    b.press(Button::Group(7));
    b.tick();
    assert_eq!(led(&b, 6), 0, "no second page, no light");
    assert!(led(&b, 5) > 0);
    // In settings the four pages are named and lit (the open one brighter); the right screen's buttons are dark.
    b.press(Button::Settings);
    b.tick();
    for n in 1..=4u8 {
        assert!(led(&b, n) > 0, "settings: button {n} is a page");
    }
    assert!(led(&b, 1) > led(&b, 2), "the open page is brighter");
    for n in 5..=8u8 {
        assert_eq!(led(&b, n), 0, "settings: button {n} is dark");
    }
}

// F-D17: choosing a track puts both screens on their first page: the steps on the left (button 1) and `sound` on the
// right (button 5), whatever page was open.
#[test]
fn f_d17_choosing_a_track_shows_buttons_1_and_5() {
    use octachrome_hw::leds::button_slot;
    let led = |b: &Bench, n: u8| b.leds().slots[button_slot(Button::Screen(n)).unwrap()];
    let mut b = Bench::demo();
    b.press(Button::Screen(3)); // players on the left
    b.press(Button::Screen(6)); // shape on the right
    b.tick();
    assert!(led(&b, 3) > led(&b, 1) && led(&b, 6) > led(&b, 5), "other pages are open");
    b.press(Button::Group(6)); // a track with two pages
    b.tick();
    assert_eq!(b.ui().right_page, 0);
    assert_eq!(b.ui().left_page, octachrome_ui::LeftPage::Steps);
    assert!(led(&b, 1) > led(&b, 3) && led(&b, 5) > led(&b, 6), "buttons 1 and 5 are the open ones");
}
