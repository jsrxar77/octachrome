//! The MK3's screens, driven from a thread of their own.
//!
//! One transfer to a screen blocks for milliseconds, so the control loop does
//! not make it: it hands over a finished frame ([`DisplayLink::submit`], a
//! memory copy) and a worker thread does the USB part. If frames arrive faster
//! than the device takes them, the worker simply sends the latest one: nothing
//! queues up and the picture never lags behind the controls.
//!
//! What is sent, per screen and per round:
//! * only the rows that changed since the last transfer, as one band of full
//!   rows (`protocol::encode_band`) so the whole change lands in one go;
//! * the whole screen again every [`FULL_REFRESH`], which repairs anything the
//!   device dropped or drew wrongly (pixels that never change are otherwise
//!   never sent again).
//!
//! Set `OP3_DISPLAY_STATS=1` to print what the worker sends, once a second.

use super::{protocol::encode_band, DISPLAY_ENDPOINT};
use crate::{FrameBuffer480x272, HwError, HwResult, Screen};
use std::{
    sync::{Arc, Condvar, Mutex},
    thread::JoinHandle,
    time::{Duration, Instant},
};

const W: usize = FrameBuffer480x272::WIDTH;
const H: usize = FrameBuffer480x272::HEIGHT;
/// Both screens are sent in full this often.
const FULL_REFRESH: Duration = Duration::from_millis(500);
const SCREENS: [Screen; 2] = [Screen::Left, Screen::Right];

struct Shared {
    /// The latest frame handed over for each screen.
    frames: [Vec<u16>; 2],
    /// Frames handed over so far, per screen (0 = none yet).
    seq: [u64; 2],
    stop: bool,
    /// Set by the worker when a transfer fails; reported by the next `submit`.
    error: Option<String>,
}

pub struct DisplayLink {
    shared: Arc<(Mutex<Shared>, Condvar)>,
    worker: Option<JoinHandle<()>>,
}

impl DisplayLink {
    /// Start the worker on a claimed display interface.
    pub fn start(iface: nusb::Interface) -> Self {
        let shared = Arc::new((
            Mutex::new(Shared {
                frames: [vec![0; W * H], vec![0; W * H]],
                seq: [0; 2],
                stop: false,
                error: None,
            }),
            Condvar::new(),
        ));
        let worker = {
            let shared = shared.clone();
            std::thread::Builder::new()
                .name("mk3-displays".into())
                .spawn(move || worker(iface, shared))
                .expect("spawn the display thread")
        };
        Self {
            shared,
            worker: Some(worker),
        }
    }

    /// Hand over a finished frame for `screen`. Returns the error of an
    /// earlier failed transfer, if there was one.
    pub fn submit(&self, screen: Screen, fb: &FrameBuffer480x272) -> HwResult<()> {
        let (lock, wake) = &*self.shared;
        let mut s = lock.lock().unwrap();
        if let Some(e) = s.error.take() {
            return Err(HwError(e));
        }
        s.frames[screen as usize].copy_from_slice(fb.pixels());
        s.seq[screen as usize] += 1;
        drop(s);
        wake.notify_one();
        Ok(())
    }
}

impl Drop for DisplayLink {
    fn drop(&mut self) {
        let (lock, wake) = &*self.shared;
        lock.lock().unwrap().stop = true;
        wake.notify_all();
        if let Some(w) = self.worker.take() {
            let _ = w.join();
        }
    }
}

/// First and last changed rows (end exclusive), if any.
fn changed_rows(new: &[u16], old: &[u16]) -> Option<(usize, usize)> {
    let differs = |y: usize| new[y * W..(y + 1) * W] != old[y * W..(y + 1) * W];
    let y0 = (0..H).find(|&y| differs(y))?;
    let y1 = (0..H).rev().find(|&y| differs(y)).unwrap_or(y0) + 1;
    Some((y0, y1))
}

fn send(iface: &nusb::Interface, buf: &mut Vec<u8>) -> Result<(), String> {
    let done = futures_lite::future::block_on(iface.bulk_out(DISPLAY_ENDPOINT, std::mem::take(buf)));
    let status = done.status;
    // Keep the allocation so steady-state frames don't allocate.
    *buf = done.data.reuse();
    status.map_err(|e| format!("display bulk transfer: {e}"))
}

/// Keeps sustained sending under a byte rate. The MK3 resets itself when it is
/// fed in bursts (seen at about twice the usual 1.1 MB/s), so a burst of
/// changing pixels is spread out instead: frames arrive a little later, the
/// device stays up. Tokens refill at `rate` and hold at most a quarter second.
struct Pacer {
    rate: f64,
    burst: f64,
    tokens: f64,
    last: Instant,
}

