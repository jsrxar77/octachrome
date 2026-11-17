//! resonator: plucked strings and struck bells, four voices (docs/instruments.md §7).
//!
//! Built on the string and modal engines of `op8-plaits` (MIT), which are the physical
//! models behind the resonator voices of the vendored code: a note excites a string or a
//! bank of vibrating modes that then rings by itself. Page 1 chooses what rings and how
//! it is built (`type`, `structure`, `bright`, `damping`); page 2 sets how long it rings,
//! the colour of the ring-out, how far apart the four voices sit in the stereo field and
//! the level. Four notes can ring at once; a fifth takes the oldest voice.

use crate::{dsp::Scope, ui};
use op8_cartridge::{
    choice_index, state, FrameBuffer480x272, InstrumentCartridge, InstrumentInfo, NoteEvent,
    NoteKind, ParamDescriptor, ParamUnit, Rgb565,
};
use op8_plaits::{MacroPatch, MacroVoice};

const CHUNK: usize = 12;
const VOICES: usize = 4;
/// Plaits engines: 19 string, 20 modal.
const ENGINES: [usize; 2] = [19, 20];
const TYPE_NAMES: [&str; 2] = ["string", "bells"];
/// A voice that has been below −60 dB for this many chunks (~100 ms) sleeps.
const SLEEP_CHUNKS: u32 = 400;
const QUIET: f32 = 1.0e-3;
const STATE_VERSION: u8 = 1;

const PARAMS: [ParamDescriptor; 4] = [
    ParamDescriptor::new("TYPE", 0.25, ParamUnit::Choice(&TYPE_NAMES)),
    ParamDescriptor::new("STRUCTURE", 0.4, ParamUnit::Percent),
    ParamDescriptor::new("BRIGHT", 0.55, ParamUnit::Percent),
    ParamDescriptor::new("DAMPING", 0.5, ParamUnit::Percent),
];
const SECONDARY: [ParamDescriptor; 4] = [
    ParamDescriptor::new("DECAY", 0.6, ParamUnit::Percent),
    ParamDescriptor::new("COLOR", 0.5, ParamUnit::Percent),
    ParamDescriptor::new("SPREAD", 0.4, ParamUnit::Percent),
    ParamDescriptor::new("LEVEL", 0.7, ParamUnit::Percent),
];

struct Voice {
    voice: MacroVoice,
    engine: usize,
    note: f32,
    velocity: f32,
    strike: bool,
    awake: bool,
    quiet_chunks: u32,
    /// Notes started since this voice last sounded: the oldest is stolen first.
    age: u64,
}

pub struct Resonator {
    values: [f32; 8],
    voices: Vec<Voice>,
    clock: u64,
    /// The pitch wheel, in semitones.
    bend: f32,
    out: [f32; CHUNK],
    aux: [f32; CHUNK],
    scope: Scope,
}

impl Default for Resonator {
    fn default() -> Self {
        Self::new()
    }
}

impl Resonator {
    pub fn new() -> Self {
        let mut values = [0.0; 8];
        for (i, p) in PARAMS.iter().chain(SECONDARY.iter()).enumerate() {
            values[i] = p.default;
        }
        let voices = (0..VOICES)
            .map(|_| Voice {
                voice: MacroVoice::new(),
                engine: usize::MAX,
                note: 48.0,
                velocity: 1.0,
                strike: false,
                awake: false,
                quiet_chunks: 0,
                age: 0,
            })
            .collect();
        Self { values, voices, clock: 0, bend: 0.0, out: [0.0; CHUNK], aux: [0.0; CHUNK], scope: Scope::new(2) }
    }

    fn kind(&self) -> usize {
        choice_index(self.values[0], ENGINES.len())
    }

    /// The voice a new note takes: one that is asleep, else the oldest.
    fn take_voice(&self) -> usize {
        self.voices
            .iter()
            .position(|v| !v.awake)
            .unwrap_or_else(|| (0..VOICES).min_by_key(|&i| self.voices[i].age).unwrap_or(0))
    }
}

impl InstrumentCartridge for Resonator {
    fn info(&self) -> InstrumentInfo {
        InstrumentInfo::new("resonator", "Synth")
    }

