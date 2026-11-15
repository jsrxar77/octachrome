//! Settings, the main mode that takes both screens (docs/features.md §C).
//!
//! The theme is global, so these tests take a lock and put it back.
use op8::bench::Bench;
use op8_gfx::{theme_id, ThemeId};
use op8_hw::{leds::button_slot, Button, Screen};
use op8_ui::{Main, Mode};
use std::sync::{Mutex, MutexGuard};

static THEME: Mutex<()> = Mutex::new(());

/// Holds the theme lock and starts from glossy; restores glossy when dropped.
struct Fresh(#[allow(dead_code)] MutexGuard<'static, ()>);

fn fresh() -> Fresh {
    let guard = THEME.lock().unwrap_or_else(|e| e.into_inner());
    op8_gfx::set_theme(ThemeId::Glossy);
    Fresh(guard)
}

impl Drop for Fresh {
    fn drop(&mut self) {
        op8_gfx::set_theme(ThemeId::Glossy);
    }
}

fn led(b: &Bench, button: Button) -> u8 {
    b.leds().slots[button_slot(button).expect("button has an LED")]
}

// F-C01: SETTINGS takes both screens, lights its LED, and the pads go dark.
#[test]
fn f_c01_settings_takes_both_screens() {
    let _t = fresh();
    let mut b = Bench::demo();
    b.run_ms(100);
    let (l, r) = (b.screen_hash(Screen::Left), b.screen_hash(Screen::Right));
    assert_eq!(b.ui().main, Main::Instrument, "composing by default");
    assert!(led(&b, Button::Settings) < 40 && led(&b, Button::ChannelMidi) > 100);

    b.press(Button::Settings);
    b.run_ms(100);
    assert_eq!(b.ui().main, Main::Settings);
    assert_ne!(b.screen_hash(Screen::Left), l, "left screen did not change");
    assert_ne!(b.screen_hash(Screen::Right), r, "right screen did not change");
    assert!(led(&b, Button::Settings) > 100, "SETTINGS should be lit");
    assert!(led(&b, Button::ChannelMidi) < 40, "CHANNEL should be dim");
    assert!((0..16).all(|p| b.leds().pad(p) == 0), "pads should be dark in settings");
    b.capture("f-c01-settings");
}

// F-C02: CHANNEL goes back to composing, and pressing SETTINGS again does too.
#[test]
fn f_c02_channel_and_settings_go_back() {
    let _t = fresh();
    let mut b = Bench::demo();
    b.press(Button::Chords);
    b.press(Button::Settings);
    assert_eq!(b.ui().main, Main::Settings);
    b.press(Button::ChannelMidi);
    assert_eq!(b.ui().main, Main::Instrument);
    assert_eq!(b.ui().mode, Mode::Brain, "the pad mode is kept");
    assert!(led(&b, Button::ChannelMidi) > 100 && led(&b, Button::Settings) < 40);

    b.press(Button::Settings);
    b.press(Button::Settings);
    assert_eq!(b.ui().main, Main::Instrument, "SETTINGS toggles");
}

// F-C03: in settings the 4D encoder (and ◀ ▶) change the theme, wrapping, and the
// screens change; outside settings the encoder still sets the tempo.
#[test]
fn f_c03_encoder_changes_the_theme() {
    let _t = fresh();
    let mut b = Bench::demo();
    let bpm = b.project().bpm;
    b.encoder(3);
    assert_eq!(theme_id(), ThemeId::Glossy, "outside settings the encoder is the tempo");
    assert_ne!(b.project().bpm, bpm);

    b.press(Button::Settings);
    b.run_ms(100);
    let mut seen = vec![b.screen_hash(Screen::Right)];
    for expect in [ThemeId::Chrome, ThemeId::Matte, ThemeId::Glossy] {
        b.encoder(1);
        b.run_ms(100);
        assert_eq!(theme_id(), expect);
        seen.push(b.screen_hash(Screen::Right));
    }
    assert_eq!(seen[0], seen[3], "back on glossy the right screen is the same again");
    assert!(seen[0] != seen[1] && seen[1] != seen[2], "each theme should look different");

    b.encoder(-1);
    assert_eq!(theme_id(), ThemeId::Matte, "turning back goes to the one before glossy");
    b.press(Button::JogLeft);
    assert_eq!(theme_id(), ThemeId::Chrome, "and wraps from the first to the last");
    b.press(Button::JogRight);
    assert_eq!(theme_id(), ThemeId::Matte);
    b.capture("f-c03-matte");
}

// F-C04: STEP, CHORDS and PERFORM leave settings and choose their pad mode.
#[test]
fn f_c04_pad_mode_buttons_leave_settings() {
    let _t = fresh();
    let mut b = Bench::demo();
    for (button, mode) in [
        (Button::Chords, Mode::Brain),
        (Button::Perform, Mode::Punch),
        (Button::Step, Mode::Step),
    ] {
        b.press(Button::Settings);
        b.press(button);
        assert_eq!(b.ui().main, Main::Instrument, "{button:?}");
        assert_eq!(b.ui().mode, mode, "{button:?}");
    }
}

// F-C05: in settings the pads, knobs and groups do nothing, and PLAY still works.
#[test]
fn f_c05_only_navigation_and_transport_work_in_settings() {
    let _t = fresh();
    let mut b = Bench::demo();
    b.press(Button::Group(2));
    let (step, param, track) = (b.project().tracks[2].steps[3].active, b.param(2, 1), b.ui().track);
    b.press(Button::Settings);

    b.tap(15); // would toggle step 4
    b.knob(5, 300);
    b.press(Button::Group(0));
    assert_eq!(b.project().tracks[2].steps[3].active, step, "a pad changed a step");
    assert_eq!(b.param(2, 1), param, "a knob moved a parameter");
    assert_eq!(b.ui().track, track, "a group selected a track");

    b.press(Button::Play);
    assert!(b.status().playing.load(std::sync::atomic::Ordering::Relaxed), "PLAY must work in settings");
}

// F-C06 is in crates/op8/src/settings.rs (the file: parse, damaged, names, save).

// F-C07: a theme change is saved to the settings file, and only then; with no
// path (the simulator, the bench) nothing is written.
#[test]
fn f_c07_theme_change_is_saved() {
    let _t = fresh();
    let dir = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join("settings-c07");
    let _ = std::fs::remove_dir_all(&dir);
    let path = dir.join("settings");

    let mut b = Bench::demo();
    b.app.settings_path = Some(path.clone());
    b.press(Button::Settings);
    assert!(!path.exists(), "nothing to save before a change");
    b.encoder(2); // glossy → chrome → matte
    assert_eq!(std::fs::read_to_string(&path).expect("saved"), "theme=matte\n");
    b.encoder(1); // glossy
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "theme=glossy\n");

    // No path: the encoder changes the theme and the disk is not touched.
    let mut b = Bench::demo();
    b.press(Button::Settings);
    b.encoder(1);
    assert!(b.app.settings_path.is_none());
}

