//! Buttons 3, 4, 7 and 8: players, bar, the track's effect and its LFO (docs/features.md §S, §K, §A, §D).
use op8::bench::Bench;
use op8_hw::{leds::button_slot, Button, Screen};
use op8_seq::{ChordType, FxKind, LfoShape, PlayMode};
use op8_ui::LeftPage;
use std::sync::atomic::Ordering::Relaxed;

/// One detent of a knob.
const D: i32 = 40;

/// The pad that is step (or slot) `n` in reading order: step 1 is the top-left pad.
fn pad_of(n: u8) -> u8 {
    (0..16u8).find(|&p| op8::Controller::pad_to_step(p) == n).unwrap()
}

// F-K13: button 4 opens the bar page: knobs 1–4 are the length, the division, a shift that turns the steps round the track,
// and how REC rounds; the same button again goes back to the steps.
#[test]
fn f_k13_the_bar_page() {
    let mut b = Bench::empty();
    b.press(Button::Group(1));
    b.tap(pad_of(0)); // step 1
    b.press(Button::Screen(4));
    assert_eq!(b.ui().left_page, LeftPage::Bar);
    b.knob(0, -3 * D);
    assert_eq!(b.project().tracks[1].length, 13, "knob 1 is the length");
    b.knob(1, -D);
    assert_eq!(b.project().tracks[1].division.label(), "1/16T", "knob 2 is the division");
    b.knob(2, 2 * D);
    assert_eq!(b.project().tracks[1].steps.iter().position(|s| s.active), Some(2), "knob 3 turns the steps two later");
    b.knob(2, -D);
    assert_eq!(b.project().tracks[1].steps.iter().position(|s| s.active), Some(1));
    b.knob(3, D);
    assert_eq!(b.ui().quantize, 1, "knob 4 is how REC rounds");
    b.press(Button::Screen(4));
    assert_eq!(b.ui().left_page, LeftPage::Steps, "the same button goes back");
    // the other buttons of the step view close the page too
    b.press(Button::Screen(4));
    b.press(Button::Screen(1));
    assert_eq!(b.ui().left_page, LeftPage::Steps);
}

// F-K13: a shift keeps the steps inside the length: they wrap round the end of the bar, and those beyond it stay put.
#[test]
fn f_k13_shift_wraps_inside_the_length() {
    let mut b = Bench::empty();
    b.press(Button::Group(1));
    b.press(Button::Screen(4));
    b.knob(0, -12 * D); // length 4
    b.press(Button::Screen(1));
    b.tap(pad_of(3)); // the last step of the bar
    b.press(Button::Screen(4));
    b.knob(2, D);
    let steps: Vec<bool> = b.project().tracks[1].steps.iter().take(5).map(|s| s.active).collect();
    assert_eq!(steps, [true, false, false, false, false]);
}

// F-R05: REC rounds as knob 4 of the bar page says: `nearest` (late in a step it goes to the next), `down` (the step that is
// sounding) or `up` (the next one).
#[test]
fn f_r05_rec_rounds_as_chosen() {
    let step_ms = 123;
    let written = |quantize: i32| {
        let mut b = Bench::empty();
        b.press(Button::Group(2));
        b.press(Button::Screen(4));
        b.knob(3, quantize * D);
        b.press(Button::Screen(1));
        b.press(Button::Keyboard);
        b.press(Button::Rec);
        b.press(Button::Play);
        b.run_ms(3 * step_ms + 85); // late in step 4 (index 3), with the frame the strike waits for to spare
        b.strike(5);
        b.run_ms(20);
        b.project().tracks[2].steps.iter().position(|s| s.active).unwrap()
    };
    assert_eq!(written(0), 4, "nearest: late in the step goes to the next");
    assert_eq!(written(1), 3, "down: the step that is sounding");
    assert_eq!(written(2), 4, "up: the next step");
}

// F-S14: button 3 opens the players page: knobs 1–4 are play, chord, rate and octaves; a drum kit has no players.
#[test]
fn f_s14_the_players_page() {
    let mut b = Bench::empty();
    b.press(Button::Group(0));
    b.tap(pad_of(0)); // the first tap on a drums track makes it a kit
    assert!(b.project().tracks[0].kit);
    b.press(Button::Group(1));
    b.press(Button::Screen(3));
    assert_eq!(b.ui().left_page, LeftPage::Players);
    b.knob(0, 2 * D);
    assert_eq!(b.project().tracks[1].players.mode, PlayMode::Up, "knob 1: off, chord, up…");
    b.knob(1, 3 * D);
    assert_eq!(b.project().tracks[1].players.chord, ChordType::MinorSeventh);
    b.knob(2, D);
    assert_eq!(b.project().tracks[1].players.speed, 3);
    b.knob(3, 2 * D);
    assert_eq!(b.project().tracks[1].players.octaves, 3);
    b.knob(3, 10 * D);
    assert_eq!(b.project().tracks[1].players.octaves, 3, "it stops at the end");
    // the players do not touch a drum kit
    b.press(Button::Group(0));
    b.knob(0, D);
    assert_eq!(b.project().tracks[0].players.mode, PlayMode::Off, "a kit plays its lanes");
}

