//! The recipes of the manual (`guide/manual.html`), played on the simulated MK3: each one presses what the manual says,
//! in the order it says it, and checks what the manual promises. Run with `OP8_BENCH_DIR=<dir>` to save the pictures the
//! manual shows (`scripts/manual-pictures.sh` does it and shrinks them).
use op8::bench::Bench;
use op8_hw::Button;
use op8_seq::{ChordType, FxKind, PlayMode, StepComponent};
use std::sync::atomic::Ordering::Relaxed;

/// One notch of a knob (the unit sends 1000 per turn, and a notch is 40 of them).
const NOTCH: i32 = 40;

/// The theme is global, and a recipe changes it: the recipes run one at a time so that no picture comes out in the wrong theme. The
/// manual is drawn in matte (the unit itself starts in chrome since 2026-11-16), so each recipe starts with matte.
static SERIAL: std::sync::Mutex<()> = std::sync::Mutex::new(());
fn serial() -> std::sync::MutexGuard<'static, ()> {
    let guard = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    op8_gfx::set_theme(op8_gfx::ThemeId::Matte);
    guard
}

/// The pad of step `n` (1..=16): step 1 is the top-left pad, and they read like a page.
fn pad(n: u8) -> u8 {
    (0..16u8).find(|&p| op8::Controller::pad_to_step(p) == n - 1).unwrap()
}

fn with_shift(b: &mut Bench, button: Button) {
    b.button_down(Button::Shift);
    b.press(button);
    b.button_up(Button::Shift);
}

// F-G04
#[test]
fn recipe_01_hear_the_demo() {
    let _one_at_a_time = serial();
    let mut b = Bench::demo();
    b.run_ms(100);
    b.capture("r01-start");
    b.press(Button::Play);
    b.run_ms(2500);
    assert!(b.rms_db(1500, 2500) > -40.0, "the demo sounds");
    b.capture("r01-playing");
    // hold MUTE and tap group B: the kick goes quiet
    b.button_down(Button::Mute);
    b.press(Button::Group(1));
    b.button_up(Button::Mute);
    assert!(b.project().tracks[1].muted);
    // choose another track: nothing mutes, the right screen shows that track's instrument
    b.press(Button::Group(3));
    assert_eq!(b.ui().track, 3);
    assert_eq!(b.instrument(3).as_deref(), Some("resonator"));
    b.press(Button::Stop);
    assert!(!b.status().playing.load(Relaxed));
}

// F-G04
#[test]
fn recipe_02_a_drum_beat() {
    let _one_at_a_time = serial();
    let mut b = Bench::demo();
    b.press(Button::File);
    b.tap(pad(10)); // an empty place of yours: a new empty project
    assert!(b.project().tracks.iter().all(|t| t.steps.iter().all(|s| !s.active)), "nothing in the steps");
    b.press(Button::Group(0)); // A is the drums
    for s in [1, 5, 9, 13] {
        b.tap(pad(s)); // the kick, on every beat
    }
    assert!(b.project().tracks[0].kit);
    b.capture("r02-kick");
    b.press(Button::Screen(6)); // shape: its last knob chooses the sound
    b.knob(7, 100); // about a tenth of a turn for each sound
    b.run_ms(50);
    assert_eq!(b.ui().lane, Some(1), "a tenth of a turn of knob 8 is the snare");
    for s in [5, 13] {
        b.tap(pad(s));
    }
    b.knob(7, 100); // the closed hat
    b.run_ms(50);
    assert_eq!(b.ui().lane, Some(2));
    for s in [3, 7, 11, 15] {
        b.tap(pad(s));
    }
    b.run_ms(100);
    b.capture("r02-beat");
    let t = &b.project().tracks[0];
    assert_eq!(t.steps[0].lanes, 0b001, "step 1: kick");
    assert_eq!(t.steps[4].lanes, 0b011, "step 5: kick and snare together");
    assert_eq!(t.steps[2].lanes, 0b100, "step 3: hat");
    b.press(Button::Play);
    b.run_ms(2200); // a bar and a bit
    assert!(b.rms_db(300, 2200) > -40.0);
    // a tap on a step that is on, with that sound chosen, switches only that sound off
    b.press(Button::Stop);
    b.tap(pad(3));
    assert_eq!(b.project().tracks[0].steps[2].lanes, 0);
}

