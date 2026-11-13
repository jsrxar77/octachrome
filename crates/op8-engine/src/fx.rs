//! Punch-in FX: momentary effects held on the pads, with intensity from
//! strike velocity and then live pad pressure.
//!
//! Some are audio effects on the master bus; the `Repeat*` ones are
//! sequencer effects (beat repeat) and are handed to the sequencer instead.

use op8_cartridge::SAMPLE_RATE;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PunchFx {
    LowPass,
    HighPass,
    Crush,
    Downsample,
    Gate16,
    Gate8,
    TapeStop,
    Kill,
    /// Beat repeat; payload = window in ticks (96 PPQN).
    Repeat(u32),
    /// Audio stutter; payload = loop length in 1/16ths × 4 (so 2 = 1/32).
    Stutter(u32),
    Echo,
}

/// Pad layout in Punch-in FX mode (pad 0 = bottom-left, pad numbers as
/// silkscreened minus one).
pub const PUNCH_SLOTS: [(PunchFx, &str); 16] = [
    (PunchFx::LowPass, "LPF"),
    (PunchFx::HighPass, "HPF"),
    (PunchFx::Crush, "CRUSH"),
    (PunchFx::Downsample, "DOWNSMP"),
    (PunchFx::Gate16, "GATE16"),
    (PunchFx::Gate8, "GATE8"),
    (PunchFx::TapeStop, "TAPESTOP"),
    (PunchFx::Kill, "KILL"),
    (PunchFx::Repeat(96), "RPT 1/4"),
    (PunchFx::Repeat(48), "RPT 1/8"),
    (PunchFx::Repeat(24), "RPT 1/16"),
    (PunchFx::Repeat(12), "RPT 1/32"),
    (PunchFx::Repeat(16), "RPT 1/16T"),
    (PunchFx::Stutter(4), "STUT 1/16"),
    (PunchFx::Stutter(2), "STUT 1/32"),
    (PunchFx::Echo, "ECHO"),
];

/// Topology-preserving state-variable filter (Simper), stereo.
#[derive(Clone, Copy, Default)]
pub struct Svf {
    ic1: [f32; 2],
    ic2: [f32; 2],
}

impl Svf {
    /// Returns (low, high) for channel `ch`.
    #[inline]
    pub fn tick(&mut self, ch: usize, x: f32, cutoff: f32, q: f32) -> (f32, f32) {
        let g = (std::f32::consts::PI * (cutoff / SAMPLE_RATE).min(0.49)).tan();
        let k = 1.0 / q;
        let a1 = 1.0 / (1.0 + g * (g + k));
        let a2 = g * a1;
        let a3 = g * a2;
        let v3 = x - self.ic2[ch];
        let v1 = a1 * self.ic1[ch] + a2 * v3;
        let v2 = self.ic2[ch] + a2 * self.ic1[ch] + a3 * v3;
        self.ic1[ch] = 2.0 * v1 - self.ic1[ch];
        self.ic2[ch] = 2.0 * v2 - self.ic2[ch];
        let low = v2;
        let high = x - k * v1 - v2;
        (low, high)
    }
}

const CAPTURE: usize = 48_000 * 2; // 2 s

/// The master-bus half of Punch-in FX plus the Smart Strip filter.
pub struct FxRack {
    active: Option<PunchFx>,
    intensity: f32,
    svf: Svf,
    strip_svf: Svf,
    /// -1..0 low-pass sweep, 0 = off, 0..1 high-pass sweep.
    pub strip: f32,
    hold: [f32; 2],
    hold_count: u32,
    capture: Box<[[f32; 2]]>,
    write: usize,
    loop_start: usize,
    read_pos: f64,
    tape_rate: f64,
    echo: Box<[[f32; 2]]>,
    echo_pos: usize,
    gate_phase: f64,
}

impl Default for FxRack {
    fn default() -> Self {
        Self::new()
    }
}

