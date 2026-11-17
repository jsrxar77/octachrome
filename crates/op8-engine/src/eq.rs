//! The EQ on the master: a low shelf at 100 Hz, a band of the middle that can be moved from 200 Hz to 5 kHz, and a high shelf at 8 kHz,
//! each of −15 to +15 dB, as the classic biquads (the ones of the "Audio EQ Cookbook"), the same on both channels.
//!
//! The settings are the project's (`op8_seq::MasterEq`, each 0..=255); the functions here say what they mean. 128 is no gain, so the
//! defaults do nothing: with the three gains at zero the sound is not touched at all.

use op8_seq::MasterEq;

const LOW_HZ: f32 = 100.0;
const HIGH_HZ: f32 = 8_000.0;
const MID_Q: f32 = 0.9;

/// The gain of a band in dB: −15 to +15, and exactly 0 at 128.
pub fn gain_db(v: u8) -> f32 {
    ((v as f32 - 128.0) / 127.0 * 15.0).clamp(-15.0, 15.0)
}
/// The frequency of the middle band: 200 Hz to 5 kHz, round a kilohertz in the middle of the knob.
pub fn freq_hz(v: u8) -> f32 {
    200.0 * 25f32.powf(v as f32 / 255.0)
}

/// One biquad's coefficients, already divided by `a0`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Coef {
    b0: f32,
    b1: f32,
    b2: f32,
    a1: f32,
    a2: f32,
}

impl Coef {
    const FLAT: Coef = Coef { b0: 1.0, b1: 0.0, b2: 0.0, a1: 0.0, a2: 0.0 };

    fn new(b: [f32; 3], a: [f32; 3]) -> Self {
        Self { b0: b[0] / a[0], b1: b[1] / a[0], b2: b[2] / a[0], a1: a[1] / a[0], a2: a[2] / a[0] }
    }

    /// A peak or dip of `db` at `hz`.
    pub fn peaking(hz: f32, q: f32, db: f32, rate: f32) -> Self {
        let (a, w) = (10f32.powf(db / 40.0), std::f32::consts::TAU * hz / rate);
        let (s, c) = w.sin_cos();
        let alpha = s / (2.0 * q);
        Self::new([1.0 + alpha * a, -2.0 * c, 1.0 - alpha * a], [1.0 + alpha / a, -2.0 * c, 1.0 - alpha / a])
    }

    /// A shelf of `db` below `hz` (slope 1).
    pub fn low_shelf(hz: f32, db: f32, rate: f32) -> Self {
        let (a, w) = (10f32.powf(db / 40.0), std::f32::consts::TAU * hz / rate);
        let (s, c) = w.sin_cos();
        let t = 2.0 * a.sqrt() * (s / 2.0 * 2f32.sqrt());
        Self::new(
            [a * ((a + 1.0) - (a - 1.0) * c + t), 2.0 * a * ((a - 1.0) - (a + 1.0) * c), a * ((a + 1.0) - (a - 1.0) * c - t)],
            [(a + 1.0) + (a - 1.0) * c + t, -2.0 * ((a - 1.0) + (a + 1.0) * c), (a + 1.0) + (a - 1.0) * c - t],
        )
    }

    /// A shelf of `db` above `hz` (slope 1).
    pub fn high_shelf(hz: f32, db: f32, rate: f32) -> Self {
        let (a, w) = (10f32.powf(db / 40.0), std::f32::consts::TAU * hz / rate);
        let (s, c) = w.sin_cos();
        let t = 2.0 * a.sqrt() * (s / 2.0 * 2f32.sqrt());
        Self::new(
            [a * ((a + 1.0) + (a - 1.0) * c + t), -2.0 * a * ((a - 1.0) + (a + 1.0) * c), a * ((a + 1.0) + (a - 1.0) * c - t)],
            [(a + 1.0) - (a - 1.0) * c + t, 2.0 * ((a - 1.0) - (a + 1.0) * c), (a + 1.0) - (a - 1.0) * c - t],
        )
    }

