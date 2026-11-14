//! drums: eight percussion sounds on one track (docs/instruments.md §7).
//!
//! Each sound is its own voice built on the vendored percussion engines in
//! `op8-plaits` (MIT): kick, snare and hi-hat, plus a noise burst for the clap
//! and a short modal voice for the rim. The sound a note plays is its pitch
//! class: C kick, C# snare, D closed hat, D# open hat, E tom, F clap, F# rim,
//! G crash; the notes above G wrap round (G# kick, A snare, ...). The knobs
//! edit the sound chosen with the `sound` knob (page 2, last knob); every sound
//! keeps its own values.

use crate::ui;
use op8_cartridge::{
    choice_index, state, FrameBuffer480x272, InstrumentCartridge, InstrumentInfo, NoteEvent,
    NoteKind, ParamDescriptor, ParamUnit, Rgb565,
};
use op8_plaits::{seed_noise, MacroPatch, MacroVoice};

const CHUNK: usize = 12;
/// ~100 ms of quiet before a voice sleeps.
const SLEEP_CHUNKS: u32 = 400;
const QUIET: f32 = 2.0e-4;
pub const SOUNDS: usize = 8;
/// Values kept per sound: tune, decay, tone, snap, body, level, pan.
const VALS: usize = 7;
const STATE_VERSION: u8 = 1;

pub const SOUND_NAMES: [&str; SOUNDS] = ["kick", "snare", "c-hat", "o-hat", "tom", "clap", "rim", "crash"];

struct Kind {
    engine: usize,
    base: f32,
    defaults: [f32; VALS],
}

/// Plaits engines: 17 noise, 20 modal, 21 kick, 22 snare, 23 hihat.
const KINDS: [Kind; SOUNDS] = [
    Kind { engine: 21, base: 36.0, defaults: [0.5, 0.45, 0.5, 0.5, 0.5, 0.75, 0.5] },
    Kind { engine: 22, base: 52.0, defaults: [0.5, 0.40, 0.5, 0.55, 0.5, 0.6, 0.5] },
    Kind { engine: 23, base: 72.0, defaults: [0.5, 0.15, 0.55, 0.5, 0.5, 0.4, 0.35] },
    Kind { engine: 23, base: 72.0, defaults: [0.5, 0.55, 0.55, 0.5, 0.5, 0.4, 0.35] },
    Kind { engine: 21, base: 48.0, defaults: [0.5, 0.40, 0.5, 0.4, 0.5, 0.6, 0.65] },
    Kind { engine: 17, base: 72.0, defaults: [0.5, 0.30, 0.6, 0.5, 0.5, 0.5, 0.5] },
    Kind { engine: 20, base: 84.0, defaults: [0.5, 0.12, 0.6, 0.5, 0.5, 0.5, 0.45] },
    Kind { engine: 23, base: 64.0, defaults: [0.5, 0.90, 0.7, 0.5, 0.5, 0.4, 0.6] },
];

const PARAMS: [ParamDescriptor; 4] = [
    ParamDescriptor::new("TUNE", 0.5, ParamUnit::Semitones { min: -12.0, max: 12.0 }),
    ParamDescriptor::new("DECAY", 0.45, ParamUnit::Percent),
    ParamDescriptor::new("TONE", 0.5, ParamUnit::Percent),
    ParamDescriptor::new("SNAP", 0.5, ParamUnit::Percent),
];
const SECONDARY: [ParamDescriptor; 4] = [
    ParamDescriptor::new("BODY", 0.5, ParamUnit::Percent),
    ParamDescriptor::new("LEVEL", 0.6, ParamUnit::Percent),
    ParamDescriptor::new("PAN", 0.5, ParamUnit::Percent),
    ParamDescriptor::new("SOUND", 0.5 / SOUNDS as f32, ParamUnit::Choice(&SOUND_NAMES)),
];

/// The sound a note plays.
pub fn sound_of(note: u8) -> usize {
    (note % 12) as usize % SOUNDS
}

struct Voice {
    voice: MacroVoice,
    /// A strike is due on the next chunk.
    strike: bool,
    velocity: f32,
    quiet_chunks: u32,
    awake: bool,
    /// 1 on a hit, falling away: what the drawing flashes.
    hit: f32,
}

