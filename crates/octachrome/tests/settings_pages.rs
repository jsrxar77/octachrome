//! SETTINGS in four pages: theme, audio, midi and light (docs/features.md §C).
use octachrome::bench::Bench;
use octachrome::controller::{DeviceList, Request};
use octachrome_gfx::{theme_id, ThemeId};
use octachrome_hw::{Button, Screen};
use octachrome_ui::{Main, NavKind};

static ONE: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// The theme is global: these tests run one at a time and put it back.
#[allow(dead_code)] // the guard is held for its lifetime, never read
struct Fresh(std::sync::MutexGuard<'static, ()>);
fn fresh() -> Fresh {
    let g = ONE.lock().unwrap_or_else(|e| e.into_inner());
    octachrome_gfx::set_theme(ThemeId::Chrome);
    Fresh(g)
}
impl Drop for Fresh {
    fn drop(&mut self) {
        octachrome_gfx::set_theme(ThemeId::Chrome);
    }
}

fn devices() -> DeviceList {
    DeviceList {
        outs: vec!["Maschine MK3".into(), "MacBook Pro Speakers".into(), "USB Audio".into()],
        ins: vec!["Maschine MK3".into(), "Scarlett 2i2".into()],
        ports: vec!["Maschine MK3 MIDI".into(), "IAC Driver Bus 1".into()],
        midi_ins: vec!["Maschine MK3 MIDI".into(), "Keystation 49".into()],
        midi_in_now: String::new(),
        out_now: "Maschine MK3".into(),
        in_now: "Maschine MK3".into(),
        port_now: "Maschine MK3 MIDI".into(),
        stream_now: "48 kHz · 2 ch · buffer 128".into(),
        supply: "the host gives the MK3 480 mA: limited: screens at 50, LEDs capped".into(),
        note: None,
    }
}

fn temp(name: &str) -> std::path::PathBuf {
    let d = std::env::temp_dir().join(format!("octachrome-pages-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    d
}

fn focus_on(b: &mut Bench, kind: NavKind) {
    for _ in 0..40 {
        if b.ui().focus == kind {
            return;
        }
        b.encoder(1);
    }
    panic!("the focus never reached {kind:?}");
}

fn saved(path: &std::path::Path) -> String {
    std::fs::read_to_string(path).unwrap_or_default()
}

// F-C10: SETTINGS has four pages, named by the buttons 1 to 4 over the left screen (the lit one is the open one, and the buttons 5 to 8 do
// nothing); the pages that show devices ask for them, the theme page keeps the 4D turning the theme, and the others use it to move.
#[test]
fn f_c09_the_four_pages_are_the_buttons_over_the_left_screen() {
    let _t = fresh();
    let mut b = Bench::demo();
    b.press(Button::Settings);
    b.run_ms(100);
    assert_eq!(b.ui().settings.page, 0, "it opens on the theme");
    let labels = octachrome_ui::screen_buttons(b.ui());
    assert_eq!(labels[..4].iter().map(|l| l.unwrap().0).collect::<Vec<_>>(), ["theme", "audio", "midi", "light"]);
    assert!(labels[0].unwrap().1 && !labels[1].unwrap().1, "the open page is the lit one");
    assert!(labels[4..].iter().all(Option::is_none), "the buttons of the right screen do nothing in settings");
    assert!(octachrome_ui::nav_items(b.ui()).is_empty(), "the theme page lists nothing: the 4D turns the theme");
    let theme = theme_id();
    b.encoder(1);
    assert_ne!(theme_id(), theme, "on the theme page the 4D turns the theme");
    b.capture("f-c09-theme");
    let hashes: Vec<_> = (2..=4)
        .map(|n| {
            b.press(Button::Screen(n));
            b.run_ms(100);
            assert_eq!(b.ui().settings.page, n - 1);
            (b.screen_hash(Screen::Left), b.screen_hash(Screen::Right))
        })
        .collect();
    assert!(hashes[0] != hashes[1] && hashes[1] != hashes[2], "each page looks different");
    let theme = theme_id();
    b.press(Button::Screen(2));
    b.encoder(1);
    assert_eq!(theme_id(), theme, "on the other pages the 4D moves the focus and does not touch the theme");
    assert!(b.app.ctl.requests.iter().filter(|r| **r == Request::ListDevices).count() >= 3, "audio and midi ask for the devices: {:?}", b.app.ctl.requests);
    b.press(Button::Screen(1));
    assert_eq!(b.ui().settings.page, 0);
    assert!(b.ui().main == Main::Settings);
}

// F-C11: the audio page lists the outputs and inputs (with `automatic` first), pushing one chooses it, asks for it, keeps it in the file
// and says which is chosen; the block size too; and a damaged list or no list is still a page that works.
#[test]
fn f_c10_the_audio_page_chooses_the_devices_and_the_block() {
    let _t = fresh();
    let path = temp("audio").join("settings");
    let mut b = Bench::demo();
    b.app.settings_path = Some(path.clone());
    b.press(Button::Settings);
    b.press(Button::Screen(2));
    b.app.ctl.requests.clear();
    b.app.ctl.devices_listed(devices());
    b.run_ms(100);
    b.capture("f-c10-audio");
    let rows = octachrome_ui::nav_items(b.ui()).iter().filter(|i| matches!(i.kind, NavKind::Pick(n) if n < 8)).count();
    assert_eq!(rows, 4, "automatic and the three outputs");
    assert_eq!(b.ui().settings.out_sel, 0, "nothing chosen: automatic");
    focus_on(&mut b, NavKind::Pick(2)); // MacBook Pro Speakers
    b.press(Button::JogPush);
    assert_eq!(b.app.ctl.settings.audio_out.as_deref(), Some("MacBook Pro Speakers"));
    assert_eq!(b.app.ctl.requests.last(), Some(&Request::AudioOut(Some("MacBook Pro Speakers".into()))));
    assert_eq!(b.ui().settings.out_sel, 2, "and it is the chosen row");
    b.run_ms(50);
    assert!(saved(&path).contains("audio_out=MacBook Pro Speakers"), "{}", saved(&path));
    focus_on(&mut b, NavKind::Pick(0));
    b.press(Button::JogPush);
    assert_eq!(b.app.ctl.settings.audio_out, None, "the first row is automatic");
    focus_on(&mut b, NavKind::Pick(10)); // the second input (8 is automatic)
    b.press(Button::JogPush);
    assert_eq!(b.app.ctl.requests.last(), Some(&Request::AudioIn(Some("Scarlett 2i2".into()))));
    focus_on(&mut b, NavKind::Pick(18)); // 256
    b.press(Button::JogPush);
    assert_eq!(b.app.ctl.requests.last(), Some(&Request::Buffer(256)));
    b.run_ms(50);
    assert_eq!(b.ui().settings.buffer, 256);
    assert!(saved(&path).contains("buffer=256") && saved(&path).contains("audio_in=Scarlett 2i2"), "{}", saved(&path));
    // what the hardware answers about what it did is shown
    let mut d = devices();
    d.note = Some("MacBook Pro Speakers: not found, using the system's".into());
    b.app.ctl.devices_listed(d);
    b.run_ms(50);
    assert!(b.ui().settings.note.contains("not found"));
    // a page with no devices still works
    b.app.ctl.devices_listed(DeviceList::default());
    b.run_ms(50);
    assert_eq!(octachrome_ui::nav_items(b.ui()).iter().filter(|i| matches!(i.kind, NavKind::Pick(n) if n < 8)).count(), 1, "only automatic");
    let _ = std::fs::remove_dir_all(path.parent().unwrap());
}

// F-C12: the MIDI page lists the ports, chooses one (asked for and kept), and has two buttons: a note, and silence.
#[test]
fn f_c11_the_midi_page_chooses_the_port_and_tests_it() {
    let _t = fresh();
    let path = temp("midi").join("settings");
    let mut b = Bench::demo();
    b.app.settings_path = Some(path.clone());
    b.press(Button::Settings);
    b.press(Button::Screen(3));
    b.app.ctl.devices_listed(devices());
    b.run_ms(100);
    b.capture("f-c11-midi");
    focus_on(&mut b, NavKind::Pick(2)); // IAC Driver Bus 1
    b.press(Button::JogPush);
    assert_eq!(b.app.ctl.requests.last(), Some(&Request::MidiPort(Some("IAC Driver Bus 1".into()))));
    b.run_ms(50);
    assert!(saved(&path).contains("midi_out=IAC Driver Bus 1"), "{}", saved(&path));
    let before = saved(&path);
    focus_on(&mut b, NavKind::Pick(12));
    b.press(Button::JogPush);
    assert_eq!(b.app.ctl.requests.last(), Some(&Request::MidiTest));
    focus_on(&mut b, NavKind::Pick(13));
    b.press(Button::JogPush);
    assert_eq!(b.app.ctl.requests.last(), Some(&Request::MidiPanic));
    b.run_ms(50);
    assert_eq!(saved(&path), before, "the buttons keep nothing");
    let _ = std::fs::remove_dir_all(path.parent().unwrap());
}

// F-C14: the light page has four steps of backlight and three levels of pad LED; pushing one keeps it in the file and the screens say which
// is chosen; the choice is a share of what the supply allows (the unit asks the surface for `effective_light`).
#[test]
fn f_c13_the_light_page_chooses_the_steps() {
    let _t = fresh();
    let path = temp("light").join("settings");
    let mut b = Bench::demo();
    b.app.settings_path = Some(path.clone());
    b.press(Button::Settings);
    b.press(Button::Screen(4));
    b.app.ctl.devices_listed(devices());
    b.run_ms(100);
    assert_eq!((b.ui().settings.screen, b.ui().settings.pads), (4, 3), "the whole of what the supply allows to begin with");
    let before = b.screen_hash(Screen::Left);
    b.capture("f-c13-light");
    focus_on(&mut b, NavKind::Pick(1)); // 50 %
    b.press(Button::JogPush);
    b.run_ms(50);
    assert_eq!(b.app.ctl.settings.screen, 2);
    assert_ne!(b.screen_hash(Screen::Left), before, "the page shows the step");
    focus_on(&mut b, NavKind::Pick(4)); // pads at level 1
    b.press(Button::JogPush);
    assert_eq!(b.app.ctl.settings.pads, 1);
    assert!(saved(&path).contains("screen=2") && saved(&path).contains("pads=1"), "{}", saved(&path));
    let l = octachrome::settings::effective_light(octachrome_gfx::theme().light, b.app.ctl.settings.screen, b.app.ctl.settings.pads);
    assert!(l.backlight <= octachrome_gfx::theme().light.backlight * 0.5 + 1e-6 && l.led_level == 1);
    assert!(b.app.ctl.requests.iter().all(|r| !matches!(r, Request::AudioOut(_) | Request::Buffer(_))), "the light asks nothing of the audio");
    let _ = std::fs::remove_dir_all(path.parent().unwrap());
}

// F-C16: the MIDI page has a button for the clock out, on by default: pushed it is off (kept in the file, the label says so, and a unit that
// was playing tells what follows it to stop once), pushed again it is on and the clocks go out again.
#[test]
fn f_c16_the_midi_page_turns_the_clock_on_and_off() {
    let _t = fresh();
    let path = temp("clock").join("settings");
    let mut b = Bench::empty();
    b.app.settings_path = Some(path.clone());
    b.press(Button::Settings);
    b.press(Button::Screen(3));
    b.app.ctl.devices_listed(devices());
    b.run_ms(100);
    assert!(b.ui().settings.midi_clock, "on by default");
    let on = b.screen_hash(Screen::Right);
    b.capture("f-c16-clock");
    b.press(Button::Settings); // out of SETTINGS, to play
    b.press(Button::Play);
    b.run_ms(600);
    assert!(b.sync_log.iter().any(|x| x.1 == 0xF8), "the clock goes out");
    b.press(Button::Settings);
    b.press(Button::Screen(3));
    focus_on(&mut b, NavKind::Pick(14));
    b.press(Button::JogPush);
    b.run_ms(100);
    assert!(!b.ui().settings.midi_clock);
    assert_ne!(b.screen_hash(Screen::Right), on, "the label says it is off");
    assert!(saved(&path).contains("midi_clock=off"), "{}", saved(&path));
    assert_eq!(b.sync_log.iter().filter(|x| x.1 == 0xFC).count(), 1, "what follows the clock is told to stop");
    let n = b.sync_log.len();
    b.run_ms(500);
    assert_eq!(b.sync_log.len(), n, "no clocks while it is off");
    b.press(Button::JogPush);
    b.run_ms(500);
    assert!(b.ui().settings.midi_clock && b.sync_log.len() > n, "on again, and the clocks go out");
    assert!(!saved(&path).contains("midi_clock"), "the default is not written");
    let _ = std::fs::remove_dir_all(path.parent().unwrap());
}