// F-C09: the theme changes the pad and group LEDs, not only the screens.
#[test]
fn f_c09_theme_changes_the_pad_and_group_leds() {
    let _t = fresh();
    let mut b = Bench::demo();
    b.press(Button::Group(1));
    let pad_colour = |b: &Bench| {
        let on = (0..16u8)
            .find(|&p| {
                let step = op8::Controller::pad_to_step(p) as usize;
                step < 16 && b.project().tracks[1].steps[step].active
            })
            .expect("an active step");
        b.leds().pad(on) >> 2
    };
    let mut seen = Vec::new();
    for id in ThemeId::ALL {
        op8_gfx::set_theme(id);
        b.tick();
        let want = op8::controller::track_led_colors(id)[1] as u8;
        assert_eq!(pad_colour(&b), want, "{id:?}: the pad takes the theme's track colour");
        let group = button_slot(Button::Group(1)).unwrap();
        assert_eq!(b.leds().slots[group] >> 2, want, "{id:?}: so does the group key");
        seen.push(want);
    }
    assert!(seen[0] != seen[1] || seen[1] != seen[2], "the three themes do not light the pads alike");
    assert_ne!(op8::controller::track_led_colors(ThemeId::Chrome), op8::controller::track_led_colors(ThemeId::Glossy));
}

// F-C09, the way a player gets there: SETTINGS, turn the 4D encoder, back to CHANNEL.
#[test]
fn f_c09_choosing_a_theme_in_settings_recolours_the_pads() {
    let _t = fresh();
    let mut b = Bench::demo();
    b.press(Button::Group(1));
    let pad = |b: &Bench| {
        let on = (0..16u8)
            .find(|&p| {
                let step = op8::Controller::pad_to_step(p) as usize;
                step < 16 && b.project().tracks[1].steps[step].active
            })
            .expect("an active step");
        b.leds().pad(on) >> 2
    };
    b.tick();
    let glossy = pad(&b);
    b.press(Button::Settings);
    b.encoder(1); // chrome
    b.encoder(1); // matte
    b.press(Button::ChannelMidi); // back to composing
    b.tick();
    assert_eq!(theme_id(), ThemeId::Matte);
    assert_ne!(pad(&b), glossy, "the pads wear the chosen theme once composing again");
    assert_eq!(pad(&b), op8::controller::track_led_colors(ThemeId::Matte)[1] as u8);
}
