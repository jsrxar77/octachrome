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
use op3_gfx::{text, Align, FontSize};

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

    fn draw_ui(&self, frame: &mut FrameBuffer480x272, accent_color: Rgb565) {
        let y = ui::AREA_Y;
        // Left: amplitude (white) and pitch (accent) envelopes.
        let decay = self.val(1);
        let pdecay = self.val(5);
        let punch = self.values[2];
        text(
            frame,
            "ENV",
            16,
            y + 4,
            FontSize::Small,
            Rgb565::WHITE.scale(0.6),
            Align::Left,
        );
        let w = 200;
        ui::draw_curve(
            frame,
            |u| (-u * 2000.0 / decay * 6.9).exp(),
            16,
            y + 20,
            w,
            150,
            2,
            Rgb565::WHITE,
        );
        ui::draw_curve(
            frame,
            |u| punch * (-u * 2000.0 / pdecay * 6.9).exp(),
            16,
            y + 20,
            w,
            150,
            1,
            accent_color,
        );
        // Right: live scope.
        text(
            frame,
            "OUT",
            248,
            y + 4,
            FontSize::Small,
            Rgb565::WHITE.scale(0.6),
            Align::Left,
        );
        ui::draw_scope(frame, &self.scope, 248, y + 20, 216, 150, accent_color);
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