impl Pacer {
    /// `bytes_per_sec == 0` means no limit.
    fn new(bytes_per_sec: f64, now: Instant) -> Self {
        let burst = bytes_per_sec * 0.25;
        Self {
            rate: bytes_per_sec,
            burst,
            tokens: burst,
            last: now,
        }
    }

    /// Spend `bytes`; returns how long to wait before sending them.
    fn wait_for(&mut self, bytes: usize, now: Instant) -> Duration {
        if self.rate <= 0.0 {
            return Duration::ZERO;
        }
        let dt = now.saturating_duration_since(self.last).as_secs_f64();
        self.last = now;
        self.tokens = (self.tokens + dt * self.rate).min(self.burst) - bytes as f64;
        if self.tokens >= 0.0 {
            Duration::ZERO
        } else {
            Duration::from_secs_f64(-self.tokens / self.rate)
        }
    }
}

/// Default ceiling, in KB/s: none. The resets that looked like a bandwidth limit
/// were the backlight (measured 2026-11-13: at brightness 50 the MK3 ran 50 s of
/// PLAY with whole rows at 3.4 MB/s and 90 transfers/s). `OP3_DISPLAY_KBPS=<n>` sets one.
const DEFAULT_KBPS: f64 = 0.0;
/// Default rounds per second: no limit. `OP3_DISPLAY_FPS=<n>` sets one.
const DEFAULT_FPS: f64 = 0.0;

