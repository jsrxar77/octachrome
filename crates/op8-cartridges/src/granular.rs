//! granular: plays a sound as a cloud of grains (docs/instruments.md §7).
//!
//! The sound is one of the sixteen places of the sampler's bank (the eight that come with the machine and the player's own). A note
//! starts a cloud of very short, overlapping grains cut from it; the notes that are held give the pitch of the grains in turn.
//! Page 1 chooses where and how: `sample`, `position` (where in the sound the grains are cut), `size` (how long each grain is) and
//! `density` (how many, and whether they come at regular intervals or by chance). Page 2 shapes the cloud: `tune`, `texture` (the
//! window of each grain, from a sharp little click to a soft swell), `spread` (how wide the grains are scattered between the
//! speakers) and `level`.
//!
//! The way grains are scheduled, shaped and added together follows the granular player of Clouds by Emilie Gillet (Mutable
//! Instruments, 2014), rewritten in Rust to play a stored sound instead of a live input, with the tables it used worked out here:
//! the density that is regular below the middle of the knob and random above it, with a dead zone in the middle where only a
//! note's own grain sounds; the grain window that is a trapezoid for a small `texture` and a raised cosine for a large one; the
//! grain cut down so that it never runs past the end of the sound; the gain that follows the number of grains that are sounding;
//! and the three qualities of interpolation, the cheaper ones used as the grains get busy. The original is MIT licensed:
//!
//! > Copyright 2014 Emilie Gillet. Permission is hereby granted, free of charge, to any person obtaining a copy of this software
//! > and associated documentation files (the "Software"), to deal in the Software without restriction, including without limitation
//! > the rights to use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of the Software, and to permit
//! > persons to whom the Software is furnished to do so, subject to the following conditions: The above copyright notice and this
//! > permission notice shall be included in all copies or substantial portions of the Software. THE SOFTWARE IS PROVIDED "AS IS",
//! > WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A
//! > PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR
//! > OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR
//! > THE USE OR OTHER DEALINGS IN THE SOFTWARE.

use crate::sampler::{self, Sample};
use op8_cartridge::{
    choice_index, state, FrameBuffer480x272, InstrumentCartridge, InstrumentInfo, NoteEvent, NoteKind, ParamDescriptor, ParamUnit,
    Rgb565, SAMPLE_RATE,
};
use std::f32::consts::FRAC_PI_2;

const PLACES: usize = 16;
const MAX_GRAINS: usize = 32;
/// The grains beyond this many (counting from the free ones) are the best quality; the busy ones use the cheaper interpolation.
const MEDIUM_FROM: usize = MAX_GRAINS * 3 / 4;
/// The notes that can be held at once; the grains take their pitch from them in turn.
const HELD: usize = 4;
const STATE_VERSION: u8 = 1;

const NAMES: [&str; PLACES] = [
    "pluck", "bell", "pad", "voice", "thump", "sweep", "glass", "glitch", "user 1", "user 2", "user 3", "user 4", "user 5", "user 6",
    "user 7", "user 8",
];

const PARAMS: [ParamDescriptor; 4] = [
    ParamDescriptor::new("SAMPLE", 2.5 / PLACES as f32, ParamUnit::Choice(&NAMES)),
    ParamDescriptor::new("POSITION", 0.3, ParamUnit::Percent),
    ParamDescriptor::new("SIZE", 0.5, ParamUnit::Millis { min: 32.0, max: 512.0 }),
    ParamDescriptor::new("DENSITY", 0.2, ParamUnit::Percent),
];
const SECONDARY: [ParamDescriptor; 4] = [
    ParamDescriptor::new("TUNE", 0.5, ParamUnit::Semitones { min: -24.0, max: 24.0 }),
    ParamDescriptor::new("TEXTURE", 0.6, ParamUnit::Percent),
    ParamDescriptor::new("SPREAD", 0.5, ParamUnit::Percent),
    ParamDescriptor::new("LEVEL", 0.8, ParamUnit::Percent),
];

