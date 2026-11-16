//! PLUG-IN: choosing the instrument of a track (docs/features.md §I).
use op8::bench::Bench;
use op8_hw::Button;
use op8_ui::Main;

fn pad_of(slot: usize) -> u8 {
    (0..16u8).find(|&p| op8::Controller::pad_to_step(p) as usize == slot).unwrap()
}

/// The name of the instrument on a track.
fn name_of(b: &Bench, track: usize) -> Option<String> {
    b.instrument(track)
}

// F-I01: PLUG-IN shows the sixteen places on both screens (the factory instruments from the most guttural to the most refined,
// then eight for installed ones), the selected track's instrument is marked, and a pad puts that instrument on the track.
#[test]
fn f_i01_a_pad_puts_the_instrument_on_the_track() {
    let mut b = Bench::demo();
    b.press(Button::Group(3)); // the resonator's track
    b.press(Button::PlugIn);
    assert_eq!(b.ui().main, Main::Plugin);
    assert_eq!(b.ui().plugs.len(), 16);
    assert_eq!(b.ui().plug_current, Some(3), "the resonator is the fourth factory instrument");
    assert!(b.ui().plugs[8..].iter().all(|p| !p.1 && p.0.is_empty()), "the places for installed ones are empty");
    b.tap(pad_of(0)); // drums
    assert_eq!(b.ui().main, Main::Instrument, "choosing goes back to composing");
    assert_eq!(name_of(&b, 3).as_deref(), Some("drums"));
    assert_eq!(b.ui().right_page, 0);
    // and back to a melodic one
    b.press(Button::PlugIn);
    b.tap(pad_of(6)); // macro
    assert_eq!(name_of(&b, 3).as_deref(), Some("macro"));
    b.press(Button::PlugIn);
    b.press(Button::PlugIn);
    assert_eq!(b.ui().main, Main::Instrument, "PLUG-IN again leaves");
}

// F-I02: an empty place says so and changes nothing.
#[test]
fn f_i02_empty_places_change_nothing() {
    let mut b = Bench::demo();
    b.press(Button::Group(3));
    b.press(Button::PlugIn);
    b.tap(pad_of(10)); // an empty place
    assert!(b.ui().plug_note.starts_with("empty"));
    assert_eq!(b.ui().main, Main::Plugin);
    assert_eq!(name_of(&b, 3).as_deref(), Some("resonator"));
    assert!(b.ui().plugs[..8].iter().all(|p| p.1), "all eight factory instruments are built");
}

// F-I03: the MIDI out is one bridge that owns the MIDI queue: choosing it moves it from the track that has it.
#[test]
fn f_i03_the_midi_out_moves() {
    let mut b = Bench::demo();
    assert_eq!(name_of(&b, 7).as_deref(), Some("MidiOutBridge"), "the MIDI out is the last track");
    b.press(Button::Group(5)); // an empty track
    b.press(Button::PlugIn);
    b.tap(pad_of(7)); // midi out, the last of the eight
    assert_eq!(name_of(&b, 5).as_deref(), Some("MidiOutBridge"), "it is here now");
    assert_eq!(name_of(&b, 7), None, "and no longer where it was");
}

// F-I04: leaving the drums turns a kit track's lanes back into notes, so nothing is lost.
#[test]
fn f_i04_leaving_the_drums_keeps_the_pattern() {
    let mut b = Bench::demo();
    assert!(b.project().tracks[0].kit);
    b.press(Button::Group(0));
    b.press(Button::PlugIn);
    b.tap(pad_of(2)); // FmBass on the drum track
    let t = b.project().tracks[0];
    assert!(!t.kit);
    assert!(t.steps[4].active && t.steps[4].lanes == 0 && t.steps[4].note == 61, "the snare step is now a note: {:?}", t.steps[4]);
}

