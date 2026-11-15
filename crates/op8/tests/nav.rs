//! Navigation with the 4D encoder (docs/features.md §N): every view that has things to choose has a focus, always visible.
use op8::bench::Bench;
use op8_hw::{Button, Screen};
use op8_ui::Main;

fn pad_of(slot: usize) -> u8 {
    (0..16u8).find(|&p| op8::Controller::pad_to_step(p) as usize == slot).unwrap()
}

fn dir(name: &str) -> std::path::PathBuf {
    let d = std::env::temp_dir().join(format!("op8-nav-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    d
}

// F-N01: in FILE the focus is always on one place, the 4D encoder turns it along the places (wrapping) and its arrows move it across
// the grid, the ring is drawn where it is, and pushing opens that place exactly as tapping its pad does.
#[test]
fn f_n01_the_focus_moves_and_pushing_opens() {
    let mut b = Bench::demo();
    b.press(Button::File);
    b.run_ms(100);
    assert_eq!(b.ui().focus, 0, "it starts on the first place");
    let hash0 = b.screen_hash(Screen::Left);
    b.encoder(1);
    b.run_ms(50);
    b.capture("f-n01-file-focus");
    assert_eq!(b.ui().focus, 1);
    assert_ne!(b.screen_hash(Screen::Left), hash0, "the ring moved with it");
    b.encoder(-2);
    assert_eq!(b.ui().focus, 15, "turning back from the first wraps to the last");
    b.press(Button::JogUp);
    assert_eq!(b.ui().focus, 11, "up is a row up");
    b.press(Button::JogLeft);
    assert_eq!(b.ui().focus, 10);
    b.press(Button::JogRight);
    b.press(Button::JogRight);
    assert_eq!(b.ui().focus, 11, "the last column has nothing to its right");
    // back on place 2 (dub), push it
    b.encoder(2 - b.ui().focus as i32);
    assert_eq!(b.ui().focus, 2);
    b.press(Button::JogPush);
    assert_eq!(b.ui().main, Main::Instrument, "it opened and went back to composing");
    assert_eq!(b.project().bpm, 70.0, "the dub: 70 bpm");
}

// F-N02: the 4D encoder does not turn the tempo by itself any more: only while TEMPO is held.
#[test]
fn f_n02_the_tempo_is_turned_only_while_tempo_is_held() {
    let mut b = Bench::demo();
    let bpm = b.project().bpm;
    b.encoder(5);
    assert_eq!(b.project().bpm, bpm);
    b.button_down(Button::Tempo);
    b.encoder(5);
    b.button_up(Button::Tempo);
    assert_eq!(b.project().bpm, bpm + 5.0);
}

// F-N03: PLUG-IN starts with the focus on the instrument the track has, and pushing puts the one that has it.
#[test]
fn f_n03_plugin_focus_starts_on_the_track_s_instrument() {
    let mut b = Bench::demo();
    b.press(Button::Group(2)); // FmBass
    b.press(Button::PlugIn);
    assert_eq!(b.ui().focus, 2, "FmBass is the third");
    b.encoder(2);
    assert_eq!(b.ui().focus, 4);
    b.press(Button::JogPush); // the sampler
    assert_eq!(b.instrument(2).as_deref(), Some("sampler"));
    assert_eq!(b.ui().main, Main::Instrument);
}

// F-N04: BROWSER: push loads a preset; SHIFT + push on the player's saves.
#[test]
fn f_n04_browser_push_loads_and_shift_push_saves() {
    let d = dir("browser");
    let mut b = Bench::demo();
    b.set_presets_dir(d.clone());
    b.press(Button::Group(1));
    b.press(Button::Browser);
    b.encoder(4);
    b.press(Button::JogPush);
    assert!(b.ui().browse_note.contains("loaded click"), "{}", b.ui().browse_note);
    b.encoder(4); // the first of the player's
    assert_eq!(b.ui().focus, 8);
    b.button_down(Button::Shift);
    b.press(Button::JogPush);
    b.button_up(Button::Shift);
    assert!(b.ui().browse_note.contains("saved in user 1"), "{}", b.ui().browse_note);
    assert!(d.join("KickSyn").join("user-1.preset").is_file());
    let _ = std::fs::remove_dir_all(&d);
}

// F-N05: SAMPLING: the focus reaches the four values under the right screen; push grabs one, turning changes it, push lets it go,
// and an arrow lets it go too.
#[test]
fn f_n05_sampling_values_are_grabbed_turned_and_let_go() {
    let mut b = Bench::demo();
    b.press(Button::Sampling);
    for _ in 0..16 {
        b.encoder(1);
    }
    assert_eq!(b.ui().focus, 16, "the sixteen places, then the first value: source");
    let src = b.ui().smp.source;
    b.encoder(-1);
    b.encoder(1);
    assert_eq!(b.ui().smp.source, src, "turning without grabbing moves the focus, not the value");
    b.press(Button::JogPush);
    assert!(b.ui().grabbed);
    b.encoder(-1);
    assert_ne!(b.ui().smp.source, src, "grabbed, turning changes the value");
    assert_eq!(b.ui().focus, 16, "and the focus stays");
    b.press(Button::JogPush);
    assert!(!b.ui().grabbed);
    b.encoder(2);
    assert_eq!(b.ui().focus, 18, "let go, it moves again");
    b.press(Button::JogPush);
    b.press(Button::JogLeft);
    assert!(!b.ui().grabbed, "an arrow lets it go and moves");
    assert_eq!(b.ui().focus, 17);
    b.run_ms(50);
    assert!(b.lit_pixels(Screen::Right) > 300);
    b.capture("f-n05-sampling-focus");
}

// F-N06: opening a piece puts the screens on its music: the first track that has steps, its first page, and on a kit the sound that has steps.
#[test]
fn f_n06_opening_a_piece_shows_its_music() {
    let mut b = Bench::demo();
    b.press(Button::File);
    b.tap(pad_of(0)); // house: the kit has hats and a clap but no kick
    assert_eq!(b.ui().track, 0);
    b.run_ms(100);
    assert_eq!(b.ui().lane, Some(2), "the kit shows the closed hat, the first sound that has steps, not the empty kick");
    b.press(Button::File);
    b.tap(pad_of(6)); // ambient: nothing hits, the music starts on the resonator
    assert_eq!(b.ui().track, 3);
    assert_eq!(b.ui().right_page, 0);
}