/// A small, fixed random generator (xorshift) so that the same notes always make the same cloud.
struct Random(u32);
impl Random {
    fn float(&mut self) -> f32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 17;
        self.0 ^= self.0 << 5;
        (self.0 >> 8) as f32 / (1u32 << 24) as f32
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Quality {
    Medium,
    High,
}

/// One grain: a short piece of the sound played at its own pitch under a window.
#[derive(Clone, Copy, Default)]
struct Grain {
    active: bool,
    /// Where in the sound it starts, and how far it has got (both in samples of the sound).
    start: f64,
    phase: f64,
    step: f64,
    /// Its length in output samples, and how many it has played.
    width: f32,
    age: f32,
    smoothness: f32,
    slope: f32,
    gain_l: f32,
    gain_r: f32,
    quality: Option<Quality>,
}

impl Grain {
    fn begin(&mut self, start: f64, step: f64, width: f32, window: f32, pan: f32, gain: f32, quality: Quality) {
        self.active = true;
        self.start = start;
        self.phase = 0.0;
        self.step = step;
        self.width = width.max(2.0);
        self.age = 0.0;
        // a texture of half or more is a raised cosine that gets smoother up to 1, below it a trapezoid that gets steeper
        if window >= 0.5 {
            self.smoothness = (window - 0.5) * 2.0;
            self.slope = 0.0;
        } else {
            self.smoothness = 0.0;
            self.slope = 0.5 / (window + 0.01);
        }
        // equal power between the speakers
        self.gain_l = (pan * FRAC_PI_2).sin() * gain;
        self.gain_r = (pan * FRAC_PI_2).cos() * gain;
        self.quality = Some(quality);
    }

    /// The window at this point: a triangle that is bent into a trapezoid or a raised cosine. `None` when the grain is over.
    fn window(&self) -> Option<f32> {
        let p = 2.0 * self.age / self.width;
        if p >= 2.0 {
            return None;
        }
        let mut g = if p >= 1.0 { 2.0 - p } else { p };
        let quality = self.quality.unwrap_or(Quality::High);
        if self.smoothness > 0.0 {
            if quality == Quality::High {
                let hann = 0.5 - 0.5 * (std::f32::consts::PI * g).cos();
                g += self.smoothness * (hann - g);
            }
        } else if quality >= Quality::Medium {
            g = (g * self.slope).min(1.0);
        }
        Some(g)
    }

    fn read(&self, data: &[f32]) -> f32 {
        let pos = self.start + self.phase;
        let i = pos as usize;
        let f = (pos - i as f64) as f32;
        let at = |k: isize| data.get((i as isize + k).max(0) as usize).copied().unwrap_or(0.0);
        match self.quality.unwrap_or(Quality::High) {
            Quality::Medium => at(0) + (at(1) - at(0)) * f,
            Quality::High => {
                // four-point Hermite
                let (xm1, x0, x1, x2) = (at(-1), at(0), at(1), at(2));
                let c = (x1 - xm1) * 0.5;
                let v = x0 - x1;
                let w = c + v;
                let a = w + v + (x2 - x0) * 0.5;
                let b = w + a;
                ((a * f - b) * f + c) * f + x0
            }
        }
    }
}

pub struct Granular {
    values: [f32; 8],
    bank: Vec<Sample>,
    grains: [Grain; MAX_GRAINS],
    held: [Option<(u8, f32)>; HELD],
    /// The pitch wheel, in semitones: the grains that start take it.
    bend: f32,
    /// The next held note to give its pitch to a grain.
    turn: usize,
    random: Random,
    /// The level of the whole cloud while a note is held and as it fades.
    amp: f32,
    num_grains: f32,
    gain_norm: f32,
    grain_size_hint: f32,
    phasor: f32,
    trigger: bool,
}

impl Default for Granular {
    fn default() -> Self {
        Self::new()
    }
}

impl Granular {
    /// The eight sounds that come with the machine and the player's own from the samples folder.
    pub fn new() -> Self {
        let user = sampler::default_dir().map(|d| sampler::load_folder(&d)).unwrap_or_default();
        Self::with_user(user)
    }

