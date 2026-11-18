//! REC: writing what is played by hand into the track (docs/features.md §R).
use octachrome::bench::Bench;
use octachrome_hw::Button;

/// One step of the demo tempo (122 bpm, sixteenths), in milliseconds.
const STEP_MS: u64 = 123;

fn armed(group: u8, mode: Button) -> Bench {
    let mut b = Bench::empty();
    b.press(Button::Group(group));
    b.press(mode);
    b.press(Button::Rec);
    b.press(Button::Play);
    b
}

// F-R01: REC arms and disarms (strong while armed, a faint white otherwise), and a red mark shows on the left screen while it is.
#[test]
fn f_r01_rec_arms_and_disarms() {
    let mut b = Bench::empty();
    assert!(!b.ui().rec);
    b.press(Button::Rec);
    assert!(b.ui().rec);
    b.press(Button::Rec);
    assert!(!b.ui().rec);
    // in settings it does nothing
    b.press(Button::Settings);
    b.press(Button::Rec);
    assert!(!b.ui().rec);
}

// F-R02: a note played while the sequencer runs is written at the nearest step: early in a step it stays there, late in it
// goes to the next, and the step takes the note and the velocity.
#[test]
fn f_r02_a_note_is_written_at_the_nearest_step() {
    let mut b = armed(2, Button::Keyboard); // FmBass, on the keyboard
    b.run_ms(3 * STEP_MS + 20); // just into step 4 (index 3)
    b.strike(5); // pad 6: C3 + 5 = F3 (53)
    b.run_ms(20);
    b.release(5);
    let s = b.project().tracks[2].steps[3];
    assert!(s.active && s.note == 53, "step 4 now has {s:?}");
    assert_eq!(s.velocity, 102, "the velocity of the pad (0.8)");
    // late in a step: the next one
    b.run_ms(STEP_MS - 40 - 20 - 20); // up to the last part of the step
    b.strike(7);
    b.run_ms(10);
    b.release(7);
    assert!(b.project().tracks[2].steps[4].active, "a late hit belongs to the next step");
}

// F-R03: on a drum kit REC switches on the lane of the sound played, several sounds can share a step, and the track becomes a kit.
#[test]
fn f_r03_the_drums_record_into_lanes() {
    let mut b = armed(0, Button::PadMode);
    b.run_ms(2 * STEP_MS + 20);
    b.strike(0); // pad 1: the kick (lane 0)
    b.strike(1); // pad 2: the snare (lane 1), at the same moment: the same step
    b.release(0);
    b.release(1);
    let t = b.project().tracks[0];
    assert!(t.kit, "the drum track became a kit");
    assert_eq!(t.steps[2].lanes, 0b11, "kick and snare together");
}

// F-R04: nothing is written when REC is not armed, when the sequencer is stopped, or for CHORDS pads.
#[test]
fn f_r04_it_writes_only_when_armed_and_running() {
    let mut b = Bench::empty();
    b.press(Button::Group(2));
    b.press(Button::Keyboard);
    b.press(Button::Play);
    b.run_ms(STEP_MS + 20);
    b.strike(3);
    b.release(3);
    assert!(!b.project().tracks[2].steps.iter().any(|s| s.active), "REC is not armed");
    b.press(Button::Rec);
    b.press(Button::Play); // stop
    b.strike(3);
    b.release(3);
    assert!(!b.project().tracks[2].steps.iter().any(|s| s.active), "the sequencer is stopped");
    b.press(Button::Play);
    b.run_ms(STEP_MS + 20);
    b.press(Button::Chords);
    b.strike(0);
    b.run_ms(10);
    b.release(0);
    assert!(!b.project().tracks[2].steps.iter().any(|s| s.active), "chords are not recorded");
}