// F-G04
#[test]
fn recipe_03_a_bass_that_follows_the_chords() {
    let _one_at_a_time = serial();
    let mut b = Bench::empty();
    b.press(Button::Group(2)); // C is the FmBass
    for s in [1, 4, 7, 11] {
        b.tap(pad(s));
    }
    with_shift(&mut b, Button::Group(2)); // SHIFT + C connects it to the Brain
    assert!(b.project().tracks[2].brain_link);
    // hold step 4 to open it and turn knob 1 to set its note
    b.strike(pad(4));
    b.run_ms(400); // keep the finger on the pad: the step opens while it is held
    assert_eq!(b.ui().inspect_step, Some(3));
    let before = b.project().tracks[2].steps[3].note;
    b.knob(0, 5 * NOTCH);
    assert_eq!(b.project().tracks[2].steps[3].note, before + 5, "a notch is a semitone");
    b.capture("r03-inspector");
    b.release(pad(4)); // let go and it closes
    assert_eq!(b.ui().inspect_step, None);
    // CHORDS: knob 1 is the key, 2 the scale, 3 how the track follows, 4 how many chords
    b.press(Button::Chords);
    b.knob(0, 2 * NOTCH);
    assert_eq!(b.project().brain.key, 2, "D");
    b.run_ms(100);
    b.capture("r03-chords");
    b.press(Button::Chords); // CHORDS again goes back
    b.press(Button::Play);
    b.run_ms(4200);
    assert!(b.rms_db(500, 4200) > -45.0, "the bass sounds");
    // the progression moves: the Brain chord changes while it plays
    let first = b.status().chord_root.load(Relaxed);
    b.run_ms(5000);
    let later = (0..8).map(|_| { b.run_ms(1000); b.status().chord_root.load(Relaxed) }).collect::<Vec<_>>();
    assert!(later.iter().any(|&r| r != first), "the chord changes over the bars");
}

// F-G04
#[test]
fn recipe_04_play_a_melody_and_record_it() {
    let _one_at_a_time = serial();
    let mut b = Bench::empty();
    b.press(Button::Group(6)); // G is the macro
    b.press(Button::Keyboard);
    b.press(Button::Rec);
    b.press(Button::Play);
    b.run_ms(300);
    b.strike(0); // C3
    b.run_ms(50);
    b.release(0);
    b.run_ms(400);
    b.strike(4);
    b.run_ms(50);
    b.release(4);
    b.run_ms(300);
    b.capture("r04-keyboard");
    let notes: Vec<u8> = b.project().tracks[6].steps.iter().filter(|s| s.active).map(|s| s.note).collect();
    assert_eq!(notes, [48, 52], "what was played is on the steps");
    // the octave: ◀ ▶ of the 4D encoder
    b.press(Button::JogRight);
    assert_eq!(b.ui().octave, 1);
    b.press(Button::Rec); // disarm
    assert!(!b.ui().rec);
}

// F-G04
#[test]
fn recipe_05_make_a_step_special() {
    let _one_at_a_time = serial();
    let mut b = Bench::empty();
    b.press(Button::Group(1));
    b.tap(pad(1));
    b.strike(pad(1)); // hold the pad: the step's components, while the finger is on it
    b.run_ms(400);
    assert_eq!(b.ui().inspect_step, Some(0));
    b.knob(3, NOTCH); // knob 4 goes through the list: the first is a ratchet of 2
    assert_eq!(b.project().tracks[1].steps[0].components.as_slice(), &[StepComponent::Ratchet(2)]);
    b.knob(3, NOTCH);
    assert_eq!(b.project().tracks[1].steps[0].components.as_slice(), &[StepComponent::Ratchet(3)], "the next one replaces it");
    b.run_ms(100);
    b.capture("r05-components");
    b.release(pad(1));
    let before = b.status().note_count[1].load(Relaxed);
    b.press(Button::Play);
    b.run_ms(200);
    assert!(b.status().note_count[1].load(Relaxed) - before >= 3, "three hits in one step");
}

