//! FmBass: monophonic two-operator FM bass with last-note priority and glide.
//!
//! A sine modulator at `ratio × f` drives a sine carrier at `f`. The
//! modulation index has its own fast decay, which gives the pluck; the amp
//! follows an ADSR. A sub-octave sine keeps the low end solid when the index
//! is high.

use crate::{
    dsp::{decay_coef, midi_to_hz, sine, Adsr, Scope},
    ui,
};
use op3_cartridge::{
    choice_index, state, FrameBuffer480x272, InstrumentCartridge, InstrumentInfo, NoteEvent,
    NoteKind, ParamDescriptor, ParamUnit, Rgb565, SAMPLE_RATE,
};
use op3_gfx::{text, Align, FontSize};

const RATIOS: [f32; 5] = [0.5, 1.0, 2.0, 3.0, 4.0];
const PARAMS: [ParamDescriptor; 4] = [
    ParamDescriptor::new(
        "RATIO",
        0.3,
        ParamUnit::Choice(&["0.5", "1", "2", "3", "4"]),
    ),
    ParamDescriptor::new("INDEX", 0.45, ParamUnit::Percent),
    ParamDescriptor::new(
        "DECAY",
        0.4,
        ParamUnit::Millis {
            min: 20.0,
            max: 2000.0,
        },
    ),
    ParamDescriptor::new(
        "RELEASE",
        0.3,
        ParamUnit::Millis {
            min: 5.0,
            max: 1500.0,
        },
    ),
];
const SECONDARY: [ParamDescriptor; 4] = [
    ParamDescriptor::new(
        "GLIDE",
        0.0,
        ParamUnit::Millis {
            min: 1.0,
            max: 500.0,
        },
    ),
    ParamDescriptor::new("SUB", 0.4, ParamUnit::Percent),
    ParamDescriptor::new("SUSTAIN", 0.6, ParamUnit::Percent),
    ParamDescriptor::new("LEVEL", 0.7, ParamUnit::Percent),
];
const STATE_VERSION: u8 = 1;
const STACK: usize = 8;

pub struct FmBass {
    values: [f32; 8],
    env: Adsr,
    index_env: f32,
    car_phase: f32,
    mod_phase: f32,
    sub_phase: f32,
    hz: f32,
    target_hz: f32,
    velocity: f32,
    stack: [u8; STACK],
    stack_len: usize,
    scope: Scope,
}

impl Default for FmBass {
    fn default() -> Self {
        Self::new()
    }
}

impl FmBass {
    pub fn new() -> Self {
        let mut values = [0.0; 8];
        for (i, p) in PARAMS.iter().chain(SECONDARY.iter()).enumerate() {
            values[i] = p.default;
        }
        Self {
            values,
            env: Adsr::new(2.0, 200.0, 0.6, 100.0),
            index_env: 0.0,
            car_phase: 0.0,
            mod_phase: 0.0,
            sub_phase: 0.0,
            hz: 55.0,
            target_hz: 55.0,
            velocity: 1.0,
            stack: [0; STACK],
            stack_len: 0,
            scope: Scope::new(2),
        }
    }

    fn val(&self, i: usize) -> f32 {
        if i < 4 {
            PARAMS[i].denormalize(self.values[i])
        } else {
            SECONDARY[i - 4].denormalize(self.values[i])
        }
    }

    fn note_on(&mut self, note: u8, velocity: f32) {
        self.stack_remove(note);
        if self.stack_len == STACK {
            self.stack.copy_within(1.., 0);
            self.stack_len -= 1;
        }
        self.stack[self.stack_len] = note;
        self.stack_len += 1;
        let legato = self.env.is_active() && self.stack_len > 1;
        self.target_hz = midi_to_hz(note as f32);
        if !legato || self.values[4] <= 0.0 {
            self.hz = self.target_hz;
        }
        if !legato {
            self.env.gate_on();
            self.index_env = 1.0;
            self.velocity = velocity;
        }
    }

    fn note_off(&mut self, note: u8) {
        self.stack_remove(note);
        if self.stack_len == 0 {
            self.env.gate_off();
        } else {
            self.target_hz = midi_to_hz(self.stack[self.stack_len - 1] as f32);
        }
    }

    fn stack_remove(&mut self, note: u8) {
        if let Some(i) = self.stack[..self.stack_len].iter().position(|&n| n == note) {
            self.stack.copy_within(i + 1..self.stack_len, i);
            self.stack_len -= 1;
        }
    }
}

impl InstrumentCartridge for FmBass {
    fn info(&self) -> InstrumentInfo {
        InstrumentInfo::new("FmBass", "Synth")
    }

