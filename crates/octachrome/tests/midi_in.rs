//! MIDI in (docs/features.md, F-M04 and F-C17).
use octachrome::bench::Bench;
use octachrome::controller::{DeviceList, Request};
use octachrome_hw::{Button, Screen};
use octachrome_ui::NavKind;

fn note_on(ch: u8, note: u8, vel: u8) -> [u8; 3] {
    [0x90 | ch, note, vel]
}

fn note_off(ch: u8, note: u8) -> [u8; 3] {
    [0x80 | ch, note, 0]
}

// F-M04: a note that comes in by MIDI plays the selected track, is heard for as long as it is held, and stops when it is let go; a note
// on (velocity 0) is a note off; the same note down twice does not stick.
#[test]
fn f_m04_a_note_that_comes_in_plays_the_selected_track() {
    let mut b = Bench::empty();
    b.press(Button::Group(2)); // the FM bass
    b.run_ms(100);
    assert!(b.silent(0, b.now_ms()));
    let from = b.now_ms();
    b.midi_in(&note_on(0, 48, 100));
    b.run_ms(600);
    assert!(b.peak_db(from, b.now_ms()) > -40.0, "it is heard: {} dB", b.peak_db(from, b.now_ms()));
    b.midi_in(&note_on(0, 48, 0)); // a note on with no velocity is a note off
    b.run_ms(1500);
    let end = b.now_ms();
    assert!(b.peak_db(end - 300, end) < -50.0, "and it stops: {} dB", b.peak_db(end - 300, end));
    // another track, another instrument: the selected one plays
    b.press(Button::Group(1));
    let from = b.now_ms();
    b.midi_in(&note_on(5, 36, 110)); // the channel does not matter while it plays "selected"
    b.run_ms(400);
    b.midi_in(&note_off(5, 36));
    assert!(b.peak_db(from, b.now_ms()) > -40.0, "the kick on B is heard");
    // the same note twice, then one off: nothing is left sounding
    b.press(Button::Group(2));
    b.midi_in(&note_on(0, 50, 90));
    b.midi_in(&note_on(0, 50, 90));
    b.midi_in(&note_off(0, 50));
    b.run_ms(1800);
    let end = b.now_ms();
    assert!(b.peak_db(end - 300, end) < -50.0, "it did not stick: {} dB", b.peak_db(end - 300, end));
}

// F-M04: with "by channel", MIDI channels 1 to 8 play the tracks A to H whatever is selected, and 9 to 16 are left alone; and a track
// that has been selected since the note came in does not take its note off.
#[test]
fn f_m04_by_channel_the_channel_is_the_track() {
    let mut b = Bench::empty();
    b.app.ctl.settings.midi_by_channel = true;
    b.press(Button::Group(0)); // A is selected
    let from = b.now_ms();
    b.midi_in(&note_on(1, 36, 110)); // channel 2: the kick on B
    b.run_ms(400);
    assert!(b.peak_db(from, b.now_ms()) > -40.0, "channel 2 played track B");
    let after = b.now_ms();
    b.midi_in(&note_on(9, 36, 110)); // channel 10: no track
    b.run_ms(300);
    assert!(b.peak_db(after, b.now_ms()) < -90.0 || b.peak_db(after, b.now_ms()) < b.peak_db(from, after), "channel 10 plays nothing new");
    // a held note keeps its own track when the selection changes
    b.midi_in(&note_on(2, 40, 100));
    b.press(Button::Group(5));
    b.midi_in(&note_off(2, 40));
    b.run_ms(2000);
    let end = b.now_ms();
    assert!(b.peak_db(end - 300, end) < -50.0, "the note went off on the track it began on: {} dB", b.peak_db(end - 300, end));
}