// F-G04
#[test]
fn recipe_06_a_note_becomes_a_chord_or_an_arpeggio() {
    let _one_at_a_time = serial();
    let mut b = Bench::empty();
    b.press(Button::Group(3)); // D, the resonator
    b.tap(pad(1));
    b.press(Button::Screen(3)); // players
    b.knob(0, NOTCH); // play: chord
    b.knob(1, NOTCH); // chord: minor
    assert_eq!(b.project().tracks[3].players.mode, PlayMode::Chord);
    assert_eq!(b.project().tracks[3].players.chord, ChordType::Minor);
    b.run_ms(100);
    b.capture("r06-players");
    let before = b.status().note_count[3].load(Relaxed);
    b.press(Button::Play);
    b.run_ms(100);
    assert_eq!(b.status().note_count[3].load(Relaxed) - before, 3, "three notes from one step");
    b.press(Button::Stop);
    b.knob(0, NOTCH); // play: up (an arpeggio)
    b.knob(2, 2 * NOTCH); // rate: four notes a step
    assert_eq!(b.project().tracks[3].players.mode, PlayMode::Up);
    b.press(Button::Screen(3)); // the same button goes back to the steps
    assert_eq!(b.ui().left_page, op8_ui::LeftPage::Steps);
}

// F-G04
#[test]
fn recipe_07_an_echo_or_a_room_for_a_track() {
    let _one_at_a_time = serial();
    let mut b = Bench::empty();
    b.press(Button::Group(3));
    b.tap(pad(1));
    b.press(Button::Screen(7)); // fx
    b.knob(4, 2 * NOTCH); // knob 5: reverb
    assert_eq!(b.project().tracks[3].fx.kind, FxKind::Reverb);
    b.knob(7, 100); // knob 8: mix
    b.run_ms(100);
    b.capture("r07-reverb");
    b.knob(4, -NOTCH); // delay
    assert_eq!(b.project().tracks[3].fx.kind, FxKind::Delay);
    b.run_ms(100);
    b.capture("r07-delay");
    b.press(Button::Play);
    b.run_ms(1800);
    assert!(b.rms_db(700, 1800) > -60.0, "the echo is heard after the note");
    b.knob(4, -NOTCH);
    assert_eq!(b.project().tracks[3].fx.kind, FxKind::Off, "turn knob 5 back to off");
}

// F-G04
#[test]
fn recipe_08_make_a_knob_move_by_itself() {
    let _one_at_a_time = serial();
    let mut b = Bench::empty();
    b.press(Button::Group(2)); // the FmBass
    b.press(Button::Screen(8)); // mod
    b.knob(7, 3 * NOTCH); // knob 8: what it moves, three notches: the instrument's second knob
    b.run_ms(50);
    assert_eq!(b.ui().lfo_target, "index");
    b.knob(4, NOTCH); // shape: triangle
    b.knob(5, -200); // a little slower
    b.run_ms(100);
    b.capture("r08-mod");
    assert!(b.project().tracks[2].lfo.is_on());
    b.knob(7, -3 * NOTCH);
    assert!(!b.project().tracks[2].lfo.is_on(), "back to nothing: it stops");
}

// F-G04
#[test]
fn recipe_09_two_lengths_drift_apart() {
    let _one_at_a_time = serial();
    let mut b = Bench::empty();
    b.press(Button::Group(1));
    b.tap(pad(1));
    b.press(Button::Group(2));
    b.tap(pad(1));
    b.press(Button::Screen(4)); // bar
    b.knob(0, -9 * NOTCH); // length 16 → 7
    assert_eq!(b.project().tracks[2].length, 7);
    b.run_ms(100);
    b.capture("r09-bar");
    b.press(Button::Screen(4));
    b.press(Button::Play);
    let mut heads = std::collections::BTreeSet::new();
    for _ in 0..200 {
        b.run_ms(30);
        heads.insert((b.status().playhead[1].load(Relaxed), b.status().playhead[2].load(Relaxed)));
    }
    assert!(heads.len() > 16, "the two playheads do not stay in step: {} meetings", heads.len());
}