    fn process(&mut self, output_l: &mut [f32], output_r: &mut [f32], events: &[NoteEvent]) {
        let ratio = RATIOS[choice_index(self.values[0], RATIOS.len())];
        let index = self.values[1] * 6.0;
        let index_c = decay_coef(self.val(2));
        self.env.decay_ms = self.val(2);
        self.env.release_ms = self.val(3);
        self.env.sustain = self.values[6];
        let glide_c = if self.values[4] > 0.0 {
            decay_coef(self.val(4))
        } else {
            0.0
        };
        let sub = self.values[5];
        let level = self.values[7];
        let mut ev = events.iter().peekable();
        for (i, (l, r)) in output_l.iter_mut().zip(output_r.iter_mut()).enumerate() {
            while let Some(e) = ev.next_if(|e| e.sample_offset as usize <= i) {
                match e.kind {
                    NoteKind::On => self.note_on(e.note, e.velocity),
                    NoteKind::Off => self.note_off(e.note),
                }
            }
            let s = if self.env.is_active() {
                self.hz = self.target_hz + (self.hz - self.target_hz) * glide_c;
                let a = self.env.next();
                self.mod_phase = (self.mod_phase + self.hz * ratio / SAMPLE_RATE).fract();
                let m = sine(self.mod_phase) * index * (0.25 + 0.75 * self.index_env);
                self.car_phase = (self.car_phase + self.hz / SAMPLE_RATE).fract();
                self.sub_phase = (self.sub_phase + self.hz * 0.5 / SAMPLE_RATE).fract();
                self.index_env *= index_c;
                let c = sine(self.car_phase + m / std::f32::consts::TAU);
                (c * (1.0 - sub * 0.5) + sine(self.sub_phase) * sub * 0.6)
                    * a
                    * self.velocity
                    * level
            } else {
                0.0
            };
            self.scope.push(s);
            *l = s;
            *r = s;
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
        self.stack_len = 0;
        self.env.kill();
    }

    fn draw_ui(&self, frame: &mut FrameBuffer480x272, accent_color: Rgb565) {
        let y = ui::AREA_Y;
        let dim = Rgb565::WHITE.scale(0.6);
        // Topology: MOD -> CAR -> OUT, with the ratio on the modulator.
        text(frame, "ALGO", 16, y + 4, FontSize::Small, dim, Align::Left);
        let ratio = PARAMS[0].format(self.values[0]);
        op3_gfx::rect_outline(frame, 16, y + 22, 64, 30, 2, accent_color);
        text(
            frame,
            &format!("x{ratio}"),
            48,
            y + 30,
            FontSize::Small,
            Rgb565::WHITE,
            Align::Center,
        );
        op3_gfx::line(frame, 80, y + 37, 104, y + 37, 1, Rgb565::WHITE);
        op3_gfx::rect_outline(frame, 104, y + 22, 64, 30, 2, Rgb565::WHITE);
        text(
            frame,
            "CAR",
            136,
            y + 30,
            FontSize::Small,
            Rgb565::WHITE,
            Align::Center,
        );
        op3_gfx::line(frame, 168, y + 37, 200, y + 37, 1, Rgb565::WHITE);
        // index amount as a bar between them
        op3_gfx::value_bar(frame, 16, y + 58, 152, 10, self.values[1], accent_color);
        // ADSR
        text(frame, "ENV", 16, y + 80, FontSize::Small, dim, Align::Left);
        ui::draw_adsr(
            frame,
            2.0,
            self.val(2),
            self.values[6],
            self.val(3),
            16,
            y + 94,
            200,
            90,
            accent_color,
        );
        // Scope
        text(frame, "OUT", 248, y + 4, FontSize::Small, dim, Align::Left);
        ui::draw_scope(frame, &self.scope, 248, y + 20, 216, 164, accent_color);
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

    fn freq_estimate(buf: &[f32]) -> f32 {
        let crossings = buf.windows(2).filter(|w| w[0] <= 0.0 && w[1] > 0.0).count();
        crossings as f32 * SAMPLE_RATE / buf.len() as f32
    }

    #[test]
    fn plays_the_right_pitch() {
        let mut b = FmBass::new();
        b.set_parameter(1, 0.0); // no FM
        b.set_parameter(5, 0.0); // no sub
        let (mut l, mut r) = (vec![0.0; 48_000], vec![0.0; 48_000]);
        b.process(&mut l, &mut r, &[NoteEvent::on(0, 45, 1.0)]); // A2 = 110 Hz
        let f = freq_estimate(&l[4800..]);
        assert!((f - 110.0).abs() < 3.0, "{f}");
    }

    #[test]
    fn last_note_priority() {
        let mut b = FmBass::new();
        let (mut l, mut r) = (vec![0.0; 64], vec![0.0; 64]);
        b.process(
            &mut l,
            &mut r,
            &[
                NoteEvent::on(0, 40, 1.0),
                NoteEvent::on(10, 47, 1.0),
                NoteEvent::off(20, 47),
            ],
        );
        assert_eq!(b.target_hz, midi_to_hz(40.0));
        assert!(b.env.is_active());
        b.process(&mut l, &mut r, &[NoteEvent::off(0, 40)]);
        assert_eq!(b.env.stage(), crate::dsp::Stage::Release);
    }
}
