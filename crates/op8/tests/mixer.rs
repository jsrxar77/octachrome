//! The mix: a level and a pan for each track, turned with VOLUME held (docs/features.md §K and §D).
use op8::bench::Bench;
use op8_hw::{Button, Screen};

// F-K18: while VOLUME is held the eight knobs are the levels of the eight tracks, and with SHIFT their pans; the other knobs and the
// master volume are untouched, a knob does its usual work again once VOLUME is let go, and the mix is saved with the project.
#[test]
fn f_k18_volume_held_and_a_knob_is_a_track_s_level() {
    let mut b = Bench::demo();
    b.run_ms(100);
    let (level, pan) = (b.project().tracks[2].level, b.project().tracks[2].pan);
    let master = b.ui().master;
    b.button_down(Button::Volume);
    b.knob(2, 40 * 3); // knob 3: track C, three detents up
    assert_eq!(b.project().tracks[2].level, level + 12, "knob 3 is the level of track C");
    assert_eq!(b.project().tracks[1].level, level, "and only that track's");
    b.knob(6, -40 * 2); // knob 7: track G, two down
    assert_eq!(b.project().tracks[6].level, level - 8, "the knobs of the right screen are the tracks E to H");
    b.button_down(Button::Shift);
    b.knob(2, 40 * 2);
    b.button_up(Button::Shift);
    assert_eq!(b.project().tracks[2].pan, pan + 8, "with SHIFT it is the pan");
    assert_eq!(b.project().tracks[2].level, level + 12, "and the level stays");
    b.knob(2, -40 * 1000);
    assert_eq!(b.project().tracks[2].level, 0, "the level stops at 0");
    b.knob(2, 40 * 1000);
    assert_eq!(b.project().tracks[2].level, 127, "and at 127");
    b.button_up(Button::Volume);
    assert_eq!(b.ui().master, master, "the master volume is not touched");
    // let go: knob 1 is the length again
    let len = b.project().tracks[b.ui().track].length;
    b.knob(0, -40 * 2);
    assert_eq!(b.project().tracks[b.ui().track].length, len - 2);
    // the mix is part of the project: what is saved and opened is the same
    let bytes = op8_seq::io::encode(b.project());
    assert_eq!(op8_seq::io::decode(&bytes).unwrap().tracks[2].pan, pan + 8);
}

// F-D20: while VOLUME is held both screens show the mix (eight faders and pans on the left, the four tracks of each knob under each screen);
// SHIFT changes it to the pans; it is gone when VOLUME is let go.
#[test]
fn f_d20_volume_held_shows_the_mix() {
    let mut b = Bench::demo();
    b.run_ms(100);
    let (left, right) = (b.screen_hash(Screen::Left), b.screen_hash(Screen::Right));
    b.button_down(Button::Volume);
    b.run_ms(100);
    b.capture("f-d20-mix-levels");
    let mix = (b.screen_hash(Screen::Left), b.screen_hash(Screen::Right));
    assert_ne!(mix.0, left, "the left screen shows the mix");
    assert_ne!(mix.1, right, "and so does the right one, under its knobs");
    b.knob(1, 40 * 4);
    b.run_ms(50);
    assert_ne!(b.screen_hash(Screen::Left), mix.0, "a fader moves");
    let faders = b.screen_hash(Screen::Left);
    b.button_down(Button::Shift);
    b.run_ms(50);
    b.capture("f-d20-mix-pans");
    assert_ne!(b.screen_hash(Screen::Left), faders, "SHIFT shows the pans");
    b.button_up(Button::Shift);
    b.button_up(Button::Volume);
    b.run_ms(200);
    assert_ne!(b.screen_hash(Screen::Left), mix.0, "and it is gone when VOLUME is let go");
}
