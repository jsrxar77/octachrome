//! KickSyn: a synthesized kick. Sine with an exponential pitch sweep, an
//! exponential amp decay, a short noise click and tanh drive.
//!
//! Note 36 (C1) plays at the Tune frequency; other notes transpose it, so a
//! linked track can play tuned kicks.

use crate::{
    dsp::{decay_coef, sine, Scope},
    ui,
};
use op3_cartridge::{
    choice_index, state, FrameBuffer480x272, InstrumentCartridge, InstrumentInfo, NoteEvent,
    NoteKind, ParamDescriptor, ParamUnit, Rgb565, SAMPLE_RATE,
};

const PARAMS: [ParamDescriptor; 4] = [
    ParamDescriptor::new(
        "TUNE",
        0.4,
        ParamUnit::Hertz {
            min: 30.0,
            max: 120.0,
        },
    ),
    ParamDescriptor::new(
        "DECAY",
        0.45,
        ParamUnit::Millis {
            min: 60.0,
            max: 2000.0,
        },
    ),
    ParamDescriptor::new("PUNCH", 0.6, ParamUnit::Percent),
    ParamDescriptor::new("DRIVE", 0.25, ParamUnit::Percent),
];
const SECONDARY: [ParamDescriptor; 4] = [
    ParamDescriptor::new("CLICK", 0.3, ParamUnit::Percent),
    ParamDescriptor::new(
        "P.DECAY",
        0.35,
        ParamUnit::Millis {
            min: 5.0,
            max: 200.0,
        },
    ),
    ParamDescriptor::new("VEL", 0.0, ParamUnit::Choice(&["VEL ON", "FIXED"])),
    ParamDescriptor::new("LEVEL", 0.8, ParamUnit::Percent),
];
const STATE_VERSION: u8 = 1;

pub struct KickSyn {
    values: [f32; 8],
    phase: f32,
    amp: f32,
    pitch_env: f32,
    click: f32,
    base_hz: f32,
    velocity: f32,
    noise: u32,
    scope: Scope,
}

impl Default for KickSyn {
    fn default() -> Self {
        Self::new()
    }
}

impl KickSyn {
    pub fn new() -> Self {
        let mut values = [0.0; 8];
        for (i, p) in PARAMS.iter().chain(SECONDARY.iter()).enumerate() {
            values[i] = p.default;
        }
        Self {
            values,
            phase: 0.0,
            amp: 0.0,
            pitch_env: 0.0,
            click: 0.0,
            base_hz: 50.0,
            velocity: 1.0,
            noise: 0x1234_5678,
            scope: Scope::new(4),
        }
    }

    fn desc(i: usize) -> &'static ParamDescriptor {
        if i < 4 {
            &PARAMS[i]
        } else {
            &SECONDARY[i - 4]
        }
    }
    fn val(&self, i: usize) -> f32 {
        Self::desc(i).denormalize(self.values[i])
    }

    fn trigger(&mut self, note: u8, velocity: f32) {
        self.base_hz = self.val(0) * 2f32.powf((note as f32 - 36.0) / 12.0);
        self.phase = 0.0;
        self.amp = 1.0;
        self.pitch_env = 1.0;
        self.click = 1.0;
        self.velocity = if choice_index(self.values[6], 2) == 1 {
            1.0
        } else {
            velocity
        };
    }

    #[inline]
    fn sample(
        &mut self,
        amp_c: f32,
        pitch_c: f32,
        punch: f32,
        drive: f32,
        click_amt: f32,
        level: f32,
    ) -> f32 {
        if self.amp < 1e-5 {
            self.amp = 0.0;
            return 0.0;
        }
        let hz = self.base_hz * (1.0 + punch * 7.0 * self.pitch_env);
        self.phase = (self.phase + hz / SAMPLE_RATE).fract();
        let mut s = sine(self.phase) * self.amp;
        // click: short burst of noise
        self.noise ^= self.noise << 13;
        self.noise ^= self.noise >> 17;
        self.noise ^= self.noise << 5;
        let n = (self.noise as f32 / u32::MAX as f32) * 2.0 - 1.0;
        s += n * self.click * click_amt * 0.5;
        self.click *= 0.995 * 0.99;
        self.amp *= amp_c;
        self.pitch_env *= pitch_c;
        let g = 1.0 + drive * 6.0;
        (s * g).tanh() / g.tanh() * self.velocity * level
    }
}

impl InstrumentCartridge for KickSyn {
    fn info(&self) -> InstrumentInfo {
        InstrumentInfo::new("KickSyn", "Drum")
    }

