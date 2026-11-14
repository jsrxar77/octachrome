//! Steps and Step Components.
//!
//! Components stack ("acumulables"): put two `Transpose(+5)` on a step and it
//! plays a fourth higher twice over; `Ratchet(2)` with `Ratchet(3)` gives six
//! hits. Each component kind has one fold rule, documented on the variant.

use crate::{fixed::FixedVec, Rng, MAX_COMPONENTS};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum StepComponent {
    #[default]
    None,
    /// Retrigger N times inside the step. Stacks by multiplying (max 16).
    Ratchet(u8),
    /// Play with this % probability. Stacks by multiplying probabilities.
    Chance(u8),
    /// Play only on loop iteration `n` (1-based) of every `of`. Stacks: all
    /// conditions must hold.
    Every { n: u8, of: u8 },
    /// Velocity change in MIDI units. Stacks by adding.
    Velocity(i8),
    /// Semitones. Stacks by adding.
    Transpose(i8),
    /// Octaves. Stacks by adding.
    Octave(i8),
    /// Extend the gate by N whole steps. Stacks by adding.
    Hold(u8),
    /// Delay the trigger by N ticks (24 ticks = one 1/16). Stacks by adding.
    Delay(u8),
    /// Random offset of 0..=N semitones. Stacks by adding ranges.
    RandomNote(u8),
    /// Parameter lock: cartridge param `index` to `value`/255 for this step
    /// only. Stacks: the last lock for an index wins.
    ParamLock { index: u8, value: u8 },
}

impl StepComponent {
    /// Short label for the inspector.
    pub fn label(&self) -> String {
        match *self {
            StepComponent::None => "-".into(),
            StepComponent::Ratchet(n) => format!("RATCHET x{n}"),
            StepComponent::Chance(p) => format!("CHANCE {p}%"),
            StepComponent::Every { n, of } => format!("EVERY {n}:{of}"),
            StepComponent::Velocity(v) => format!("VEL {v:+}"),
            StepComponent::Transpose(s) => format!("TRANS {s:+}"),
            StepComponent::Octave(o) => format!("OCT {o:+}"),
            StepComponent::Hold(n) => format!("HOLD +{n}"),
            StepComponent::Delay(t) => format!("DELAY {t}t"),
            StepComponent::RandomNote(r) => format!("RAND {r}st"),
            StepComponent::ParamLock { index, value } => {
                format!("P{} {:.0}%", index + 1, value as f32 / 2.55)
            }
        }
    }
}

pub type Components = FixedVec<StepComponent, MAX_COMPONENTS>;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Step {
    pub active: bool,
    /// MIDI note. For Brain-linked tracks this is read relative to C.
    pub note: u8,
    /// 1..=127
    pub velocity: u8,
    /// Gate as % of the step, 1..=100.
    pub gate: u8,
    pub components: Components,
    /// Drum lanes switched on in this step (bit n = sound n), for a track with `kit` set.
    /// A kit step plays one note per lane, `KIT_BASE_NOTE + lane`; its `active` is `lanes != 0`.
    pub lanes: u8,
}

/// The note of lane 0 on a kit track; lane n is this plus n (its pitch class picks the sound).
pub const KIT_BASE_NOTE: u8 = 60;

impl Default for Step {
    fn default() -> Self {
        Self {
            active: false,
            note: 60,
            velocity: 100,
            gate: 50,
            components: Components::new(),
            lanes: 0,
        }
    }
}

impl Step {
    pub fn on(note: u8) -> Self {
        Self {
            active: true,
            note,
            ..Self::default()
        }
    }
    /// A kit step with these lanes on (bit n = sound n).
    pub fn kit(lanes: u8) -> Self {
        Self {
            active: lanes != 0,
            lanes,
            ..Self::default()
        }
    }
    pub fn with(mut self, c: StepComponent) -> Self {
        self.components.push(c);
        self
    }
}

/// What a step does on one particular pass, after folding its components.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ResolvedStep {
    pub fire: bool,
    pub note: u8,
    pub velocity: f32,
    pub ratchet: u8,
    /// Gate length in steps (e.g. 0.5 = half a step, 2.5 = two and a half).
    pub gate_steps: f32,
    pub delay_ticks: u32,
    /// Up to 8 locks, `None` where unlocked.
    pub locks: [Option<f32>; 8],
}