impl FxRack {
    pub fn new() -> Self {
        Self {
            active: None,
            intensity: 0.0,
            svf: Svf::default(),
            strip_svf: Svf::default(),
            strip: 0.0,
            hold: [0.0; 2],
            hold_count: 0,
            capture: vec![[0.0; 2]; CAPTURE].into_boxed_slice(),
            write: 0,
            loop_start: 0,
            read_pos: 0.0,
            tape_rate: 1.0,
            echo: vec![[0.0; 2]; CAPTURE].into_boxed_slice(),
            echo_pos: 0,
            gate_phase: 0.0,
        }
    }

    pub fn active(&self) -> Option<PunchFx> {
        self.active
    }

    /// Engage (or change) an effect.
    pub fn engage(&mut self, fx: Option<PunchFx>, intensity: f32) {
        if fx != self.active {
            self.loop_start = self.write;
            self.read_pos = self.write as f64;
            self.tape_rate = 1.0;
            self.gate_phase = 0.0;
        }
        self.active = fx;
        self.intensity = intensity.clamp(0.0, 1.0);
    }

    pub fn set_intensity(&mut self, v: f32) {
        self.intensity = v.clamp(0.0, 1.0);
    }

    /// Process the master bus in place. `samples_per_16th` comes from the
    /// tempo so gates and stutters stay in time.
    pub fn process(&mut self, l: &mut [f32], r: &mut [f32], samples_per_16th: f64) {
        let it = self.intensity;
        for (sl, sr) in l.iter_mut().zip(r.iter_mut()) {
            let mut x = [*sl, *sr];
            // Smart Strip master filter.
            if self.strip.abs() > 0.01 {
                for (ch, v) in x.iter_mut().enumerate() {
                    if self.strip < 0.0 {
                        let fc = 20_000.0 * (60.0f32 / 20_000.0).powf(-self.strip);
                        *v = self.strip_svf.tick(ch, *v, fc, 1.2).0;
                    } else {
                        let fc = 20.0 * (8_000.0f32 / 20.0).powf(self.strip);
                        *v = self.strip_svf.tick(ch, *v, fc, 1.2).1;
                    }
                }
            }
            self.capture[self.write] = x;
            match self.active {
                None | Some(PunchFx::Repeat(_)) => {}
                Some(PunchFx::LowPass) => {
                    let fc = 18_000.0 * (80.0f32 / 18_000.0).powf(it);
                    for (ch, v) in x.iter_mut().enumerate() {
                        *v = self.svf.tick(ch, *v, fc, 0.7 + it * 4.0).0;
                    }
                }
                Some(PunchFx::HighPass) => {
                    let fc = 30.0 * (6_000.0f32 / 30.0).powf(it);
                    for (ch, v) in x.iter_mut().enumerate() {
                        *v = self.svf.tick(ch, *v, fc, 0.7 + it * 3.0).1;
                    }
                }
                Some(PunchFx::Crush) => {
                    let levels = 2f32.powf(12.0 - it * 10.0);
                    for v in x.iter_mut() {
                        *v = (*v * levels).round() / levels;
                    }
                }
                Some(PunchFx::Downsample) => {
                    let factor = 1 + (it * 31.0) as u32;
                    if self.hold_count == 0 {
                        self.hold = x;
                    }
                    self.hold_count = (self.hold_count + 1) % factor;
                    x = self.hold;
                }
                Some(PunchFx::Gate16) | Some(PunchFx::Gate8) => {
                    let len = if self.active == Some(PunchFx::Gate16) {
                        samples_per_16th
                    } else {
                        samples_per_16th * 2.0
                    };
                    let open = (self.gate_phase / len).fract() < 0.5;
                    self.gate_phase += 1.0;
                    if !open {
                        for v in x.iter_mut() {
                            *v *= 1.0 - it;
                        }
                    }
                }
                Some(PunchFx::TapeStop) => {
                    self.tape_rate =
                        (self.tape_rate - (0.2 + it * 2.0) as f64 / SAMPLE_RATE as f64).max(0.0);
                    self.read_pos += self.tape_rate;
                    x = self.read_capture(self.read_pos);
                }
                Some(PunchFx::Stutter(q)) => {
                    let len = (samples_per_16th * q as f64 / 4.0).max(64.0);
                    let off = (self.write + CAPTURE - self.loop_start) % CAPTURE;
                    let pos = self.loop_start as f64 + (off as f64 % len);
                    let wet = self.read_capture(pos);
                    for (v, w) in x.iter_mut().zip(wet) {
                        *v = *v * (1.0 - it) + w * it.max(0.6);
                    }
                }
                Some(PunchFx::Kill) => x = [0.0; 2],
                Some(PunchFx::Echo) => {}
            }
            // Echo runs always so tails ring out after release.
            let d = (samples_per_16th * 3.0) as usize % CAPTURE;
            let rd = (self.echo_pos + CAPTURE - d.max(1)) % CAPTURE;
            let tap = self.echo[rd];
            let send = if self.active == Some(PunchFx::Echo) {
                0.4 + 0.6 * it
            } else {
                0.0
            };
            for ch in 0..2 {
                self.echo[self.echo_pos][ch] = x[ch] * send + tap[ch] * 0.55;
                x[ch] += tap[ch];
            }
            self.echo_pos = (self.echo_pos + 1) % CAPTURE;
            self.write = (self.write + 1) % CAPTURE;
            *sl = x[0];
            *sr = x[1];
        }
    }

