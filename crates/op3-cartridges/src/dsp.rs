//! Minimal DSP building blocks shared by the cartridges.

use op3_cartridge::SAMPLE_RATE;
use std::f32::consts::TAU;

/// Coefficient for a one-pole exponential segment of `ms` milliseconds
/// (time to fall to ~1/e... scaled so it reads as the audible length).
pub fn decay_coef(ms: f32) -> f32 {
    let samples = (ms.max(0.1) / 1000.0) * SAMPLE_RATE / 6.9; // -60 dB at `ms`
    (-1.0 / samples).exp()
}

#[inline]
pub fn sine(phase: f32) -> f32 {
    (phase * TAU).sin()
}

#[inline]
pub fn midi_to_hz(note: f32) -> f32 {
    440.0 * 2f32.powf((note - 69.0) / 12.0)
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Stage {
    Idle,
    Attack,
    Decay,
    Sustain,
    Release,
}

/// Linear-attack, exponential decay/release ADSR.
#[derive(Clone, Copy, Debug)]
pub struct Adsr {
    pub attack_ms: f32,
    pub decay_ms: f32,
    pub sustain: f32,
    pub release_ms: f32,
    stage: Stage,
    level: f32,
}

impl Adsr {
    pub fn new(attack_ms: f32, decay_ms: f32, sustain: f32, release_ms: f32) -> Self {
        Self {
            attack_ms,
            decay_ms,
            sustain,
            release_ms,
            stage: Stage::Idle,
            level: 0.0,
        }
    }
    pub fn gate_on(&mut self) {
        self.stage = Stage::Attack;
    }
    pub fn gate_off(&mut self) {
        if self.stage != Stage::Idle {
            self.stage = Stage::Release;
        }
    }
    pub fn kill(&mut self) {
        self.stage = Stage::Idle;
        self.level = 0.0;
    }
    /// Current envelope output, 0..=1.
    pub fn level(&self) -> f32 {
        self.level
    }
    pub fn is_active(&self) -> bool {
        self.stage != Stage::Idle
    }
    pub fn stage(&self) -> Stage {
        self.stage
    }
    #[inline]
    pub fn next(&mut self) -> f32 {
        match self.stage {
            Stage::Idle => {}
            Stage::Attack => {
                self.level += 1.0 / (self.attack_ms.max(0.1) / 1000.0 * SAMPLE_RATE);
                if self.level >= 1.0 {
                    self.level = 1.0;
                    self.stage = Stage::Decay;
                }
            }
            Stage::Decay => {
                let c = decay_coef(self.decay_ms);
                self.level = self.sustain + (self.level - self.sustain) * c;
                if (self.level - self.sustain).abs() < 1e-4 {
                    self.stage = Stage::Sustain;
                }
            }
            Stage::Sustain => self.level = self.sustain,
            Stage::Release => {
                self.level *= decay_coef(self.release_ms);
                if self.level < 1e-5 {
                    self.kill();
                }
            }
        }
        self.level
    }
}

/// Fixed ring of recent output samples for the oscilloscope.
#[derive(Clone)]
pub struct Scope {
    buf: [f32; Self::LEN],
    pos: usize,
    decim: usize,
    count: usize,
}

impl Default for Scope {
    fn default() -> Self {
        Self::new(2)
    }
}

impl Scope {
    pub const LEN: usize = 480;

    /// Keep one sample in `decim`.
    pub fn new(decim: usize) -> Self {
        Self {
            buf: [0.0; Self::LEN],
            pos: 0,
            decim: decim.max(1),
            count: 0,
        }
    }
    #[inline]
    pub fn push(&mut self, s: f32) {
        self.count += 1;
        if self.count >= self.decim {
            self.count = 0;
            self.buf[self.pos] = s;
            self.pos = (self.pos + 1) % Self::LEN;
        }
    }
    /// Oldest to newest, starting at a rising zero crossing when there is
    /// one, so periodic signals stand still on screen.
    pub fn samples(&self) -> impl Iterator<Item = f32> + '_ {
        let mut start = self.pos;
        for i in 0..Self::LEN / 2 {
            let a = self.buf[(self.pos + i) % Self::LEN];
            let b = self.buf[(self.pos + i + 1) % Self::LEN];
            if a <= 0.0 && b > 0.0 {
                start = self.pos + i + 1;
                break;
            }
        }
        (0..Self::LEN / 2 + Self::LEN / 4).map(move |i| self.buf[(start + i) % Self::LEN])
    }
}
