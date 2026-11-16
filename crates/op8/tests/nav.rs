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
    assert_eq!(op8_ui::nav_index(b.ui()), 0, "it starts on the first place");
    let hash0 = b.screen_hash(Screen::Left);
    b.encoder(1);
    b.run_ms(50);
    b.capture("f-n01-file-focus");
    assert_eq!(op8_ui::nav_index(b.ui()), 1);
    assert_ne!(b.screen_hash(Screen::Left), hash0, "the ring moved with it");
    b.encoder(-2);
    assert_eq!(op8_ui::nav_index(b.ui()), 15, "turning back from the first wraps to the last");
    b.press(Button::JogUp);
    assert_eq!(op8_ui::nav_index(b.ui()), 11, "up is a row up");
    b.press(Button::JogLeft);
    assert_eq!(op8_ui::nav_index(b.ui()), 10);
    b.press(Button::JogRight);
    b.press(Button::JogRight);
    assert_eq!(op8_ui::nav_index(b.ui()), 11, "the last column has nothing to its right");
    // back on place 2 (dub), push it
    b.encoder(2 - op8_ui::nav_index(b.ui()) as i32);
    assert_eq!(op8_ui::nav_index(b.ui()), 2);
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
    assert_eq!(op8_ui::nav_index(b.ui()), 2, "FmBass is the third");
    b.encoder(2);
    assert_eq!(op8_ui::nav_index(b.ui()), 4);
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
    assert_eq!(op8_ui::nav_index(b.ui()), 8);
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
    assert_eq!(op8_ui::nav_index(b.ui()), 16, "the sixteen places, then the first value: source");
    let src = b.ui().smp.source;
    b.encoder(-1);
    b.encoder(1);
    assert_eq!(b.ui().smp.source, src, "turning without grabbing moves the focus, not the value");
    b.press(Button::JogPush);
    assert!(b.ui().grabbed);
    b.encoder(-1);
    assert_ne!(b.ui().smp.source, src, "grabbed, turning changes the value");
    assert_eq!(op8_ui::nav_index(b.ui()), 16, "and the focus stays");
    b.press(Button::JogPush);
    assert!(!b.ui().grabbed);
    b.encoder(2);
    assert_eq!(op8_ui::nav_index(b.ui()), 18, "let go, it moves again");
    b.press(Button::JogPush);
    b.press(Button::JogLeft);
    assert!(!b.ui().grabbed, "an arrow lets it go and moves");
    assert_eq!(op8_ui::nav_index(b.ui()), 17);
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

/// Turn the 4D until the focus is on `kind` (at most a lap).
fn focus_on(b: &mut Bench, kind: op8_ui::NavKind) {
    for _ in 0..60 {
        if b.ui().focus == kind {
            return;
        }
        b.encoder(1);
    }
    panic!("the focus never reached {kind:?}");
}

// F-N07: composing has a focus too: turning the 4D visits the eight screen buttons, the eight tracks, the sixteen pads of the grid and
// the eight values (forty things) and comes back round; the ring is drawn and moves.
#[test]
fn f_n07_composing_has_a_focus_that_visits_everything() {
    use op8_ui::NavKind;
    let mut b = Bench::demo();
    b.press(Button::Group(1)); // KickSyn: a drum kit has its eight sounds to visit too (F-N14)
    b.run_ms(100);
    let start = b.ui().focus;
    let mut seen = std::collections::HashSet::new();
    let mut hashes = std::collections::HashSet::new();
    for _ in 0..40 {
        seen.insert(format!("{:?}", b.ui().focus));
        hashes.insert((b.screen_hash(Screen::Left), b.screen_hash(Screen::Right)));
        b.encoder(1);
        b.run_ms(20);
    }
    assert_eq!(b.ui().focus, start, "forty things, forty turns: back where it started");
    assert_eq!(seen.len(), 40, "every one of them was visited once: {seen:?}");
    assert!(hashes.len() > 20, "the ring moves on the screens");
    let strips = seen.iter().filter(|k| k.starts_with("Strip")).count();
    assert_eq!((strips, seen.iter().filter(|k| k.starts_with("Track")).count()), (8, 8));
    let _ = NavKind::Track(0);
}