// F-M04: REC writes what comes in by MIDI like what is played on the pads; controllers 7, 10 and 91 are the level, the pan and the
// reverb send of the track, 123 lets go of every held note.
#[test]
fn f_m04_rec_writes_it_and_the_controllers_mix() {
    let mut b = Bench::empty();
    b.press(Button::Group(2));
    b.press(Button::Rec);
    b.press(Button::Play);
    b.run_ms(300);
    b.midi_in(&note_on(0, 55, 100));
    b.run_ms(100);
    b.midi_in(&note_off(0, 55));
    let steps = &b.project().tracks[2].steps;
    assert!(steps.iter().any(|s| s.active && s.note == 55), "REC wrote the note");
    b.midi_in(&[0xB0, 7, 40]);
    b.midi_in(&[0xB0, 10, 100]);
    b.midi_in(&[0xB0, 91, 64]);
    let t = b.project().tracks[2];
    assert_eq!((t.level, t.pan, t.send_b), (40, 100, 64), "level, pan and reverb send of the selected track");
    assert_eq!(b.project().tracks[0].level, octachrome_seq::project::DEFAULT_LEVEL, "the others are as they were");
    b.press(Button::Stop);
    b.midi_in(&note_on(0, 50, 90));
    b.midi_in(&[0xB0, 123, 0]);
    b.run_ms(1800);
    let end = b.now_ms();
    assert!(b.peak_db(end - 300, end) < -50.0, "all notes off lets go of it");
    // messages that mean nothing here are left alone
    b.midi_in(&[]);
    b.midi_in(&[0xE0, 0, 64]);
    b.midi_in(&[0xC0, 5]);
}