impl Step {
    /// Fold the components for loop `iteration` (0-based). Consumes
    /// randomness only for components that need it, so results are
    /// reproducible from the seed.
    pub fn resolve(&self, iteration: u64, rng: &mut Rng) -> ResolvedStep {
        let mut r = ResolvedStep {
            fire: self.active,
            note: self.note,
            velocity: 0.0,
            ratchet: 1,
            gate_steps: self.gate.clamp(1, 100) as f32 / 100.0,
            delay_ticks: 0,
            locks: [None; 8],
        };
        let mut vel = self.velocity as i32;
        let mut semis = 0i32;
        let mut ratchet = 1u32;
        let mut chance = 1.0f32;
        let mut rand_range = 0u32;
        for c in self.components.iter() {
            match *c {
                StepComponent::None => {}
                StepComponent::Ratchet(n) => ratchet = (ratchet * n.max(1) as u32).min(16),
                StepComponent::Chance(p) => chance *= p.min(100) as f32 / 100.0,
                StepComponent::Every { n, of } => {
                    let of = of.max(1) as u64;
                    if iteration % of != (n.max(1) as u64 - 1) % of {
                        r.fire = false;
                    }
                }
                StepComponent::Velocity(v) => vel += v as i32,
                StepComponent::Transpose(s) => semis += s as i32,
                StepComponent::Octave(o) => semis += 12 * o as i32,
                StepComponent::Hold(n) => r.gate_steps += n as f32,
                StepComponent::Delay(t) => r.delay_ticks += t as u32,
                StepComponent::RandomNote(n) => rand_range += n as u32,
                StepComponent::ParamLock { index, value } => {
                    if let Some(slot) = r.locks.get_mut(index as usize) {
                        *slot = Some(value as f32 / 255.0);
                    }
                }
            }
        }
        if r.fire && chance < 1.0 {
            r.fire = (rng.below(10_000) as f32) < chance * 10_000.0;
        }
        if rand_range > 0 {
            semis += rng.below(rand_range + 1) as i32;
        }
        r.ratchet = ratchet as u8;
        r.velocity = vel.clamp(1, 127) as f32 / 127.0;
        r.note = (self.note as i32 + semis).clamp(0, 127) as u8;
        r
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn components_accumulate() {
        let s = Step::on(60)
            .with(StepComponent::Transpose(5))
            .with(StepComponent::Transpose(7))
            .with(StepComponent::Octave(-1))
            .with(StepComponent::Ratchet(2))
            .with(StepComponent::Ratchet(3))
            .with(StepComponent::Velocity(-20));
        let r = s.resolve(0, &mut Rng::new(1));
        assert!(r.fire);
        assert_eq!(r.note, 60);
        assert_eq!(r.ratchet, 6);
        assert!((r.velocity - 80.0 / 127.0).abs() < 1e-6);
    }

    #[test]
    fn every_condition() {
        let s = Step::on(60).with(StepComponent::Every { n: 2, of: 4 });
        let fired: Vec<bool> = (0..8)
            .map(|i| s.resolve(i, &mut Rng::new(1)).fire)
            .collect();
        assert_eq!(
            fired,
            [false, true, false, false, false, true, false, false]
        );
    }

    #[test]
    fn chance_is_roughly_right_and_deterministic() {
        let s = Step::on(60)
            .with(StepComponent::Chance(50))
            .with(StepComponent::Chance(50));
        let run = |seed| {
            let mut rng = Rng::new(seed);
            (0..4000).filter(|&i| s.resolve(i, &mut rng).fire).count()
        };
        let n = run(7);
        assert!((800..1200).contains(&n), "{n}");
        assert_eq!(n, run(7));
    }

    #[test]
    fn last_param_lock_wins() {
        let s = Step::on(60)
            .with(StepComponent::ParamLock { index: 1, value: 0 })
            .with(StepComponent::ParamLock {
                index: 1,
                value: 255,
            });
        let r = s.resolve(0, &mut Rng::new(1));
        assert_eq!(r.locks[1], Some(1.0));
        assert_eq!(r.locks[0], None);
    }

    #[test]
    fn hold_extends_gate() {
        let s = Step {
            gate: 50,
            ..Step::on(60)
        }
        .with(StepComponent::Hold(1))
        .with(StepComponent::Hold(2));
        assert_eq!(s.resolve(0, &mut Rng::new(1)).gate_steps, 3.5);
    }
}