    /// The gain of this filter at `hz`, in dB.
    pub fn response_db(&self, hz: f32, rate: f32) -> f32 {
        let w = std::f32::consts::TAU * hz / rate;
        let (c1, s1, c2, s2) = (w.cos(), w.sin(), (2.0 * w).cos(), (2.0 * w).sin());
        let num = ((self.b0 + self.b1 * c1 + self.b2 * c2).powi(2) + (self.b1 * s1 + self.b2 * s2).powi(2)).max(1e-20);
        let den = ((1.0 + self.a1 * c1 + self.a2 * c2).powi(2) + (self.a1 * s1 + self.a2 * s2).powi(2)).max(1e-20);
        10.0 * (num / den).log10()
    }
}

/// The three filters the settings ask for, at `rate`.
pub fn coefficients(s: &MasterEq, rate: f32) -> [Coef; 3] {
    [
        Coef::low_shelf(LOW_HZ, gain_db(s.low), rate),
        Coef::peaking(freq_hz(s.freq), MID_Q, gain_db(s.mid), rate),
        Coef::high_shelf(HIGH_HZ, gain_db(s.high), rate),
    ]
}

/// The gain of the whole EQ at `hz`, in dB: what the right screen draws as the curve.
pub fn response_db(s: &MasterEq, hz: f32, rate: f32) -> f32 {
    coefficients(s, rate).iter().map(|c| c.response_db(hz, rate)).sum()
}

/// No gain on any band: the EQ does nothing.
pub fn is_flat(s: &MasterEq) -> bool {
    gain_db(s.low) == 0.0 && gain_db(s.mid) == 0.0 && gain_db(s.high) == 0.0
}

/// One biquad's memory (transposed direct form II), per channel.
#[derive(Clone, Copy, Default)]
struct State {
    z1: f32,
    z2: f32,
}

impl State {
    #[inline]
    fn tick(&mut self, c: &Coef, x: f32) -> f32 {
        let y = c.b0 * x + self.z1;
        self.z1 = c.b1 * x - c.a1 * y + self.z2;
        self.z2 = c.b2 * x - c.a2 * y;
        y
    }
}

/// The EQ between blocks: the settings it was last given and the memory of its filters.
pub struct Eq {
    rate: f32,
    last: MasterEq,
    coef: [Coef; 3],
    state: [[State; 3]; 2],
}

impl Eq {
    pub fn new(rate: f32) -> Self {
        let last = MasterEq::default();
        Self { rate, last, coef: coefficients(&last, rate), state: [[State::default(); 3]; 2] }
    }