// F-C17: the midi page lists the MIDI inputs (off first), choosing one is asked for and kept, and a button chooses what an input plays
// (the selected track, or the track of the channel); the four buttons under the list still do what they did.
#[test]
fn f_c17_the_midi_page_chooses_the_input() {
    let d = std::env::temp_dir().join(format!("octachrome-midiin-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    let path = d.join("settings");
    let mut b = Bench::demo();
    b.app.settings_path = Some(path.clone());
    b.press(Button::Settings);
    b.press(Button::Screen(3));
    b.app.ctl.devices_listed(DeviceList {
        ports: vec!["Maschine MK3 MIDI".into()],
        midi_ins: vec!["Maschine MK3 MIDI".into(), "Keystation 49".into()],
        ..DeviceList::default()
    });
    b.run_ms(100);
    b.capture("f-c17-midi-in");
    for _ in 0..40 {
        if b.ui().focus == NavKind::Pick(10) {
            break;
        }
        b.encoder(1);
    }
    assert_eq!(b.ui().focus, NavKind::Pick(10), "the second input (8 is off)");
    b.press(Button::JogPush);
    assert_eq!(b.app.ctl.requests.last(), Some(&Request::MidiIn(Some("Keystation 49".into()))));
    b.run_ms(50);
    let text = std::fs::read_to_string(&path).unwrap_or_default();
    assert!(text.contains("midi_in=Keystation 49"), "{text}");
    let before = b.screen_hash(Screen::Right);
    for _ in 0..40 {
        if b.ui().focus == NavKind::Pick(15) {
            break;
        }
        b.encoder(1);
    }
    b.press(Button::JogPush);
    b.run_ms(100);
    assert!(b.app.ctl.settings.midi_by_channel);
    assert_ne!(b.screen_hash(Screen::Right), before, "the label says by channel");
    assert!(std::fs::read_to_string(&path).unwrap().contains("midi_channels=on"));
    // off is the first row
    for _ in 0..40 {
        if b.ui().focus == NavKind::Pick(8) {
            break;
        }
        b.encoder(1);
    }
    b.press(Button::JogPush);
    assert_eq!(b.app.ctl.requests.last(), Some(&Request::MidiIn(None)));
    let _ = std::fs::remove_dir_all(&d);
}

// F-M05: a program change puts the instrument of that place of PLUG-IN on the track (the selected one, or the channel's with "by channel"),
// a place with nothing in it changes nothing, and the pitch wheel bends what plays (two semitones at the ends).
#[test]
fn f_m05_program_change_picks_the_instrument() {
    let mut b = Bench::empty();
    b.press(Button::Group(0));
    b.midi_in(&[0xC0, 2]);
    assert_eq!(b.instrument(0).as_deref(), Some("FmBass"), "place 3 of PLUG-IN");
    b.midi_in(&[0xC0, 4]);
    assert_eq!(b.instrument(0).as_deref(), Some("sampler"));
    b.midi_in(&[0xC0, 12]);
    assert_eq!(b.instrument(0).as_deref(), Some("sampler"), "an empty place changes nothing");
    b.app.ctl.settings.midi_by_channel = true;
    b.midi_in(&[0xC0 | 3, 2]); // channel 4: track D
    assert_eq!(b.instrument(3).as_deref(), Some("FmBass"));
    assert_eq!(b.instrument(0).as_deref(), Some("sampler"), "and the selected one is left");
}

// F-M05: the pitch wheel bends a note that is sounding up and down by up to two semitones, and back to centre.
#[test]
fn f_m05_the_pitch_wheel_bends_what_plays() {
    // the number of times the sound crosses zero in the second half of a run is its pitch
    let crossings = |wheel: Option<[u8; 3]>| {
        let mut b = Bench::empty();
        b.press(Button::Group(2)); // the FM bass
        b.midi_in(&note_on(0, 48, 110));
        if let Some(w) = wheel {
            b.midi_in(&w);
        }
        b.run_ms(1000);
        let end = b.now_ms();
        b.zero_crossings(end - 400, end)
    };
    let centre = crossings(None);
    let up = crossings(Some([0xE0, 127, 127])); // all the way up
    let down = crossings(Some([0xE0, 0, 0]));
    let mid = crossings(Some([0xE0, 0, 64])); // 8192: the centre
    assert!(centre > 20, "it sounds: {centre}");
    assert!(up as f32 > centre as f32 * 1.10, "up: {up} against {centre}");
    assert!((down as f32) < centre as f32 * 0.92, "down: {down} against {centre}");
    assert_eq!(mid, centre, "the centre is no bend");
}

fn focus_pick(b: &mut Bench, n: u8) {
    for _ in 0..60 {
        if b.ui().focus == NavKind::Pick(n) {
            return;
        }
        b.encoder(1);
    }
    panic!("the focus never reached pick {n}");
}

// F-M07: `learn` on the midi page waits for a knob of the right screen of an instrument and then for a control: from then on that
// controller turns that parameter of the instrument of the track that plays; it is kept in the settings; `forget` takes them all away;
// and the clock in has its button there.
#[test]
fn f_m07_a_control_can_be_learned_for_a_parameter() {
    let d = std::env::temp_dir().join(format!("octachrome-learn-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    let path = d.join("settings");
    let mut b = Bench::demo();
    b.app.settings_path = Some(path.clone());
    b.press(Button::Group(2)); // the FM bass is selected
    b.press(Button::Settings);
    b.press(Button::Screen(3));
    b.run_ms(100);
    let idle = b.screen_hash(Screen::Right);
    b.capture("f-m07-midi-page");
    focus_pick(&mut b, 17);
    b.press(Button::JogPush);
    assert_eq!(b.ui().settings.learn, 1, "waiting for a knob");
    b.run_ms(100);
    assert_ne!(b.screen_hash(Screen::Right), idle, "the button says so");
    b.knob(2, 50); // a knob under the left screen is not a parameter
    assert_eq!(b.ui().settings.learn, 1);
    b.knob(5, 50); // the second of the right screen: parameter 2 (index 1) of the first page
    assert_eq!(b.ui().settings.learn, 2, "waiting for a control");
    b.midi_in(&[0xB0, 21, 100]);
    assert_eq!(b.ui().settings.learn, 0);
    assert_eq!(b.app.ctl.settings.midi_map, vec![(21, 1)]);
    b.midi_in(&[0xB0, 21, 0]);
    b.run_ms(50);
    assert!(b.param(2, 1) < 0.02, "the control is the parameter: {}", b.param(2, 1));
    b.midi_in(&[0xB0, 21, 127]);
    b.run_ms(50);
    assert!(b.param(2, 1) > 0.98, "{}", b.param(2, 1));
    assert!(std::fs::read_to_string(&path).unwrap().contains("midi_map=21:1"));
    // the controllers that mix are still there until one is learned over them
    b.midi_in(&[0xB0, 7, 64]);
    assert_eq!(b.project().tracks[2].level, 64);
    // forget
    focus_pick(&mut b, 18);
    b.press(Button::JogPush);
    assert!(b.app.ctl.settings.midi_map.is_empty());
    b.midi_in(&[0xB0, 21, 0]);
    b.run_ms(50);
    assert!(b.param(2, 1) > 0.98, "nothing turns it any more");
    // the clock in
    focus_pick(&mut b, 16);
    b.press(Button::JogPush);
    assert!(b.app.ctl.settings.midi_clock_in);
    assert!(std::fs::read_to_string(&path).unwrap().contains("midi_clock_in=on"));
    let _ = std::fs::remove_dir_all(&d);
}
