//! ARRANGER: the song, a chain of scenes (docs/features.md, F-K27).
use octachrome::bench::Bench;
use octachrome_hw::Button;
use octachrome_ui::Main;
use std::sync::atomic::Ordering::Relaxed;

fn pad_of(slot: usize) -> u8 {
    (0..16u8).find(|&p| octachrome::Controller::pad_to_step(p) as usize == slot).unwrap()
}

// F-K27: ARRANGER opens both screens and is lit (again goes back); a tap on a pad chooses a step of the song; knob 5 puts a scene in a free
// step (and then turns it), knob 6 sets its bars, knob 7 follows the song or not; ERASE + a tap takes a step out and the rest move up; the
// 4D reaches the sixteen cells; the song is saved with the project (format version 10).
#[test]
fn f_k27_the_song_is_made_of_steps_with_a_scene_and_some_bars() {
    let mut b = Bench::demo();
    b.press(Button::Arranger);
    assert_eq!(b.ui().main, Main::Arranger);
    b.run_ms(100);
    assert!(b.leds().pad(pad_of(0)) & 3 == 1, "an empty song: dim");
    b.tap(pad_of(0));
    b.knob(4, 40);
    assert_eq!(b.project().song.len, 1);
    assert_eq!((b.project().song.steps[0].scene, b.project().song.steps[0].bars), (0, 4), "a free step comes in with scene 1 for 4 bars");
    b.knob(4, 80);
    b.knob(5, -80);
    assert_eq!((b.project().song.steps[0].scene, b.project().song.steps[0].bars), (2, 2));
    assert!(b.ui().inspect_step.is_none(), "a tap in another view does not open the step inspector");
    b.tap(pad_of(2)); // a free step two places on: the one between is filled in
    b.knob(4, 40);
    assert_eq!(b.project().song.len, 3);
    assert_eq!(b.project().song.steps[1].bars, 4);
    assert!(!b.project().song.on);
    b.knob(6, 40);
    assert!(b.project().song.on);
    b.run_ms(100);
    b.capture("f-k27-song");
    assert!(b.leds().pad(pad_of(1)) & 3 == 2 && b.leds().pad(pad_of(5)) & 3 == 1);
    assert_eq!(octachrome_ui::nav_items(b.ui()).len(), 16);
    b.button_down(Button::Erase);
    b.tap(pad_of(1));
    b.button_up(Button::Erase);
    assert_eq!(b.project().song.len, 2);
    assert_eq!(b.project().song.steps[1].scene, 0, "the one after it moved up");
    let q = octachrome_seq::io::decode(&octachrome_seq::io::encode(b.project())).unwrap();
    assert_eq!(q.song, b.project().song);
    b.press(Button::Arranger);
    assert_eq!(b.ui().main, Main::Instrument);
}

// F-K27: a song that is followed plays its steps: the sequencer starts on the first, the pad of the step that plays is the brightest, and
// the scene's patterns are the ones that play.
#[test]
fn f_k27_a_followed_song_plays_the_scenes() {
    let mut b = Bench::empty();
    b.app.handle.seq(octachrome_seq::Command::SetStep { track: 1, step: 0, value: octachrome_seq::Step::on(36) });
    b.press(Button::Group(1));
    b.press(Button::Pattern);
    b.tap(pad_of(1));
    b.press(Button::Pattern);
    b.press(Button::Scene);
    b.button_down(Button::Shift);
    b.tap(pad_of(0)); // scene 1: the kick on pattern 2
    b.button_up(Button::Shift);
    b.press(Button::Arranger);
    b.tap(pad_of(0));
    b.knob(4, 40);
    b.knob(6, 40);
    b.press(Button::Pattern);
    b.tap(pad_of(0)); // what is shown is pattern 1; the song says pattern 2
    b.press(Button::Pattern);
    b.press(Button::Play);
    b.run_ms(300);
    assert_eq!(b.status().song_step.load(Relaxed), 0);
    assert_eq!(b.status().pattern[1].load(Relaxed), 1, "the song says pattern 2 although pattern 1 was the one shown");
    // what is shown follows what plays, by itself
    assert_eq!(b.project().current[1], 1, "the view moved to pattern 2");
    b.press(Button::Group(1));
    b.press(Button::Pattern);
    b.run_ms(300);
    b.capture("f-k27-song-follows");
    assert!(b.leds().pad(pad_of(1)) & 3 == 3, "its pad is the brightest");
    // a tap on another pattern while the song goes does nothing, and says why
    b.tap(pad_of(5));
    assert_eq!(b.project().current[1], 1);
    assert!(b.ui().pattern_note.contains("the song decides"), "{}", b.ui().pattern_note);
    // stopped, a tap works again
    b.press(Button::Stop);
    b.tap(pad_of(5));
    assert_eq!(b.project().current[1], 5);
    let _ = Main::Arranger;
}