// F-N08: pushing does what tapping does: a screen button changes the page, a track is chosen, a pad of the grid toggles its step,
// SHIFT + push on a pad opens the step's inspector and push closes it.
#[test]
fn f_n08_pushing_does_what_tapping_does() {
    use op8_ui::{LeftPage, NavKind};
    let mut b = Bench::demo();
    focus_on(&mut b, NavKind::Strip(2)); // players
    b.press(Button::JogPush);
    assert_eq!(b.ui().left_page, LeftPage::Players);
    focus_on(&mut b, NavKind::Strip(0)); // step
    b.press(Button::JogPush);
    assert_eq!(b.ui().left_page, LeftPage::Steps);
    focus_on(&mut b, NavKind::Strip(6)); // fx
    b.press(Button::JogPush);
    assert_eq!(b.ui().right_page, 2);
    focus_on(&mut b, NavKind::Track(3));
    b.press(Button::JogPush);
    assert_eq!(b.ui().track, 3);
    b.press(Button::Group(1)); // a track with a kick
    let before = b.project().tracks[1].steps[1].active;
    focus_on(&mut b, NavKind::Cell(1));
    b.press(Button::JogPush);
    assert_ne!(b.project().tracks[1].steps[1].active, before, "a pad of the grid toggles its step");
    b.button_down(Button::Shift);
    b.press(Button::JogPush);
    b.button_up(Button::Shift);
    assert_eq!(b.ui().inspect_step, Some(1), "SHIFT + push opens the step");
    b.press(Button::JogPush);
    assert_eq!(b.ui().inspect_step, None, "and push closes it");
}

// F-N09: a value of a footer is grabbed with push, turned, and let go: the knob under it does what turning it does.
#[test]
fn f_n09_a_knob_value_is_grabbed_and_turned() {
    use op8_ui::NavKind;
    let mut b = Bench::demo();
    b.press(Button::Group(1)); // KickSyn
    focus_on(&mut b, NavKind::Value(4)); // the first of the right screen: tune
    let tune = b.param(1, 0);
    b.encoder(1);
    assert_eq!(b.param(1, 0), tune, "not grabbed: turning moves the focus, not the value");
    focus_on(&mut b, NavKind::Value(4));
    b.press(Button::JogPush);
    b.encoder(2);
    assert!(b.param(1, 0) > tune, "grabbed: turning changes the knob: {} vs {tune}", b.param(1, 0));
    assert_eq!(b.ui().focus, NavKind::Value(4));
    b.press(Button::JogPush);
    let kept = b.param(1, 0);
    b.encoder(1);
    assert_eq!(b.param(1, 0), kept, "let go: it moves the focus again");
}

// F-N10: the arrows cross from the left screen to the right one; with KEYBOARD, ◀ ▶ stay the octave and the turn and ▲ ▼ move the focus.
#[test]
fn f_n10_the_arrows_cross_the_screens_and_keyboard_keeps_its_octave() {
    use op8_ui::NavKind;
    let mut b = Bench::demo();
    focus_on(&mut b, NavKind::Value(3)); // the last of the left footer
    b.press(Button::JogRight);
    let f = b.ui().focus;
    assert!(matches!(f, NavKind::Value(k) if k >= 4), "right of the left footer is the right footer: {f:?}");
    b.press(Button::JogLeft);
    assert!(matches!(b.ui().focus, NavKind::Value(k) if k < 4));
    b.press(Button::Keyboard);
    let before = b.ui().focus;
    let oct = b.ui().octave;
    b.press(Button::JogRight);
    assert_eq!(b.ui().focus, before, "in KEYBOARD ◀ ▶ are the octave");
    assert_ne!(b.ui().octave, oct);
    b.press(Button::JogUp);
    assert_ne!(b.ui().focus, before, "but ▲ moves the focus");
}

