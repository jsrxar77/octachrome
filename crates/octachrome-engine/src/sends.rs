//! The two sends: every track can send some of itself (after its level and pan) to a **delay** that keeps time with the tempo and to a
//! **reverb**; what they give back is added to the mix before the performance effects, the EQ and the compressor.
//!
//! The settings are the project's (`octachrome_seq::MasterSends`, each 0..=255); the functions here say what they mean. With nothing sent the
//! effects have nothing to ring and give back exact silence, so a project that sends nothing sounds as it did.
//!
//! The reverb is a Freeverb (Jezar at Dreampoint, public domain): eight lowpass-feedback combs and four allpasses a side, tuned for
//! 44.1 kHz and stretched to the sample rate.

use octachrome_seq::MasterSends;

/// The delay's six times, in quarter notes of the tempo: a sixteenth, an eighth, a dotted eighth, a quarter, a dotted quarter, a half.
pub const DELAY_BEATS: [f32; 6] = [0.25, 0.5, 0.75, 1.0, 1.5, 2.0];
/// How they are written on the screen.
pub const DELAY_NAMES: [&str; 6] = ["1/16", "1/8", "1/8.", "1/4", "1/4.", "1/2"];
/// The longest the delay can be, in seconds (a slow tempo cannot ask for more).
const MAX_DELAY_S: f32 = 2.0;

/// Which of the six times the setting is.
pub fn delay_index(v: u8) -> usize {
    (v as usize * DELAY_BEATS.len() / 256).min(DELAY_BEATS.len() - 1)
}
/// How much of the delay comes round again: 0 to 95 %.
pub fn feedback(v: u8) -> f32 {
    0.95 * v as f32 / 255.0
}
/// How long the reverb rings (the feedback of its combs): 0.70 to 0.98.
pub fn room(v: u8) -> f32 {
    0.70 + 0.28 * v as f32 / 255.0
}
/// How quickly the reverb loses its highs: 0.5 (dull) to 0 (bright).
pub fn damping(v: u8) -> f32 {
    0.5 * (1.0 - v as f32 / 255.0)
}

const COMB_TUNING: [usize; 8] = [1116, 1188, 1277, 1356, 1422, 1491, 1557, 1617];
const ALLPASS_TUNING: [usize; 4] = [556, 441, 341, 225];
const STEREO_SPREAD: usize = 23;

fn flush(x: f32) -> f32 {
    if x.abs() < 1e-20 {
        0.0
    } else {
        x
    }
}

struct Comb {
    buf: Vec<f32>,
    at: usize,
    store: f32,
}
impl Comb {
    fn new(n: usize) -> Self {
        Self { buf: vec![0.0; n.max(1)], at: 0, store: 0.0 }
    }
    #[inline]
    fn tick(&mut self, x: f32, room: f32, damp: f32) -> f32 {
        let out = self.buf[self.at];
        self.store = flush(out * (1.0 - damp) + self.store * damp);
        self.buf[self.at] = flush(x + self.store * room);
        self.at = (self.at + 1) % self.buf.len();
        out
    }
}

struct Allpass {
    buf: Vec<f32>,
    at: usize,
}
impl Allpass {
    fn new(n: usize) -> Self {
        Self { buf: vec![0.0; n.max(1)], at: 0 }
    }
    #[inline]
    fn tick(&mut self, x: f32) -> f32 {
        let held = self.buf[self.at];
        self.buf[self.at] = flush(x + held * 0.5);
        self.at = (self.at + 1) % self.buf.len();
        held - x
    }
}

/// The delay and the reverb, with what they hold between blocks.
pub struct Sends {
    rate: f32,
    dl: Vec<f32>,
    dr: Vec<f32>,
    at: usize,
    /// The lowpass in the delay's feedback, per channel, so that each repeat is a little duller.
    tone: [f32; 2],
    combs: [Vec<Comb>; 2],
    allpass: [Vec<Allpass>; 2],
}

