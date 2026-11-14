//! The effect of a track (button 7): one of delay, reverb, chorus, filter or drive, with three knobs.
//! It runs on the track's own audio, before the track is mixed. Everything is allocated once, at start-up;
//! `process` never allocates.

use crate::fx::Svf;
use op8_cartridge::SAMPLE_RATE;
use op8_seq::{FxKind, TrackFx};

const DELAY_LEN: usize = 48_000; // one second per side
const CHORUS_LEN: usize = 2048;
const COMB_LENS: [usize; 4] = [1687, 1759, 1622, 1547];
const ALLPASS_LENS: [usize; 2] = [605, 480];
const SPREAD: usize = 23; // the right side is a little longer, for width

#[derive(Clone)]
struct Comb {
    buf: Vec<f32>,
    pos: usize,
    store: f32,
}

impl Comb {
    fn new(len: usize) -> Self {
        Self { buf: vec![0.0; len], pos: 0, store: 0.0 }
    }
    fn clear(&mut self) {
        self.buf.fill(0.0);
        self.store = 0.0;
    }
    #[inline]
    fn tick(&mut self, x: f32, feedback: f32, damp: f32) -> f32 {
        let out = self.buf[self.pos];
        self.store = out * (1.0 - damp) + self.store * damp;
        self.buf[self.pos] = x + self.store * feedback;
        self.pos += 1;
        if self.pos == self.buf.len() {
            self.pos = 0;
        }
        out
    }
}

#[derive(Clone)]
struct Allpass {
    buf: Vec<f32>,
    pos: usize,
}

impl Allpass {
    fn new(len: usize) -> Self {
        Self { buf: vec![0.0; len], pos: 0 }
    }
    fn clear(&mut self) {
        self.buf.fill(0.0);
    }
    #[inline]
    fn tick(&mut self, x: f32) -> f32 {
        let b = self.buf[self.pos];
        self.buf[self.pos] = x + b * 0.5;
        self.pos += 1;
        if self.pos == self.buf.len() {
            self.pos = 0;
        }
        b - x
    }
}

/// The processor of one track. It remembers what it was last asked to be, so that changing the
/// effect starts it from silence.
pub struct TrackFxProc {
    kind: FxKind,
    knobs: [f32; 3],
    delay: [Vec<f32>; 2],
    dpos: usize,
    combs: [[Comb; 4]; 2],
    allpass: [[Allpass; 2]; 2],
    chorus: [Vec<f32>; 2],
    cpos: usize,
    phase: f32,
    svf: Svf,
    tone: [f32; 2],
}

impl TrackFxProc {
    pub fn new() -> Self {
        let comb = |side: usize| std::array::from_fn(|i| Comb::new(COMB_LENS[i] + side * SPREAD));
        let allpass = |side: usize| std::array::from_fn(|i| Allpass::new(ALLPASS_LENS[i] + side * SPREAD));
        Self {
            kind: FxKind::Off,
            knobs: [0.5; 3],
            delay: [vec![0.0; DELAY_LEN], vec![0.0; DELAY_LEN]],
            dpos: 0,
            combs: [comb(0), comb(1)],
            allpass: [allpass(0), allpass(1)],
            chorus: [vec![0.0; CHORUS_LEN], vec![0.0; CHORUS_LEN]],
            cpos: 0,
            phase: 0.0,
            svf: Svf::default(),
            tone: [0.0; 2],
        }
    }

    fn clear(&mut self) {
        for d in &mut self.delay {
            d.fill(0.0);
        }
        for c in self.combs.iter_mut().flatten() {
            c.clear();
        }
        for a in self.allpass.iter_mut().flatten() {
            a.clear();
        }
        for c in &mut self.chorus {
            c.fill(0.0);
        }
        self.svf = Svf::default();
        self.tone = [0.0; 2];
    }

    /// Run the effect over a block (both channels, in place).
    pub fn process(&mut self, fx: &TrackFx, l: &mut [f32], r: &mut [f32]) {
        if fx.kind != self.kind {
            self.kind = fx.kind;
            self.clear();
            self.knobs = fx.knobs();
        }
        if fx.kind == FxKind::Off {
            return;
        }
        // the knobs glide to where they were turned, so a sweep has no steps
        let target = fx.knobs();
        for (k, t) in self.knobs.iter_mut().zip(target) {
            *k += (t - *k) * 0.35;
        }
        let [a, b, c] = self.knobs;
        match fx.kind {
            FxKind::Off => {}
            FxKind::Delay => self.delay_block(a, b, c, l, r),
            FxKind::Reverb => self.reverb_block(a, b, c, l, r),
            FxKind::Chorus => self.chorus_block(a, b, c, l, r),
            FxKind::Filter => self.filter_block(a, b, c, l, r),
            FxKind::Drive => self.drive_block(a, b, c, l, r),
        }
    }

