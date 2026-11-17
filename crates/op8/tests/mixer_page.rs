//! MIXER: four channels on the left, the compressor of the master on the right (docs/features.md, F-A27, F-K21).
use op8::bench::Bench;
use op8_hw::{Button, Screen};
use op8_ui::{Main, NavKind};
use std::sync::atomic::Ordering::Relaxed;

fn focus_on(b: &mut Bench, kind: NavKind) {
    for _ in 0..20 {
        if b.ui().focus == kind {
            return;
        }
        b.encoder(1);
    }
    panic!("the focus never reached {kind:?}");
}

// F-K21: MIXER opens the mixer on both screens and is lit; buttons 1 and 2 choose the channels (a–d, e–h), whose levels the knobs 1 to 4
// are (the pans with SHIFT); buttons 5 and 6 choose the page of the compressor, whose values knobs 5 to 8 are; MIXER again goes back.
#[test]
fn f_k21_the_mixer_has_channels_and_a_compressor() {
    let mut b = Bench::demo();
    b.press(Button::Mixer);
    assert_eq!(b.ui().main, Main::Mixer);
    b.run_ms(100);
    let labels = op8_ui::screen_buttons(b.ui());
    assert_eq!(labels.iter().flatten().map(|l| l.0).collect::<Vec<_>>(), ["a–d", "e–h", "comp", "output"]);
    b.capture("f-k21-mixer");
    // knobs 1 to 4: the levels of A to D
    let before = b.project().tracks[1].level;
    b.knob(1, 80);
    assert_eq!(b.project().tracks[1].level, before + 8, "two detents of four");
    b.button_down(Button::Shift);
    let pan = b.project().tracks[1].pan;
    b.knob(1, 40);
    b.button_up(Button::Shift);
    assert_eq!(b.project().tracks[1].pan, pan + 4);
    // buttons 2: the knobs are the levels of E to H
    b.press(Button::Screen(2));
    let e = b.project().tracks[4].level;
    b.knob(0, -40);
    assert_eq!(b.project().tracks[4].level, e - 4, "knob 1 is now track E");
    assert_eq!(b.project().tracks[0].level, op8_seq::project::DEFAULT_LEVEL, "and A is as it was");
    // knobs 5 to 8: the compressor
    assert_eq!(b.project().comp, op8_seq::MasterComp::default());
    b.knob(4, -400); // threshold goes down half a sweep
    let thr = b.project().comp.threshold;
    assert!((120..135).contains(&thr), "{thr}");
    b.knob(5, 100);
    assert!(b.project().comp.ratio > 102, "ratio went up");
    b.press(Button::Screen(6)); // output
    b.knob(4, 400); // make-up
    assert!(b.project().comp.makeup > 100);
    b.knob(6, 400); // there is no third value on this page
    assert_eq!(b.ui().mixer.page, 1);
    b.press(Button::Mixer);
    assert_eq!(b.ui().main, Main::Instrument, "MIXER again goes back");
}

// F-K21: the 4D reaches the four levels and the values of the compressor's page, grabs one and turns it.
#[test]
fn f_k21_the_4d_reaches_the_mixer() {
    let mut b = Bench::demo();
    b.press(Button::Mixer);
    focus_on(&mut b, NavKind::Value(5)); // ratio
    let r = b.project().comp.ratio;
    b.press(Button::JogPush);
    b.encoder(3);
    b.press(Button::JogPush);
    assert!(b.project().comp.ratio > r, "the 4D turned the ratio");
    b.press(Button::Screen(6));
    let n = op8_ui::nav_items(b.ui()).len();
    assert_eq!(n, 4 + 2, "the output page has two values: nothing else to reach");
    focus_on(&mut b, NavKind::Value(2));
    b.press(Button::JogPush);
    let l = b.project().tracks[2].level;
    b.encoder(-3);
    assert!(b.project().tracks[2].level < l, "the 4D turned the level of C");
}

// F-A27: with the compressor set, what plays gets quieter, and the screen says how much it is being brought down; with the defaults the
// sound is as it was; the settings are saved with the project.
#[test]
fn f_a27_the_master_compressor_brings_the_sound_down() {
    let measure = |set: bool| {
        let mut b = Bench::demo();
        if set {
            for (i, v) in [(0u8, 60u8), (1, 255), (2, 0), (4, 0)] {
                b.app.handle.seq(op8_seq::Command::SetMasterComp { index: i, value: v });
            }
        }
        b.press(Button::Play);
        b.run_ms(3000);
        (b.rms_db(1000, 3000), b.status().comp_reduction.load(Relaxed))
    };
    let (plain, plain_gr) = measure(false);
    let (squashed, _) = measure(true);
    assert_eq!(f32::from_bits(plain_gr), 0.0, "nothing is reduced by default");
    assert!(squashed < plain - 3.0, "the compressor brings it down: {plain} dB and then {squashed} dB");
    // the right screen shows it
    let mut b = Bench::demo();
    b.press(Button::Mixer);
    b.press(Button::Play);
    b.run_ms(1500);
    let still = b.screen_hash(Screen::Right);
    for (i, v) in [(0u8, 60u8), (1, 255), (2, 0)] {
        b.app.handle.seq(op8_seq::Command::SetMasterComp { index: i, value: v });
    }
    b.run_ms(1500);
    assert_ne!(b.screen_hash(Screen::Right), still, "the curve and the bar change");
    assert!(b.ui().mixer.reduction < -1.0, "and it says by how much: {}", b.ui().mixer.reduction);
    b.capture("f-a27-compressing");
    // saved with the project
    let bytes = op8_seq::io::encode(b.project());
    assert_eq!(op8_seq::io::decode(&bytes).unwrap().comp, b.project().comp);
}