pub struct Drums {
    values: [[f32; VALS]; SOUNDS],
    selected: usize,
    voices: Vec<Voice>,
    out: [f32; CHUNK],
    aux: [f32; CHUNK],
}

impl Default for Drums {
    fn default() -> Self {
        Self::new()
    }
}

impl Drums {
    pub fn new() -> Self {
        // The hats, snare and clap use random noise: start it from the same place every time.
        seed_noise(0x0123_4567);
        let mut values = [[0.0; VALS]; SOUNDS];
        let mut voices = Vec::with_capacity(SOUNDS);
        let (mut out, mut aux) = ([0.0; CHUNK], [0.0; CHUNK]);
        for (i, k) in KINDS.iter().enumerate() {
            values[i] = k.defaults;
            let mut voice = MacroVoice::new();
            // A first silent render fixes the engine, so the first strike lands on it.
            let p = MacroPatch { engine: k.engine, note: k.base, ..Default::default() };
            voice.render(&p, &mut out, &mut aux);
            voices.push(Voice { voice, strike: false, velocity: 1.0, quiet_chunks: 0, awake: false, hit: 0.0 });
        }
        Self { values, selected: 0, voices, out, aux }
    }

    pub fn selected(&self) -> usize {
        self.selected
    }
}

impl InstrumentCartridge for Drums {
    fn info(&self) -> InstrumentInfo {
        InstrumentInfo::new("drums", "Drum")
    }

