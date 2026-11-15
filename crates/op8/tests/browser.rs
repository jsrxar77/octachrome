//! BROWSER: the presets of an instrument (docs/features.md §K, §D).
use op8::bench::Bench;
use op8_hw::Button;
use op8_ui::Main;

fn pad_of(slot: usize) -> u8 {
    (0..16u8).find(|&p| op8::Controller::pad_to_step(p) as usize == slot).unwrap()
}

fn dir(name: &str) -> std::path::PathBuf {
    let d = std::env::temp_dir().join(format!("op8-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    d
}

fn shift_tap(b: &mut Bench, pad: u8) {
    b.button_down(Button::Shift);
    b.tap(pad);
    b.button_up(Button::Shift);
}

// F-K15: SHIFT + a pad of the bottom two rows saves the instrument's knobs, a tap loads them back (and the engine keeps
// them), per instrument, and the files are where they should be.
#[test]
fn f_k15_a_preset_saves_and_loads_the_knobs() {
    let d = dir("presets");
    let mut b = Bench::demo();
    b.set_presets_dir(d.clone());
    b.press(Button::Group(1)); // KickSyn
    b.press(Button::Browser);
    assert_eq!(b.ui().main, Main::Browser);
    assert_eq!(b.ui().browse_inst, "KickSyn");
    assert_eq!(b.ui().browse.len(), 16);
    assert!(b.ui().browse.iter().all(|c| !c.1), "nothing saved yet");
    b.press(Button::Browser);
    assert_eq!(b.ui().main, Main::Instrument);
    // a sound to keep: the first knob turned up a good way
    b.knob(4, 300);
    let kept: Vec<f32> = (0..8).map(|i| b.param(1, i)).collect();
    b.press(Button::Browser);
    shift_tap(&mut b, pad_of(8));
    assert!(b.ui().browse_note.contains("saved in user 1"), "{}", b.ui().browse_note);
    assert!(b.ui().browse[8].1 && !b.ui().browse[9].1);
    assert!(d.join("KickSyn").join("user-1.preset").is_file());
    assert!(!d.join("KickSyn").join("user-1.tmp").exists());
    // the knob moves on, the preset brings it back
    b.press(Button::Browser);
    b.knob(4, -500);
    assert!((b.param(1, 0) - kept[0]).abs() > 0.2);
    b.press(Button::Browser);
    let from = b.now_ms();
    b.tap(pad_of(8));
    b.run_ms(300);
    assert!(b.ui().browse_note.contains("loaded user 1"));
    for i in 0..8 {
        assert!((b.param(1, i) - kept[i]).abs() < 0.01, "knob {i}: {} vs {}", b.param(1, i), kept[i]);
    }
    assert!(b.peak_db(from, b.now_ms()) > -40.0, "the preset is heard");
    // another instrument has its own presets
    b.press(Button::Browser);
    b.press(Button::Group(2));
    b.press(Button::Browser);
    assert_ne!(b.ui().browse_inst, "KickSyn");
    assert!(!b.ui().browse[8].1, "KickSyn's preset is not here");
    let _ = std::fs::remove_dir_all(&d);
}

// F-K15: it says so when there is nothing to load, when the top rows (none come with the instrument yet) are tapped,
// and when there is nowhere to save.
#[test]
fn f_k15_it_says_what_it_cannot_do() {
    let d = dir("presets-none");
    let mut b = Bench::demo();
    b.press(Button::Group(1));
    b.press(Button::Browser);
    b.tap(pad_of(9));
    assert!(b.ui().browse_note.contains("nowhere to save"), "{}", b.ui().browse_note);
    b.set_presets_dir(d.clone());
    b.press(Button::Browser);
    b.press(Button::Browser);
    b.tap(pad_of(9));
    assert!(b.ui().browse_note.starts_with("empty"), "{}", b.ui().browse_note);
    b.tap(pad_of(2));
    assert!(b.ui().browse_note.contains("no presets come"), "{}", b.ui().browse_note);
    // a damaged file is not loaded
    std::fs::create_dir_all(d.join("KickSyn")).unwrap();
    std::fs::write(d.join("KickSyn").join("user-2.preset"), b"garbage").unwrap();
    b.press(Button::Browser);
    b.press(Button::Browser);
    b.tap(pad_of(9));
    assert!(b.ui().browse_note.starts_with("empty"), "{}", b.ui().browse_note);
    let _ = std::fs::remove_dir_all(&d);
}

// F-D18: BROWSER draws both screens (the sixteen presets on the left, the instrument's name and what the pads do on the
// right), they move with the clock, and its button and the pad LEDs follow it.
#[test]
fn f_d18_browser_draws_both_screens() {
    let d = dir("presets-draw");
    let mut b = Bench::demo();
    b.set_presets_dir(d.clone());
    b.press(Button::Group(1));
    b.press(Button::Browser);
    b.run_ms(100);
    assert!(b.lit_pixels(op8_hw::Screen::Left) > 500 && b.lit_pixels(op8_hw::Screen::Right) > 300);
    b.capture("f-d18-browser");
    let l0 = b.screen_hash(op8_hw::Screen::Left);
    shift_tap(&mut b, pad_of(8));
    b.run_ms(100);
    assert_ne!(b.screen_hash(op8_hw::Screen::Left), l0, "a saved preset changes the left screen");
    assert!(b.leds().pad(pad_of(8)) & 3 >= 2, "a preset's pad is lit");
    let _ = std::fs::remove_dir_all(&d);
}
