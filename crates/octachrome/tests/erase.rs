//! Names and erasing in FILE and BROWSER (docs/features.md §F and §K).
use octachrome::bench::Bench;
use octachrome_hw::Button;

fn pad_of(slot: usize) -> u8 {
    (0..16u8).find(|&p| octachrome::Controller::pad_to_step(p) as usize == slot).unwrap()
}

fn dir(name: &str) -> std::path::PathBuf {
    let d = std::env::temp_dir().join(format!("octachrome-erase-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    d
}

fn shift_tap(b: &mut Bench, pad: u8) {
    b.button_down(Button::Shift);
    b.tap(pad);
    b.button_up(Button::Shift);
}

fn erase_tap(b: &mut Bench, pad: u8) {
    b.button_down(Button::Erase);
    b.tap(pad);
    b.button_up(Button::Erase);
}

// F-F08: what the unit saves is named: a project by the day and the tempo, a preset by the day, in a text file next to it that the
// player can change; a place with no name is called `user N`, and a name the player wrote is the one shown (and kept when saving again).
#[test]
fn f_f08_what_is_saved_has_a_name() {
    let d = dir("names");
    let mut b = Bench::demo();
    b.set_projects_dir(d.clone());
    b.press(Button::File);
    let bpm = format!(" {:.0}", b.project().bpm);
    shift_tap(&mut b, pad_of(8));
    let name = b.ui().files[8].0.clone();
    assert!(name.ends_with(&bpm) && name.len() >= 7, "the day and the tempo: {name:?}");
    assert_eq!(std::fs::read_to_string(d.join("user-1.name")).unwrap().trim(), name);
    // the player writes a name of their own
    std::fs::write(d.join("user-1.name"), "my groove\n").unwrap();
    b.press(Button::File);
    b.press(Button::File);
    assert_eq!(b.ui().files[8].0, "my groove");
    shift_tap(&mut b, pad_of(8)); // saving again keeps it
    assert_eq!(b.ui().files[8].0, "my groove");
    // a place with a file but no name file is `user N`
    std::fs::remove_file(d.join("user-1.name")).unwrap();
    b.press(Button::File);
    b.press(Button::File);
    assert_eq!(b.ui().files[8].0, "user 1");
    let _ = std::fs::remove_dir_all(&d);
}

// F-F09: ERASE held and a pad of FILE erases one of the player's projects, on the second touch of the same place: the first marks it
// and says so, any other touch lets it be; the projects that come with it, and a place with nothing in it, are not touched.
#[test]
fn f_f09_erasing_a_project_asks_twice() {
    let d = dir("project");
    let mut b = Bench::demo();
    b.set_projects_dir(d.clone());
    b.press(Button::File);
    shift_tap(&mut b, pad_of(8));
    shift_tap(&mut b, pad_of(9));
    assert!(b.ui().files[8].1 && b.ui().files[9].1);
    erase_tap(&mut b, pad_of(8));
    assert_eq!(b.ui().confirm, Some(8), "the first touch marks it");
    assert!(b.ui().file_note.starts_with("erase ") && b.ui().file_note.ends_with("touch it again"), "{}", b.ui().file_note);
    assert!(d.join("user-1.ocp").is_file(), "and nothing is erased yet");
    b.run_ms(50);
    b.capture("f-f09-erase-marked");
    erase_tap(&mut b, pad_of(9)); // another place: the mark moves
    assert_eq!(b.ui().confirm, Some(9));
    assert!(d.join("user-1.ocp").is_file() && d.join("user-2.ocp").is_file());
    erase_tap(&mut b, pad_of(9));
    assert!(!d.join("user-2.ocp").exists() && !d.join("user-2.name").exists(), "the second touch erases the file and its name");
    assert!(b.ui().file_note.starts_with("erased"), "{}", b.ui().file_note);
    assert!(!b.ui().files[9].1 && b.ui().files[8].1, "only that place is empty now");
    assert_eq!(b.ui().confirm, None);
    erase_tap(&mut b, pad_of(9));
    assert!(b.ui().file_note.contains("nothing to erase"), "{}", b.ui().file_note);
    erase_tap(&mut b, pad_of(2)); // a project that comes with it
    assert!(b.ui().file_note.contains("stay as they are"), "{}", b.ui().file_note);
    // a tap that is not ERASE lets a marked place be
    erase_tap(&mut b, pad_of(8));
    assert_eq!(b.ui().confirm, Some(8));
    b.tap(pad_of(8)); // it opens the project: no mark, nothing erased
    assert_eq!(b.ui().confirm, None);
    assert!(d.join("user-1.ocp").is_file());
    let _ = std::fs::remove_dir_all(&d);
}

// F-K19: the same in BROWSER, for the presets of the instrument: named by the day, erased on the second touch, those that come with it stay.
#[test]
fn f_k19_a_preset_has_a_name_and_can_be_erased() {
    let d = dir("presets");
    let mut b = Bench::demo();
    b.set_presets_dir(d.clone());
    b.press(Button::Group(1));
    b.press(Button::Browser);
    shift_tap(&mut b, pad_of(8));
    let name = b.ui().browse[8].0.clone();
    assert_ne!(name, "user 1", "named by the day: {name}");
    assert!(d.join("KickSyn").join("user-1.name").is_file());
    erase_tap(&mut b, pad_of(8));
    assert_eq!(b.ui().confirm, Some(8));
    erase_tap(&mut b, pad_of(8));
    assert!(!b.ui().browse[8].1, "erased");
    assert!(!d.join("KickSyn").join("user-1.preset").exists() && !d.join("KickSyn").join("user-1.name").exists());
    erase_tap(&mut b, pad_of(0));
    assert!(b.ui().browse_note.contains("stay as they are"), "{}", b.ui().browse_note);
    assert!(b.ui().browse[0].1, "the preset that comes with it is still there");
    let _ = std::fs::remove_dir_all(&d);
}