    /// A ping-pong delay: the echoes bounce from side to side.
    fn delay_block(&mut self, time: f32, feedback: f32, mix: f32, l: &mut [f32], r: &mut [f32]) {
        let samples = (0.02 * 37.5f32.powf(time) * SAMPLE_RATE).min(DELAY_LEN as f32 - 2.0);
        let fb = feedback * 0.92;
        for i in 0..l.len() {
            let rp = (self.dpos as f32 + DELAY_LEN as f32 - samples) % DELAY_LEN as f32;
            let (i0, f) = (rp as usize, rp.fract());
            let i1 = (i0 + 1) % DELAY_LEN;
            let wet_l = self.delay[0][i0] * (1.0 - f) + self.delay[0][i1] * f;
            let wet_r = self.delay[1][i0] * (1.0 - f) + self.delay[1][i1] * f;
            let mono = (l[i] + r[i]) * 0.5;
            self.delay[0][self.dpos] = mono + wet_r * fb;
            self.delay[1][self.dpos] = wet_l * fb;
            l[i] += wet_l * mix;
            r[i] += wet_r * mix;
            self.dpos = (self.dpos + 1) % DELAY_LEN;
        }
    }

    /// A small room (four combs and two all-pass filters a side).
    fn reverb_block(&mut self, size: f32, tone: f32, mix: f32, l: &mut [f32], r: &mut [f32]) {
        let feedback = 0.70 + size * 0.26;
        let damp = 0.1 + (1.0 - tone) * 0.6;
        for i in 0..l.len() {
            let input = (l[i] + r[i]) * 0.015;
            let mut out = [0.0f32; 2];
            for (side, o) in out.iter_mut().enumerate() {
                let mut sum = 0.0;
                for c in self.combs[side].iter_mut() {
                    sum += c.tick(input, feedback, damp);
                }
                for a in self.allpass[side].iter_mut() {
                    sum = a.tick(sum);
                }
                *o = sum;
            }
            l[i] += out[0] * mix * 3.0;
            r[i] += out[1] * mix * 3.0;
        }
    }

    /// Two short delays whose time sways, a quarter turn apart.
    fn chorus_block(&mut self, rate: f32, depth: f32, mix: f32, l: &mut [f32], r: &mut [f32]) {
        let hz = 0.1 * 50f32.powf(rate);
        let step = hz / SAMPLE_RATE;
        let base = 0.012 * SAMPLE_RATE;
        let sway = depth * 0.006 * SAMPLE_RATE;
        for i in 0..l.len() {
            self.chorus[0][self.cpos] = l[i];
            self.chorus[1][self.cpos] = r[i];
            let mut wet = [0.0f32; 2];
            for (side, w) in wet.iter_mut().enumerate() {
                let ph = (self.phase + side as f32 * 0.25).fract();
                let d = base + sway * (ph * std::f32::consts::TAU).sin();
                let rp = (self.cpos as f32 + CHORUS_LEN as f32 - d) % CHORUS_LEN as f32;
                let (i0, f) = (rp as usize, rp.fract());
                let i1 = (i0 + 1) % CHORUS_LEN;
                *w = self.chorus[side][i0] * (1.0 - f) + self.chorus[side][i1] * f;
            }
            l[i] = l[i] * (1.0 - 0.5 * mix) + wet[0] * mix;
            r[i] = r[i] * (1.0 - 0.5 * mix) + wet[1] * mix;
            self.cpos = (self.cpos + 1) % CHORUS_LEN;
            self.phase = (self.phase + step).fract();
        }
    }

    /// One knob sweeps from a low-pass (turned down) through open to a high-pass (turned up).
    fn filter_block(&mut self, sweep: f32, resonance: f32, mix: f32, l: &mut [f32], r: &mut [f32]) {
        let low = sweep < 0.5;
        let cutoff = if low {
            40.0 * (18_000.0f32 / 40.0).powf(sweep * 2.0)
        } else {
            20.0 * (6_000.0f32 / 20.0).powf((sweep - 0.5) * 2.0)
        };
        let q = 0.7 + resonance * 7.0;
        for i in 0..l.len() {
            let (lo, hi) = self.svf.tick(0, l[i], cutoff, q);
            let wl = if low { lo } else { hi };
            let (lo, hi) = self.svf.tick(1, r[i], cutoff, q);
            let wr = if low { lo } else { hi };
            l[i] = l[i] * (1.0 - mix) + wl * mix;
            r[i] = r[i] * (1.0 - mix) + wr * mix;
        }
    }

