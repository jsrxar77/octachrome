//! The compressor on the master: a feed-forward compressor that listens to the louder of the two channels (so the image does not
//! shift), with a soft knee, an attack and a release that move the gain in decibels, a make-up gain and a dry/wet mix.
//!
//! The six settings are the project's (`op8_seq::MasterComp`, each 0..=255); the functions here say what they mean. The defaults do
//! nothing: the threshold is at 0 dB.

use op8_seq::MasterComp;

const KNEE_DB: f32 = 6.0;

/// The level above which the sound is brought down: −40 dB to 0 dB.
pub fn threshold_db(v: u8) -> f32 {
    -40.0 + 40.0 * v as f32 / 255.0
}
/// How much a level over the threshold is brought down: 1:1 (nothing) to 20:1 (a limiter), finer at the low end.
pub fn ratio(v: u8) -> f32 {
    let u = v as f32 / 255.0;
    1.0 + 19.0 * u * u
}
/// How fast it takes hold, 0.1 ms to 100 ms.
pub fn attack_ms(v: u8) -> f32 {
    0.1 * 1000f32.powf(v as f32 / 255.0)
}
/// How fast it lets go, 20 ms to 1 s.
pub fn release_ms(v: u8) -> f32 {
    20.0 * 50f32.powf(v as f32 / 255.0)
}
/// The gain given back after the reduction: 0 to +24 dB.
pub fn makeup_db(v: u8) -> f32 {
    24.0 * v as f32 / 255.0
}
/// How much of the compressed sound is in the output, 0 to 1 (the rest is the sound as it was).
pub fn mix(v: u8) -> f32 {
    v as f32 / 255.0
}

/// By how many decibels the compressor brings down a level of `in_db` (0 or less): the curve the right screen draws.
pub fn reduction_db(in_db: f32, threshold_db: f32, ratio: f32) -> f32 {
    let over = in_db - threshold_db;
    let out = if 2.0 * over < -KNEE_DB {
        in_db
    } else if 2.0 * over.abs() <= KNEE_DB {
        in_db + (1.0 / ratio - 1.0) * (over + KNEE_DB / 2.0).powi(2) / (2.0 * KNEE_DB)
    } else {
        threshold_db + over / ratio
    };
    out - in_db
}

/// The state of the compressor between blocks.
pub struct Compressor {
    sample_rate: f32,
    /// The reduction now, in dB (0 or less), smoothed.
    gr: f32,
    /// The deepest reduction and the loudest input of the last block, for the screen.
    pub block_gr: f32,
    pub block_in_db: f32,
}

impl Compressor {
    pub fn new(sample_rate: f32) -> Self {
        Self { sample_rate, gr: 0.0, block_gr: 0.0, block_in_db: -90.0 }
    }

