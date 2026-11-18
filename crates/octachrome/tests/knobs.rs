//! Encoders 5–8 and the touch overlay (docs/features.md §K).
use octachrome::bench::Bench;
use octachrome_cartridge::InstrumentCartridge;
use octachrome_hw::{Button, Screen};

// F-K01: encoder 5 moves parameter 1 of the selected track's instrument, in the direction it is turned.
#[test]
fn f_k01_knob5_moves_parameter() {
    let mut b = Bench::demo();
    b.press(Button::Group(2)); // FmBass
    let before = b.param(2, 1);
    b.knob(5, 200);
    let up = b.param(2, 1);
    assert!(up > before, "turning up did not raise the parameter ({before} → {up})");
    b.knob(5, -400);
    assert!(b.param(2, 1) < up, "turning down did not lower it");
}

// F-K02: with page 6 open, encoders 5–8 move parameters 5–8.
#[test]
fn f_k02_page_two_moves_second_bank() {
    let mut b = Bench::demo();
    b.press(Button::Group(2));
    b.press(Button::Screen(6));
    assert_eq!(b.ui().right_page, 1);
    let (p0, p4) = (b.param(2, 0), b.param(2, 4));
    b.knob(4, 300);
    assert_eq!(b.param(2, 0), p0, "page 1 parameter moved while page 2 is open");
    assert_ne!(b.param(2, 4), p4, "page 2 parameter did not move");
}

// F-K03: touching an encoder shows the overlay on the right screen.
#[test]
fn f_k03_touch_shows_overlay() {
    let mut b = Bench::demo();
    b.press(Button::Group(2));
    b.run_ms(100);
    let idle = b.screen_hash(Screen::Right);
    b.touch(5, true);
    assert!(b.ui().knob_touch[5]);
    let touched = b.screen_hash(Screen::Right);
    assert_ne!(idle, touched, "overlay did not appear");
    b.capture("f-k03-overlay");
    b.touch(5, false);
    assert!(!b.ui().knob_touch[5]);
}

// F-K04: every built-in instrument exposes at most eight parameters.
#[test]
fn f_k04_at_most_eight_parameters() {
    for name in octachrome_cartridges::builtin_names() {
        let c: Box<dyn InstrumentCartridge> = octachrome_cartridges::create(name).expect("registered");
        let n = c.parameters().len() + c.secondary_parameters().map_or(0, |s| s.len());
        assert!(n <= 8, "{name} exposes {n} parameters");
    }
}

// F-K09: a kit step has no note: the inspector's first cell is empty and its knob does nothing; the sound is a lane.
#[test]
fn f_k09_a_kit_step_has_no_note() {
    let mut b = Bench::demo();
    b.press(Button::Group(0)); // track A: drums, a kit
    b.press(Button::Screen(2)); // opens the inspector on step 1
    b.run_ms(100);
    let before = b.project().tracks[0].steps[0];
    b.knob(0, 40);
    b.run_ms(50);
    assert_eq!(b.project().tracks[0].steps[0], before, "the first knob changes nothing on a kit step");
    // a plain-pitch track still has its note knob
    b.press(Button::Group(2));
    b.press(Button::Screen(2));
    b.run_ms(50);
    let note = b.project().tracks[2].steps[0].note;
    b.knob(0, 40);
    b.run_ms(50);
    assert_eq!(b.project().tracks[2].steps[0].note, note + 1);
}

// F-K10: on a drum kit the pads are the steps of the lane chosen on the right (`sound`): a tap puts that sound
// on the step or takes it off, and leaves the other lanes of the step as they are.
#[test]
fn f_k10_a_tap_toggles_the_chosen_lane() {
    let mut b = Bench::demo();
    b.press(Button::Group(0));
    b.press(Button::Screen(6)); // shape: the second page, where `sound` is the last knob
    b.knob(7, 120); // 3 detents: kick -> snare (lane 1)
    let pad = (0..16u8)
        .find(|&p| b.project().tracks[0].steps[octachrome::Controller::pad_to_step(p) as usize].lanes == 0)
        .expect("an empty step");
    let step = octachrome::Controller::pad_to_step(pad) as usize;
    b.tap(pad);
    b.run_ms(50);
    assert_eq!(b.project().tracks[0].steps[step].lanes, 0b10, "the snare lane is on");
    // choose another sound, tap the same step: both lanes are on (two sounds in one step)
    b.knob(7, -120);
    b.tap(pad);
    b.run_ms(50);
    assert_eq!(b.project().tracks[0].steps[step].lanes, 0b11, "kick and snare together");
    // tapping a lane that is on takes only that one off
    b.tap(pad);
    b.run_ms(50);
    assert_eq!(b.project().tracks[0].steps[step].lanes, 0b10, "only the kick left");
}

// F-K11: holding TEMPO, SWING or VOLUME and turning the 4D encoder turns that one, and the left screen shows it big while it is held.
#[test]
fn f_k11_held_buttons_take_the_encoder() {
    let mut b = Bench::demo();
    let (bpm, swing) = (b.project().bpm, b.project().swing);
    b.button_down(Button::Swing);
    b.encoder(4);
    b.run_ms(50);
    assert_eq!(b.ui().held, Some(octachrome_ui::Held::Swing));
    assert!(b.project().swing > swing, "swing went up");
    assert_eq!(b.project().bpm, bpm, "and the tempo did not move");
    b.button_up(Button::Swing);
    assert_eq!(b.ui().held, None);
    b.button_down(Button::Tempo);
    b.encoder(-3);
    b.button_up(Button::Tempo);
    assert_eq!(b.project().bpm, bpm - 3.0);
    // VOLUME is the master volume: half of it, half the level
    b.press(Button::Play);
    b.run_ms(1500);
    let loud = b.peak(500, 1500);
    b.button_down(Button::Volume);
    b.encoder(-50);
    b.button_up(Button::Volume);
    assert!((b.ui().master - 0.5).abs() < 0.02 || b.ui().master < 0.6, "master {}", b.ui().master);
    b.run_ms(1500);
    let quiet = b.peak(2000, 3000);
    assert!(quiet < loud * 0.8, "quieter: {quiet} vs {loud}");
}

// F-K12: FIXED VEL makes every pad play and REC write the same velocity, whatever the strike.
#[test]
fn f_k12_fixed_velocity() {
    let mut b = Bench::empty();
    b.press(Button::Group(2));
    b.press(Button::Keyboard);
    b.press(Button::Rec);
    b.press(Button::Play);
    b.run_ms(20);
    b.strike_at(0, 0.3);
    b.release(0);
    let s = b.project().tracks[2].steps.iter().find(|s| s.active).copied().expect("recorded");
    assert_eq!(s.velocity, 38, "a soft strike is soft");
    b.press(Button::FixedVel);
    assert!(b.ui().fixed_vel);
    b.run_ms(600);
    b.strike_at(1, 0.3);
    b.release(1);
    let fixed: Vec<u8> = b.project().tracks[2].steps.iter().filter(|s| s.active && s.note == 49).map(|s| s.velocity).collect();
    assert_eq!(fixed, [100], "with FIXED VEL the same soft strike is written as 100");
}