fn worker(iface: nusb::Interface, shared: Arc<(Mutex<Shared>, Condvar)>) {
    let (lock, wake) = &*shared;
    let stats = std::env::var_os("OP3_DISPLAY_STATS").is_some();
    let mut buf = Vec::with_capacity(300_000);
    let mut local = [vec![0u16; W * H], vec![0u16; W * H]];
    let mut shown = [vec![0u16; W * H], vec![0u16; W * H]];
    let mut known = [false; 2];
    let mut seen = [0u64; 2];
    let mut last_full = Instant::now();
    // (transfers, bytes, time in transfers) since the last stats line.
    let mut tally = (0u32, 0usize, Duration::ZERO);
    let mut tally_since = Instant::now();
    let kbps = std::env::var("OP3_DISPLAY_KBPS")
        .ok()
        .and_then(|v| v.parse::<f64>().ok())
        .unwrap_or(DEFAULT_KBPS);
    let mut pacer = Pacer::new(kbps * 1024.0, Instant::now());
    // `OP3_DISPLAY_DELTA=1` skips the pixel pairs that did not change instead of
    // sending whole rows. Off by default: it cut the traffic by four, but it has not
    // been looked at on the real screens, and the traffic was never the problem.
    let delta = std::env::var("OP3_DISPLAY_DELTA").is_ok_and(|v| v == "1");
    // Rounds per second (each sends up to one transfer per screen). 0 = no limit.
    let fps = std::env::var("OP3_DISPLAY_FPS")
        .ok()
        .and_then(|v| v.parse::<f64>().ok())
        .unwrap_or(DEFAULT_FPS);
    let round_period = if fps > 0.0 {
        Duration::from_secs_f64(1.0 / fps)
    } else {
        Duration::ZERO
    };
    // (when, bytes) of recent transfers, to say how loaded the link was if one fails.
    let mut recent: std::collections::VecDeque<(Instant, usize)> = Default::default();

    // The first transfer after claiming the interface can come out cut. Send a
    // blank frame to each screen first, and treat them as unknown afterwards,
    // so the first real frame goes out in full.
    for screen in SCREENS {
        encode_band(screen, &shown[0], None, 0, H, &mut buf);
        if let Err(e) = send(&iface, &mut buf) {
            lock.lock().unwrap().error = Some(e);
            return;
        }
    }

    loop {
        let mut g = lock.lock().unwrap();
        loop {
            if g.stop {
                return;
            }
            let started = g.seq != [0, 0];
            if started && (g.seq != seen || last_full.elapsed() >= FULL_REFRESH) {
                break;
            }
            let wait = if started {
                FULL_REFRESH.saturating_sub(last_full.elapsed())
            } else {
                FULL_REFRESH
            };
            g = wake.wait_timeout(g, wait.max(Duration::from_millis(1))).unwrap().0;
        }
        let round_start = Instant::now();
        let full = last_full.elapsed() >= FULL_REFRESH;
        let mut todo = [false; 2];
        for s in 0..2 {
            if g.seq[s] != 0 && (g.seq[s] != seen[s] || full) {
                local[s].copy_from_slice(&g.frames[s]);
                seen[s] = g.seq[s];
                todo[s] = true;
            }
        }
        drop(g);
        if full {
            last_full = Instant::now();
        }

        for s in 0..2 {
            if !todo[s] {
                continue;
            }
            let rows = if full || !known[s] {
                Some((0, H))
            } else {
                changed_rows(&local[s], &shown[s])
            };
            let Some((y0, y1)) = rows else { continue };
            // Unchanged pixel pairs are skipped, but only when the screen is known to
            // show `shown[s]`: a full refresh (or a first frame) sends everything.
            let prev = if delta && !full && known[s] {
                Some(&shown[s][..])
            } else {
                None
            };
            encode_band(SCREENS[s], &local[s], prev, y0, y1, &mut buf);
            let wait = pacer.wait_for(buf.len(), Instant::now());
            if !wait.is_zero() {
                std::thread::sleep(wait.min(Duration::from_millis(250)));
            }
            let (bytes, started) = (buf.len(), Instant::now());
            if let Err(e) = send(&iface, &mut buf) {
                recent.retain(|(t, _)| t.elapsed() < Duration::from_secs(1));
                let kb: usize = recent.iter().map(|(_, b)| b).sum::<usize>() / 1024;
                lock.lock().unwrap().error = Some(format!("{e} ({kb} KB sent in the last second)"));
                return;
            }
            recent.push_back((Instant::now(), bytes));
            shown[s].copy_from_slice(&local[s]);
            known[s] = true;
            tally = (tally.0 + 1, tally.1 + bytes, tally.2 + started.elapsed());
        }

        if stats && tally_since.elapsed() >= Duration::from_secs(1) {
            let secs = tally_since.elapsed().as_secs_f64();
            eprintln!(
                "display: {:.0} transfers/s, {:.0} KB/s, {:.1} ms each",
                tally.0 as f64 / secs,
                tally.1 as f64 / 1024.0 / secs,
                tally.2.as_secs_f64() * 1000.0 / tally.0.max(1) as f64,
            );
            tally = (0, 0, Duration::ZERO);
            tally_since = Instant::now();
        }

        // At most `fps` rounds a second. Frames that arrive meanwhile are not
        // queued: the next round sends the latest one.
        if let Some(rest) = round_period.checked_sub(round_start.elapsed()) {
            std::thread::sleep(rest);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // F-H01 (docs/features.md)
    #[test]
    fn pacer_lets_a_burst_through_then_spreads_the_rest() {
        let t0 = Instant::now();
        let mut p = Pacer::new(1000.0, t0); // 1000 B/s, burst 250 B
        assert_eq!(p.wait_for(100, t0), Duration::ZERO);
        assert_eq!(p.wait_for(100, t0), Duration::ZERO);
        // 50 B over the burst: 50 ms at 1000 B/s.
        let w = p.wait_for(100, t0);
        assert!((w.as_secs_f64() - 0.05).abs() < 1e-6, "{w:?}");
        // Half a second later the bucket has refilled (up to the burst).
        assert_eq!(p.wait_for(100, t0 + Duration::from_millis(500)), Duration::ZERO);
    }

    // F-H01
    #[test]
    fn pacer_holds_the_average_under_the_rate() {
        let t0 = Instant::now();
        let mut p = Pacer::new(100_000.0, t0);
        let (mut now, mut sent) = (t0, 0usize);
        // Ten seconds of 50 KB bands offered as fast as the pacer allows.
        while now.duration_since(t0) < Duration::from_secs(10) {
            now += p.wait_for(50_000, now);
            sent += 50_000;
        }
        let rate = sent as f64 / 10.0;
        assert!(rate < 110_000.0, "{rate:.0} B/s against a limit of 100000");
        assert!(rate > 90_000.0, "{rate:.0} B/s: the pacer is too strict");
    }

    // F-H01
    #[test]
    fn a_zero_rate_means_no_limit() {
        let t0 = Instant::now();
        let mut p = Pacer::new(0.0, t0);
        assert_eq!(p.wait_for(10_000_000, t0), Duration::ZERO);
    }

    #[test]
    fn changed_rows_finds_the_first_and_last() {
        let old = vec![0u16; W * H];
        let mut new = old.clone();
        assert_eq!(changed_rows(&new, &old), None);
        new[5 * W + 100] = 1;
        assert_eq!(changed_rows(&new, &old), Some((5, 6)));
        new[200 * W] = 1;
        assert_eq!(changed_rows(&new, &old), Some((5, 201)));
        new[(H - 1) * W + W - 1] = 1;
        assert_eq!(changed_rows(&new, &old), Some((5, H)));
    }
}
