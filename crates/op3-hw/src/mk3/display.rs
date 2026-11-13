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

    // The first transfer after claiming the interface can come out cut. Send a
    // blank frame to each screen first, and treat them as unknown afterwards,
    // so the first real frame goes out in full.
    for screen in SCREENS {
        encode_band(screen, &shown[0], 0, H, &mut buf);
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
            encode_band(SCREENS[s], &local[s], y0, y1, &mut buf);
            let (bytes, started) = (buf.len(), Instant::now());
            if let Err(e) = send(&iface, &mut buf) {
                lock.lock().unwrap().error = Some(e);
                return;
            }
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
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
