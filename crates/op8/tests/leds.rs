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
    b.press(Button::Group(1)); // track B (KickSyn)
    let on_pad = (0..16u8).find(|&p| {
        let step = op8::Controller::pad_to_step(p) as usize;
        step < 16 && b.project().tracks[1].steps[step].active
    });
    let off_pad = (0..16u8).find(|&p| {
        let step = op8::Controller::pad_to_step(p) as usize;
        step < 16 && !b.project().tracks[1].steps[step].active
    });
    let (on, off) = (on_pad.unwrap(), off_pad.unwrap());
    assert_eq!(b.leds().pad(on) & 3, 3, "active pad {on} should be bright");
    assert_eq!(b.leds().pad(on) >> 2, op8::controller::track_led_colors(op8_gfx::theme_id())[1] as u8, "active pad should be the track colour");
    assert_eq!(b.leds().pad(off) & 3, 1, "inactive pad {off} should be dim");
}

// F-L07: an empty step is a dim white, not a dim colour, so it stays apart from an active pad under any cap.
#[test]
fn f_l07_empty_steps_are_dim_white() {
    let mut b = Bench::demo();
    b.press(Button::Group(1));
    let off_pad = (0..16u8)
        .find(|&p| {
            let step = op8::Controller::pad_to_step(p) as usize;
            step < 16 && !b.project().tracks[1].steps[step].active
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

// F-L09: every button follows one rule: dark if it does nothing in this context, a very faint white if it does
// something, a strong white while it is pressed or has the focus.
#[test]
fn f_l09_buttons_are_dark_faint_or_strong() {
    let mut b = Bench::demo();
    b.tick();
    // (mono buttons: 0..127; colour buttons: the level in the two low bits)
    let led = |b: &Bench, btn: Button| {
        let v = b.leds().slots[button_slot(btn).unwrap()];
        if op8_hw::leds::button_is_colour(btn) { [0u32, 10, 60, 120][(v & 3) as usize] } else { v as u32 }
    };
    let off = |b: &Bench, btn| led(b, btn) == 0;
    let faint = |b: &Bench, btn| (1..=30).contains(&led(b, btn));
    let strong = |b: &Bench, btn| led(b, btn) >= 100;
    // composing, step mode: STEP has the focus, the other pad modes can be chosen, REC does nothing yet
    assert!(strong(&b, Button::Step) && faint(&b, Button::Chords) && faint(&b, Button::Perform), "the mode in use is strong, the others faint");
    assert!(faint(&b, Button::PadMode) && faint(&b, Button::Keyboard) && faint(&b, Button::Shift) && faint(&b, Button::Mute) && faint(&b, Button::Erase));
    assert!(faint(&b, Button::Rec), "REC can be armed");
    assert!(faint(&b, Button::Tap) && faint(&b, Button::Tempo) && faint(&b, Button::Swing) && faint(&b, Button::Volume) && faint(&b, Button::FixedVel));
    // the views open from here: faint white, strong while open (and while held)
    for btn in [Button::Pattern, Button::Scene, Button::Arranger, Button::Mixer, Button::Browser, Button::Sampling] {
        assert!(faint(&b, btn), "{btn:?} does something: faint white at rest");
    }
    for btn in [Button::Pattern, Button::Scene, Button::Arranger] {
        b.press(btn);
        b.tick();
        assert!(strong(&b, btn), "{btn:?} is strong while its view is open (it has the focus)");
        b.press(btn);
        b.tick();
        assert!(faint(&b, btn), "{btn:?} is faint again once it is closed");
    }
    assert!(strong(&b, Button::ChannelMidi) && faint(&b, Button::Settings), "composing: CHANNEL has the focus");
    // a button held down has the focus: SHIFT, then PERFORM held
    b.button_down(Button::Shift);
    b.tick();
    assert!(strong(&b, Button::Shift), "held SHIFT is strong");
    b.button_up(Button::Shift);
    b.tick();
    assert!(faint(&b, Button::Shift));
    b.button_down(Button::Restart);
    b.tick();
    assert!(strong(&b, Button::Restart), "a pressed button is strong while it is down");
    b.button_up(Button::Restart);
    // the mode that is chosen takes the focus
    b.press(Button::Keyboard);
    b.tick();
    assert!(strong(&b, Button::Keyboard) && faint(&b, Button::Step));
    assert!(faint(&b, Button::JogLeft) && faint(&b, Button::JogRight), "the encoder's sides move the octave");
    // settings: SETTINGS has the focus, the groups, MUTE, ERASE and the jog sides tell what they do here
    b.press(Button::Settings);
    b.tick();
    assert!(strong(&b, Button::Settings) && faint(&b, Button::ChannelMidi), "settings has the focus");
    assert!(off(&b, Button::Mute) && off(&b, Button::Erase), "nothing to mute or erase in settings");
    assert!(faint(&b, Button::JogLeft), "◀ ▶ choose the theme");
    // the four pages are the buttons 1 to 4 (the open one strong), the buttons of the right screen do nothing
    assert!(strong(&b, Button::Screen(1)) && faint(&b, Button::Screen(2)) && faint(&b, Button::Screen(3)) && faint(&b, Button::Screen(4)), "the pages of settings");
    for n in 5..=8u8 {
        assert!(off(&b, Button::Screen(n)), "the buttons of the right screen do nothing in settings");
    }
    assert!(!off(&b, Button::Play) && !off(&b, Button::Restart) && !off(&b, Button::Stop), "the transport still works");
    // SELECT and the 4D: lit only where they do something (SELECT names things in FILE, BROWSER and SAMPLING;
    // the 4D moves through whatever the pads show)
    b.press(Button::ChannelMidi);
    b.press(Button::Step);
    b.tick();
    assert!(off(&b, Button::Select), "nothing to name while composing");
    for view in [Button::File, Button::Browser, Button::Sampling] {
        b.press(view);
        b.tick();
        assert!(faint(&b, Button::Select), "SELECT names things in {view:?}");
        b.press(view);
        b.tick();
    }
    b.press(Button::Pattern);
    b.tick();
    assert!(faint(&b, Button::JogUp) && faint(&b, Button::JogDown) && faint(&b, Button::JogLeft), "the 4D moves over the sixteen patterns");
    assert!(off(&b, Button::Select), "and SELECT does nothing there");
    b.press(Button::Pattern);
    b.tick();
    assert!(faint(&b, Button::JogUp) && faint(&b, Button::JogDown), "composing: the 4D moves over the tracks, the buttons and the values");
}

// F-H08: closing the program leaves the controller as it was before OP-8: every LED off and both screens black,
// not what was last shown.
#[test]
fn f_h08_closing_leaves_the_controller_clean() {
    let mut b = Bench::demo();
    b.press(Button::Play);
    b.run_ms(300);
    assert!(b.app.surface.leds.slots.iter().any(|&v| v != 0), "something is lit while playing");
    assert!(b.app.surface.screens.iter().any(|s| s.pixels().iter().any(|&p| p != 0)), "and something is drawn");
    b.app.shutdown().unwrap();
    assert!(b.app.surface.leds.slots.iter().all(|&v| v == 0), "every LED is off");
    for s in &b.app.surface.screens {
        assert!(s.pixels().iter().all(|&p| p == 0), "both screens are black");
    }
}