    fn process(&mut self, output_l: &mut [f32], output_r: &mut [f32], events: &[NoteEvent]) {
        let n = output_l.len().min(output_r.len());
        let mut ev = events.iter().peekable();
        let mut start = 0;
        while start < n {
            let end = (start + CHUNK).min(n);
            // Events land on the chunk that contains them (≤ 0.25 ms late).
            while let Some(e) = ev.next_if(|e| (e.sample_offset as usize) < end) {
                if e.kind == NoteKind::On {
                    let v = &mut self.voices[sound_of(e.note)];
                    v.strike = true;
                    v.velocity = e.velocity.clamp(0.0, 1.0);
                    v.awake = true;
                    v.quiet_chunks = 0;
                    v.hit = 1.0;
                }
            }
            let len = end - start;
            output_l[start..end].fill(0.0);
            output_r[start..end].fill(0.0);
            for (i, v) in self.voices.iter_mut().enumerate() {
                v.hit *= 0.985;
                if !v.awake {
                    continue;
                }
                let (k, val) = (&KINDS[i], &self.values[i]);
                let patch = MacroPatch {
                    note: k.base + (val[0] - 0.5) * 24.0,
                    engine: k.engine,
                    harmonics: val[3],
                    timbre: val[2],
                    morph: val[4],
                    decay: val[1],
                    colour: 0.5,
                    trigger: std::mem::take(&mut v.strike),
                };
                v.voice.render(&patch, &mut self.out[..len], &mut self.aux[..len]);
                // Constant-power pan; the centre is unity.
                let a = val[6] * std::f32::consts::FRAC_PI_2;
                let (gl, gr) = (a.cos() * std::f32::consts::SQRT_2, a.sin() * std::f32::consts::SQRT_2);
                // Headroom: eight voices share one track, and the percussion engines are loud.
                let g = v.velocity * val[5] * 0.5;
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
            // Eight voices can add up: round the peaks off instead of clipping them.
            for j in start..end {
                output_l[j] = output_l[j].tanh();
                output_r[j] = output_r[j].tanh();
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
        let v = normalized_value.clamp(0.0, 1.0);
        match index {
            0..=6 => self.values[self.selected][index] = v,
            7 => self.selected = choice_index(v, SOUNDS),
            _ => {}
        }
    }
    fn parameter_value(&self, index: usize) -> Option<f32> {
        match index {
            0..=6 => Some(self.values[self.selected][index]),
            7 => Some((self.selected as f32 + 0.5) / SOUNDS as f32),
            _ => None,
        }
    }
    fn note_label(&self, note: u8) -> Option<&'static str> {
        Some(SOUND_NAMES[sound_of(note)])
    }
    fn all_notes_off(&mut self) {
        for v in &mut self.voices {
            v.strike = false;
            v.awake = false;
        }
    }

    /// The eight sounds as a 4 × 2 grid of keys, like the eight group keys, each
    /// with a small picture of the sound that flashes when it plays; the chosen
    /// one is outlined and drawn large on the right, where the four colours of
    /// knobs 5–8 show tune (height), decay (size), tone (rings) and snap (spikes).
    fn draw_ui(&self, frame: &mut FrameBuffer480x272, _accent_color: Rgb565) {
        use op8_gfx::{knob, paint::*, theme};
        let t = theme();
        let (x0, y0, cw, ch, gap) = (24.0, 76.0, 52.0, 52.0, 8.0);
        for i in 0..SOUNDS {
            let (x, y) = (x0 + (i % 4) as f32 * (cw + gap), y0 + (i / 4) as f32 * (ch + gap));
            let hit = self.voices[i].hit;
            let sel = i == self.selected;
            fill_round_rect(frame, x, y, cw, ch, 8.0, t.panel.lerp(t.fg, hit * 0.35));
            stroke_round_rect(frame, x, y, cw, ch, 8.0, if sel { 2.0 } else { 1.0 }, if sel { t.fg } else { t.faint });
            glyph(frame, i, x + cw / 2.0, y + ch / 2.0, 14.0, if sel { t.fg } else { t.dim }, hit);
        }
        // the chosen sound, large
        let v = &self.values[self.selected];
        let (cx, cy) = (372.0, 124.0);
        let hit = self.voices[self.selected].hit;
        let size = 22.0 + v[1] * 46.0 + hit * 6.0;
        let lift = (0.5 - v[0]) * 60.0;
        ring(frame, cx, cy, 78.0, 1.0, t.faint);
        // tone: concentric rings, more of them as it brightens
        for r in 0..=(v[2] * 3.0).round() as i32 {
            ring(frame, cx, cy + lift, size + 8.0 + r as f32 * 7.0, 1.0, knob(6));
        }
        // snap: spikes around the sound
        for s in 0..(2.0 + v[3] * 10.0).round() as i32 {
            let a = s as f32 / (2.0 + v[3] * 10.0).round() * std::f32::consts::TAU;
            let (r0, r1) = (size + 6.0, size + 14.0 + v[3] * 14.0);
            line_aa(frame, cx + a.cos() * r0, cy + lift + a.sin() * r0, cx + a.cos() * r1, cy + lift + a.sin() * r1, 1.5, knob(7));
        }
        glyph(frame, self.selected, cx, cy + lift, size * 0.7, t.fg, hit);
        // tune: a mark on the left edge that rides up and down with the pitch
        line_aa(frame, cx - 90.0, cy + lift, cx - 76.0, cy + lift, 2.0, knob(4));
        let _ = ui::AREA_Y;
    }

    fn save_state(&self) -> Vec<u8> {
        let mut all = Vec::with_capacity(SOUNDS * VALS + 1);
        for v in &self.values {
            all.extend_from_slice(v);
        }
        all.push((self.selected as f32 + 0.5) / SOUNDS as f32);
        state::encode_f32s(STATE_VERSION, &all)
    }
    fn load_state(&mut self, data: &[u8]) {
        let mut all = [0.0f32; SOUNDS * VALS + 1];
        if state::decode_f32s(STATE_VERSION, data, &mut all) {
            for (i, v) in self.values.iter_mut().enumerate() {
                for (j, x) in v.iter_mut().enumerate() {
                    *x = all[i * VALS + j].clamp(0.0, 1.0);
                }
            }
            self.selected = choice_index(all[SOUNDS * VALS], SOUNDS);
        }
    }
}

/// A small picture of each sound, centred on (cx, cy), `s` its half-size.
fn glyph(frame: &mut FrameBuffer480x272, i: usize, cx: f32, cy: f32, s: f32, c: Rgb565, hit: f32) {
    use op8_gfx::paint::*;
    let s = s * (1.0 + hit * 0.25);
    match i {
        0 => fill_circle(frame, cx, cy, s * 0.8, c),
        1 => {
            ring(frame, cx, cy, s * 0.8, 1.5, c);
            for k in -1..=1 {
                line_aa(frame, cx - s * 0.5, cy + k as f32 * s * 0.3, cx + s * 0.5, cy + k as f32 * s * 0.3, 1.2, c);
            }
        }
        2 => {
            line_aa(frame, cx - s * 0.8, cy, cx + s * 0.8, cy, 2.0, c);
            line_aa(frame, cx - s * 0.5, cy - s * 0.35, cx + s * 0.5, cy - s * 0.35, 1.2, c);
        }
        3 => {
            line_aa(frame, cx - s * 0.8, cy + s * 0.25, cx + s * 0.8, cy + s * 0.25, 2.0, c);
            line_aa(frame, cx - s * 0.8, cy - s * 0.35, cx + s * 0.8, cy - s * 0.35, 2.0, c);
        }
        4 => {
            ring(frame, cx, cy, s * 0.8, 1.5, c);
            fill_circle(frame, cx, cy, s * 0.35, c);
        }
        5 => {
            for k in -1..=1 {
                fill_circle(frame, cx + k as f32 * s * 0.5, cy + (k * k) as f32 * s * 0.25, s * 0.17, c);
            }
        }
        6 => line_aa(frame, cx - s * 0.6, cy + s * 0.5, cx + s * 0.6, cy - s * 0.5, 2.5, c),
        _ => {
            for k in 0..6 {
                let a = k as f32 / 6.0 * std::f32::consts::TAU;
                line_aa(frame, cx + a.cos() * s * 0.25, cy + a.sin() * s * 0.25, cx + a.cos() * s * 0.85, cy + a.sin() * s * 0.85, 1.5, c);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn energy(d: &mut Drums, note: u8) -> f32 {
        let (mut l, mut r) = (vec![0.0; 256], vec![0.0; 256]);
        d.process(&mut l, &mut r, &[NoteEvent::on(3, note, 1.0)]);
        let mut e: f32 = l.iter().map(|x| x * x).sum();
        for _ in 0..40 {
            d.process(&mut l, &mut r, &[]);
            e += l.iter().map(|x| x * x).sum::<f32>();
        }
        e
    }

    // F-A08
    #[test]
    fn every_sound_makes_sound_on_its_own_note() {
        for s in 0..SOUNDS {
            let mut d = Drums::new();
            let e = energy(&mut d, 36 + s as u8);
            assert!(e > 1e-3, "sound {s} ({}) silent", SOUND_NAMES[s]);
        }
    }

    // F-A08
    #[test]
    fn the_note_chooses_the_sound() {
        assert_eq!(sound_of(60), 0);
        assert_eq!(sound_of(61), 1);
        assert_eq!(sound_of(67), 7);
        assert_eq!(sound_of(68), 0, "above G the sounds wrap round");
        // only the chosen sound is heard: a snare note leaves the kick's voice asleep
        let mut d = Drums::new();
        energy(&mut d, 61);
        assert!(d.voices[1].awake && !d.voices[0].awake);
    }

    // F-A09
    #[test]
    fn knobs_edit_the_chosen_sound_and_each_keeps_its_values() {
        let mut d = Drums::new();
        d.set_parameter(1, 0.9); // kick decay
        d.set_parameter(7, 0.5 / 8.0 + 1.0 / 8.0); // select the snare
        assert_eq!(d.selected(), 1);
        assert_eq!(d.parameter_value(1), Some(0.40), "the snare has its own decay");
        d.set_parameter(1, 0.2);
        d.set_parameter(7, 0.0); // back to the kick
        assert_eq!(d.parameter_value(1), Some(0.9), "the kick kept its value");
    }

    // F-A10
    #[test]
    fn sleeping_voices_cost_nothing_and_the_sum_stays_bounded() {
        let mut d = Drums::new();
        let (mut l, mut r) = (vec![9.0; 512], vec![9.0; 512]);
        d.process(&mut l, &mut r, &[]);
        assert!(l.iter().chain(&r).all(|&s| s == 0.0), "silent without notes");
        let hits: Vec<NoteEvent> = (0..8).map(|i| NoteEvent::on(0, 36 + i, 1.0)).collect();
        d.process(&mut l, &mut r, &hits);
        assert!(l.iter().chain(&r).all(|s| s.is_finite() && s.abs() <= 1.0), "all eight at once stay within 1");
    }
}
