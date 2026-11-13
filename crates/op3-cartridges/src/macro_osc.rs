//! macro: 24 synthesis engines behind four knobs (docs/instruments.md).
//!
//! Built on the vendored macro-oscillator DSP in `op3-plaits` (MIT). Page 1
//! picks the engine and shapes it (harmonics, timbre, morph); page 2 sets
//! the built-in low-pass gate (decay, colour), the variant (crossfade to the
//! engine's alternate output) and level. Monophonic; the "chords" engine
//! plays chords on its own.

use crate::{dsp::Scope, ui};
use op3_cartridge::{
    choice_index, state, FrameBuffer480x272, InstrumentCartridge, InstrumentInfo, NoteEvent,
    NoteKind, ParamDescriptor, ParamUnit, Rgb565,
};
use op3_plaits::{MacroPatch, MacroVoice, ENGINES, ENGINE_NAMES};

const CHUNK: usize = 12;
/// ~100 ms of quiet before the voice sleeps.
const SLEEP_CHUNKS: u32 = 400;
const QUIET: f32 = 2.0e-4;

const PARAMS: [ParamDescriptor; 4] = [
    ParamDescriptor::new(
        "ENGINE",
        (8.0 + 0.5) / ENGINES as f32,
        ParamUnit::Choice(&ENGINE_NAMES),
    ),
    ParamDescriptor::new("HARMONICS", 0.5, ParamUnit::Percent),
    ParamDescriptor::new("TIMBRE", 0.5, ParamUnit::Percent),
    ParamDescriptor::new("MORPH", 0.5, ParamUnit::Percent),
];
const SECONDARY: [ParamDescriptor; 4] = [
    ParamDescriptor::new("DECAY", 0.5, ParamUnit::Percent),
    ParamDescriptor::new("COLOR", 0.5, ParamUnit::Percent),
    ParamDescriptor::new("VARIANT", 0.0, ParamUnit::Percent),
    ParamDescriptor::new("LEVEL", 0.8, ParamUnit::Percent),
];
const STATE_VERSION: u8 = 1;

pub struct MacroOsc {
    values: [f32; 8],
    voice: MacroVoice,
    note: f32,
    velocity: f32,
    gain: f32,
    /// Gate held by the last note (some engines, like the 6-op FM, sustain
    /// while it is high; the others only react to its rising edge).
    gate: bool,
    /// A new note arrived while the gate was high: drop it for one chunk so
    /// the engines see a fresh rising edge.
    retrigger: bool,
    held_note: u8,
    /// Chunks rendered in a row below the silence threshold. The voice
    /// sleeps (renders nothing, outputs exact zeros) once this passes
    /// `SLEEP_CHUNKS`, which also saves CPU on the Pi.
    quiet_chunks: u32,
    awake: bool,
    out: [f32; CHUNK],
    aux: [f32; CHUNK],
    scope: Scope,
}

impl Default for MacroOsc {
    fn default() -> Self {
        Self::new()
    }
}

impl MacroOsc {
    pub fn new() -> Self {
        let mut values = [0.0; 8];
        for (i, p) in PARAMS.iter().chain(SECONDARY.iter()).enumerate() {
            values[i] = p.default;
        }
        Self {
            values,
            voice: MacroVoice::new(),
            note: 48.0,
            velocity: 1.0,
            gain: 1.0,
            gate: false,
            retrigger: false,
            held_note: 0,
            quiet_chunks: 0,
            awake: false,
            out: [0.0; CHUNK],
            aux: [0.0; CHUNK],
            scope: Scope::new(2),
        }
    }

    fn engine(&self) -> usize {
        choice_index(self.values[0], ENGINES)
    }
}

impl InstrumentCartridge for MacroOsc {
    fn info(&self) -> InstrumentInfo {
        InstrumentInfo::new("macro", "Synth")
    }