    /// Filter the block in place. With no gain on any band the sound passes untouched.
    pub fn process(&mut self, l: &mut [f32], r: &mut [f32], s: &MasterEq) {
        if is_flat(s) {
            // forget what the filters held, so that the next time starts clean
            self.state = [[State::default(); 3]; 2];
            self.last = *s;
            return;
        }
        if *s != self.last {
            self.coef = coefficients(s, self.rate);
            self.last = *s;
        }
        for (ch, buf) in [l, r].into_iter().enumerate() {
            let st = &mut self.state[ch];
            for x in buf.iter_mut() {
                let mut y = *x;
                for k in 0..3 {
                    y = st[k].tick(&self.coef[k], y);
                }
                *x = y;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const RATE: f32 = 48_000.0;

    fn tone(hz: f32, n: usize) -> Vec<f32> {
        (0..n).map(|i| (i as f32 / RATE * hz * std::f32::consts::TAU).sin() * 0.3).collect()
    }
    /// How much `eq` changes a tone of `hz`, in dB, measured on the audio once it has settled.
    fn measured(s: &MasterEq, hz: f32) -> f32 {
        let mut e = Eq::new(RATE);
        let (mut l, mut r) = (tone(hz, 24_000), tone(hz, 24_000));
        let before = tone(hz, 24_000);
        e.process(&mut l, &mut r, s);
        let rms = |v: &[f32]| (v[12_000..].iter().map(|x| x * x).sum::<f32>() / 12_000.0).sqrt();
        20.0 * (rms(&l) / rms(&before)).log10()
    }
    fn eq(low: u8, mid: u8, freq: u8, high: u8) -> MasterEq {
        MasterEq { low, mid, freq, high }
    }

    // F-A28
    #[test]
    fn the_defaults_do_nothing() {
        assert!(is_flat(&MasterEq::default()));
        assert_eq!(gain_db(128), 0.0);
        let mut e = Eq::new(RATE);
        let (mut l, mut r) = (tone(440.0, 4800), tone(3000.0, 4800));
        let (bl, br) = (l.clone(), r.clone());
        e.process(&mut l, &mut r, &MasterEq::default());
        assert_eq!((l, r), (bl, br), "the sound passes untouched");
        assert!((freq_hz(128) - 1000.0).abs() < 20.0, "the middle of the knob is about a kilohertz: {}", freq_hz(128));
        assert!((gain_db(255) - 15.0).abs() < 1e-4 && (gain_db(0) + 15.0).abs() < 0.2, "{} {}", gain_db(255), gain_db(0));
    }

    // F-A28
    #[test]
    fn each_band_changes_its_own_part_of_the_sound() {
        // +12 dB of low: the bass goes up by 12, the middle and the treble stay
        let low = eq(128 + 102, 128, 128, 128); // about +12 dB
        let g = gain_db(low.low);
        assert!((measured(&low, 30.0) - g).abs() < 1.0, "bass: {} dB for {g}", measured(&low, 30.0));
        assert!(measured(&low, 2_000.0).abs() < 0.6, "the middle is left: {}", measured(&low, 2_000.0));
        assert!(measured(&low, 12_000.0).abs() < 0.3, "and the treble");
        // the same for the high shelf, upside down
        let high = eq(128, 128, 128, 128 - 102); // about −12 dB
        let h = gain_db(high.high);
        assert!((measured(&high, 16_000.0) - h).abs() < 1.5, "treble: {} dB for {h}", measured(&high, 16_000.0));
        assert!(measured(&high, 200.0).abs() < 0.3, "the bass is left");
        // the middle band is at its frequency, and the frequency knob moves it
        let mid = eq(128, 128 + 102, 128, 128);
        let m = gain_db(mid.mid);
        assert!((measured(&mid, freq_hz(128)) - m).abs() < 0.6, "at its centre: {}", measured(&mid, freq_hz(128)));
        assert!(measured(&mid, 60.0).abs() < 1.0 && measured(&mid, 15_000.0).abs() < 1.0, "far from it, nothing");
        let moved = eq(128, 128 + 102, 220, 128); // up to about 3.3 kHz
        assert!(measured(&moved, freq_hz(220)) > measured(&moved, 1_000.0) + 4.0, "the band went up with the knob");
    }

    // F-A28
    #[test]
    fn the_curve_the_screen_draws_is_what_the_audio_does() {
        for s in [eq(30, 200, 90, 220), eq(250, 20, 200, 60), eq(128, 250, 40, 128)] {
            for hz in [40.0, 120.0, 500.0, 2_000.0, 6_000.0, 14_000.0] {
                let (drawn, heard) = (response_db(&s, hz, RATE), measured(&s, hz));
                assert!((drawn - heard).abs() < 0.8, "{hz} Hz: drawn {drawn}, heard {heard}");
            }
        }
    }

    // F-A28
    #[test]
    fn going_back_to_flat_leaves_nothing_behind() {
        let mut e = Eq::new(RATE);
        let hot = eq(255, 255, 128, 255);
        let (mut l, mut r) = (tone(100.0, 9600), tone(100.0, 9600));
        e.process(&mut l, &mut r, &hot);
        let (mut l, mut r) = (tone(440.0, 4800), tone(440.0, 4800));
        let before = l.clone();
        e.process(&mut l, &mut r, &MasterEq::default());
        assert_eq!(l, before, "flat again: untouched, and no tail of the filters");
    }
}