    fn process(&mut self, output_l: &mut [f32], output_r: &mut [f32], events: &[NoteEvent]) {
        let n = output_l.len().min(output_r.len());
        let engine = ENGINES[self.kind()];
        let (structure, bright, damping) = (self.values[1], self.values[2], self.values[3]);
        let (decay, colour, spread, level) = (self.values[4], self.values[5], self.values[6], self.values[7]);
        let bend = self.bend;
        let mut ev = events.iter().peekable();
        let mut start = 0;
        while start < n {
            let end = (start + CHUNK).min(n);
            // Events land on the chunk that contains them (≤ 0.25 ms late). Note-offs do nothing:
            // a string or a bell rings out by itself.
            while let Some(e) = ev.next_if(|e| (e.sample_offset as usize) < end) {
                if e.kind == NoteKind::On {
                    let i = self.take_voice();
                    self.clock += 1;
                    let v = &mut self.voices[i];
                    v.note = e.note as f32;
                    v.velocity = e.velocity.clamp(0.0, 1.0);
                    v.strike = true;
                    v.awake = true;
                    v.quiet_chunks = 0;
                    v.age = self.clock;
                }
            }
            let len = end - start;
            output_l[start..end].fill(0.0);
            output_r[start..end].fill(0.0);
            for (i, v) in self.voices.iter_mut().enumerate() {
                if !v.awake {
                    continue;
                }
                let mut patch = MacroPatch {
                    note: v.note + bend,
                    engine,
                    harmonics: structure,
                    timbre: bright,
                    morph: damping,
                    decay,
                    colour,
                    trigger: false,
                };
                // A change of type takes effect on the next render: do it on a silent chunk.
                if v.engine != engine {
                    v.voice.render(&patch, &mut self.out[..len], &mut self.aux[..len]);
                    v.engine = engine;
                }
                patch.trigger = std::mem::take(&mut v.strike);
                v.voice.render(&patch, &mut self.out[..len], &mut self.aux[..len]);
                // The four voices fan out across the stereo field by `spread` (constant power).
                let pos = 0.5 + (i as f32 / (VOICES - 1) as f32 - 0.5) * spread;
                let a = pos * std::f32::consts::FRAC_PI_2;
                let (gl, gr) = (a.cos() * std::f32::consts::SQRT_2, a.sin() * std::f32::consts::SQRT_2);
                let g = v.velocity * level * 0.5;
                let mut peak = 0.0f32;
                for j in 0..len {
                    let s = self.out[j] * g;
                    peak = peak.max(s.abs());
                    output_l[start + j] += s * gl;
                    output_r[start + j] += s * gr;
                }
                if peak < QUIET {
                    v.quiet_chunks += 1;
                    if v.quiet_chunks > SLEEP_CHUNKS {
                        v.awake = false;
                    }
                } else {
                    v.quiet_chunks = 0;
                }
            }
            for j in start..end {
                output_l[j] = output_l[j].tanh();
                output_r[j] = output_r[j].tanh();
                self.scope.push((output_l[j] + output_r[j]) * 0.5);
            }
            start = end;
        }
    }

    fn set_pitch_bend(&mut self, semitones: f32) {
        self.bend = semitones;
    }

    fn parameters(&self) -> &[ParamDescriptor; 4] {
        &PARAMS
    }
    fn secondary_parameters(&self) -> Option<&[ParamDescriptor; 4]> {
        Some(&SECONDARY)
    }
    fn set_parameter(&mut self, index: usize, normalized_value: f32) {
        if let Some(v) = self.values.get_mut(index) {
            *v = normalized_value.clamp(0.0, 1.0);
        }
    }
    fn parameter_value(&self, index: usize) -> Option<f32> {
        self.values.get(index).copied()
    }
    fn all_notes_off(&mut self) {
        for v in &mut self.voices {
            v.strike = false;
            v.awake = false;
        }
    }

