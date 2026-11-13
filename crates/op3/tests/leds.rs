//! LEDs (docs/features.md §L).
use op3::bench::Bench;
use op3_hw::{leds::button_slot, Button, NiColor};

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
        let step = op3::Controller::pad_to_step(p) as usize;
        step < 16 && b.project().tracks[0].steps[step].active
    });
    let off_pad = (0..16u8).find(|&p| {
        let step = op3::Controller::pad_to_step(p) as usize;
        step < 16 && !b.project().tracks[0].steps[step].active
    });
    let (on, off) = (on_pad.unwrap(), off_pad.unwrap());
    assert_eq!(b.leds().pad(on) & 3, 3, "active pad {on} should be bright");
    assert_eq!(b.leds().pad(on) >> 2, NiColor::Red as u8, "active pad should be the track colour");
    assert_eq!(b.leds().pad(off) & 3, 1, "inactive pad {off} should be dim");
}