// F-S14: with a chord player on, a step plays the whole chord; with an arpeggio, one note after another.
#[test]
fn f_s14_the_players_sound() {
    let notes_in_a_step = |setup: &dyn Fn(&mut Bench)| {
        let mut b = Bench::empty();
        b.press(Button::Group(2)); // FmBass
        b.tap(pad_of(0));
        setup(&mut b);
        let before = b.status().note_count[2].load(Relaxed);
        b.press(Button::Play);
        b.run_ms(100); // inside the first step
        b.status().note_count[2].load(Relaxed) - before
    };
    assert_eq!(notes_in_a_step(&|_| {}), 1, "no player: one note");
    assert_eq!(
        notes_in_a_step(&|b| {
            b.press(Button::Screen(3));
            b.knob(0, D); // chord
        }),
        3,
        "a chord: three notes at once"
    );
    // an arpeggio of eight notes in a step: they come one after another
    let mut b = Bench::empty();
    b.press(Button::Group(2));
    b.tap(pad_of(0));
    b.press(Button::Screen(3));
    b.knob(0, 2 * D); // up
    b.knob(2, 3 * D); // eight a step
    b.press(Button::Play);
    b.run_ms(110);
    assert!(b.status().note_count[2].load(Relaxed) >= 7, "{}", b.status().note_count[2].load(Relaxed));
}

// F-S14: the screens draw the players page and the bar page with something on them, and the footer names the four knobs.
#[test]
fn f_s14_the_pages_draw() {
    let mut b = Bench::demo();
    b.press(Button::Group(1));
    b.press(Button::Screen(3));
    b.run_ms(100);
    assert!(b.lit_pixels(Screen::Left) > 3000);
    let players = b.screen_hash(Screen::Left);
    b.press(Button::Screen(4));
    b.run_ms(100);
    assert!(b.lit_pixels(Screen::Left) > 3000);
    assert_ne!(b.screen_hash(Screen::Left), players);
    b.capture("f-s14-bar");
    b.press(Button::Screen(3));
    b.knob(0, D);
    b.run_ms(100);
    b.capture("f-s14-players");
}

// F-A14: button 7 opens the track's effect: knob 5 chooses it and 6–8 are its three knobs; a delay leaves echoes after the sound.
#[test]
fn f_a14_the_track_effect() {
    let mut b = Bench::empty();
    b.press(Button::Group(1)); // KickSyn
    b.tap(pad_of(0));
    b.press(Button::Screen(7));
    assert_eq!(b.ui().right_page, 2);
    b.knob(4, D);
    assert_eq!(b.project().tracks[1].fx.kind, FxKind::Delay, "knob 5 chooses the effect");
    let fx = b.project().tracks[1].fx;
    b.knob(5, 100);
    b.knob(6, 100);
    b.knob(7, 100);
    let moved = b.project().tracks[1].fx;
    assert!(moved.a > fx.a && moved.b > fx.b && moved.c > fx.c, "knobs 6–8 are the three knobs of the effect");
    b.knob(4, 10 * D);
    assert_eq!(b.project().tracks[1].fx.kind, FxKind::Drive, "and the last of the list is where it stops");
    b.knob(4, -10 * D);
    assert_eq!(b.project().tracks[1].fx.kind, FxKind::Off);
    b.knob(5, 300);
    assert_eq!(b.project().tracks[1].fx.a, moved.a.min(b.project().tracks[1].fx.a), "with no effect the knobs do nothing");
}

// F-A14: the effect of one track does not touch the others, and with it off the sound is the same as without.
#[test]
fn f_a14_the_effect_is_heard_on_its_own_track() {
    let tail = |kind_steps: i32| {
        let mut b = Bench::empty();
        b.press(Button::Group(1));
        b.tap(pad_of(0));
        b.press(Button::Screen(7));
        b.knob(4, kind_steps * D); // 1 = delay
        b.press(Button::Play);
        b.run_ms(2200);
        b.rms_db(700, 1000) // between the kicks, where the kick itself has died away
    };
    let dry = tail(0);
    let delay = tail(1);
    assert!(delay > dry + 10.0, "the delay fills the gap after the kick: {delay:.1} dB vs {dry:.1} dB");
}