    /// Compress the block in place. A threshold of 0 dB with no make-up gain does nothing at all (the sound passes untouched).
    pub fn process(&mut self, l: &mut [f32], r: &mut [f32], s: &MasterComp) {
        let (thr, ratio, makeup, mix) = (threshold_db(s.threshold), ratio(s.ratio), makeup_db(s.makeup), mix(s.mix));
        self.block_gr = 0.0;
        self.block_in_db = -90.0;
        if s.threshold == 255 && s.makeup == 0 {
            self.gr = 0.0;
            return;
        }
        let coef = |ms: f32| 1.0 - (-1.0 / (ms * 0.001 * self.sample_rate)).exp();
        let (atk, rel) = (coef(attack_ms(s.attack)), coef(release_ms(s.release)));
        for (sl, sr) in l.iter_mut().zip(r.iter_mut()) {
            let level = sl.abs().max(sr.abs()).max(1e-6);
            let in_db = 20.0 * level.log10();
            let target = reduction_db(in_db, thr, ratio);
            // more reduction takes hold at the attack, less lets go at the release
            let c = if target < self.gr { atk } else { rel };
            self.gr += (target - self.gr) * c;
            let gain = 10f32.powf((self.gr + makeup) / 20.0);
            *sl = *sl * (1.0 - mix) + *sl * gain * mix;
            *sr = *sr * (1.0 - mix) + *sr * gain * mix;
            self.block_gr = self.block_gr.min(self.gr);
            self.block_in_db = self.block_in_db.max(in_db);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tone(amp: f32, n: usize) -> Vec<f32> {
        (0..n).map(|i| (i as f32 * 0.1).sin() * amp).collect()
    }
    fn rms(v: &[f32]) -> f32 {
        (v.iter().map(|x| x * x).sum::<f32>() / v.len() as f32).sqrt()
    }
    fn run(c: &mut Compressor, s: &MasterComp, amp: f32) -> f32 {
        let (mut l, mut r) = (tone(amp, 48_000), tone(amp, 48_000));
        c.process(&mut l, &mut r, s);
        rms(&l[24_000..]) / rms(&tone(amp, 48_000)[24_000..])
    }

    // F-A27
    #[test]
    fn the_defaults_do_nothing() {
        let mut c = Compressor::new(48_000.0);
        let (mut l, mut r) = (tone(0.9, 4800), tone(0.9, 4800));
        let before = l.clone();
        c.process(&mut l, &mut r, &MasterComp::default());
        assert_eq!(l, before, "the sound passes untouched");
        assert_eq!(c.block_gr, 0.0);
    }

    // F-A27
    #[test]
    fn a_loud_sound_is_brought_down_by_the_ratio_and_a_quiet_one_is_left() {
        let mut s = MasterComp::default();
        s.threshold = 128; // about -20 dB
        s.ratio = 200; // about 12:1
        s.attack = 0; // as fast as it goes
        let mut c = Compressor::new(48_000.0);
        // a peak of 0 dB is 20 dB over: it comes out near threshold + 20 / ratio, which is a loss of about 18 dB
        let loud = run(&mut c, &s, 1.0);
        let lost_db = 20.0 * loud.log10();
        assert!((-20.0..-12.0).contains(&lost_db), "a loud sound is down by {lost_db} dB");
        assert!(c.block_gr < -10.0, "and the screen can tell: {}", c.block_gr);
        let mut c2 = Compressor::new(48_000.0);
        let quiet = run(&mut c2, &s, 0.01); // -40 dB: far under
        assert!((20.0 * quiet.log10()).abs() < 0.3, "a quiet one is left as it is: {}", 20.0 * quiet.log10());
    }

    // F-A27
    #[test]
    fn make_up_and_mix_do_what_they_say() {
        let mut s = MasterComp::default();
        s.threshold = 128;
        s.ratio = 200;
        s.attack = 0;
        s.makeup = 85; // +8 dB
        let mut c = Compressor::new(48_000.0);
        let with = 20.0 * run(&mut c, &s, 1.0).log10();
        s.makeup = 0;
        let mut c = Compressor::new(48_000.0);
        let without = 20.0 * run(&mut c, &s, 1.0).log10();
        assert!((with - without - 8.0).abs() < 0.2, "{with} against {without}");
        s.mix = 0; // all dry
        let mut c = Compressor::new(48_000.0);
        assert!((run(&mut c, &s, 1.0) - 1.0).abs() < 1e-4, "no mix is the sound as it was");
        s.mix = 128; // half: between the two
        let mut c = Compressor::new(48_000.0);
        let half = run(&mut c, &s, 1.0);
        assert!(half > 10f32.powf(without / 20.0) && half < 1.0, "{half}");
    }

    // F-A27
    #[test]
    fn the_attack_lets_the_start_through_and_the_release_lets_go() {
        let mut s = MasterComp::default();
        s.threshold = 100;
        s.ratio = 255;
        s.attack = 200; // slow: tens of ms
        s.release = 128;
        let mut c = Compressor::new(48_000.0);
        let mut l = tone(0.9, 4800);
        let mut r = l.clone();
        c.process(&mut l[..480].to_vec(), &mut r[..480].to_vec(), &s);
        let early = c.block_gr;
        let (mut l2, mut r2) = (tone(0.9, 24_000), tone(0.9, 24_000));
        c.process(&mut l2, &mut r2, &s);
        assert!(c.block_gr < early - 3.0, "it takes hold over time: {early} then {}", c.block_gr);
        // silence lets it go
        let (mut a, mut b) = (vec![0.0; 48_000], vec![0.0; 48_000]);
        c.process(&mut a, &mut b, &s);
        let (mut a, mut b) = (vec![0.0; 480], vec![0.0; 480]);
        c.process(&mut a, &mut b, &s);
        assert!(c.gr > -0.5, "it has let go: {}", c.gr);
    }

    // F-A27
    #[test]
    fn the_curve_is_straight_under_the_knee_and_bends_after_it() {
        assert_eq!(reduction_db(-60.0, -20.0, 4.0), 0.0);
        assert!((reduction_db(0.0, -20.0, 4.0) - (-15.0)).abs() < 1e-4, "20 dB over at 4:1 gives back 15");
        let at = reduction_db(-20.0, -20.0, 4.0);
        assert!(at < 0.0 && at > -1.0, "the knee starts to bite at the threshold: {at}");
        assert_eq!(reduction_db(-10.0, -20.0, 1.0), 0.0, "1:1 is nothing");
    }
}