    pub fn with_user(user: Vec<Sample>) -> Self {
        let mut bank = sampler::factory_bank();
        bank.extend(user.into_iter().take(PLACES - 8));
        while bank.len() < PLACES {
            bank.push(Sample::new(NAMES[bank.len()], Vec::new()));
        }
        let mut values = [0.0; 8];
        for (i, p) in PARAMS.iter().chain(SECONDARY.iter()).enumerate() {
            values[i] = p.default;
        }
        Self {
            values,
            bank,
            grains: [Grain::default(); MAX_GRAINS],
            held: [None; HELD],
            bend: 0.0,
            turn: 0,
            random: Random(0x2545_F491),
            amp: 0.0,
            num_grains: 0.0,
            gain_norm: 1.0,
            grain_size_hint: 1536.0,
            phasor: 0.0,
            trigger: false,
        }
    }

    fn chosen(&self) -> usize {
        choice_index(self.values[0], PLACES)
    }

    /// The length of a grain in output samples: 32 ms at the smallest `size`, 512 ms at the largest, doubling every quarter.
    fn grain_size(&self) -> f32 {
        1536.0 * 2f32.powf(4.0 * self.values[2])
    }

    /// `density` is two knobs in one: below the middle the grains come at regular intervals, above it by chance, and the further
    /// from the middle the more of them; in the middle (a little dead zone) only the grain that a note starts is heard.
    fn overlap(&self) -> (f32, bool) {
        let d = self.values[3];
        let overlap = if d >= 0.53 {
            (d - 0.53) * 2.12
        } else if d <= 0.47 {
            (0.47 - d) * 2.12
        } else {
            0.0
        };
        (overlap, d < 0.5)
    }

    fn texture(&self) -> f32 {
        self.values[5]
    }

    fn note_on(&mut self, note: u8, velocity: f32) {
        let slot = self.held.iter().position(|h| h.map(|(n, _)| n) == Some(note)).or_else(|| self.held.iter().position(Option::is_none));
        let slot = slot.unwrap_or_else(|| {
            // a fifth note takes the place of the first
            self.turn % HELD
        });
        self.held[slot] = Some((note, velocity));
        self.trigger = true;
    }

    fn note_off(&mut self, note: u8) {
        for h in self.held.iter_mut() {
            if h.map(|(n, _)| n) == Some(note) {
                *h = None;
            }
        }
    }

    /// The next held note's pitch (in semitones from the sound's own) and velocity, in turn.
    fn next_note(&mut self) -> Option<(f32, f32)> {
        for k in 0..HELD {
            let i = (self.turn + k) % HELD;
            if let Some((n, v)) = self.held[i] {
                self.turn = i + 1;
                return Some((n as f32 - 60.0, v));
            }
        }
        None
    }

    /// Start a grain at the position, with the size and pitch asked for.
    fn schedule(&mut self, quality: Quality) {
        let Some((semis, velocity)) = self.next_note() else { return };
        let data_len = self.bank.get(self.chosen()).map_or(0, Sample::len);
        if data_len < 64 {
            return;
        }
        let pitch = semis + self.bend + (self.values[4] - 0.5) * 48.0;
        let ratio = 2f32.powf(pitch / 12.0);
        let mut size = self.grain_size();
        // a grain never eats more than half of the sound, so that it ends before the sound does
        if size * ratio > data_len as f32 * 0.5 {
            size = data_len as f32 * 0.5 / ratio;
        }
        let eaten = size * ratio;
        let start = self.values[1] as f64 * (data_len as f32 - eaten).max(0.0) as f64;
        let pan = (0.5 + self.values[6] * (self.random.float() - 0.5)).clamp(0.0, 1.0);
        let width = (size as i32 & !1) as f32;
        let window = self.texture();
        if let Some(g) = self.grains.iter_mut().find(|g| !g.active) {
            g.begin(start, ratio as f64, width, window, pan, velocity, quality);
        }
        self.grain_size_hint += (size - self.grain_size_hint) * 0.1;
    }
}

impl InstrumentCartridge for Granular {
    fn info(&self) -> InstrumentInfo {
        InstrumentInfo::new("granular", "Granular")
    }

