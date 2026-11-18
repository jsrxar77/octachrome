//! MIDI clock out: start, 24 clocks to the quarter note, stop.
use octachrome::bench::Bench;
use octachrome_hw::Button;

fn count(b: &Bench, byte: u8) -> usize {
    b.sync_log.iter().filter(|(_, x)| *x == byte).count()
}

// F-M03: PLAY sends start and then a clock every 1/24 of a quarter note, at the project's tempo; STOP sends stop and the clocks cease.
#[test]
fn f_m03_the_clock_follows_the_sequencer() {
    let mut b = Bench::empty();
    b.run_ms(300);
    assert!(b.sync_log.is_empty(), "nothing while stopped");
    let bpm = b.project().bpm;
    b.press(Button::Play);
    b.run_ms(2000);
    assert_eq!(b.sync_log.first().map(|x| x.1), Some(0xFA), "start comes first");
    assert_eq!(count(&b, 0xFA), 1);
    // the gaps between clocks: 60 / bpm / 24 seconds, to a sample
    let clocks: Vec<u64> = b.sync_log.iter().filter(|(_, x)| *x == 0xF8).map(|(t, _)| *t).collect();
    let want = 48_000.0 * 60.0 / (bpm as f64 * 24.0);
    assert!(clocks.len() > 40, "{}", clocks.len());
    for w in clocks.windows(2) {
        assert!(((w[1] - w[0]) as f64 - want).abs() <= 1.01, "{} apart, want {want}", w[1] - w[0]);
    }
    // two seconds at the tempo hold about bpm / 30 quarter notes, 24 clocks each
    let beats = 2.0 * bpm as f64 / 60.0;
    assert!((clocks.len() as f64 - beats * 24.0).abs() <= 2.0, "{} clocks in 2 s at {bpm}", clocks.len());
    b.press(Button::Stop);
    b.run_ms(100);
    assert_eq!(count(&b, 0xFC), 1, "stop is sent once");
    let n = count(&b, 0xF8);
    b.run_ms(500);
    assert_eq!(count(&b, 0xF8), n, "no clocks while stopped");
    b.press(Button::Stop);
    b.run_ms(100);
    assert_eq!(count(&b, 0xFC), 1, "STOP again says nothing");
}

// F-M03: a new tempo changes the gap between clocks at once, and RESTART starts again (a new start).
#[test]
fn f_m03_the_clock_follows_the_tempo_and_restarts() {
    let mut b = Bench::empty();
    b.press(Button::Play);
    b.run_ms(500);
    b.app.handle.seq(octachrome_seq::Command::SetBpm(60.0));
    b.run_ms(300);
    let from = b.sync_log.len();
    b.run_ms(1000);
    let clocks: Vec<u64> = b.sync_log[from..].iter().filter(|(_, x)| *x == 0xF8).map(|(t, _)| *t).collect();
    let gap = (clocks[3] - clocks[2]) as f64;
    assert!((gap - 48_000.0 / 24.0).abs() <= 1.01, "60 bpm: {gap} samples between clocks");
    b.press(Button::Restart);
    b.run_ms(300);
    assert_eq!(count(&b, 0xFA), 2, "RESTART starts again");
}