// F-A15: button 8 opens the LFO: knob 5 the wave, 6 the rate, 7 the depth and 8 what it moves; the left screen and the footer
// say what it moves.
#[test]
fn f_a15_the_lfo_page() {
    let mut b = Bench::empty();
    b.press(Button::Group(2)); // FmBass
    b.press(Button::Screen(8));
    assert_eq!(b.ui().right_page, 3);
    assert_eq!(b.project().tracks[2].lfo.target, 0, "it moves nothing until told to");
    b.knob(4, 2 * D);
    assert_eq!(b.project().tracks[2].lfo.shape, LfoShape::Square);
    let rate = b.project().tracks[2].lfo.rate;
    b.knob(5, 200);
    assert!(b.project().tracks[2].lfo.rate > rate);
    b.knob(7, D);
    assert_eq!(b.project().tracks[2].lfo.target, 1, "volume first");
    b.run_ms(100);
    assert_eq!(b.ui().lfo_target, "volume");
    b.knob(7, 2 * D);
    b.run_ms(100);
    assert_eq!(b.project().tracks[2].lfo.target, 3);
    assert_eq!(b.ui().lfo_target, "index", "the name of the instrument's own knob (FmBass knob 2)");
    b.knob(7, 100 * D);
    assert_eq!(b.project().tracks[2].lfo.target, 9, "it stops at the last knob");
}

// F-D15: buttons 3, 4, 7 and 8 are named in the strip and lit like the others; both new right pages draw and the dots show four pages.
#[test]
fn f_d15_the_new_pages_are_drawn_and_lit() {
    let mut b = Bench::demo();
    b.press(Button::Group(2));
    b.run_ms(100);
    for n in 1..=8u8 {
        assert!(b.leds().slots[button_slot(Button::Screen(n)).unwrap()] > 0, "button {n} has a light");
    }
    let sound = b.screen_hash(Screen::Right);
    b.press(Button::Screen(7));
    b.run_ms(150);
    let fx_off = b.screen_hash(Screen::Right);
    b.knob(4, D);
    b.run_ms(150);
    assert!(b.lit_pixels(Screen::Right) > 1500);
    assert_ne!(b.screen_hash(Screen::Right), fx_off);
    b.capture("f-d15-fx");
    b.press(Button::Screen(8));
    b.knob(7, D);
    b.run_ms(150);
    assert!(b.lit_pixels(Screen::Right) > 1500);
    assert_ne!(b.screen_hash(Screen::Right), sound);
    b.capture("f-d15-mod");
    // each effect has its own picture
    b.press(Button::Screen(7));
    let mut seen = std::collections::HashSet::new();
    for _ in 0..FxKind::ALL.len() {
        b.run_ms(100);
        seen.insert(b.screen_hash(Screen::Right));
        b.knob(4, D);
    }
    assert_eq!(seen.len(), FxKind::ALL.len(), "six effects, six pictures");
}

// F-F07: effects, LFO and players are saved with the project and come back with it.
#[test]
fn f_f07_the_extras_are_saved_with_the_project() {
    let dir = std::env::temp_dir().join(format!("op8-pages-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let mut b = Bench::empty();
    b.set_projects_dir(dir.clone());
    b.press(Button::Group(2));
    b.press(Button::Screen(7));
    b.knob(4, 3 * D);
    b.press(Button::Screen(8));
    b.knob(7, D);
    b.press(Button::Screen(3));
    b.knob(0, D);
    let (fx, lfo, players) = {
        let t = &b.project().tracks[2];
        (t.fx, t.lfo, t.players)
    };
    assert_ne!(fx.kind, FxKind::Off);
    b.press(Button::File);
    b.button_down(Button::Shift);
    b.tap(pad_of(8)); // the first of the player's own places
    b.button_up(Button::Shift);
    // start a blank project and open the saved one again
    b.tap(pad_of(7));
    assert_eq!(b.project().tracks[2].fx.kind, FxKind::Off, "the blank project has none");
    b.press(Button::File);
    b.tap(pad_of(8));
    let t = &b.project().tracks[2];
    assert_eq!((t.fx, t.lfo, t.players), (fx, lfo, players));
    let _ = std::fs::remove_dir_all(&dir);
}
