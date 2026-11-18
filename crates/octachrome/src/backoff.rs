//! How long to wait before reconnecting to the MK3.
//!
//! A MK3 that has just reset needs a few seconds before it behaves. Opening it
//! again at once makes it fail again, and the app ends up in a loop of the
//! screens and the sound switching on and off. So each reconnect waits, and
//! the wait grows while failures keep coming close together.

use std::time::{Duration, Instant};

/// The first wait after a failure.
const BASE: Duration = Duration::from_secs(4);
/// The longest wait.
const MAX: Duration = Duration::from_secs(30);
/// This long without a failure and the waits start over.
const STEADY: Duration = Duration::from_secs(60);

#[derive(Default)]
pub struct Backoff {
    failures: u32,
    last: Option<Instant>,
}

impl Backoff {
    pub fn new() -> Self {
        Self::default()
    }

    /// Record a failure at `now`; returns how long to wait before trying again.
    pub fn failed(&mut self, now: Instant) -> Duration {
        if self.last.is_some_and(|t| now.duration_since(t) > STEADY) {
            self.failures = 0;
        }
        self.last = Some(now);
        let wait = BASE * 2u32.pow(self.failures.min(4));
        self.failures += 1;
        wait.min(MAX)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // F-H02 (docs/features.md)
    #[test]
    fn waits_grow_while_failures_keep_coming_and_stop_at_the_cap() {
        let (mut b, mut t) = (Backoff::new(), Instant::now());
        let mut waits = Vec::new();
        for _ in 0..7 {
            let w = b.failed(t);
            waits.push(w.as_secs());
            t += w + Duration::from_secs(2); // it came back, then failed again
        }
        assert_eq!(waits, [4, 8, 16, 30, 30, 30, 30]);
    }

    // F-H02
    #[test]
    fn a_steady_minute_starts_the_waits_over() {
        let (mut b, t) = (Backoff::new(), Instant::now());
        b.failed(t);
        b.failed(t + Duration::from_secs(10));
        assert_eq!(b.failed(t + Duration::from_secs(20)).as_secs(), 16);
        assert_eq!(b.failed(t + Duration::from_secs(200)).as_secs(), 4, "steady for a minute");
    }
}