    fn process(&mut self, output_l: &mut [f32], output_r: &mut [f32], events: &[NoteEvent]) {
        let n = output_l.len().min(output_r.len());
        let mut patch = MacroPatch {
            engine: self.engine(),
            harmonics: self.values[1],
            timbre: self.values[2],
            morph: self.values[3],
            decay: self.values[4],
            colour: self.values[5],
            ..Default::default()
        };
        let (variant, level) = (self.values[6], self.values[7]);
        let mut ev = events.iter().peekable();
        let mut start = 0;
        while start < n {
            let end = (start + CHUNK).min(n);
            // Events land on the chunk that contains them (≤ 0.25 ms late).
            while let Some(e) = ev.next_if(|e| (e.sample_offset as usize) < end) {
                match e.kind {
                    NoteKind::On => {
                        self.note = e.note as f32;
                        self.held_note = e.note;
                        self.velocity = e.velocity.clamp(0.0, 1.0);
                        self.retrigger = self.gate;
                        self.gate = true;
                        self.awake = true;
                        self.quiet_chunks = 0;
                    }
                    NoteKind::Off if e.note == self.held_note => self.gate = false,
                    NoteKind::Off => {}
                }
            }
            let len = end - start;
            if !self.awake {
                output_l[start..end].fill(0.0);
                output_r[start..end].fill(0.0);
                for _ in 0..len {
                    self.scope.push(0.0);
                }
                start = end;
                continue;
            }
            // An engine switch takes effect on the next render; do it on a
            // silent chunk so the strike lands on the new engine.
            if self.voice.active_engine() != patch.engine {
                patch.trigger = false;
                self.voice
                    .render(&patch, &mut self.out[..len], &mut self.aux[..len]);
            }
            patch.note = self.note;
            patch.trigger = self.gate && !std::mem::take(&mut self.retrigger);
            self.voice
                .render(&patch, &mut self.out[..len], &mut self.aux[..len]);
            let mut peak = 0.0f32;
            for i in 0..len {
                self.gain += (self.velocity - self.gain) * 0.02;
                let s = (self.out[i] + (self.aux[i] - self.out[i]) * variant) * self.gain * level;
                peak = peak.max(s.abs());
                self.scope.push(s);
                output_l[start + i] = s;
                output_r[start + i] = s;
            }
            if peak < QUIET && !self.gate {
                self.quiet_chunks += 1;
                if self.quiet_chunks > SLEEP_CHUNKS {
                    self.awake = false;
                }
            } else {
                self.quiet_chunks = 0;
            }
            start = end;
        }
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
        self.gate = false;
        self.velocity = 0.0;
        self.awake = false;
    }

    /// The 24 engines as a ring of dots (encoder 5 lights the chosen one),
    /// the sound itself wrapped around the centre in polar form (encoder 6,
    /// harmonics, shapes it), and a timbre × morph field on the right
    /// (encoders 7 and 8 move the cross).
    fn draw_ui(&self, frame: &mut FrameBuffer480x272, _accent_color: Rgb565) {
        use op3_gfx::{knob, paint::*, theme};
        use std::f32::consts::{FRAC_PI_2, TAU};
        let t = theme();
        let (cx, cy) = (176.0, 124.0);
        let level = (ui::scope_rms(&self.scope) * 3.0).min(1.0);
        // engine ring: three banks of eight with a gap between banks
        let engine = self.engine();
        for i in 0..ENGINES {
            let bank = (i / 8) as f32;
            let a = -FRAC_PI_2 + (i as f32 + bank * 1.0) / (ENGINES as f32 + 3.0) * TAU;
            let (x, y) = (cx + a.cos() * 84.0, cy + a.sin() * 84.0);
            if i == engine {
                fill_circle(frame, x, y, 6.0 + level * 2.0, knob(4));
            } else {
                fill_circle(frame, x, y, 2.0, t.dim);
            }
        }
        // polar waveform
        let samples: Vec<f32> = self.scope.samples().collect();
        let st = ui::trigger_index(&samples);
        let n = 180;
        let pts: Vec<(f32, f32)> = (0..n)
            .map(|i| {
                let s = samples.get(st + i).copied().unwrap_or(0.0);
                let a = -FRAC_PI_2 + i as f32 / n as f32 * TAU;
                let r = 44.0 + (s * 2.2).clamp(-1.0, 1.0) * 24.0;
                (cx + a.cos() * r, cy + a.sin() * r)
            })
            .collect();
        ring(frame, cx, cy, 44.0, 1.0, t.faint);
        path(frame, &pts, 1.5, true, knob(5));
        fill_circle(frame, cx, cy, 8.0 + level * 10.0, t.fg);
        // timbre (x) × morph (y) field
        let (fx, fy, fs) = (316.0, 64.0, 120.0);
        stroke_round_rect(frame, fx, fy, fs, fs, 6.0, 1.0, t.faint);
        let px = fx + 6.0 + self.values[2] * (fs - 12.0);
        let py = fy + fs - 6.0 - self.values[3] * (fs - 12.0);
        line_aa(frame, px, fy + 4.0, px, fy + fs - 4.0, 1.5, knob(6));
        line_aa(frame, fx + 4.0, py, fx + fs - 4.0, py, 1.5, knob(7));
        fill_circle(frame, px, py, 5.0 + level * 3.0, t.fg);
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

    #[test]
    fn a_note_makes_sound_on_every_engine() {
        let mut m = MacroOsc::new();
        let (mut l, mut r) = (vec![0.0; 256], vec![0.0; 256]);
        for (e, name) in ENGINE_NAMES.iter().enumerate() {
            m.set_parameter(0, (e as f32 + 0.5) / ENGINES as f32);
            m.process(&mut l, &mut r, &[NoteEvent::on(3, 48, 1.0)]);
            let mut energy: f32 = l.iter().map(|x| x * x).sum();
            for b in 0..20 {
                let off = [NoteEvent::off(0, 48)];
                m.process(&mut l, &mut r, if b == 10 { &off } else { &[] });
                energy += l.iter().map(|x| x * x).sum::<f32>();
            }
            assert!(energy > 1e-3, "engine {e} ({name}) silent");
            assert!(l.iter().all(|x| x.is_finite()));
        }
    }
}
