//! What the project saved under its first name (`OP8P` projects, `OP8S` presets) is still read.
use octachrome::bench::Bench;
use octachrome::projectfile::ProjectFile;
use octachrome_hw::Button;

fn pad_of(slot: usize) -> u8 {
    (0..16u8).find(|&p| octachrome::Controller::pad_to_step(p) as usize == slot).unwrap()
}

// F-F19: a project written with the first magic (`OP8P`) opens as it was, and one with the new magic (`OCTP`) is what is written now.
#[test]
fn f_f19_a_project_of_the_old_name_opens() {
    let file = octachrome::factory::file(1);
    let mut bytes = file.encode();
    assert_eq!(&bytes[..4], b"OCTP", "what is written now");
    assert!(ProjectFile::decode(&bytes).is_some());
    bytes[..4].copy_from_slice(b"OP8P");
    let old = ProjectFile::decode(&bytes).expect("the old magic is read");
    assert_eq!(old.project, file.project);
    bytes[..4].copy_from_slice(b"XXXX");
    assert!(ProjectFile::decode(&bytes).is_none(), "any other is refused");
}

// F-F20: a preset saved with the first magic (`OP8S`) loads its knobs back.
#[test]
fn f_f20_a_preset_of_the_old_name_loads() {
    let d = std::env::temp_dir().join(format!("octachrome-legacy-preset-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    let mut b = Bench::demo();
    b.set_presets_dir(d.clone());
    b.press(Button::Group(1));
    b.knob(4, 300);
    let kept: Vec<f32> = (0..8).map(|i| b.param(1, i)).collect();
    b.press(Button::Browser);
    b.button_down(Button::Shift);
    b.tap(pad_of(8));
    b.button_up(Button::Shift);
    let file = d.join("KickSyn").join("user-1.preset");
    let mut bytes = std::fs::read(&file).unwrap();
    assert_eq!(&bytes[..5], b"OCTS\x01", "what is written now");
    bytes[..4].copy_from_slice(b"OP8S");
    std::fs::write(&file, &bytes).unwrap();
    b.press(Button::Browser);
    b.knob(4, -500);
    b.press(Button::Browser);
    b.tap(pad_of(8));
    b.run_ms(300);
    assert!(b.ui().browse_note.contains("loaded user 1"), "{}", b.ui().browse_note);
    for i in 0..8 {
        assert!((b.param(1, i) - kept[i]).abs() < 0.01, "knob {i}");
    }
    let _ = std::fs::remove_dir_all(&d);
}