// F-G04
#[test]
fn recipe_10_perform() {
    let _one_at_a_time = serial();
    let mut b = Bench::demo();
    b.press(Button::Play);
    b.run_ms(800);
    b.press(Button::Perform);
    b.strike_at(0, 0.9); // bottom-left: the low-pass
    b.run_ms(300);
    assert!(b.ui().punch.is_some());
    b.capture("r10-perform");
    b.release(0);
    assert!(b.ui().punch.is_none(), "let go and it is over");
    b.strip(Some(0.2)); // the Smart Strip: a filter on everything
    b.run_ms(200);
    b.strip(None);
    // with the 4D encoder: push keeps an effect going, turning is its intensity, push again stops it
    b.press(Button::JogPush);
    assert!(b.ui().punch.is_some());
    b.encoder(3);
    b.press(Button::JogPush);
    assert!(b.ui().punch.is_none());
    b.press(Button::JogPush);
    b.press(Button::Perform); // back to the steps: it stops
    assert_eq!(b.ui().mode, op8_ui::Mode::Step);
    assert!(b.ui().punch.is_none());
}

// F-G04
#[test]
fn recipe_11_choose_an_instrument() {
    let _one_at_a_time = serial();
    let mut b = Bench::empty();
    b.press(Button::Group(5)); // F: the granular, which the resonator replaces
    b.press(Button::PlugIn);
    b.run_ms(100);
    b.capture("r11-plugin");
    b.tap(pad(4)); // the fourth: the resonator
    assert_eq!(b.instrument(5).as_deref(), Some("resonator"));
    assert_eq!(b.ui().main, op8_ui::Main::Instrument, "it goes back to composing");
    b.press(Button::Screen(6)); // the second page of that instrument
    b.run_ms(100);
    b.capture("r11-shape");
}