    fn read_capture(&self, pos: f64) -> [f32; 2] {
        let i = pos.floor() as usize % CAPTURE;
        let j = (i + 1) % CAPTURE;
        let f = pos.fract() as f32;
        let (a, b) = (self.capture[i], self.capture[j]);
        [a[0] + (b[0] - a[0]) * f, a[1] + (b[1] - a[1]) * f]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tone(n: usize) -> Vec<f32> {
        (0..n).map(|i| (i as f32 * 0.3).sin() * 0.5).collect()
    }

    #[test]
    fn kill_silences_and_release_restores() {
        let mut fx = FxRack::new();
        let (mut l, mut r) = (tone(256), tone(256));
        fx.engage(Some(PunchFx::Kill), 1.0);
        fx.process(&mut l, &mut r, 6000.0);
        assert!(l.iter().all(|&s| s == 0.0));
        fx.engage(None, 0.0);
        let (mut l, mut r) = (tone(256), tone(256));
        fx.process(&mut l, &mut r, 6000.0);
        assert!(l.iter().any(|&s| s != 0.0));
    }

    #[test]
    fn lowpass_attenuates_highs() {
        let mut fx = FxRack::new();
        let hi: Vec<f32> = (0..4800)
            .map(|i| if i % 2 == 0 { 0.5 } else { -0.5 })
            .collect();
        let (mut l, mut r) = (hi.clone(), hi);
        fx.engage(Some(PunchFx::LowPass), 1.0);
        fx.process(&mut l, &mut r, 6000.0);
        let rms = (l[2400..].iter().map(|s| s * s).sum::<f32>() / 2400.0).sqrt();
        assert!(rms < 0.02, "{rms}");
    }

    #[test]
    fn all_effects_stay_finite() {
        for (fx_kind, _) in PUNCH_SLOTS {
            let mut fx = FxRack::new();
            fx.strip = -0.7;
            fx.engage(Some(fx_kind), 1.0);
            for _ in 0..50 {
                let (mut l, mut r) = (tone(512), tone(512));
                fx.process(&mut l, &mut r, 6000.0);
                assert!(
                    l.iter().chain(&r).all(|s| s.is_finite() && s.abs() < 4.0),
                    "{fx_kind:?}"
                );
            }
        }
    }
}