    fn process(&mut self, output_l: &mut [f32], output_r: &mut [f32], events: &[NoteEvent]) {
        let n = output_l.len().min(output_r.len());
        let level = self.values[7] * 0.8;
        let (overlap, deterministic) = self.overlap();
        let target = MAX_GRAINS as f32 * overlap * overlap * overlap;
        let spacing = if target > 0.0 { self.grain_size_hint / target } else { f32::INFINITY };
        let p = if deterministic { -1.0 } else { target / self.grain_size_hint };
        let window_gain = (1.0 + 2.0 * self.texture()).clamp(1.0, 2.0);
        let (attack, release) = (1.0 - (-1.0 / (0.008 * SAMPLE_RATE)).exp(), 1.0 - (-1.0 / (0.12 * SAMPLE_RATE)).exp());
        let data_of = |bank: &[Sample], i: usize| -> Option<std::sync::Arc<[f32]>> { bank.get(i).map(|s| s.data()) };
        let mut data = data_of(&self.bank, self.chosen());
        let mut ev = events.iter().peekable();
        for k in 0..n {
            while let Some(e) = ev.next_if(|e| e.sample_offset as usize <= k) {
                match e.kind {
                    NoteKind::On => self.note_on(e.note, e.velocity.clamp(0.0, 1.0)),
                    NoteKind::Off => self.note_off(e.note),
                }
            }
            let held = self.held.iter().any(Option::is_some);
            // when the player has not made a regular cloud, the clock is parked far from its next grain
            if deterministic {
                self.phasor += 1.0;
            } else {
                self.phasor = -1000.0;
            }
            let free = self.grains.iter().filter(|g| !g.active).count();
            if held && free > 0 {
                let seed_random = self.random.float() < p && target > self.num_grains;
                let seed_regular = self.phasor >= spacing;
                if seed_random || seed_regular || self.trigger {
                    let quality = if free < MAX_GRAINS - MEDIUM_FROM { Quality::Medium } else { Quality::High };
                    self.schedule(quality);
                    self.phasor = 0.0;
                    self.trigger = false;
                }
            }
            if data.is_none() {
                data = data_of(&self.bank, self.chosen());
            }
            // the cloud: the grains added up, then the gain that follows how many are sounding
            let (mut l, mut r) = (0.0f32, 0.0f32);
            let mut active = 0usize;
            if let Some(d) = data.as_ref() {
                for g in self.grains.iter_mut().filter(|g| g.active) {
                    let Some(w) = g.window() else {
                        g.active = false;
                        continue;
                    };
                    active += 1;
                    let s = g.read(d) * w;
                    l += s * g.gain_l;
                    r += s * g.gain_r;
                    g.phase += g.step;
                    g.age += 1.0;
                }
            }
            let target_grains = active as f32;
            self.num_grains += (if target_grains > self.num_grains { 0.9 } else { 0.2 }) * (target_grains - self.num_grains);
            let mut norm = if self.num_grains > 2.0 { 1.0 / (self.num_grains - 1.0).sqrt() } else { 1.0 };
            norm *= 1.0 + (window_gain - 1.0) * overlap;
            self.gain_norm += (norm - self.gain_norm) * 0.01;
            let want = if held { 1.0 } else { 0.0 };
            self.amp += (want - self.amp) * if want > self.amp { attack } else { release };
            let g = self.gain_norm * self.amp * level;
            output_l[k] = (l * g).tanh();
            output_r[k] = (r * g).tanh();
        }
        // a cloud that has faded is over: nothing is left running
        if !self.held.iter().any(Option::is_some) && self.amp < 1e-4 {
            self.amp = 0.0;
            for g in self.grains.iter_mut() {
                g.active = false;
            }
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
    fn load_sample(&mut self, place: usize, data: Vec<f32>) -> bool {
        if place >= PLACES - 8 {
            return false;
        }
        let k = 8 + place;
        if self.chosen() == k {
            for g in self.grains.iter_mut() {
                g.active = false;
            }
        }
        self.bank[k] = Sample::new(NAMES[k], data);
        true
    }
    fn all_notes_off(&mut self) {
        self.held = [None; HELD];
        self.amp = 0.0;
        for g in self.grains.iter_mut() {
            g.active = false;
        }
    }

    /// The wave of the chosen sound across the screen, with the stretch where grains are cut from (`position`, in the colour of
    /// encoder 6) as a band over it, and every grain that is sounding as a bar where it is reading: as wide as the grain is long
    /// (`size`, encoder 7), as tall as its window is open and higher or lower on the screen as it is placed between the speakers
    /// (`spread`). The pitch is a mark on a short scale under the wave (encoder 5 of the second page).
    fn draw_ui(&self, frame: &mut FrameBuffer480x272, accent_color: Rgb565) {
        use op8_gfx::{knob, paint::*, theme};
        let t = theme();
        let (x0, w, cy, amp) = (30.0f32, 420.0f32, 118.0f32, 56.0f32);
        let i = self.chosen();
        let Some(s) = self.bank.get(i).filter(|s| !s.is_empty()) else {
            line_aa(frame, x0, cy, x0 + w, cy, 1.0, t.faint);
            ring(frame, 240.0, cy, 26.0, 2.0, t.faint);
            return;
        };
        let len = s.len() as f32;
        // where a grain can start: from the beginning to the last place where a whole grain still fits
        let size = self.grain_size().min(len * 0.5);
        let reach = ((len - size).max(0.0) / len).clamp(0.0, 1.0);
        let at = self.values[1] * reach;
        fill_round_rect(frame, x0, cy - amp - 6.0, w * reach.max(0.02), amp * 2.0 + 12.0, 6.0, t.panel);
        line_aa(frame, x0, cy, x0 + w, cy, 1.0, t.faint);
        let col = w / s.peaks().len() as f32;
        for (c, &(lo, hi)) in s.peaks().iter().enumerate() {
            let x = x0 + c as f32 * col;
            let inside = x < x0 + w * reach;
            line_aa(frame, x, cy - hi * amp, x, cy - lo * amp - 0.5, 1.6, if inside { accent_color } else { t.faint });
        }
        // the position: a line where the cloud is cut from, and how much the grain takes in from there
        let xp = x0 + at * w;
        line_aa(frame, xp, cy - amp - 10.0, xp, cy + amp + 10.0, 2.0, knob(5));
        let xe = (xp + (size / len) * w).min(x0 + w);
        line_aa(frame, xp, cy + amp + 10.0, xe, cy + amp + 10.0, 2.0, knob(6));
        // every grain that sounds
        for g in self.grains.iter().filter(|g| g.active) {
            let Some(win) = g.window() else { continue };
            let x = x0 + ((g.start + g.phase) as f32 / len).clamp(0.0, 1.0) * w;
            let pan = g.gain_l.atan2(g.gain_r) / FRAC_PI_2; // 0 left .. 1 right
            let y = cy + (pan - 0.5) * 2.0 * (amp + 4.0);
            let half = 3.0 + 14.0 * win;
            line_aa(frame, x, y - half, x, y + half, (g.width / 1536.0 * 2.0).clamp(1.5, 5.0), t.fg);
        }
        // the pitch: a mark on a short scale under the wave
        let (px0, px1, py) = (170.0f32, 310.0f32, 204.0f32);
        line_aa(frame, px0, py, px1, py, 1.0, t.faint);
        fill_circle(frame, (px0 + px1) / 2.0, py, 2.0, t.dim);
        fill_circle(frame, px0 + (px1 - px0) * self.values[4], py, 5.0, knob(4));
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

    fn tone(hz: f32, secs: f32) -> Vec<f32> {
        (0..(secs * SAMPLE_RATE) as usize).map(|i| (std::f32::consts::TAU * hz * i as f32 / SAMPLE_RATE).sin()).collect()
    }

    /// An instrument whose place `user 1` is `data`, chosen, with these knobs: position, size, density, tune, texture, spread.
    fn with(data: Vec<f32>, knobs: [f32; 6]) -> Granular {
        let mut g = Granular::with_user(Vec::new());
        assert!(g.load_sample(0, data));
        g.set_parameter(0, (8.0 + 0.5) / PLACES as f32);
        for (i, k) in [1, 2, 3, 4, 5, 6].into_iter().zip(knobs) {
            g.set_parameter(i, k);
        }
        g
    }

    /// Hold `note` for `held` blocks of 512, then let go and play `after` more; the left and right outputs.
    fn play(g: &mut Granular, note: u8, held: usize, after: usize) -> (Vec<f32>, Vec<f32>) {
        let (mut l, mut r) = (vec![0.0; 512], vec![0.0; 512]);
        let (mut out_l, mut out_r) = (Vec::new(), Vec::new());
        for b in 0..held + after {
            let mut ev = Vec::new();
            if b == 0 {
                ev.push(NoteEvent::on(0, note, 1.0));
            }
            if b == held {
                ev.push(NoteEvent::off(0, note));
            }
            g.process(&mut l, &mut r, &ev);
            out_l.extend_from_slice(&l);
            out_r.extend_from_slice(&r);
        }
        (out_l, out_r)
    }

    fn rms(x: &[f32]) -> f32 {
        (x.iter().map(|v| v * v).sum::<f32>() / x.len().max(1) as f32).sqrt()
    }
    fn crossings(x: &[f32]) -> usize {
        x.windows(2).filter(|w| (w[0] < 0.0) != (w[1] < 0.0)).count()
    }

    // F-A21
    #[test]
    fn it_is_silent_until_a_note_and_a_note_makes_a_cloud_that_fades_when_let_go() {
        let mut g = with(tone(220.0, 1.0), [0.3, 0.5, 0.2, 0.5, 0.6, 0.5]);
        let (l, _) = play(&mut g, 60, 0, 4);
        assert!(l.iter().all(|&s| s == 0.0), "nothing without a note");
        let (l, _) = play(&mut g, 60, 40, 60);
        assert!(rms(&l[512 * 10..512 * 30]) > 0.02, "a held note sounds");
        assert!(rms(&l[512 * 95..]) < 1e-3, "and it fades after it is let go");
    }

    // F-A21
    #[test]
    fn position_chooses_where_in_the_sound_the_grains_are_cut() {
        // the sound is a low tone for its first half and a high one for the second
        let mut data = tone(110.0, 0.5);
        data.extend(tone(1760.0, 0.5));
        let (mut a, mut b) = (with(data.clone(), [0.0, 0.3, 0.2, 0.5, 0.6, 0.5]), with(data, [1.0, 0.3, 0.2, 0.5, 0.6, 0.5]));
        let (la, _) = play(&mut a, 60, 30, 0);
        let (lb, _) = play(&mut b, 60, 30, 0);
        assert!(crossings(&lb) > crossings(&la) * 4, "the end of the sound is higher than the beginning: {} vs {}", crossings(&lb), crossings(&la));
    }

    // F-A21: the middle of density is the dead zone where only the grain a note starts is heard; the sides are clouds.
    #[test]
    fn density_is_regular_on_one_side_random_on_the_other_and_a_single_grain_in_the_middle() {
        let sound = |d: f32| {
            let mut g = with(tone(220.0, 1.0), [0.3, 0.3, d, 0.5, 0.6, 0.5]);
            play(&mut g, 60, 60, 0).0
        };
        let (middle, regular, random) = (sound(0.5), sound(0.1), sound(0.9));
        let late = |x: &[f32]| rms(&x[512 * 40..]);
        assert!(late(&middle) < 1e-3, "in the middle a held note is one grain and then silence: {}", late(&middle));
        assert!(late(&regular) > 0.02 && late(&random) > 0.02, "either side keeps on sounding");
        assert!(rms(&middle[..2000]) > 0.02, "but the grain itself is heard");
        // further from the middle there are more grains (the level is kept the same: the gain follows how many sound)
        let grains = |d: f32| {
            let mut g = with(tone(220.0, 1.0), [0.3, 0.3, d, 0.5, 0.6, 0.5]);
            play(&mut g, 60, 40, 0);
            g.grains.iter().filter(|g| g.active).count()
        };
        assert!(grains(0.0) > grains(0.4) && grains(1.0) > grains(0.6), "{} {} {} {}", grains(0.0), grains(0.4), grains(0.6), grains(1.0));
    }

    // F-A21
    #[test]
    fn size_is_how_long_the_single_grain_lasts() {
        let long_of = |size: f32| {
            let mut g = with(tone(220.0, 1.0), [0.3, size, 0.5, 0.5, 0.6, 0.5]);
            let (l, _) = play(&mut g, 60, 60, 0);
            l.iter().rposition(|x| x.abs() > 1e-3).unwrap_or(0)
        };
        let (small, big) = (long_of(0.0), long_of(1.0));
        assert!(big > small * 5, "a bigger grain lasts longer: {big} vs {small}");
        assert!((small as f32 - 1536.0).abs() < 400.0, "the smallest is 32 ms: {small} samples");
    }

    // F-A21
    #[test]
    fn notes_set_the_pitch_of_the_grains_and_tune_moves_them_all() {
        // one grain at a time (the middle of density), so that what is counted is the grain and not grains interfering
        let one = |note: u8, tune: f32| {
            let mut g = with(tone(220.0, 1.0), [0.3, 0.5, 0.5, tune, 0.6, 0.5]);
            crossings(&play(&mut g, note, 20, 0).0[..6000])
        };
        let base = one(60, 0.5) as f32;
        assert!((one(72, 0.5) as f32 / base - 2.0).abs() < 0.15, "an octave up is twice the crossings");
        assert!((one(60, 0.75) as f32 / base - 2.0).abs() < 0.15, "tune +12 is an octave too");
        assert!((one(48, 0.5) as f32 / base - 0.5).abs() < 0.1, "and an octave down is half");
    }

    // F-A21
    #[test]
    fn spread_scatters_the_grains_between_the_speakers() {
        let mut centre = with(tone(220.0, 1.0), [0.3, 0.5, 0.1, 0.5, 0.6, 0.0]);
        let (l, r) = play(&mut centre, 60, 30, 0);
        assert!(l.iter().zip(&r).all(|(a, b)| (a - b).abs() < 1e-5), "no spread: both speakers the same");
        let mut wide = with(tone(220.0, 1.0), [0.3, 0.5, 0.1, 0.5, 0.6, 1.0]);
        let (l, r) = play(&mut wide, 60, 30, 0);
        let diff: f32 = l.iter().zip(&r).map(|(a, b)| (a - b).abs()).sum::<f32>() / l.len() as f32;
        assert!(diff > 0.01, "wide: the speakers differ ({diff})");
    }

    // F-A21
    #[test]
    fn the_same_notes_make_the_same_cloud_and_the_busiest_one_stays_in_bounds() {
        let run = || {
            let mut g = with(tone(220.0, 1.0), [0.3, 1.0, 1.0, 0.9, 0.0, 1.0]);
            let (mut l, mut r) = (vec![0.0; 512], vec![0.0; 512]);
            let mut out = Vec::new();
            for b in 0..100 {
                let ev: Vec<NoteEvent> = if b == 0 { (0..6).map(|k| NoteEvent::on(0, 48 + 4 * k, 1.0)).collect() } else { Vec::new() };
                g.process(&mut l, &mut r, &ev);
                assert!(l.iter().chain(&r).all(|s| s.is_finite() && s.abs() <= 1.0), "bounded and finite");
                out.extend_from_slice(&l);
            }
            out
        };
        assert_eq!(run(), run(), "no chance in it: the same notes, the same cloud");
    }

    // F-A21: a sound put in a place of the player's while it plays (what SAMPLING does) is the one that is heard.
    #[test]
    fn a_recorded_sound_can_be_put_in_a_place_while_it_plays() {
        let mut g = with(tone(220.0, 1.0), [0.3, 0.5, 0.1, 0.5, 0.6, 0.5]);
        let low = crossings(&play(&mut g, 60, 30, 0).0);
        assert!(g.load_sample(0, tone(880.0, 1.0)));
        let high = crossings(&play(&mut g, 60, 30, 0).0);
        assert!(high > low * 3, "the new sound is heard: {high} vs {low}");
        assert!(!g.load_sample(8, vec![0.0; 10]), "only the player's eight places take one");
    }

    // F-A21: the state is the eight knobs, and a project that has the instrument opens it as it was.
    #[test]
    fn an_empty_place_is_silent_and_the_drawing_shows_the_grains() {
        let mut g = Granular::with_user(Vec::new());
        g.set_parameter(0, (8.0 + 0.5) / PLACES as f32); // user 1, empty
        let (l, _) = play(&mut g, 60, 10, 0);
        assert!(l.iter().all(|&s| s == 0.0), "a place with nothing makes no sound");
        let mut g = with(tone(220.0, 1.0), [0.3, 0.5, 0.1, 0.5, 0.6, 0.5]);
        play(&mut g, 60, 10, 0);
        let lit = |g: &Granular| {
            let mut fb = FrameBuffer480x272::new();
            g.draw_ui(&mut fb, Rgb565::WHITE);
            fb.pixels().iter().filter(|&&p| p != 0).count()
        };
        let drawn = |g: &Granular| {
            let mut fb = FrameBuffer480x272::new();
            g.draw_ui(&mut fb, Rgb565::WHITE);
            fb.pixels().to_vec()
        };
        let busy = drawn(&g);
        assert!(lit(&g) > 1000);
        g.all_notes_off();
        assert_ne!(busy, drawn(&g), "the grains that sound are drawn");
    }

    // F-A21: the busiest cloud (32 grains, four notes) costs a small part of the time it makes: measured only in a release build.
    #[test]
    fn the_busiest_cloud_costs_little() {
        if cfg!(debug_assertions) {
            return;
        }
        let mut g = with(tone(220.0, 2.0), [0.3, 1.0, 1.0, 1.0, 0.9, 1.0]);
        let (mut l, mut r) = (vec![0.0; 128], vec![0.0; 128]);
        let on: Vec<NoteEvent> = (0..4).map(|k| NoteEvent::on(0, 48 + 4 * k, 1.0)).collect();
        g.process(&mut l, &mut r, &on);
        let t = std::time::Instant::now();
        for _ in 0..(10.0 * SAMPLE_RATE / 128.0) as usize {
            g.process(&mut l, &mut r, &[]);
        }
        let spent = t.elapsed().as_secs_f32();
        eprintln!("granular: ten seconds of the busiest cloud took {spent:.3} s");
        assert!(spent < 10.0 * 0.1, "{spent} s for ten seconds of sound");
    }
}
