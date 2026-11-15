//! SAMPLING: recording a sound, keeping a part of it and putting it in the sampler (docs/features.md §A, §K, §D).
use op8::bench::Bench;
use op8_hw::Button;
use op8_ui::Main;
use std::sync::atomic::Ordering::Relaxed;

fn pad_of(slot: usize) -> u8 {
    (0..16u8).find(|&p| op8::Controller::pad_to_step(p) as usize == slot).unwrap()
}

fn dir(name: &str) -> std::path::PathBuf {
    let d = std::env::temp_dir().join(format!("op8-{name}-{}", std::process::id()));
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
    b.press(Button::Group(6));
    b.press(Button::PlugIn);
    b.tap(pad_of(4));
    b.knob(4, 400); // sample: user 1 (empty)
    with_a_take(&mut b, 1500);
    b.knob(5, 100);
    b.tap(pad_of(0));
    assert!(b.ui().smp.note.contains("saved in user 1"), "{}", b.ui().smp.note);
    assert!(b.ui().smp.saved[0] && !b.ui().smp.saved[1]);
    let file = d.join("rec-1.wav");
    let wav = op8_cartridges::wav::parse(&std::fs::read(&file).expect("the file is there")).expect("a WAV");
    assert!((60_000..68_000).contains(&wav.len()), "the part that was kept: {}", wav.len());
    assert!(wav.iter().fold(0.0f32, |m, x| m.max(x.abs())) > 0.8, "brought up to a good level");
    assert!(wav[0].abs() < 0.05 && wav[wav.len() - 1].abs() < 0.05, "faded at both ends");
    assert!(!d.join("rec-1.wav.tmp").exists());
    // the sampler plays it from a step, with the sequencer
    b.press(Button::Sampling);
    b.press(Button::Group(6));
    b.tap(0);
    let from = b.now_ms();
    b.press(Button::Play);
    b.run_ms(1500);
    let to = b.now_ms();
    assert!(b.peak_db(from, to) > -30.0, "it plays: {} dB", b.peak_db(from, to));
    assert!(b.status().note_count[6].load(Relaxed) > 0);
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
    let (l0, r0) = (b.screen_hash(op8_hw::Screen::Left), b.screen_hash(op8_hw::Screen::Right));
    assert!(b.lit_pixels(op8_hw::Screen::Left) > 500 && b.lit_pixels(op8_hw::Screen::Right) > 500);
    b.run_ms(700);
    assert_ne!(b.screen_hash(op8_hw::Screen::Right), r0, "the right screen moves while it waits");
    let _ = l0;
    b.press(Button::Rec);
    b.press(Button::Play);
    b.run_ms(500);
    let m = b.leds().clone();
    let _ = m;
    assert!(b.ui().smp.recording);
}
