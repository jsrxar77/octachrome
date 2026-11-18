//! SAMPLING: recording a sound, keeping a part of it and putting it in the sampler (docs/features.md §A, §K, §D).
use octachrome::bench::Bench;
use octachrome_hw::{Button, Screen};
use octachrome_ui::Main;
use std::sync::atomic::Ordering::Relaxed;

fn pad_of(slot: usize) -> u8 {
    (0..16u8).find(|&p| octachrome::Controller::pad_to_step(p) as usize == slot).unwrap()
}

fn dir(name: &str) -> std::path::PathBuf {
    let d = std::env::temp_dir().join(format!("octachrome-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    d
}

/// The demo plays for a while and the master is recorded for `ms`; SAMPLING is left with a take.
fn with_a_take(b: &mut Bench, ms: u64) {
    b.press(Button::Play);
    b.run_ms(300);
    b.press(Button::Sampling);
    b.press(Button::Rec);
    b.run_ms(ms);
    b.press(Button::Rec);
    b.run_ms(100);
    b.press(Button::Stop);
    b.run_ms(300);
}

// F-K14: SAMPLING takes both screens, REC records the master (the default source) until REC again, and the
// recording is shown as a wave with its length.
#[test]
fn f_k14_rec_records_the_master() {
    let mut b = Bench::demo();
    b.press(Button::Play);
    b.run_ms(200);
    b.press(Button::Sampling);
    assert_eq!(b.ui().main, Main::Sampling);
    assert_eq!(b.ui().smp.len, 0);
    b.press(Button::Rec);
    b.run_ms(1500);
    assert!(b.ui().smp.recording);
    assert!(b.ui().smp.level > 0.0, "the level moves while it records");
    b.press(Button::Rec);
    b.run_ms(200);
    let s = &b.ui().smp;
    assert!(!s.recording);
    assert!((70_000..76_000).contains(&s.len), "about a second and a half: {}", s.len);
    assert!(s.peaks.iter().any(|&(lo, hi)| hi - lo > 0.05), "the wave has something in it");
    b.capture("f-k14-sampling");
    // SAMPLING again goes back to composing
    b.press(Button::Sampling);
    assert_eq!(b.ui().main, Main::Instrument);
}

// F-K14: the knobs choose the source and cut the take; they cannot cross, and the part is what is heard and saved.
#[test]
fn f_k14_the_knobs_cut_the_take() {
    let mut b = Bench::demo();
    with_a_take(&mut b, 2000);
    assert_eq!(b.ui().smp.source, 1);
    b.knob(4, -200);
    assert_eq!(b.ui().smp.source, 0, "knob 5 chooses the source: input");
    b.knob(4, 200);
    assert_eq!(b.ui().smp.source, 1);
    b.knob(5, 200); // start +25%
    b.knob(6, -200); // end −25%
    let s = &b.ui().smp;
    assert!((s.start - 0.25).abs() < 0.01 && (s.end - 0.75).abs() < 0.01, "{} {}", s.start, s.end);
    b.knob(5, 5000);
    assert!(b.ui().smp.start < b.ui().smp.end, "the ends never cross");
    b.knob(5, -5000);
    b.knob(7, 400);
    assert!(b.ui().smp.fade > 0.4);
}

// F-K14: the bottom pads play the kept part; they say so when there is nothing to play.
#[test]
fn f_k14_the_bottom_pads_listen() {
    let mut b = Bench::demo();
    b.press(Button::Sampling);
    b.tap(pad_of(10));
    assert!(b.ui().smp.note.contains("nothing recorded"));
    with_a_take(&mut b, 1500);
    let from = b.now_ms();
    b.tap(pad_of(10));
    b.run_ms(600);
    let to = b.now_ms();
    assert!(b.peak_db(from, to) > -30.0, "it is heard: {} dB", b.peak_db(from, to));
    // and it stops by itself at the end of the part
    b.run_ms(2500);
    let end = b.now_ms();
    assert!(b.silent(end - 200, end), "the preview ended");
}

// F-A17: a pad of the top two rows saves the kept part as that place's WAV file, the sampler that plays has it at
// once, and a file left from before is read again when the instrument is made.
#[test]
fn f_a17_a_pad_saves_the_part_into_the_sampler() {
    let d = dir("sampling");
    let mut b = Bench::demo();
    b.set_samples_dir(d.clone());
    // a sampler on an empty track first, so that it is already playing when the sound arrives
    b.press(Button::Group(5));
    b.press(Button::PlugIn);
    b.tap(pad_of(4));
    b.knob(4, 400); // sample: user 1 (empty)
    with_a_take(&mut b, 1500);
    b.knob(5, 100);
    b.tap(pad_of(0));
    assert!(b.ui().smp.note.contains("saved in user 1"), "{}", b.ui().smp.note);
    assert!(b.ui().smp.saved[0] && !b.ui().smp.saved[1]);
    let file = d.join("rec-1.wav");
    let wav = octachrome_cartridges::wav::parse(&std::fs::read(&file).expect("the file is there")).expect("a WAV");
    assert!((60_000..68_000).contains(&wav.len()), "the part that was kept: {}", wav.len());
    assert!(wav.iter().fold(0.0f32, |m, x| m.max(x.abs())) > 0.8, "brought up to a good level");
    assert!(wav[0].abs() < 0.05 && wav[wav.len() - 1].abs() < 0.05, "faded at both ends");
    assert!(!d.join("rec-1.wav.tmp").exists());
    // the sampler plays it from a step, with the sequencer
    b.press(Button::Sampling);
    b.press(Button::Group(5));
    b.tap(0);
    let from = b.now_ms();
    b.press(Button::Play);
    b.run_ms(1500);
    let to = b.now_ms();
    assert!(b.peak_db(from, to) > -30.0, "it plays: {} dB", b.peak_db(from, to));
    assert!(b.status().note_count[5].load(Relaxed) > 0);
    let _ = std::fs::remove_dir_all(&d);
}

// F-A17: nothing is saved while it records, with no take, or with no folder; each says so.
#[test]
fn f_a17_saving_says_when_it_cannot() {
    let mut b = Bench::demo();
    b.press(Button::Sampling);
    b.tap(pad_of(0));
    assert!(b.ui().smp.note.contains("nothing recorded"));
    b.press(Button::Play);
    b.press(Button::Rec);
    b.run_ms(300);
    b.tap(pad_of(0));
    assert!(b.ui().smp.note.contains("stop recording"));
    b.press(Button::Rec);
    b.run_ms(100);
    b.tap(pad_of(0));
    assert!(b.ui().smp.note.contains("no folder"), "{}", b.ui().smp.note);
}

// F-D16: SAMPLING draws both screens (the pads on the left, the wave on the right), both move, and the buttons light.
#[test]
fn f_d16_both_screens_draw_and_move() {
    let mut b = Bench::demo();
    b.press(Button::Sampling);
    b.run_ms(100);
    let (l0, r0) = (b.screen_hash(octachrome_hw::Screen::Left), b.screen_hash(octachrome_hw::Screen::Right));
    assert!(b.lit_pixels(octachrome_hw::Screen::Left) > 500 && b.lit_pixels(octachrome_hw::Screen::Right) > 500);
    b.run_ms(700);
    assert_ne!(b.screen_hash(octachrome_hw::Screen::Right), r0, "the right screen moves while it waits");
    let _ = l0;
    b.press(Button::Rec);
    b.press(Button::Play);
    b.run_ms(500);
    let m = b.leds().clone();
    let _ = m;
    assert!(b.ui().smp.recording);
}

// F-I06: what SAMPLING saves is heard in a granular that is already playing, as it is in a sampler.
#[test]
fn f_i06_a_recording_can_be_the_sound_of_the_granular() {
    let d = dir("granular");
    let mut b = Bench::demo();
    b.set_samples_dir(d.clone());
    b.press(Button::Group(5));
    b.press(Button::PlugIn);
    b.tap(pad_of(5)); // the granular
    b.knob(4, 400); // sample: user 1 (empty)
    with_a_take(&mut b, 1500);
    b.knob(5, 100);
    b.tap(pad_of(0));
    assert!(b.ui().smp.note.contains("saved in user 1"), "{}", b.ui().smp.note);
    b.press(Button::Sampling);
    b.press(Button::Group(5));
    b.tap(12);
    let from = b.now_ms();
    b.press(Button::Play);
    b.run_ms(1500);
    assert!(b.peak_db(from, b.now_ms()) > -40.0, "the granular plays what was recorded: {} dB", b.peak_db(from, b.now_ms()));
    let _ = std::fs::remove_dir_all(&d);
}

// F-K20: TAP in SAMPLING turns the count-in on: REC then counts three beats of the project's tempo (a click on each, the last higher, and
// the beats on the right screen) before it records; REC during the count cancels it, leaving SAMPLING too, and with it off REC starts at once.
#[test]
fn f_k20_the_count_in_comes_before_the_recording() {
    let mut b = Bench::empty();
    b.press(Button::Sampling);
    b.press(Button::Rec);
    b.run_ms(100);
    assert!(b.ui().smp.recording && b.ui().smp.counting == 0, "off: REC starts at once");
    b.press(Button::Rec);
    b.run_ms(100);
    b.press(Button::Tap);
    assert!(b.ui().smp.count_in && b.ui().smp.note.contains("count-in on"));
    let beat = (60_000.0 / b.project().bpm) as u64;
    let from = b.now_ms();
    b.press(Button::Rec);
    b.run_ms(60);
    assert!(b.ui().smp.counting == 3 && !b.ui().smp.recording, "counting three, not recording");
    assert!(b.peak_db(from, from + 60) > -40.0, "the first click is heard");
    let counting = b.screen_hash(Screen::Right);
    b.capture("f-k20-count-in");
    b.run_ms(beat);
    assert_eq!(b.ui().smp.counting, 2);
    assert_ne!(b.screen_hash(Screen::Right), counting, "the number changes on the screen");
    b.run_ms(beat);
    assert_eq!(b.ui().smp.counting, 1);
    b.run_ms(beat);
    assert!(b.ui().smp.recording && b.ui().smp.counting == 0, "after the third beat it records");
    b.press(Button::Rec);
    b.run_ms(100);
    assert!(!b.ui().smp.recording);
    // REC during the count cancels it, and nothing records afterwards
    b.press(Button::Rec);
    b.run_ms(beat);
    b.press(Button::Rec);
    b.run_ms(beat * 4);
    assert!(b.ui().smp.counting == 0 && !b.ui().smp.recording, "cancelled");
    // leaving SAMPLING cancels it too
    b.press(Button::Rec);
    b.run_ms(100);
    b.press(Button::Sampling);
    b.run_ms(beat * 4);
    assert!(!b.ui().smp.recording && b.ui().smp.counting == 0, "left: nothing records");
    // TAP again turns it off
    b.press(Button::Sampling);
    b.press(Button::Tap);
    assert!(!b.ui().smp.count_in);
}

// F-A25: the ends of the part (knobs 6 and 7) move while it plays: the end cuts it short, the start jumps ahead.
#[test]
fn f_a25_the_cut_moves_while_it_plays() {
    let heard = |move_cut: bool| {
        let mut b = Bench::demo();
        with_a_take(&mut b, 1500);
        let from = b.now_ms();
        b.tap(pad_of(10));
        b.run_ms(300);
        if move_cut {
            b.knob(6, -700);
        }
        b.run_ms(900);
        (b.peak_db(from + 500, from + 1000), b.ui().smp.end)
    };
    let (still, _) = heard(false);
    assert!(still > -20.0, "without moving it, it is still playing: {still} dB");
    let (cut, end) = heard(true);
    assert!(end < 0.2, "{end}");
    assert!(cut < still - 30.0, "the end moved behind it and it stopped: {cut} dB against {still} dB");
}
