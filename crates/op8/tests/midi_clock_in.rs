//! MIDI clock in (docs/features.md, F-M06): the tempo, the start, the continue, the stop and the song position pointer from the input.
use op8::bench::Bench;
use std::sync::atomic::Ordering::Relaxed;

/// Send pulses of a clock at `bpm` for `secs`, each at its own time (the bench runs on until that time), starting at `*t_us`.
fn pulses(b: &mut Bench, bpm: f64, secs: f64, t_us: &mut u64) {
    let gap = 60_000_000.0 / 24.0 / bpm;
    let end = *t_us + (secs * 1e6) as u64;
    let mut k = 0.0;
    while *t_us < end {
        while b.now_ms() * 1000 < *t_us {
            b.run_ms(1);
        }
        b.midi_in_at(&[0xF8], *t_us);
        k += 1.0;
        *t_us += gap as u64;
        let _ = k;
    }
}

fn clock_bench() -> Bench {
    let mut b = Bench::empty();
    b.app.ctl.settings.midi_clock_in = true;
    b
}

// F-M06: with the clock in on, a start makes the sequencer play, its tempo becomes the one of the pulses (and follows it when it
// changes), it does not drift from the pulses (where it is stays within a few ticks of where they say it is), and a stop stops it.
#[test]
fn f_m06_the_clock_in_gives_the_tempo_the_start_and_the_stop() {
    let mut b = clock_bench();
    let mut t = 1_000_000u64;
    b.midi_in_at(&[0xFA], t);
    pulses(&mut b, 90.0, 8.0, &mut t);
    assert!(b.status().playing.load(Relaxed), "the start played it");
    let bpm = b.project().bpm;
    assert!((bpm - 90.0).abs() < 1.0, "the tempo is the clock's: {bpm}");
    // where it is, against where 8 s of 90 BPM puts it: 96 ticks to a quarter note, 1.5 of them to a second
    let want = (8.0f64 * 90.0 / 60.0 * 96.0) as i64;
    let got = b.status().tick.load(Relaxed) as i64;
    assert!((got - want).abs() < 16, "it keeps up with the pulses: {got} ticks where the clock says {want}");
    // a change of tempo is followed
    pulses(&mut b, 140.0, 6.0, &mut t);
    let bpm = b.project().bpm;
    assert!((bpm - 140.0).abs() < 2.0, "{bpm}");
    b.midi_in_at(&[0xFC], t);
    b.run_ms(100);
    assert!(!b.status().playing.load(Relaxed), "the stop stopped it");
}

// F-M06: a song position pointer and a continue start from that place; a start starts from the top; and with the clock in off all of it
// is ignored.
#[test]
fn f_m06_the_song_position_and_continue_start_from_a_place() {
    let mut b = clock_bench();
    b.midi_in(&[0xF2, 8, 0]); // eight sixteenths: two quarter notes, 192 ticks
    b.midi_in(&[0xFB]);
    b.run_ms(100);
    assert!(b.status().playing.load(Relaxed));
    let tick = b.status().tick.load(Relaxed);
    assert!((192..192 + 96).contains(&tick), "it went on from the second beat: {tick}");
    b.midi_in(&[0xFA]);
    b.run_ms(60);
    assert!(b.status().tick.load(Relaxed) < 96, "a start is from the top");
    b.midi_in(&[0xFC]);
    b.run_ms(60);
    // off: nothing happens
    b.app.ctl.settings.midi_clock_in = false;
    b.midi_in(&[0xFA]);
    b.run_ms(100);
    assert!(!b.status().playing.load(Relaxed), "with the clock in off a start does nothing");
}