    /// What rings, drawn as what it is. A string: a line fixed at both ends that shivers with
    /// the sound; `structure` stiffens it (the shape of its bow) and `bright` thickens it.
    /// Bells: rings whose spacing is the structure of the modes (even for a string-like
    /// series, bunched and uneven for a bell), brighter ones drawn thicker; `damping` is how
    /// fast the picture fades back to rest.
    fn draw_ui(&self, frame: &mut FrameBuffer480x272, _accent_color: Rgb565) {
        use op8_gfx::{knob, paint::*, theme};
        let t = theme();
        let level = (ui::scope_rms(&self.scope) * 4.0).min(1.0);
        let (structure, bright, damping) = (self.values[1], self.values[2], self.values[3]);
        let (cx, cy) = (240.0, 124.0);
        let samples: Vec<f32> = self.scope.samples().collect();
        if self.kind() == 0 {
            // the string: 120 points between two bridges
            let (x0, x1) = (60.0, 420.0);
            fill_circle(frame, x0, cy, 5.0, t.dim);
            fill_circle(frame, x1, cy, 5.0, t.dim);
            let st = ui::trigger_index(&samples);
            let n = 120;
            let pts: Vec<(f32, f32)> = (0..n)
                .map(|i| {
                    let u = i as f32 / (n - 1) as f32;
                    let s = samples.get(st + i).copied().unwrap_or(0.0);
                    // the string is held at its ends and bowed by its stiffness
                    let fix = (u * std::f32::consts::PI).sin();
                    let bow = structure * 14.0 * fix * (1.0 - level * 0.5);
                    (x0 + u * (x1 - x0), cy - bow + (s * 3.0).clamp(-1.0, 1.0) * 48.0 * fix)
                })
                .collect();
            path(frame, &pts, 1.0 + bright * 3.0, false, knob(5));
            line_aa(frame, x0, cy + 56.0, x1, cy + 56.0, 1.0, t.faint);
            fill_circle(frame, x0 + (x1 - x0) * (0.2 + damping * 0.6), cy + 56.0, 4.0 + level * 4.0, knob(7));
        } else {
            // the bells: eight rings, spaced as the modes are
            for k in 1..=8 {
                let ratio = (k as f32).powf(0.5 + structure * 1.0);
                let r = 10.0 + ratio * 9.0 * (1.0 + level * 0.12);
                let w = (0.5 + bright * 2.5) * (1.0 - (k as f32 - 1.0) / 10.0);
                let a = 1.0 - damping * (k as f32 - 1.0) / 9.0;
                ring(frame, cx, cy, r.min(112.0), w, knob(5).scale(0.35 + 0.65 * a));
            }
            fill_circle(frame, cx, cy, 4.0 + level * 10.0, t.fg);
        }
        let _ = knob(4);
    }

    fn save_state(&self) -> Vec<u8> {
        state::encode_f32s(STATE_VERSION, &self.values)
    }
    fn load_state(&mut self, data: &[u8]) {
        state::decode_f32s(STATE_VERSION, data, &mut self.values);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn energy(r: &mut Resonator, events: &[NoteEvent]) -> f32 {
        let (mut l, mut rr) = (vec![0.0; 256], vec![0.0; 256]);
        r.process(&mut l, &mut rr, events);
        let mut e: f32 = l.iter().map(|x| x * x).sum();
        for _ in 0..30 {
            r.process(&mut l, &mut rr, &[]);
            e += l.iter().map(|x| x * x).sum::<f32>();
        }
        e
    }

    // F-A13
    #[test]
    fn both_kinds_ring_when_struck() {
        for kind in 0..2 {
            let mut r = Resonator::new();
            r.set_parameter(0, (kind as f32 + 0.5) / 2.0);
            let e = energy(&mut r, &[NoteEvent::on(3, 55, 1.0)]);
            assert!(e > 1e-3, "{} silent", TYPE_NAMES[kind]);
        }
    }

    // F-A13
    #[test]
    fn four_notes_ring_together_and_a_fifth_takes_the_oldest() {
        let mut r = Resonator::new();
        let one = energy(&mut r, &[NoteEvent::on(0, 52, 1.0)]);
        let mut r = Resonator::new();
        let four = energy(&mut r, &[NoteEvent::on(0, 52, 1.0), NoteEvent::on(1, 55, 1.0), NoteEvent::on(2, 59, 1.0), NoteEvent::on(3, 64, 1.0)]);
        assert!(four > one * 1.5, "four notes carry more than one: {four} vs {one}");
        assert_eq!(r.voices.iter().filter(|v| v.awake).count(), 4);
        // a fifth note goes to the voice that started first
        let oldest = (0..VOICES).min_by_key(|&i| r.voices[i].age).unwrap();
        assert_eq!(r.take_voice(), oldest);
    }

    // F-A13
    #[test]
    fn it_dies_away_and_stays_bounded() {
        let mut r = Resonator::new();
        let (mut l, mut rr) = (vec![9.0; 512], vec![9.0; 512]);
        r.process(&mut l, &mut rr, &[]);
        assert!(l.iter().chain(&rr).all(|&s| s == 0.0), "silent without notes");
        let hits: Vec<NoteEvent> = (0..8).map(|i| NoteEvent::on(i * 8, 48 + i as u8 * 2, 1.0)).collect();
        for p in 0..8 {
            r.set_parameter(p, 1.0);
        }
        r.process(&mut l, &mut rr, &hits);
        assert!(l.iter().chain(&rr).all(|s| s.is_finite() && s.abs() <= 1.0));
    }
}
