//! MIDI clock in: the tempo of the pulses that come in (24 to the quarter note), and where in the song they say it is.
//!
//! The tempo is the time that up to 48 pulses (two quarter notes) took, so that the little unevenness in when each one is read by the
//! thread that listens (about a millisecond) is spread over a long stretch and is under a tenth of a BPM at 120; it is given from half a
//! quarter note on, so that it does not start far from the tempo that comes in. The position is counted from
//! the start message, in the ticks of the sequencer (96 to the quarter note, so four to a pulse), and is what keeps the sequencer from
//! drifting off from the one that sends the clock.
use std::collections::VecDeque;

/// How many pulses the tempo is measured over at most: two quarter notes. It is given from `MIN_PULSES` on.
const WINDOW: usize = 48;
const MIN_PULSES: usize = 12;
/// How far off (in ticks, half a quarter note) the sequencer can be before it jumps to where the clock says instead of bending the tempo.
pub const JUMP_TICKS: i64 = 48;
/// Ticks of the sequencer to a pulse of the clock: 96 / 24.
pub const TICKS_PER_PULSE: u64 = 4;
/// How far behind or ahead (in ticks, about 20 ms at 120 BPM) the sequencer can be before it is pulled back to where the clock says.
const DEAD_TICKS: i64 = 4;
/// The most that the tempo is bent to pull it back, as a share of the tempo.
const MAX_PULL: f32 = 0.04;

#[derive(Default)]
pub struct ClockFollower {
    stamps: VecDeque<u64>,
    /// The tick the song started from and how many pulses have come in since.
    origin: u64,
    pulses: u64,
    /// The start or continue message has come and the stop has not.
    pub running: bool,
    /// Where the last song position pointer said to go, in ticks.
    pub position: u64,
}

impl ClockFollower {
    /// A pulse came in at `at_us`. The tempo (BPM) over the last two quarter notes, once there are that many pulses.
    pub fn pulse(&mut self, at_us: u64) -> Option<f32> {
        // time that goes backwards means another port or another start of the clock: begin again
        if self.stamps.back().is_some_and(|&t| at_us < t) {
            self.stamps.clear();
        }
        self.stamps.push_back(at_us);
        if self.stamps.len() > WINDOW + 1 {
            self.stamps.pop_front();
        }
        if self.stamps.len() <= MIN_PULSES {
            return None;
        }
        let (span, n) = ((at_us - self.stamps[0]) as f64, self.stamps.len() - 1);
        (span > 0.0).then(|| (n as f64 / 24.0 * 60_000_000.0 / span) as f32)
    }

    /// The song starts (from `tick`).
    pub fn start(&mut self, tick: u64) {
        self.origin = tick;
        self.pulses = 0;
        self.running = true;
    }

    pub fn stop(&mut self) {
        self.running = false;
    }

    /// The song position pointer: 16th notes from the start of the song, which are 24 ticks each.
    pub fn set_position(&mut self, sixteenths: u16) {
        self.position = sixteenths as u64 * 24;
    }

    /// While it runs, a pulse has been counted: where the sequencer should be now, in ticks.
    pub fn expected_tick(&mut self) -> u64 {
        let t = self.origin + self.pulses * TICKS_PER_PULSE;
        self.pulses += 1;
        t
    }

    /// The tempo to ask for: the one measured, bent a little (at most 4 %) when the sequencer is more than a few ticks off from where the
    /// clock says it should be, so that it catches up (or waits) within a few beats and never drifts.
    pub fn tempo(measured: f32, expected: u64, actual: u64) -> f32 {
        let err = expected as i64 - actual as i64;
        if err.abs() <= DEAD_TICKS {
            return measured;
        }
        let pull = (err as f32 / 96.0 * 0.25).clamp(-MAX_PULL, MAX_PULL);
        measured * (1.0 + pull)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // F-M06
    #[test]
    fn the_tempo_is_what_the_pulses_say_and_not_before_there_are_enough() {
        let mut c = ClockFollower::default();
        let gap = (60_000_000.0 / 24.0 / 100.0) as u64; // 100 BPM
        let mut last = None;
        for k in 0..60u64 {
            let t = c.pulse(10_000 + k * gap);
            if k < 12 {
                assert!(t.is_none(), "pulse {k}: not enough yet");
            }
            if k >= 13 {
                assert!((t.unwrap() - 100.0).abs() < 0.05, "pulse {k}: {t:?}");
            }
            last = t;
        }
        assert!((last.unwrap() - 100.0).abs() < 0.05, "{last:?}");
        // a clock that restarts (time goes back) is measured afresh
        assert!(c.pulse(5).is_none());
    }

    // F-M06
    #[test]
    fn it_is_pulled_back_only_when_it_is_off_by_more_than_a_few_ticks() {
        assert_eq!(ClockFollower::tempo(120.0, 100, 98), 120.0, "two ticks: left alone");
        assert!(ClockFollower::tempo(120.0, 200, 100) > 120.0, "behind: faster");
        assert!(ClockFollower::tempo(120.0, 100, 200) < 120.0, "ahead: slower");
        assert!((ClockFollower::tempo(120.0, 10_000, 0) - 120.0 * 1.04).abs() < 1e-3, "never more than 4 %");
    }

    // F-M06
    #[test]
    fn the_position_counts_from_where_it_started() {
        let mut c = ClockFollower::default();
        c.set_position(8);
        assert_eq!(c.position, 192, "eight sixteenths are two quarter notes: 192 ticks");
        c.start(c.position);
        assert_eq!((c.expected_tick(), c.expected_tick(), c.expected_tick()), (192, 196, 200));
    }
}
