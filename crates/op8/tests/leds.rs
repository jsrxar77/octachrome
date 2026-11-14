//! LEDs (docs/features.md §L).
use op8::bench::Bench;
use op8_hw::{leds::button_slot, Button, NiColor};

// F-L01: PLAY is green and bright while playing, dim when stopped.
#[test]
fn f_l01_play_led_follows_transport() {
    let mut b = Bench::demo();
    b.tick(); // the LEDs are written on the first frame
    let slot = button_slot(Button::Play).unwrap();
    assert_eq!(b.leds().slots[slot] & 3, 1, "stopped: dim");
    b.press(Button::Play);
    assert_eq!(b.leds().slots[slot] >> 2, NiColor::Green as u8, "playing: green");
    assert_eq!(b.leds().slots[slot] & 3, 3, "playing: bright");
    b.press(Button::Play);
    assert_eq!(b.leds().slots[slot] & 3, 1, "paused: dim again");
}

// F-L02: the pads of the selected track show its colour: active steps bright, inactive dim.
#[test]
fn f_l02_step_pads_show_track_colour() {
    let mut b = Bench::demo();
    b.press(Button::Group(0)); // track A: red
    let on_pad = (0..16u8).find(|&p| {
        let step = op8::Controller::pad_to_step(p) as usize;
        step < 16 && b.project().tracks[0].steps[step].active
    });
    let off_pad = (0..16u8).find(|&p| {
        let step = op8::Controller::pad_to_step(p) as usize;
        step < 16 && !b.project().tracks[0].steps[step].active
    });
    let (on, off) = (on_pad.unwrap(), off_pad.unwrap());
    assert_eq!(b.leds().pad(on) & 3, 3, "active pad {on} should be bright");
    assert_eq!(b.leds().pad(on) >> 2, NiColor::Red as u8, "active pad should be the track colour");
    assert_eq!(b.leds().pad(off) & 3, 1, "inactive pad {off} should be dim");
}

// F-L07: an empty step is a dim white, not a dim colour, so it stays apart from an active pad under any cap.
#[test]
fn f_l07_empty_steps_are_dim_white() {
    let mut b = Bench::demo();
    b.press(Button::Group(0));
    let off_pad = (0..16u8)
        .find(|&p| {
            let step = op8::Controller::pad_to_step(p) as usize;
            step < 16 && !b.project().tracks[0].steps[step].active
        })
        .expect("an empty step");
    assert_eq!(b.leds().pad(off_pad) >> 2, NiColor::White as u8);
    assert_eq!(b.leds().pad(off_pad) & 3, 1);
}

// F-D12: screen button 1 goes back to the step view and 2 opens the components; they do nothing in settings.
#[test]
fn f_d12_screen_buttons_1_and_2() {
    let mut b = Bench::demo();
    b.press(Button::Perform);
    b.press(Button::Screen(1));
    assert_eq!(b.ui().mode, op8_ui::Mode::Step);
    b.press(Button::Screen(2));
    assert!(b.ui().inspect_step.is_some(), "components opens the inspector");
    b.press(Button::Screen(1));
    assert!(b.ui().inspect_step.is_none(), "step closes it");
    b.press(Button::Settings);
    b.press(Button::Screen(2));
    assert!(b.ui().inspect_step.is_none(), "nothing happens in settings");
}