// F-N11: in PERFORM the focus visits the sixteen effects, and its frame is on the place where each one is drawn (the pad that plays
// it is that effect); Brain, which draws no pads, lists none.
#[test]
fn f_n11_perform_lists_the_sixteen_effects_where_they_are_drawn() {
    use op8_ui::NavKind;
    let mut b = Bench::demo();
    b.press(Button::Perform);
    let cells: Vec<_> = op8_ui::nav_items(b.ui()).into_iter().filter(|i| matches!(i.kind, NavKind::Cell(_))).collect();
    assert_eq!(cells.len(), 16);
    for it in &cells {
        let NavKind::Cell(c) = it.kind else { unreachable!() };
        let slot = op8_ui::nav_punch_slot(c as usize);
        // punch_view: pad 0 bottom-left, 116 px between columns, 42 between rows
        let (x, y) = (12.0 + (slot % 4) as f32 * 116.0, 52.0 + (3 - slot / 4) as f32 * 42.0);
        assert_eq!((it.rect.0, it.rect.1), (x, y), "cell {c} is the effect of pad {slot}");
        assert_eq!(pad_of(c as usize) as usize, slot, "the pad of reading place {c}");
    }
    b.press(Button::Chords);
    assert!(!op8_ui::nav_items(b.ui()).iter().any(|i| matches!(i.kind, NavKind::Cell(_))), "Brain draws no pads");
}

// F-N12: pushing an effect engages it and keeps it, turning is its intensity while the arrows still move the focus, a second push
// lets it go, and leaving PERFORM lets it go too.
#[test]
fn f_n12_push_engages_an_effect_and_the_wheel_is_its_intensity() {
    use op8_ui::NavKind;
    let mut b = Bench::demo();
    b.press(Button::Perform);
    focus_on(&mut b, NavKind::Cell(0)); // top-left: pad 12, the effect in slot 12
    b.press(Button::JogPush);
    let slot = op8_ui::nav_punch_slot(0);
    assert_eq!(b.ui().punch, Some((slot, 0.5)), "engaged at half and kept after the push");
    b.encoder(5);
    assert!(b.ui().punch.unwrap().1 > 0.6, "the wheel is the intensity: {:?}", b.ui().punch);
    assert_eq!(b.ui().focus, NavKind::Cell(0), "and it does not move the focus");
    b.press(Button::JogRight);
    assert_eq!(b.ui().focus, NavKind::Cell(1), "the arrows do");
    assert_eq!(b.ui().punch.map(|p| p.0), Some(slot), "and the effect goes on sounding");
    b.press(Button::JogPush);
    assert_eq!(b.ui().punch.map(|p| p.0), Some(op8_ui::nav_punch_slot(1)), "another effect takes its place");
    b.press(Button::JogPush);
    assert_eq!(b.ui().punch, None, "the second push lets it go");
    b.press(Button::JogPush);
    assert!(b.ui().punch.is_some());
    b.press(Button::Step);
    assert_eq!(b.ui().punch, None, "leaving PERFORM lets it go");
}

// F-N14: a drum kit draws its eight sounds on the right screen and the 4D reaches them: the focus visits them, pushing one chooses it
// (the same as the `sound` knob: the screen shows it large, the lane of the steps is its) and plays it; another instrument has none.
#[test]
fn f_n14_the_sounds_of_a_kit_can_be_chosen_with_the_4d() {
    use op8_ui::NavKind;
    let mut b = Bench::demo();
    b.run_ms(100);
    let sounds = |b: &Bench| op8_ui::nav_items(b.ui()).iter().filter(|i| matches!(i.kind, NavKind::Sound(_))).count();
    assert_eq!(sounds(&b), 8, "the drums, track A");
    assert_eq!(b.ui().lane, Some(0));
    focus_on(&mut b, NavKind::Sound(5));
    let before = b.screen_hash(Screen::Right);
    b.press(Button::JogPush);
    b.run_ms(100);
    assert_eq!(b.ui().lane, Some(5), "the sixth sound is the chosen one");
    assert_ne!(b.screen_hash(Screen::Right), before, "and the right screen shows it");
    assert_eq!(b.ui().focus, NavKind::Sound(5), "the focus stays on it");
    b.press(Button::Group(1)); // KickSyn
    b.run_ms(100);
    assert_eq!(sounds(&b), 0, "an instrument that is not a kit has no sounds to choose");
}