// F-I05: the sampler can be chosen like any instrument, plays one of its eight sounds from a step, and its two pages are
// `sample, start, length, tune` and `attack, release, loop, level`; the right screen draws its wave.
#[test]
fn f_i05_the_sampler_plays_from_a_step() {
    let mut b = Bench::empty();
    b.press(Button::Group(5));
    b.press(Button::PlugIn);
    b.tap(pad_of(4)); // the sampler
    assert_eq!(name_of(&b, 5).as_deref(), Some("sampler"));
    assert_eq!(b.ui().main, Main::Instrument);
    b.run_ms(100);
    b.capture("f-i05-sampler");
    // turn knob 5 to another sound: it is auditioned, and the step plays it
    b.knob(4, 100);
    b.tap(0);
    let before = b.status().note_count[5].load(std::sync::atomic::Ordering::Relaxed);
    b.press(Button::Play);
    b.run_ms(1500);
    assert!(b.status().note_count[5].load(std::sync::atomic::Ordering::Relaxed) > before);
    assert!(b.rms_db(0, 1500) > -50.0, "the sound is heard: {} dB", b.rms_db(0, 1500));
    assert!(b.ui().has_second, "it has a second page");
}

// F-I06: the granular can be chosen like any instrument, plays a cloud from a step, has the two pages `sample, position, size, density`
// and `tune, texture, spread, level`, draws its sound and the grains, has eight presets in BROWSER, and what SAMPLING records can be its
// sound: it is heard in the cloud as soon as it is saved.
#[test]
fn f_i06_the_granular_plays_a_cloud_from_a_step() {
    let mut b = Bench::empty();
    b.press(Button::Group(5));
    b.press(Button::PlugIn);
    b.tap(pad_of(5)); // the granular
    assert_eq!(name_of(&b, 5).as_deref(), Some("granular"));
    assert_eq!(b.ui().main, Main::Instrument);
    assert!(b.ui().has_second, "it has a second page");
    b.run_ms(100);
    b.capture("f-i06-granular");
    b.tap(12); // the first step
    let before = b.status().note_count[5].load(std::sync::atomic::Ordering::Relaxed);
    b.press(Button::Play);
    b.run_ms(1500);
    assert!(b.status().note_count[5].load(std::sync::atomic::Ordering::Relaxed) > before);
    assert!(b.rms_db(0, 1500) > -50.0, "the cloud is heard: {} dB", b.rms_db(0, 1500));
    // BROWSER: eight presets that come with it
    b.press(Button::Stop);
    b.press(Button::Browser);
    assert_eq!(b.ui().browse_inst, "granular");
    assert!(b.ui().browse[..8].iter().all(|c| c.1), "eight presets");
    b.tap(pad_of(3));
    assert!(b.ui().browse_note.starts_with("loaded"), "{}", b.ui().browse_note);
}

// F-I07: the standard tracks hold the eight instruments in the order of PLUG-IN, from the most guttural to the most refined, with the MIDI out
// last: A drums, B KickSyn, C FmBass, D resonator, E sampler, F granular, G macro, H MIDI out. The unit starts so, and so does every project
// that comes with it and a new empty one.
#[test]
fn f_i07_the_standard_tracks_hold_the_eight_instruments_in_order() {
    let order = ["drums", "KickSyn", "FmBass", "resonator", "sampler", "granular", "macro", "MidiOutBridge"];
    let check = |b: &Bench, what: &str| {
        for (t, name) in order.iter().enumerate() {
            assert_eq!(b.instrument(t).as_deref(), Some(*name), "{what}: track {t}");
        }
    };
    check(&Bench::demo(), "the unit as it starts");
    check(&Bench::empty(), "the empty rig");
    let mut b = Bench::demo();
    for slot in 0..8 {
        b.press(Button::File);
        b.tap(pad_of(slot));
        b.run_ms(50);
        check(&b, &format!("factory project {slot}"));
    }
    b.press(Button::File);
    b.tap(pad_of(10)); // an empty place of the player's: a new empty project
    b.run_ms(50);
    check(&b, "a new empty project");
}
