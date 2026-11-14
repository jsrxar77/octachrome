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
    b.tap(pad_of(7)); // macro
    assert_eq!(name_of(&b, 3).as_deref(), Some("macro"));
    b.press(Button::PlugIn);
    b.press(Button::PlugIn);
    assert_eq!(b.ui().main, Main::Instrument, "PLUG-IN again leaves");
}

// F-I02: an instrument that is not built yet, and an empty place, say so and change nothing.
#[test]
fn f_i02_unbuilt_and_empty_places_change_nothing() {
    let mut b = Bench::demo();
    b.press(Button::Group(3));
    b.press(Button::PlugIn);
    b.tap(pad_of(4)); // the sampler: not yet
    assert_eq!(b.ui().main, Main::Plugin);
    assert!(b.ui().plug_note.contains("not built"));
    b.tap(pad_of(10)); // an empty place
    assert!(b.ui().plug_note.starts_with("empty"));
    assert_eq!(name_of(&b, 3).as_deref(), Some("resonator"));
}

// F-I03: the MIDI out is one bridge that owns the MIDI queue: choosing it moves it from the track that has it.
#[test]
fn f_i03_the_midi_out_moves() {
    let mut b = Bench::demo();
    assert_eq!(name_of(&b, 4).as_deref(), Some("MidiOutBridge"));
    b.press(Button::Group(6)); // an empty track
    b.press(Button::PlugIn);
    b.tap(pad_of(6)); // midi out
    assert_eq!(name_of(&b, 6).as_deref(), Some("MidiOutBridge"), "it is here now");
    assert_eq!(name_of(&b, 4), None, "and no longer where it was");
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
