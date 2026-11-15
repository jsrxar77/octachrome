//! FILE: saving and opening projects (docs/features.md §F).
use op8::bench::Bench;
use op8_hw::Button;
use op8_ui::{Main, Mode};

fn temp_dir(name: &str) -> std::path::PathBuf {
    let d = std::env::temp_dir().join(format!("op8-test-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    d
}

/// The pad that is the slot `slot` in reading order (the top-left pad is slot 0).
fn pad_of(slot: usize) -> u8 {
    (0..16u8).find(|&p| op8::Controller::pad_to_step(p) as usize == slot).unwrap()
}

// F-F04: FILE shows the sixteen slots on both screens (the factory projects in the top two rows); a pad opens a project and
// goes back to composing, with the project and its instruments loaded.
#[test]
fn f_f04_a_pad_opens_a_factory_project() {
    let mut b = Bench::demo();
    b.press(Button::File);
    assert_eq!(b.ui().main, Main::File);
    assert_eq!(b.ui().files.len(), 16);
    assert!(b.ui().files[..8].iter().all(|f| f.1), "the factory projects are there");
    b.run_ms(100);
    b.tap(pad_of(2)); // backbeat: 96 bpm, the kit alone
    b.run_ms(100);
    assert_eq!(b.ui().main, Main::Instrument, "opening goes back to composing");
    assert_eq!(b.ui().mode, Mode::Step);
    assert_eq!(b.project().bpm, 96.0);
    assert!(b.project().tracks[0].kit && b.project().tracks[0].steps[4].lanes & 0b100010 == 0b100010, "snare and clap together on step 5");
    assert!(!b.project().tracks[1].steps[0].active, "the demo's kick is gone");
    // the second tap of the button leaves FILE too
    b.press(Button::File);
    assert_eq!(b.ui().main, Main::File);
    b.press(Button::File);
    assert_eq!(b.ui().main, Main::Instrument);
}

// F-F05: SHIFT + a pad of the player's slots saves into it, and a tap opens it again, with the instruments' state too.
#[test]
fn f_f05_save_and_open_a_user_slot() {
    let dir = temp_dir("save");
    let mut b = Bench::demo();
    b.set_projects_dir(dir.clone());
    // change things: the tempo, a knob of the first instrument
    b.encoder(7); // tempo +7
    let bpm = b.project().bpm;
    b.press(Button::Screen(5)); // sound
    b.knob(4, 120);
    let tune = b.param(0, 0);
    b.press(Button::File);
    assert!(!b.ui().files[8].1, "the slot is empty");
    b.button_down(Button::Shift);
    b.tap(pad_of(8)); // the first of the player's slots
    b.button_up(Button::Shift);
    assert!(b.ui().file_note.starts_with("saved"), "{}", b.ui().file_note);
    assert!(dir.join("user-1.op8p").is_file());
    assert!(b.ui().files[8].1, "now it is occupied");
    // open another project, then come back to the saved one
    b.tap(pad_of(2));
    assert_ne!(b.project().bpm, bpm);
    b.press(Button::File);
    b.tap(pad_of(8));
    assert_eq!(b.project().bpm, bpm, "the tempo is back");
    b.run_ms(100);
    assert!((b.param(0, 0) - tune).abs() < 1e-3, "and so is the instrument's knob");
    let _ = std::fs::remove_dir_all(&dir);
}

// F-F06: an empty slot says so and does not load; SHIFT on a factory slot does not overwrite it; a damaged file is refused
// and the project stays as it was.
#[test]
fn f_f06_empty_factory_and_damaged_slots() {
    let dir = temp_dir("damaged");
    let mut b = Bench::demo();
    b.set_projects_dir(dir.clone());
    let before = *b.project();
    b.press(Button::File);
    b.tap(pad_of(10)); // an empty slot of the player's
    assert_eq!(b.ui().main, Main::File, "still in FILE");
    assert!(b.ui().file_note.starts_with("empty"));
    b.button_down(Button::Shift);
    b.tap(pad_of(1)); // a factory slot
    b.button_up(Button::Shift);
    assert!(b.ui().file_note.contains("factory"));
    // a damaged file
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("user-1.op8p"), b"not a project").unwrap();
    b.press(Button::File); // out of FILE
    b.press(Button::File); // and in again, so it looks at the folder
    b.tap(pad_of(8));
    assert_eq!(b.ui().main, Main::File, "a damaged file is not loaded");
    assert_eq!(*b.project(), before, "and the project is untouched");
    let _ = std::fs::remove_dir_all(&dir);
}