// F-K27: SHIFT + a tap puts a step in front of the one tapped (a copy of it) and the later ones move down; the mutes a scene sets while the
// song plays are what the screens and the track LEDs show; the top bar says which pattern is shown and which step of the song plays.
#[test]
fn f_k27_insert_a_step_and_the_mutes_of_a_playing_song_show() {
    let mut b = Bench::empty();
    b.press(Button::Group(2));
    b.button_down(Button::Mute);
    b.press(Button::Group(2));
    b.button_up(Button::Mute);
    assert!(b.project().tracks[2].muted);
    b.press(Button::Scene);
    b.button_down(Button::Shift);
    b.tap(pad_of(0)); // scene 1: track C muted
    b.button_up(Button::Shift);
    b.press(Button::Scene);
    b.button_down(Button::Mute);
    b.press(Button::Group(2));
    b.button_up(Button::Mute);
    assert!(!b.project().tracks[2].muted);
    b.press(Button::Arranger);
    b.tap(pad_of(0));
    b.knob(4, 40);
    b.tap(pad_of(1));
    b.knob(4, 40);
    b.knob(4, -40);
    assert_eq!(b.project().song.len, 2);
    b.button_down(Button::Shift);
    b.tap(pad_of(0));
    b.button_up(Button::Shift);
    assert_eq!(b.project().song.len, 3, "one more");
    assert_eq!(b.project().song.steps[0], b.project().song.steps[1], "a copy of the one tapped");
    b.knob(6, 40);
    b.press(Button::Arranger);
    b.press(Button::Play);
    b.run_ms(300);
    assert!(!b.project().tracks[2].muted, "the project's own mute is not touched");
    assert!(b.status().muted(2, false), "but the scene of the song mutes it");
    assert!(!b.status().muted(3, false));
    assert_eq!(b.status().song_step.load(Relaxed), 0);
    b.press(Button::Stop);
    b.run_ms(100);
    assert!(!b.status().muted(2, false), "stopped: the track's own mute again");
}

// F-K27: with the song playing, a tap (or a push of the 4D) on a step goes there from the next bar, SHIFT + a tap at once; the screen counts
// the bars of the step that plays; a step moves with knob 4, or with a pad held and another tapped.
#[test]
fn f_k27_a_tap_goes_to_that_part_and_the_steps_can_be_moved() {
    use octachrome_seq::Command::*;
    let mut b = Bench::empty();
    b.app.handle.seq(SetStep { track: 1, step: 0, value: octachrome_seq::Step::on(36) });
    b.app.handle.seq(SetPattern { track: 1, index: 1, now: false });
    b.app.handle.seq(StoreScene { index: 0 }); // scene 1: the kick on pattern 2 (empty)
    b.app.handle.seq(SetPattern { track: 1, index: 0, now: false });
    b.app.handle.seq(StoreScene { index: 1 }); // scene 2: pattern 1
    for (i, (scene, bars)) in [(0, 2), (1, 2), (0, 1)].into_iter().enumerate() {
        b.app.handle.seq(SetSongStep { index: i as u8, scene, bars });
    }
    b.app.handle.seq(SetSongOn(true));
    b.press(Button::Arranger);
    b.press(Button::Play);
    b.run_ms(400);
    assert_eq!(b.status().song_step.load(Relaxed), 0);
    assert!(b.ui().arr.playing.is_some_and(|(p, bar, bars)| (p, bar, bars) == (0, 1, 2)), "{:?}", b.ui().arr.playing);
    b.capture("f-k27-song-playing");
    // a tap: not at once, from the next bar
    b.tap(pad_of(1));
    assert_eq!(b.ui().arr.sel, 1, "it is chosen");
    b.run_ms(300);
    assert_eq!(b.status().song_step.load(Relaxed), 0, "it waits for the bar");
    b.run_ms(1800);
    assert_eq!(b.status().song_step.load(Relaxed), 1, "and it went there");
    // SHIFT + a tap: at once
    b.button_down(Button::Shift);
    b.tap(pad_of(2));
    b.button_up(Button::Shift);
    b.run_ms(100);
    assert_eq!(b.status().song_step.load(Relaxed), 2);
    // the 4D reaches a step and a push goes there
    b.press(Button::JogLeft);
    b.press(Button::JogLeft);
    b.press(Button::JogPush);
    b.run_ms(2300);
    assert_eq!(b.status().song_step.load(Relaxed), 0, "a push of the 4D is a tap");
    b.press(Button::Stop);
    // moving: knob 4 takes the chosen step one place later
    b.tap(pad_of(0));
    b.knob(3, 40);
    assert_eq!((b.project().song.steps[0].scene, b.project().song.steps[0].bars), (1, 2), "the one it passed moved up");
    assert_eq!((b.project().song.steps[1].scene, b.project().song.steps[1].bars), (0, 2), "and it is now second");
    assert_eq!(b.ui().arr.sel, 1, "the choice goes with it");
    // a pad held and another tapped: the held one goes to the place of the other
    b.strike(pad_of(2));
    b.tap(pad_of(0));
    b.release(pad_of(2));
    assert_eq!((b.project().song.steps[0].scene, b.project().song.steps[0].bars), (0, 1), "the third went first");
    assert_eq!(b.project().song.len, 3, "nothing was lost");
    // stopped, a tap only chooses
    b.tap(pad_of(1));
    assert_eq!(b.ui().arr.sel, 1);
}