    fn process(&mut self, output_l: &mut [f32], output_r: &mut [f32], events: &[NoteEvent]) {
        let amp_c = decay_coef(self.val(1));
        let pitch_c = decay_coef(self.val(5));
        let (punch, drive, click, level) = (
            self.values[2],
            self.values[3],
            self.values[4],
            self.values[7],
        );
        let mut ev = events.iter().peekable();
        for (i, (l, r)) in output_l.iter_mut().zip(output_r.iter_mut()).enumerate() {
            while let Some(e) = ev.next_if(|e| e.sample_offset as usize <= i) {
                if e.kind == NoteKind::On {
                    self.trigger(e.note, e.velocity);
                }
                // Note-off is ignored: a kick always plays out its decay.
            }
            let s = self.sample(amp_c, pitch_c, punch, drive, click, level);
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
        self.amp = 0.0;
    }

    /// A drum head seen from above (design.md: drawings, not text). Tune sets
    /// its size, decay the arc around it, punch the pitch rings that ripple
    /// out on a hit, drive the spikes on the shell. Colours: encoders 5–8.
    fn draw_ui(&self, frame: &mut FrameBuffer480x272, _accent_color: Rgb565) {
        use op3_gfx::{knob, paint::*, theme};
        use std::f32::consts::{FRAC_PI_2, TAU};
        let t = theme();
        let (cx, cy) = (240.0, 124.0);
        let r = 34.0 + self.values[0] * 40.0;
        let hit = self.amp.clamp(0.0, 1.0);
        let lvl = (ui::scope_rms(&self.scope) * 3.0).min(1.0);
        let orbit = ui::clock() * 1.2;
        // shell
        ring(frame, cx, cy, r + 14.0, 6.0, t.faint);
        // membrane, wobbling with drive and level
        let lobes = 3.0 + (self.values[3] * 8.0).round();
        let pts: Vec<(f32, f32)> = (0..96)
            .map(|i| {
                let a = i as f32 / 96.0 * TAU;
                let rr = r * (1.0 + lvl * 0.06 * (a * lobes + orbit * 6.0).sin());
                (cx + a.cos() * rr, cy + a.sin() * rr)
            })
            .collect();
        path(frame, &pts, 2.0, true, t.fg);
        // punch (encoder 7): pitch rings rippling out on each hit
        for i in 0..3 {
            let k = ((1.0 - hit) + i as f32 / 3.0) % 1.0;
            let a = ((1.0 - k) * hit * self.values[2] * 1.4).min(1.0);
            if a > 0.02 {
                ring(frame, cx, cy, r * k, 2.0, Rgb565::BLACK.lerp(knob(6), a));
            }
        }
        // beater (encoder 5: tune)
        fill_circle(frame, cx, cy, 6.0 + hit * 10.0, knob(4));
        // decay (encoder 6): arc around the shell
        let span = TAU * self.values[1] * (0.6 + 0.4 * hit);
        arc(
            frame,
            cx,
            cy,
            r + 24.0,
            3.0,
            -FRAC_PI_2,
            -FRAC_PI_2 + span.max(0.05),
            knob(5),
        );
        // drive (encoder 8): spikes
        let n = (self.values[3] * 16.0).round() as i32;
        for i in 0..n {
            let a = i as f32 / n as f32 * TAU + orbit * 0.2;
            let (r0, r1) = (r + 30.0, r + 36.0 + lvl * 8.0);
            line_aa(
                frame,
                cx + a.cos() * r0,
                cy + a.sin() * r0,
                cx + a.cos() * r1,
                cy + a.sin() * r1,
                2.0,
                knob(7),
            );
        }
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
    fn kick_has_a_loud_start_and_decays() {
        let mut k = KickSyn::new();
        let (mut l, mut r) = (vec![0.0; 4800], vec![0.0; 4800]);
        k.process(&mut l, &mut r, &[NoteEvent::on(0, 36, 1.0)]);
        let early = l[..480].iter().map(|s| s.abs()).fold(0.0, f32::max);
        let late = l[4320..].iter().map(|s| s.abs()).fold(0.0, f32::max);
        assert!(early > 0.5, "{early}");
        assert!(late < early * 0.8, "{late} vs {early}");
    }

    #[test]
    fn event_offset_is_respected() {
        let mut k = KickSyn::new();
        let (mut l, mut r) = (vec![0.0; 256], vec![0.0; 256]);
        k.process(&mut l, &mut r, &[NoteEvent::on(100, 36, 1.0)]);
        assert!(l[..100].iter().all(|&s| s == 0.0));
        assert!(l[101..].iter().any(|&s| s != 0.0));
    }
}