// F-G04
#[test]
fn recipe_12_save_and_come_back() {
    let _one_at_a_time = serial();
    let dir = std::env::temp_dir().join(format!("op8-manual-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let mut b = Bench::demo();
    b.set_projects_dir(dir.clone());
    b.press(Button::File);
    b.run_ms(100);
    b.capture("r12-file");
    b.button_down(Button::Shift);
    b.tap(pad(9)); // the first of your own
    b.button_up(Button::Shift);
    assert!(b.ui().file_note.starts_with("saved"));
    b.tap(pad(7)); // open the ambient piece, which has no kick
    assert!(!b.project().tracks[1].steps[0].active);
    b.press(Button::File);
    b.tap(pad(9));
    assert!(b.project().tracks[1].steps[0].active, "your project is back");
    assert!(dir.join("user-1.name").is_file(), "it carries a name, the day and the tempo");
    // to get rid of it: ERASE and the place, twice
    b.press(Button::File);
    for _ in 0..2 {
        b.button_down(Button::Erase);
        b.tap(pad(9));
        b.button_up(Button::Erase);
    }
    assert!(!dir.join("user-1.op8p").exists() && !dir.join("user-1.name").exists(), "erased");
    let _ = std::fs::remove_dir_all(&dir);
}

// F-G04
#[test]
fn recipe_13_tempo_swing_volume_and_the_metronome() {
    let _one_at_a_time = serial();
    let mut b = Bench::empty();
    let mut at = Vec::new();
    for _ in 0..4 {
        at.push(b.now_ms());
        b.press(Button::Tap);
        b.run_ms(450);
    }
    let pace = (at[3] - at[0]) as f32 / 3.0;
    assert!((b.project().bpm - 60_000.0 / pace).abs() < 2.0, "four taps {pace} ms apart: {}", b.project().bpm);
    b.button_down(Button::Tempo);
    b.encoder(10);
    b.run_ms(100);
    b.capture("r13-tempo");
    b.button_up(Button::Tempo);
    assert!(b.project().bpm > 60_000.0 / pace + 5.0, "the encoder raises it");
    b.button_down(Button::Swing);
    b.encoder(10);
    b.button_up(Button::Swing);
    assert!(b.project().swing > 0.5);
    b.button_down(Button::Volume);
    b.encoder(-20);
    b.button_up(Button::Volume);
    assert!(b.ui().master < 1.0);
    with_shift(&mut b, Button::Tap);
    assert!(b.ui().metronome);
}

// F-G04
#[test]
fn recipe_14_the_look() {
    let _one_at_a_time = serial();
    let mut b = Bench::demo();
    b.press(Button::Settings);
    b.run_ms(100);
    b.capture("r14-settings");
    let before = op8_gfx::theme_id();
    b.encoder(1);
    assert_ne!(op8_gfx::theme_id(), before, "the 4D encoder changes the theme at once");
    b.run_ms(100);
    b.capture("r14-glossy");
    b.encoder(-1);
    assert_eq!(op8_gfx::theme_id(), before);
    b.press(Button::ChannelMidi);
    assert_eq!(b.ui().main, op8_ui::Main::Instrument);
}

// F-G04
#[test]
fn recipe_15_play_a_recorded_sound() {
    let _one_at_a_time = serial();
    let mut b = Bench::empty();
    b.press(Button::Group(4)); // E is the sampler
    assert_eq!(b.instrument(4).as_deref(), Some("sampler"));
    b.knob(4, 50); // knob 5, sample: one click is the next sound, the bell
    b.run_ms(100);
    b.capture("r15-bell");
    b.knob(5, 300); // knob 6, start: skip the first part of the sound
    b.knob(6, -300); // knob 7, length: and stop before the end
    b.press(Button::Screen(6)); // shape
    b.knob(6, 800); // knob 7, loop: on
    b.run_ms(100);
    b.capture("r15-shape");
    b.press(Button::Screen(5));
    b.press(Button::Keyboard);
    let t0 = b.now_ms();
    b.strike(0); // C3: an octave below the sound's own pitch
    b.run_ms(300);
    assert!(b.rms_db(t0, t0 + 300) > -50.0, "the sound is heard: {} dB", b.rms_db(t0, t0 + 300));
    b.release(0);
    b.press(Button::Keyboard);
}

// F-G04
#[test]
fn recipe_16_record_your_own_sound() {
    let _one_at_a_time = serial();
    let d = std::env::temp_dir().join(format!("op8-manual-rec-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    let mut b = Bench::demo();
    b.set_samples_dir(d.clone());
    b.press(Button::Play);
    b.run_ms(400);
    b.press(Button::Sampling);
    b.run_ms(300);
    b.capture("r16-waiting");
    // SHIFT + REC waits for the sound (here the music is stopped, so there is none); the music starting is what starts it
    b.press(Button::Stop);
    b.run_ms(3000);
    b.button_down(Button::Shift);
    b.press(Button::Rec);
    b.button_up(Button::Shift);
    b.run_ms(100);
    assert!(b.ui().smp.armed && !b.ui().smp.recording);
    b.capture("r16-armed");
    b.press(Button::Play);
    b.run_ms(600);
    assert!(b.ui().smp.recording && !b.ui().smp.armed, "the sound started it");
    b.press(Button::Rec);
    b.run_ms(100);
    // the count-in: TAP turns it on and REC counts
    b.press(Button::Tap);
    b.press(Button::Rec);
    b.run_ms(150);
    assert_eq!(b.ui().smp.counting, 3);
    b.capture("r16-count");
    b.press(Button::Rec); // cancelled
    b.press(Button::Tap); // and off
    b.run_ms(100);
    b.press(Button::Rec);
    b.run_ms(2000);
    b.press(Button::Rec);
    b.run_ms(200);
    b.knob(5, 120); // knob 6, start
    b.knob(6, -120); // knob 7, end
    b.knob(7, 100); // knob 8, fade
    b.run_ms(100);
    b.capture("r16-take");
    b.press(Button::Stop);
    b.run_ms(300);
    let t0 = b.now_ms();
    b.tap(pad(10)); // a listen pad (the tenth)
    b.run_ms(500);
    assert!(b.rms_db(t0, t0 + 500) > -50.0, "the part is heard: {} dB", b.rms_db(t0, t0 + 500));
    b.tap(pad(1)); // user 1
    b.run_ms(100);
    b.capture("r16-saved");
    assert!(d.join("rec-1.wav").is_file());
    b.press(Button::Sampling);
    let _ = std::fs::remove_dir_all(&d);
}

// F-G04
#[test]
fn recipe_17_presets_load_one_keep_your_own() {
    let _one_at_a_time = serial();
    let d = std::env::temp_dir().join(format!("op8-manual-presets-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    let mut b = Bench::demo();
    b.set_presets_dir(d.clone());
    b.press(Button::Group(1));
    b.press(Button::Browser);
    b.run_ms(100);
    b.capture("r17-factory");
    let from = b.now_ms();
    b.tap(pad(3)); // boom, the third of the kick's own
    b.run_ms(400);
    assert!(b.ui().browse_note.contains("loaded boom"), "{}", b.ui().browse_note);
    assert!(b.peak_db(from, b.now_ms()) > -40.0, "the preset is heard");
    let kept = b.param(1, 1);
    b.press(Button::Browser);
    b.knob(4, 300);
    b.press(Button::Browser);
    b.button_down(Button::Shift);
    b.tap(pad(9)); // user 1
    b.button_up(Button::Shift);
    b.run_ms(100);
    b.capture("r17-saved");
    assert!(d.join("KickSyn").join("user-1.preset").is_file());
    b.tap(pad(1)); // deep: another sound
    assert!((b.param(1, 1) - kept).abs() > 0.1);
    b.tap(pad(9));
    assert!(b.ui().browse_note.contains("loaded user 1"), "{}", b.ui().browse_note);
    b.press(Button::Browser);
    let _ = std::fs::remove_dir_all(&d);
}

// F-G04
#[test]
fn recipe_18_move_without_the_pads() {
    use op8_ui::NavKind;
    let _one_at_a_time = serial();
    let mut b = Bench::demo();
    b.press(Button::File);
    b.run_ms(100);
    b.encoder(1);
    b.run_ms(60);
    b.capture("r18-file");
    b.encoder(1);
    b.press(Button::JogPush); // the dub
    assert_eq!(b.project().bpm, 70.0);
    // composing: a value of the right screen, grabbed
    for _ in 0..80 {
        if b.ui().focus == NavKind::Value(5) {
            break;
        }
        b.encoder(1);
    }
    assert_eq!(b.ui().focus, NavKind::Value(5));
    b.press(Button::JogPush);
    b.encoder(1);
    b.run_ms(60);
    b.capture("r18-values");
    assert!(b.ui().grabbed);
    b.press(Button::JogPush);
    assert!(!b.ui().grabbed);
}

// F-G04
#[test]
fn recipe_19_the_mix() {
    let _one_at_a_time = serial();
    let mut b = Bench::demo();
    b.run_ms(100);
    b.press(Button::Play);
    b.run_ms(600);
    let (level, pan) = (b.project().tracks[2].level, b.project().tracks[2].pan);
    b.button_down(Button::Volume);
    b.knob(2, 40 * 3); // knob 3: track C a little louder
    b.knob(0, -40 * 4); // knob 1: the drums quieter
    b.run_ms(60);
    b.capture("r19-mix");
    assert_eq!(b.project().tracks[2].level, level + 12);
    b.button_down(Button::Shift);
    b.knob(2, -40 * 4); // SHIFT: the pan of track C to the left
    b.run_ms(60);
    b.capture("r19-pan");
    assert_eq!(b.project().tracks[2].pan, pan - 16);
    b.button_up(Button::Shift);
    b.button_up(Button::Volume);
    b.run_ms(100);
    // the mix is kept with the project: saved and opened again, it is the same
    let bytes = op8_seq::io::encode(b.project());
    assert_eq!(op8_seq::io::decode(&bytes).unwrap().tracks[2].pan, pan - 16);
}

// F-G04
#[test]
fn recipe_20_granular_a_cloud_from_a_sound() {
    let _one_at_a_time = serial();
    let mut b = Bench::demo();
    b.press(Button::Group(5)); // F: the granular is there already
    assert_eq!(b.instrument(5).as_deref(), Some("granular"));
    b.press(Button::PlugIn);
    b.tap(pad(6)); // choosing it again changes nothing
    assert_eq!(b.instrument(5).as_deref(), Some("granular"));
    b.press(Button::Keyboard);
    b.strike_at(0, 0.9); // hold a pad
    b.run_ms(300);
    b.knob(5, 40 * 3); // position
    b.run_ms(200);
    b.capture("r20-cloud");
    b.release(0);
    b.press(Button::Screen(6)); // shape
    b.run_ms(100);
    b.capture("r20-shape");
    b.press(Button::Browser);
    assert!(b.ui().browse[..8].iter().all(|c| c.1), "eight presets");
    b.tap(pad(4));
    assert!(b.ui().browse_note.starts_with("loaded"), "{}", b.ui().browse_note);
}

// F-G04
#[test]
fn recipe_21_slices_a_kit_made_of_a_sound() {
    let _one_at_a_time = serial();
    let mut b = Bench::empty();
    b.press(Button::Group(4)); // E: the sampler
    b.knob(4, 100); // the bell
    b.press(Button::Screen(6)); // shape
    b.knob(6, 500); // loop: slices 8
    b.run_ms(100);
    assert_eq!(b.param(4, 6), 0.625);
    b.capture("r21-slices");
    b.press(Button::Keyboard);
    b.strike_at(0, 0.9); // C3: the first slice
    b.run_ms(300);
    assert!(b.rms_db(0, b.now_ms()) > -60.0, "a slice is heard");
    b.release(0);
}

// F-G04
#[test]
fn recipe_22_settings_audio_midi_and_light() {
    use op8::controller::{DeviceList, Request};
    use op8_ui::NavKind;
    let _one_at_a_time = serial();
    let mut b = Bench::demo();
    b.press(Button::Settings);
    b.press(Button::Screen(2)); // audio
    assert!(b.app.ctl.requests.contains(&Request::ListDevices), "it asks for the devices");
    b.app.ctl.devices_listed(DeviceList {
        outs: vec!["Maschine MK3".into(), "MacBook Pro Speakers".into(), "USB Audio".into()],
        ins: vec!["Maschine MK3".into(), "Scarlett 2i2".into()],
        ports: vec!["Maschine MK3 MIDI".into(), "IAC Driver Bus 1".into()],
        out_now: "Maschine MK3".into(),
        in_now: "Maschine MK3".into(),
        port_now: "Maschine MK3 MIDI".into(),
        stream_now: "48 kHz · 2 ch · buffer 128".into(),
        supply: "the host gives the MK3 480 mA: limited: screens at 50, LEDs capped".into(),
        note: None,
    });
    b.run_ms(100);
    b.capture("r22-audio");
    for _ in 0..40 {
        if b.ui().focus == NavKind::Pick(2) {
            break;
        }
        b.encoder(1);
    }
    b.press(Button::JogPush);
    assert_eq!(b.app.ctl.settings.audio_out.as_deref(), Some("MacBook Pro Speakers"));
    b.press(Button::Screen(3)); // midi
    b.run_ms(100);
    b.capture("r22-midi");
    b.press(Button::Screen(4)); // light
    b.run_ms(100);
    b.capture("r22-light");
    assert_eq!((b.ui().settings.screen, b.ui().settings.pads), (4, 3));
    b.press(Button::Screen(1));
    b.press(Button::ChannelMidi);
}
