//! The sampler's gaps, closed: WAV files copied in while it runs, a recording that waits for the sound, and slices.
use octachrome::bench::Bench;
use octachrome_hw::{Button, Screen};
use std::sync::atomic::Ordering::Relaxed;

fn pad_of(slot: usize) -> u8 {
    (0..16u8).find(|&p| octachrome::Controller::pad_to_step(p) as usize == slot).unwrap()
}

fn dir(name: &str) -> std::path::PathBuf {
    let d = std::env::temp_dir().join(format!("octachrome-gaps-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn tone(secs: f32) -> Vec<u8> {
    let data: Vec<f32> = (0..(secs * 48_000.0) as usize).map(|i| (i as f32 * 0.05).sin()).collect();
    octachrome_cartridges::wav::encode_16(&data, 48_000)
}

// F-A24: WAV files copied into the folder while Octachrome runs are put in the instruments that play when SAMPLING or PLUG-IN is opened: only the
// ones that are new or changed, it says how many, a place whose file is gone becomes empty, and what SAMPLING saved is not loaded again.
#[test]
fn f_a24_files_copied_in_are_heard_without_restarting() {
    let d = dir("hot");
    let mut b = Bench::empty();
    b.set_samples_dir(d.clone());
    // a sampler on E: sample user 1 (empty)
    b.press(Button::Group(4));
    b.knob(4, 400);
    b.tap(12);
    b.press(Button::Play);
    b.run_ms(1500);
    b.press(Button::Stop);
    let before = b.peak_db(0, b.now_ms());
    std::fs::write(d.join("mine.wav"), tone(1.0)).unwrap(); // copied in while it runs
    b.press(Button::Sampling);
    assert_eq!(b.ui().smp.note, "loaded 1 sound from the folder", "{}", b.ui().smp.note);
    assert!(!b.ui().smp.saved[0], "it is a file of her own, not a recording: only rec-N.wav are marked as recordings");
    b.press(Button::Sampling);
    let from = b.now_ms();
    b.press(Button::Play);
    b.run_ms(1500);
    assert!(before < -60.0, "an empty place is silent: {before} dB");
    assert!(b.peak_db(from, b.now_ms()) > -30.0, "the new sound is heard: {} dB", b.peak_db(from, b.now_ms()));
    b.press(Button::Stop);
    // nothing changed: nothing is loaded again
    b.press(Button::Sampling);
    assert_eq!(b.ui().smp.note, "", "{}", b.ui().smp.note);
    b.press(Button::Sampling);
    // PLUG-IN looks too
    std::fs::write(d.join("zeta.wav"), tone(0.5)).unwrap();
    b.press(Button::PlugIn);
    assert_eq!(b.ui().plug_note, "loaded 1 sound from the folder", "{}", b.ui().plug_note);
    b.press(Button::PlugIn);
    // a file that is gone: its place is emptied
    std::fs::remove_file(d.join("zeta.wav")).unwrap();
    b.press(Button::Sampling);
    assert_eq!(b.ui().smp.note, "loaded 1 sound from the folder", "an emptied place is a change too");
    let _ = std::fs::remove_dir_all(&d);
}

// F-A24: what SAMPLING saves is in the instruments already, so looking at the folder does not load it again.
#[test]
fn f_a24_a_recording_is_not_loaded_twice() {
    let d = dir("twice");
    let mut b = Bench::demo();
    b.set_samples_dir(d.clone());
    b.press(Button::Play);
    b.run_ms(300);
    b.press(Button::Sampling);
    b.press(Button::Rec);
    b.run_ms(1500);
    b.press(Button::Rec);
    b.run_ms(100);
    b.press(Button::Stop);
    b.tap(pad_of(0));
    assert!(b.ui().smp.note.contains("saved in user 1"), "{}", b.ui().smp.note);
    b.press(Button::Sampling);
    b.press(Button::Sampling);
    assert_eq!(b.ui().smp.note, "", "the file it wrote is the one it has: {}", b.ui().smp.note);
    let _ = std::fs::remove_dir_all(&d);
}

// F-A23: SHIFT + REC in SAMPLING waits for a sound: the screen says so and shows where the sound has to get to, nothing is kept while it
// waits, the sound starts it, and REC (or leaving) disarms it; the pads and the source knob wait.
#[test]
fn f_a23_shift_rec_waits_for_the_sound() {
    let mut b = Bench::demo();
    b.press(Button::Sampling);
    b.run_ms(100);
    let idle = b.screen_hash(Screen::Right);
    b.button_down(Button::Shift);
    b.press(Button::Rec);
    b.button_up(Button::Shift);
    b.run_ms(100);
    assert!(b.ui().smp.armed && !b.ui().smp.recording, "armed, not recording");
    assert_eq!(b.ui().smp.len, 0, "nothing is kept while it waits");
    assert_ne!(b.screen_hash(Screen::Right), idle, "the screen says it is waiting");
    b.capture("f-a23-armed");
    b.tap(pad_of(0));
    assert!(b.ui().smp.note.contains("stop recording first"), "{}", b.ui().smp.note);
    // the sound: the sequencer plays
    b.press(Button::Sampling);
    b.press(Button::Sampling); // leaving disarms
    assert!(!b.ui().smp.armed || !b.status().armed.load(Relaxed));
    b.run_ms(100);
    assert!(!b.ui().smp.armed, "leaving SAMPLING disarms it");
    b.press(Button::Stop);
    b.run_ms(300);
    b.button_down(Button::Shift);
    b.press(Button::Rec);
    b.button_up(Button::Shift);
    b.run_ms(200);
    assert!(b.ui().smp.armed, "armed again, with nothing playing");
    b.press(Button::Play);
    b.run_ms(900);
    assert!(b.ui().smp.recording && !b.ui().smp.armed, "the sound started it");
    assert!(b.ui().smp.len > 480, "and it has the sound: {}", b.ui().smp.len);
    b.press(Button::Rec);
    b.run_ms(100);
    assert!(!b.ui().smp.recording, "REC stops it");
    // armed and then REC again: disarmed
    b.press(Button::Stop);
    b.run_ms(3000); // the tails of the sound die away: they would start it
    b.button_down(Button::Shift);
    b.press(Button::Rec);
    b.button_up(Button::Shift);
    b.run_ms(100);
    assert!(b.ui().smp.armed);
    b.press(Button::Rec);
    b.run_ms(100);
    assert!(!b.ui().smp.armed && !b.ui().smp.recording, "REC while it waits disarms it");
}

// F-A22: the sampler with `loop` on slices plays a part of the sound for each note, from the steps and the pads, as a kit made of it.
#[test]
fn f_a22_slices_play_from_the_steps() {
    let mut b = Bench::empty();
    b.press(Button::Group(4)); // E: the sampler
    assert_eq!(b.instrument(4).as_deref(), Some("sampler"));
    b.press(Button::Screen(6)); // the second page
    b.knob(6, 400); // loop: one shot, loop, slices 8
    b.run_ms(100);
    assert_eq!(b.param(4, 6), 0.5, "loop is on slices 8");
    b.capture("f-a22-slices");
    b.press(Button::Screen(5));
    b.tap(12);
    b.press(Button::Play);
    b.run_ms(1500);
    assert!(b.rms_db(0, 1500) > -60.0, "a slice is heard: {} dB", b.rms_db(0, 1500));
}
