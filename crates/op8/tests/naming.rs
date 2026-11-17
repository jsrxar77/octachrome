//! Naming a project or a preset on the unit (docs/features.md, F-N16 and F-N17).
use op8::bench::Bench;
use op8_hw::{Button, Screen};
use op8_ui::Main;

fn pad_of(slot: usize) -> u8 {
    (0..16u8).find(|&p| op8::Controller::pad_to_step(p) as usize == slot).unwrap()
}

fn dir(name: &str) -> std::path::PathBuf {
    let d = std::env::temp_dir().join(format!("op8-naming-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    d
}

fn shift_tap(b: &mut Bench, pad: u8) {
    b.button_down(Button::Shift);
    b.tap(pad);
    b.button_up(Button::Shift);
}

fn select_tap(b: &mut Bench, pad: u8) {
    b.button_down(Button::Select);
    b.tap(pad);
    b.button_up(Button::Select);
}

/// Turn the encoder until the letter under the cursor is `c`.
fn letter(b: &mut Bench, c: char) {
    for _ in 0..70 {
        if b.ui().naming.chars[b.ui().naming.cursor] == c {
            return;
        }
        b.encoder(1);
    }
    panic!("the letter {c} never came");
}

// F-N16: SELECT held and a pad of FILE chooses a place of the player's own that has a project to be named; the 4D turns the letter under
// the cursor, ◀ ▶ and the pads move the cursor, push saves the name next to the file and goes back to FILE, which shows it.
#[test]
fn f_n16_a_project_is_named_on_the_unit() {
    let d = dir("project");
    let mut b = Bench::demo();
    b.set_projects_dir(d.clone());
    b.press(Button::File);
    shift_tap(&mut b, pad_of(10)); // saved in user 3, named by the day
    select_tap(&mut b, pad_of(10));
    assert_eq!(b.ui().main, Main::Name);
    assert!(b.ui().naming.what.contains("user 3"), "{}", b.ui().naming.what);
    b.run_ms(100);
    b.capture("f-n16-name");
    // the name it had is the start; clear, then type "my jam" with the 4D and the arrows
    b.tap(pad_of(12)); // clear
    assert_eq!(b.ui().naming.text(), "");
    letter(&mut b, 'm');
    b.press(Button::JogRight);
    letter(&mut b, 'y');
    b.press(Button::JogRight);
    b.press(Button::JogRight); // a space
    letter(&mut b, 'j');
    b.tap(pad_of(4)); // the pad of place 5 puts the cursor there
    assert_eq!(b.ui().naming.cursor, 4);
    letter(&mut b, 'a');
    b.press(Button::JogRight);
    letter(&mut b, 'm');
    b.run_ms(100);
    assert_eq!(b.ui().naming.text(), "my jam");
    // delete and insert move the rest; SHIFT + turn goes ten at a time
    b.tap(pad_of(1));
    b.tap(pad_of(14)); // insert: a space opens at place 2
    assert_eq!(b.ui().naming.text(), "m y jam");
    b.tap(pad_of(13)); // delete it again
    assert_eq!(b.ui().naming.text(), "my jam");
    b.tap(pad_of(11));
    b.button_down(Button::Shift);
    b.encoder(1);
    b.button_up(Button::Shift);
    assert_eq!(b.ui().naming.chars[11], 'j', "ten letters from the space: the tenth small letter");
    b.encoder(-10);
    assert_eq!(b.ui().naming.chars[11], ' ');
    // push saves, goes back to FILE, which shows the name; the file on disk has it
    b.press(Button::JogPush);
    assert_eq!(b.ui().main, Main::File);
    assert_eq!(std::fs::read_to_string(d.join("user-3.name")).unwrap().trim(), "my jam");
    assert_eq!(b.ui().files[10].0, "my jam");
    assert!(b.ui().file_note.contains("named my jam"), "{}", b.ui().file_note);
    // the pad that is `save` does the same: a name changed and saved by it
    select_tap(&mut b, pad_of(10));
    assert_eq!(b.ui().naming.text(), "my jam", "it starts from the name it has");
    b.tap(pad_of(0));
    letter(&mut b, 'M');
    b.tap(pad_of(15));
    assert_eq!(b.ui().main, Main::File);
    assert_eq!(b.ui().files[10].0, "My jam");
    // an empty name takes the name away
    select_tap(&mut b, pad_of(10));
    b.tap(pad_of(12));
    b.press(Button::JogPush);
    assert_eq!(b.ui().files[10].0, "user 3");
    assert!(!d.join("user-3.name").exists());
    // FILE cancels: nothing is changed
    select_tap(&mut b, pad_of(10));
    b.tap(pad_of(12));
    letter(&mut b, 'z');
    b.press(Button::File);
    assert_eq!(b.ui().main, Main::File);
    assert_eq!(b.ui().files[10].0, "user 3");
    let _ = std::fs::remove_dir_all(&d);
}

// F-N16: the places that cannot be named say why and leave the view as it is: a factory project, and a place with nothing in it.
#[test]
fn f_n16_only_places_with_something_are_named() {
    let d = dir("refuse");
    let mut b = Bench::demo();
    b.set_projects_dir(d.clone());
    b.press(Button::File);
    select_tap(&mut b, pad_of(2)); // a factory project
    assert_eq!(b.ui().main, Main::File);
    assert!(b.ui().file_note.contains("keep their names"), "{}", b.ui().file_note);
    select_tap(&mut b, pad_of(12)); // a place of her own with nothing in it
    assert_eq!(b.ui().main, Main::File);
    assert!(b.ui().file_note.contains("save a project first"), "{}", b.ui().file_note);
    let _ = std::fs::remove_dir_all(&d);
}

// F-N17: the same in BROWSER for the presets of an instrument; the screens say what they are, and the pad LEDs show the twelve places, the
// cursor brighter, and the four actions.
#[test]
fn f_n17_a_preset_is_named_and_the_screens_move() {
    let d = dir("preset");
    let mut b = Bench::demo();
    b.set_presets_dir(d.clone());
    b.press(Button::Browser);
    shift_tap(&mut b, pad_of(9)); // the drum kit's user 2
    assert!(b.ui().browse_note.contains("saved in user 2"), "{}", b.ui().browse_note);
    select_tap(&mut b, pad_of(9));
    assert_eq!(b.ui().main, Main::Name);
    assert!(b.ui().naming.what.contains("preset user 2"), "{}", b.ui().naming.what);
    b.run_ms(100);
    let before = b.screen_hash(Screen::Right);
    b.encoder(5);
    b.run_ms(100);
    assert_ne!(b.screen_hash(Screen::Right), before, "the strip of letters turns");
    let leds = |b: &Bench, slot: usize| b.leds().pad(pad_of(slot));
    assert!(leds(&b, 0) != leds(&b, 1), "the cursor's pad is brighter");
    assert_eq!(leds(&b, 1), leds(&b, 5), "the other places are alike");
    b.capture("f-n17-letters");
    b.tap(pad_of(12));
    letter(&mut b, 'k');
    b.press(Button::JogPush);
    assert_eq!(b.ui().main, Main::Browser);
    assert_eq!(b.ui().browse[9].0, "k");
    assert!(b.ui().browse_note.contains("named k"), "{}", b.ui().browse_note);
    let _ = std::fs::remove_dir_all(&d);
}

// F-N18: SELECT held and a pad of SAMPLING names the recording kept in that place (a place with no recording says so), push saves the name
// next to the file and goes back to SAMPLING, whose cell shows it; an empty name takes it away.
#[test]
fn f_n18_a_recording_is_named_on_the_unit() {
    let d = dir("recording");
    std::fs::create_dir_all(&d).unwrap();
    let mut b = Bench::demo();
    b.set_samples_dir(d.clone());
    b.press(Button::Play);
    b.run_ms(300);
    b.press(Button::Sampling);
    b.press(Button::Rec);
    b.run_ms(1000);
    b.press(Button::Rec);
    b.run_ms(100);
    b.press(Button::Stop);
    b.tap(pad_of(2)); // saved in user 3
    assert!(b.ui().smp.note.contains("saved in user 3"), "{}", b.ui().smp.note);
    // a place with nothing in it
    select_tap(&mut b, pad_of(5));
    assert_eq!(b.ui().main, Main::Sampling, "nothing to name there");
    assert!(b.ui().smp.note.contains("nothing there to name"), "{}", b.ui().smp.note);
    let before = b.screen_hash(Screen::Left);
    select_tap(&mut b, pad_of(2));
    assert_eq!(b.ui().main, Main::Name);
    assert!(b.ui().naming.what.contains("recording user 3"), "{}", b.ui().naming.what);
    for c in ['k', 'i', 'c', 'k'] {
        letter(&mut b, c);
        b.press(Button::JogRight);
    }
    b.press(Button::JogPush);
    assert_eq!(b.ui().main, Main::Sampling);
    assert_eq!(b.ui().smp.names[2], "kick");
    assert!(b.ui().smp.note.contains("named kick"), "{}", b.ui().smp.note);
    b.run_ms(100);
    assert_ne!(b.screen_hash(Screen::Left), before, "the cell shows the name");
    b.capture("f-n18-named-recording");
    assert_eq!(std::fs::read_to_string(d.join("rec-3.name")).unwrap().trim(), "kick");
    // and an empty name takes it away
    select_tap(&mut b, pad_of(2));
    for _ in 0..12 {
        b.tap(pad_of(12)); // clear
    }
    b.press(Button::JogPush);
    assert!(b.ui().smp.names[2].is_empty());
    assert!(!d.join("rec-3.name").exists());
    let _ = std::fs::remove_dir_all(&d);
}