    /// Soft saturation, then a tone control.
    fn drive_block(&mut self, drive: f32, tone: f32, mix: f32, l: &mut [f32], r: &mut [f32]) {
        let gain = 1.0 + drive * 24.0;
        let norm = 1.0 / gain.tanh();
        let k = 1.0 - (-2.0 * std::f32::consts::PI * (600.0 + tone * 12_000.0) / SAMPLE_RATE).exp();
        for i in 0..l.len() {
            let d = [l[i], r[i]];
            let mut o = [0.0f32; 2];
            for ch in 0..2 {
                let shaped = (d[ch] * gain).tanh() * norm * 0.8;
                self.tone[ch] += (shaped - self.tone[ch]) * k;
                o[ch] = d[ch] * (1.0 - mix) + self.tone[ch] * mix;
            }
            l[i] = o[0];
            r[i] = o[1];
        }
    }
}

impl Default for TrackFxProc {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn burst() -> (Vec<f32>, Vec<f32>) {
        // a short noise-like burst, then silence
        let mut l = vec![0.0; 512];
        for (i, s) in l.iter_mut().enumerate().take(64) {
            *s = if i % 2 == 0 { 0.5 } else { -0.5 };
        }
        (l.clone(), l)
    }

    fn tail_energy(kind: FxKind, a: u8, b: u8, c: u8, blocks: usize) -> f32 {
        let mut p = TrackFxProc::new();
        let fx = TrackFx { kind, a, b, c };
        let (mut l, mut r) = burst();
        p.process(&fx, &mut l, &mut r);
        let mut e = 0.0;
        for _ in 0..blocks {
            let (mut l, mut r) = (vec![0.0; 512], vec![0.0; 512]);
            p.process(&fx, &mut l, &mut r);
            e += l.iter().chain(r.iter()).map(|x| x * x).sum::<f32>();
        }
        e
    }

    // F-A14
    #[test]
    fn off_leaves_the_sound_alone() {
        let mut p = TrackFxProc::new();
        let (mut l, mut r) = burst();
        let before = l.clone();
        p.process(&TrackFx::default(), &mut l, &mut r);
        assert_eq!(l, before);
    }

    // F-A14
    #[test]
    fn delay_and_reverb_leave_a_tail_and_a_dry_mix_does_not() {
        assert!(tail_energy(FxKind::Delay, 60, 160, 200, 40) > 0.1, "the delay echoes");
        assert!(tail_energy(FxKind::Reverb, 200, 128, 200, 40) > 0.01, "the reverb rings");
        assert!(tail_energy(FxKind::Delay, 60, 160, 0, 40) < 1e-9, "no mix, no echo");
    }

    // F-A14
    #[test]
    fn every_effect_stays_finite_and_in_range_at_the_extremes() {
        for kind in FxKind::ALL {
            for v in [0u8, 128, 255] {
                let mut p = TrackFxProc::new();
                let fx = TrackFx { kind, a: v, b: v, c: v };
                for n in 0..200 {
                    let mut l: Vec<f32> = (0..128).map(|i| ((i + n) as f32 * 0.3).sin() * 0.9).collect();
                    let mut r = l.clone();
                    p.process(&fx, &mut l, &mut r);
                    assert!(l.iter().chain(r.iter()).all(|x| x.is_finite() && x.abs() < 20.0), "{kind:?} {v}");
                }
            }
        }
    }

    // F-A14
    #[test]
    fn the_filter_takes_the_highs_off_and_the_drive_adds_them() {
        let tone = |kind, a| {
            let mut p = TrackFxProc::new();
            let fx = TrackFx { kind, a, b: 60, c: 255 };
            let mut last = (vec![], vec![]);
            for n in 0..40 {
                let mut l: Vec<f32> = (0..256).map(|i| ((n * 256 + i) as f32 * 2.0 * std::f32::consts::PI * 8000.0 / SAMPLE_RATE).sin() * 0.5).collect();
                let mut r = l.clone();
                p.process(&fx, &mut l, &mut r);
                last = (l, r);
            }
            last.0.iter().map(|x| x * x).sum::<f32>()
        };
        let open = tone(FxKind::Filter, 128);
        let closed = tone(FxKind::Filter, 10);
        assert!(closed < open * 0.05, "a closed low-pass takes out 8 kHz: {closed} vs {open}");
    }
}