impl Sends {
    pub fn new(rate: f32) -> Self {
        let k = rate / 44_100.0;
        let size = |n: usize| (n as f32 * k) as usize;
        let side = |extra: usize| -> (Vec<Comb>, Vec<Allpass>) {
            (
                COMB_TUNING.iter().map(|&n| Comb::new(size(n + extra))).collect(),
                ALLPASS_TUNING.iter().map(|&n| Allpass::new(size(n + extra))).collect(),
            )
        };
        let (cl, al) = side(0);
        let (cr, ar) = side(STEREO_SPREAD);
        let n = (MAX_DELAY_S * rate) as usize + 1;
        Self { rate, dl: vec![0.0; n], dr: vec![0.0; n], at: 0, tone: [0.0; 2], combs: [cl, cr], allpass: [al, ar] }
    }

    /// Take what was sent to the delay (`a`) and to the reverb (`b`) and add what they give back to `out`.
    pub fn process(&mut self, a: (&[f32], &[f32]), b: (&[f32], &[f32]), out: (&mut [f32], &mut [f32]), s: &MasterSends, bpm: f32) {
        let n = self.dl.len();
        let delay = ((DELAY_BEATS[delay_index(s.time)] * 60.0 / bpm.max(1.0)).min(MAX_DELAY_S) * self.rate) as usize;
        let delay = delay.clamp(1, n - 1);
        let (fb, room, damp) = (feedback(s.feedback), room(s.size), damping(s.tone));
        for i in 0..out.0.len() {
            // the delay is a ping-pong: what comes out of one side is fed, duller, into the other
            let read = (self.at + n - delay) % n;
            let (ol, or) = (self.dl[read], self.dr[read]);
            self.tone[0] = flush(self.tone[0] + (or - self.tone[0]) * 0.45);
            self.tone[1] = flush(self.tone[1] + (ol - self.tone[1]) * 0.45);
            let mono = (a.0[i] + a.1[i]) * 0.5;
            self.dl[self.at] = flush(mono + self.tone[0] * fb);
            self.dr[self.at] = flush(self.tone[1] * fb);
            self.at = (self.at + 1) % n;
            // the reverb: the eight combs in parallel and the four allpasses in a row, on each side
            let input = (b.0[i] + b.1[i]) * 0.015;
            let mut wet = [0.0f32; 2];
            for ch in 0..2 {
                let mut sum = 0.0;
                for c in self.combs[ch].iter_mut() {
                    sum += c.tick(input, room, damp);
                }
                for ap in self.allpass[ch].iter_mut() {
                    sum = ap.tick(sum);
                }
                wet[ch] = sum * 3.0;
            }
            out.0[i] += ol + wet[0];
            out.1[i] += or + wet[1];
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const RATE: f32 = 48_000.0;

    fn run(sends: &mut Sends, a: &[f32], b: &[f32], s: &MasterSends, bpm: f32) -> (Vec<f32>, Vec<f32>) {
        let (mut l, mut r) = (vec![0.0; a.len()], vec![0.0; a.len()]);
        sends.process((a, a), (b, b), (&mut l, &mut r), s, bpm);
        (l, r)
    }
    fn click(n: usize) -> Vec<f32> {
        let mut v = vec![0.0; n];
        v[0] = 1.0;
        v
    }

    // F-A29
    #[test]
    fn nothing_sent_is_exact_silence() {
        let mut s = Sends::new(RATE);
        let zero = vec![0.0; 4800];
        let (l, r) = run(&mut s, &zero, &zero, &MasterSends::default(), 120.0);
        assert!(l.iter().chain(r.iter()).all(|x| *x == 0.0), "nothing in, nothing out, to the bit");
    }

    // F-A29
    #[test]
    fn the_delay_repeats_in_time_with_the_tempo_and_dies_away() {
        // an eighth at 120 bpm is a quarter of a second: 12 000 samples
        let s = MasterSends { time: 70, feedback: 128, size: 0, tone: 255, tap_a: 0, tap_b: 0 }; // time index 1: an eighth
        assert_eq!(delay_index(s.time), 1);
        let mut fx = Sends::new(RATE);
        let (l, r) = run(&mut fx, &click(60_000), &vec![0.0; 60_000], &s, 120.0);
        let peaks: Vec<usize> = (1..l.len() - 1).filter(|&i| (l[i].abs().max(r[i].abs())) > 0.01 && l[i].abs().max(r[i].abs()) >= l[i - 1].abs().max(r[i - 1].abs()) && l[i].abs().max(r[i].abs()) >= l[i + 1].abs().max(r[i + 1].abs())).collect();
        assert!(peaks.len() >= 3, "it repeats: {peaks:?}");
        assert!(peaks[0].abs_diff(12_000) <= 2, "the first repeat is an eighth after: {}", peaks[0]);
        assert!(peaks[1].abs_diff(24_000) <= 2, "and the next an eighth later: {}", peaks[1]);
        assert!(l[peaks[1]].abs().max(r[peaks[1]].abs()) < l[peaks[0]].abs().max(r[peaks[0]].abs()), "each is quieter");
        // at 60 bpm the same setting is twice as long
        let mut slow = Sends::new(RATE);
        let (l2, r2) = run(&mut slow, &click(30_000), &vec![0.0; 30_000], &s, 60.0);
        let first = (1..l2.len()).find(|&i| l2[i].abs().max(r2[i].abs()) > 0.01).unwrap();
        assert!(first.abs_diff(24_000) <= 2, "{first}");
        // and a repeat goes to one side, the next to the other (a ping-pong)
        assert!(l[peaks[0]].abs() > 0.1 && r[peaks[1]].abs() > 0.05, "left then right");
    }

    // F-A29
    #[test]
    fn the_reverb_rings_and_a_bigger_room_rings_longer() {
        let ring = |size: u8, tone: u8| {
            let mut fx = Sends::new(RATE);
            let st = MasterSends { time: 0, feedback: 0, size, tone, tap_a: 0, tap_b: 0 };
            let (l, r) = run(&mut fx, &vec![0.0; 96_000], &click(96_000), &st, 120.0);
            let energy = |a: usize, b: usize| l[a..b].iter().chain(r[a..b].iter()).map(|x| x * x).sum::<f32>();
            (energy(0, 24_000), energy(48_000, 96_000))
        };
        let (early_small, late_small) = ring(0, 128);
        let (early_big, late_big) = ring(255, 128);
        assert!(early_small > 0.0 && early_big > 0.0, "it sounds");
        assert!(late_big > late_small * 5.0, "a bigger room rings longer: {late_small} and {late_big}");
        let (_, dull) = ring(200, 0);
        let (_, bright) = ring(200, 255);
        assert!(bright > dull, "a brighter tone keeps more");
        // it is stable: a long loud input does not run away (a full-scale tone that sits on a resonance of the room is the worst there
        // is, and it builds up to about 4.6 and no further; the master's soft clip is after it)
        let mut fx = Sends::new(RATE);
        let loud: Vec<f32> = (0..96_000).map(|i| (i as f32 * 0.01).sin()).collect();
        let (l, _) = run(&mut fx, &vec![0.0; 96_000], &loud, &MasterSends { time: 0, feedback: 0, size: 255, tone: 255, tap_a: 0, tap_b: 0 }, 120.0);
        assert!(l.iter().all(|x| x.is_finite() && x.abs() < 8.0), "it stays in bounds");
    }

    // F-A29
    #[test]
    fn the_settings_mean_what_they_say() {
        assert_eq!(delay_index(0), 0);
        assert_eq!(delay_index(255), 5);
        assert_eq!(DELAY_NAMES[delay_index(128)], "1/4");
        assert!(feedback(0) == 0.0 && (feedback(255) - 0.95).abs() < 1e-6);
        assert!(room(0) < room(255) && damping(0) > damping(255));
    }
}
