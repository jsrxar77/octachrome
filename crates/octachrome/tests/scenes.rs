//! SCENE: the sixteen scenes of a project (docs/features.md, F-K26).
use octachrome::bench::Bench;
use octachrome_hw::Button;
use octachrome_ui::Main;
use std::sync::atomic::Ordering::Relaxed;

fn pad_of(slot: usize) -> u8 {
    (0..16u8).find(|&p| octachrome::Controller::pad_to_step(p) as usize == slot).unwrap()
}

fn shift_tap(b: &mut Bench, pad: u8) {
    b.button_down(Button::Shift);
    b.tap(pad);
    b.button_up(Button::Shift);
}

// F-K26: SCENE opens both screens and is lit (again goes back); SHIFT + a pad stores the pattern each track shows and the mutes that are
// now; a plain tap brings them back (the pattern of every track is shown, the mutes are set); ERASE + a tap empties; a scene that was
// never stored says so and changes nothing; the pads are lit more when they have something.
#[test]
fn f_k26_a_scene_stores_and_brings_back_patterns_and_mutes() {
    let mut b = Bench::demo();
    b.press(Button::Scene);
    assert_eq!(b.ui().main, Main::Scene);
    b.run_ms(100);
    b.capture("f-k26-scenes");
    assert!(b.leds().pad(pad_of(0)) & 3 == 1, "nothing stored: dim");
    b.tap(pad_of(0));
    assert!(b.ui().scene_note.contains("is empty"), "{}", b.ui().scene_note);
    // set the music up: track A on pattern 2, track C muted
    b.press(Button::Scene);
    b.press(Button::Group(0));
    b.press(Button::Pattern);
    b.tap(pad_of(1));
    b.press(Button::Pattern);
    b.press(Button::Group(2));
    b.button_down(Button::Mute);
    b.press(Button::Group(2));
    b.button_up(Button::Mute);
    assert!(b.project().tracks[2].muted, "MUTE + the track button mutes it");
    b.press(Button::Scene);
    shift_tap(&mut b, pad_of(4));
    assert!(b.ui().scene_note.contains("scene 5 stored"), "{}", b.ui().scene_note);
    assert!(b.project().scenes[4].used);
    assert_eq!(b.project().scenes[4].pattern[0], 1);
    assert_eq!(b.project().scenes[4].mute >> 2 & 1, 1, "track C is muted in the scene");
    assert!(b.leds().pad(pad_of(4)) & 3 == 3, "the last one is the brightest");
    assert!(b.leds().pad(pad_of(5)) & 3 == 1);
    // change it all, then launch the scene
    b.press(Button::Scene);
    b.press(Button::Group(0));
    b.press(Button::Pattern);
    b.tap(pad_of(0));
    b.press(Button::Scene);
    assert_eq!(b.project().current[0], 0);
    b.tap(pad_of(4));
    assert_eq!(b.project().current[0], 1, "the pattern of the track is back");
    assert!(b.ui().scene_note.contains("scene 5"), "{}", b.ui().scene_note);
    // ERASE + a tap
    b.button_down(Button::Erase);
    b.tap(pad_of(4));
    b.button_up(Button::Erase);
    assert!(!b.project().scenes[4].used);
    // saved with the project
    shift_tap(&mut b, pad_of(7));
    let q = octachrome_seq::io::decode(&octachrome_seq::io::encode(b.project())).unwrap();
    assert_eq!(q.scenes[7], b.project().scenes[7]);
}

// F-K26: a launched scene reaches the playing sequencer: the track goes to its pattern from its next loop, the mute is at once.
#[test]
fn f_k26_a_launched_scene_plays_from_the_next_loop() {
    let mut b = Bench::empty();
    b.app.handle.seq(octachrome_seq::Command::SetStep { track: 1, step: 0, value: octachrome_seq::Step::on(36) });
    b.press(Button::Group(1));
    b.press(Button::Pattern);
    b.tap(pad_of(1));
    b.press(Button::Pattern);
    b.press(Button::Scene);
    shift_tap(&mut b, pad_of(0)); // scene 1: the kick on its second pattern
    b.press(Button::Scene);
    b.press(Button::Pattern);
    b.tap(pad_of(0));
    b.press(Button::Pattern);
    b.press(Button::Play);
    b.run_ms(300);
    assert_eq!(b.status().pattern[1].load(Relaxed), 0);
    b.press(Button::Scene);
    b.tap(pad_of(0));
    assert_eq!(b.status().pattern[1].load(Relaxed), 0, "not before the loop ends");
    b.run_ms(2200);
    assert_eq!(b.status().pattern[1].load(Relaxed), 1);
}
