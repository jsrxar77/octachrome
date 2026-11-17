//! sampler: plays recorded sounds with the keyboard and the steps (docs/instruments.md §7).
//!
//! Sixteen places, as everywhere in OP-8: eight sounds that come with the machine (made by the code
//! below, so there is nothing to install) and eight for the player's own WAV files, read from
//! `~/.config/op8/samples/` (`OP8_SAMPLES` overrides the folder) in the order of their names.
//! A note plays the chosen sound at its own pitch for C4 and shifted by the interval for any
//! other note, up to eight at once.
//!
//! Page 1 chooses and cuts the sound (`sample`, `start`, `length`, `tune`); page 2 shapes how it
//! begins and ends (`attack`, `release`, `loop`, `level`). One shot: the sound plays through and
//! fades over the last `release`. Loop: it repeats between `start` and the end of `length` for as
//! long as the note is held, and fades over `release` when it is let go. Slices (8 or 16): the part between
//! `start` and the end of `length` is cut into equal slices and a note plays one of them, as a kit made of
//! the sound: C3 (48) is the first slice, the next note the next one, and the notes repeat round after the last
//! slice; the slices keep their own pitch (`tune` moves them all).

use crate::ui;
use op8_cartridge::{
    choice_index, state, FrameBuffer480x272, InstrumentCartridge, InstrumentInfo, NoteEvent,
    NoteKind, ParamDescriptor, ParamUnit, Rgb565, SAMPLE_RATE,
};
use std::f32::consts::TAU;
use std::path::{Path, PathBuf};
use std::sync::Arc;

const VOICES: usize = 8;
const PLACES: usize = 16;
const FACTORY: usize = 8;
const STATE_VERSION: u8 = 3;
/// Columns of the waveform kept for the drawing.
const COLUMNS: usize = 240;
/// A window never gets shorter than this many samples.
const MIN_WINDOW: f64 = 256.0;

const NAMES: [&str; PLACES] = [
    "pluck", "bell", "pad", "voice", "thump", "sweep", "glass", "glitch", "user 1", "user 2", "user 3",
    "user 4", "user 5", "user 6", "user 7", "user 8",
];
const LOOP_NAMES: [&str; 5] = ["one shot", "loop", "slices 8", "slices 16", "hits"];

const PARAMS: [ParamDescriptor; 4] = [
    ParamDescriptor::new("SAMPLE", 0.5 / PLACES as f32, ParamUnit::Choice(&NAMES)),
    ParamDescriptor::new("START", 0.0, ParamUnit::Percent),
    ParamDescriptor::new("LENGTH", 1.0, ParamUnit::Percent),
    ParamDescriptor::new("TUNE", 0.5, ParamUnit::Semitones { min: -24.0, max: 24.0 }),
];
const SECONDARY: [ParamDescriptor; 4] = [
    ParamDescriptor::new("ATTACK", 0.0, ParamUnit::Millis { min: 0.5, max: 2000.0 }),
    ParamDescriptor::new("RELEASE", 0.3, ParamUnit::Millis { min: 5.0, max: 4000.0 }),
    ParamDescriptor::new("LOOP", 0.0, ParamUnit::Choice(&LOOP_NAMES)),
    ParamDescriptor::new("LEVEL", 0.8, ParamUnit::Percent),
];

/// A sound in the bank: its samples (mono, at the engine's rate) and a small picture of its wave.
#[derive(Clone)]
pub struct Sample {
    pub name: String,
    data: Arc<[f32]>,
    /// (lowest, highest) of each of `COLUMNS` slices, for the drawing.
    peaks: Vec<(f32, f32)>,
    /// Where the sound hits (a drum, a note starting), in samples and by how much each stands out: the places `hits` cuts at.
    onsets: Vec<(usize, f32)>,
}

/// The shortest slice `hits` makes, and the most slices it makes (the keyboard has 16 pads).
const MIN_SLICE: usize = 480;
const MAX_SLICES: usize = 16;

/// Where `data` hits: the places where the loudness (in 5 ms steps over 10 ms) jumps by 9 dB or more over what was just before,
/// at least 60 ms apart, each as the first sample that is loud for its step less a millisecond (so the attack is not clipped). The
/// strongest 32 are kept, in order of place, with their strength.
fn find_onsets(data: &[f32]) -> Vec<(usize, f32)> {
    const HOP: usize = 240;
    const WIN: usize = 480;
    const RISE_DB: f32 = 9.0;
    const GATE_DB: f32 = -45.0;
    const APART: usize = 12; // in steps: 60 ms
    if data.len() < WIN * 2 {
        return Vec::new();
    }
    let n = (data.len() - WIN) / HOP + 1;
    let level: Vec<f32> = (0..n)
        .map(|i| {
            let e = data[i * HOP..i * HOP + WIN].iter().map(|x| x * x).sum::<f32>() / WIN as f32;
            10.0 * (e + 1e-9).log10()
        })
        .collect();
    let mut cand: Vec<(usize, f32)> = (2..n)
        .filter_map(|i| {
            let rise = level[i] - level[i - 1].max(level[i - 2]);
            (rise >= RISE_DB && level[i] > GATE_DB).then_some((i, rise))
        })
        .collect();
    cand.sort_by(|a, b| b.1.total_cmp(&a.1));
    let mut kept: Vec<(usize, f32)> = Vec::new();
    for (i, rise) in cand {
        if kept.iter().all(|&(j, _)| j.abs_diff(i) >= APART) {
            kept.push((i, rise));
        }
    }
    kept.truncate(32);
    let mut out: Vec<(usize, f32)> = kept
        .into_iter()
        .map(|(i, rise)| {
            let seg = &data[i * HOP..i * HOP + WIN];
            let top = seg.iter().fold(0.0f32, |m, x| m.max(x.abs()));
            let first = seg.iter().position(|x| x.abs() >= 0.3 * top).unwrap_or(0);
            ((i * HOP + first).saturating_sub(48), rise)
        })
        .collect();
    out.sort_by_key(|o| o.0);
    out
}

impl Sample {
    pub fn new(name: impl Into<String>, mut data: Vec<f32>) -> Self {
        // the loudest point is 0.9, so that every sound sits at the same level
        let peak = data.iter().fold(0.0f32, |m, x| m.max(x.abs()));
        if peak > 1e-6 {
            let g = 0.9 / peak;
            data.iter_mut().for_each(|x| *x *= g);
        }
        let n = data.len().max(1);
        let peaks = (0..COLUMNS)
            .map(|c| {
                let (a, b) = (c * n / COLUMNS, ((c + 1) * n / COLUMNS).max(c * n / COLUMNS + 1).min(n));
                data.get(a..b).unwrap_or(&[]).iter().fold((0.0f32, 0.0f32), |(lo, hi), &x| (lo.min(x), hi.max(x)))
            })
            .collect();
        let onsets = find_onsets(&data);
        Self { name: name.into(), data: data.into(), peaks, onsets }
    }
    /// The edges of the slices `hits` makes of the window `start..end`: each slice runs from one hit to the next, the last to the end
    /// of the window, and what comes before the first hit is not a slice. The strongest hits (up to sixteen) that leave 10 ms to the
    /// end are used, and one that falls within 10 ms before the start counts as at the start. A window with no hits is one slice.
    pub fn hit_edges(&self, start: f64, end: f64) -> Vec<f64> {
        let mut inside: Vec<(usize, f32)> = self
            .onsets
            .iter()
            .copied()
            .filter(|&(p, _)| p as f64 + MIN_SLICE as f64 >= start && p as f64 + MIN_SLICE as f64 <= end)
            .collect();
        inside.sort_by(|a, b| b.1.total_cmp(&a.1));
        inside.truncate(MAX_SLICES);
        inside.sort_by_key(|o| o.0);
        let mut edges: Vec<f64> = inside.into_iter().map(|(p, _)| (p as f64).max(start)).collect();
        edges.dedup();
        if edges.is_empty() {
            edges.push(start);
        }
        edges.push(end);
        edges
    }
    pub fn len(&self) -> usize {
        self.data.len()
    }
    /// The samples (shared, not copied).
    pub fn data(&self) -> Arc<[f32]> {
        self.data.clone()
    }
    /// (lowest, highest) of each slice of the wave, for a drawing.
    pub fn peaks(&self) -> &[(f32, f32)] {
        &self.peaks
    }
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }
}

// ---- the eight that come with the machine ----------------------------------------------------

struct Noise(u32);
impl Noise {
    fn next(&mut self) -> f32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 17;
        self.0 ^= self.0 << 5;
        self.0 as f32 / u32::MAX as f32 * 2.0 - 1.0
    }
}

fn secs(s: f32) -> usize {
    (s * SAMPLE_RATE) as usize
}

/// A plucked string: a burst of noise round a short delay line (Karplus–Strong), about A3.
fn make_pluck() -> Vec<f32> {
    let period = (SAMPLE_RATE / 220.0) as usize;
    let mut line: Vec<f32> = {
        let mut n = Noise(0x1234_5678);
        (0..period).map(|_| n.next()).collect()
    };
    let mut out = Vec::with_capacity(secs(1.4));
    for i in 0..secs(1.4) {
        let j = i % period;
        let next = (line[j] + line[(j + 1) % period]) * 0.5 * 0.998;
        out.push(line[j]);
        line[j] = next;
    }
    out
}

/// A struck bell: inharmonic partials, the high ones dying first.
fn make_bell() -> Vec<f32> {
    let partials = [(1.0, 1.0, 1.6), (2.76, 0.6, 1.1), (5.4, 0.4, 0.7), (8.93, 0.25, 0.45), (11.34, 0.15, 0.3)];
    (0..secs(2.0))
        .map(|i| {
            let t = i as f32 / SAMPLE_RATE;
            partials.iter().map(|&(r, a, d)| (TAU * 392.0 * r * t).sin() * a * (-t / d * 3.0).exp()).sum()
        })
        .collect()
}

/// A slow pad: a few detuned waves that swell and fade.
fn make_pad() -> Vec<f32> {
    let n = secs(2.4);
    (0..n)
        .map(|i| {
            let t = i as f32 / SAMPLE_RATE;
            let env = (t / 0.6).min(1.0) * ((n - i) as f32 / SAMPLE_RATE / 0.8).min(1.0);
            let w: f32 = [-0.07f32, 0.0, 0.06, 0.13].iter().map(|d| {
                let f = 196.0 * (1.0 + d * 0.01 * 10.0);
                let p = (f * t).fract();
                (2.0 * p - 1.0) * 0.5 + (TAU * f * 2.0 * t).sin() * 0.3
            }).sum();
            w * env
        })
        .collect()
}

/// A sung "ah": a pulse train through three resonances (the formants of that vowel).
fn make_voice() -> Vec<f32> {
    let n = secs(1.2);
    let formants = [(800.0f32, 1.0f32), (1150.0, 0.5), (2900.0, 0.25)];
    let mut state = [[0.0f32; 2]; 3];
    (0..n)
        .map(|i| {
            let t = i as f32 / SAMPLE_RATE;
            let vib = 1.0 + 0.012 * (TAU * 5.0 * t).sin();
            let pulse = if (130.0 * vib * t).fract() < 0.05 { 1.0 } else { 0.0 };
            let mut out = 0.0;
            for (k, &(f, g)) in formants.iter().enumerate() {
                let (r, w) = (0.985f32, TAU * f / SAMPLE_RATE);
                let y = pulse + 2.0 * r * w.cos() * state[k][0] - r * r * state[k][1];
                state[k][1] = state[k][0];
                state[k][0] = y;
                out += y * g * 0.05;
            }
            let env = (t / 0.08).min(1.0) * ((n - i) as f32 / SAMPLE_RATE / 0.3).min(1.0);
            out * env
        })
        .collect()
}

/// A low thump: a sine that falls fast, with a little noise at the start.
fn make_thump() -> Vec<f32> {
    let mut n = Noise(0x9E37_79B9);
    let mut phase = 0.0f32;
    (0..secs(0.7))
        .map(|i| {
            let t = i as f32 / SAMPLE_RATE;
            let f = 42.0 + 110.0 * (-t * 22.0).exp();
            phase = (phase + f / SAMPLE_RATE).fract();
            (TAU * phase).sin() * (-t * 5.0).exp() + n.next() * 0.3 * (-t * 90.0).exp()
        })
        .collect()
}

/// A riser: noise that gets brighter and louder, and stops at its peak.
fn make_sweep() -> Vec<f32> {
    let n = secs(1.6);
    let mut noise = Noise(0x0BAD_CAFE);
    let mut lp = 0.0f32;
    (0..n)
        .map(|i| {
            let u = i as f32 / n as f32;
            let k = 0.02 + 0.9 * u * u;
            lp += (noise.next() - lp) * k;
            lp * (0.1 + 0.9 * u) * (((n - i) as f32 / 400.0).min(1.0))
        })
        .collect()
}

/// A glassy tone: two sines that modulate each other, dying out.
fn make_glass() -> Vec<f32> {
    (0..secs(1.3))
        .map(|i| {
            let t = i as f32 / SAMPLE_RATE;
            let m = (TAU * 1320.0 * t).sin() * 2.2 * (-t * 3.0).exp();
            ((TAU * 660.0 * t) + m).sin() * (-t * 2.2).exp()
        })
        .collect()
}

/// Four little bursts of noise and pulses, each a bit quieter: a glitch to chop up.
fn make_glitch() -> Vec<f32> {
    let n = secs(0.6);
    let mut noise = Noise(0x1357_9BDF);
    (0..n)
        .map(|i| {
            let t = i as f32 / SAMPLE_RATE;
            let hit = (t / 0.15).floor();
            let local = t - hit * 0.15;
            let env = (-local * 45.0).exp() * (1.0 - hit * 0.2);
            let tone = if (local * (900.0 + hit * 400.0)).fract() < 0.5 { 0.5 } else { -0.5 };
            (noise.next() * 0.6 + tone) * env
        })
        .collect()
}

/// The eight sounds that come with the machine.
pub fn factory_bank() -> Vec<Sample> {
    let makers: [fn() -> Vec<f32>; FACTORY] =
        [make_pluck, make_bell, make_pad, make_voice, make_thump, make_sweep, make_glass, make_glitch];
    makers.iter().zip(NAMES).map(|(m, name)| Sample::new(name, m())).collect()
}

/// The name a recording is saved under: `rec-1.wav` is the first of the player's places, `rec-8.wav` the last.
pub fn recording_file(place: usize) -> String {
    format!("rec-{}.wav", place + 1)
}

/// A WAV file of the player's folder that was read, and the sound in it.
pub struct Found {
    pub path: PathBuf,
    pub sample: Sample,
}

/// The WAV files of a folder as the eight places of the player's own. A file called `rec-N.wav` (what
/// SAMPLING saves) is place N; the others fill the places that are left, in the order of their names.
/// A place with nothing is `None`; a file that cannot be read does not use up a place.
pub fn scan_folder(dir: &Path) -> Vec<Option<Found>> {
    let mut places: Vec<Option<Found>> = (0..PLACES - FACTORY).map(|_| None).collect();
    let Ok(rd) = std::fs::read_dir(dir) else { return places };
    let mut files: Vec<PathBuf> = rd
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|x| x.eq_ignore_ascii_case("wav")))
        .collect();
    files.sort();
    let read = |p: &PathBuf| -> Option<Found> {
        let data = crate::wav::parse(&std::fs::read(p).ok()?)?;
        Some(Found { path: p.clone(), sample: Sample::new(p.file_stem()?.to_string_lossy().to_string(), data) })
    };
    let mut others = Vec::new();
    for p in &files {
        let name = p.file_name().map(|n| n.to_string_lossy().to_lowercase()).unwrap_or_default();
        match (0..PLACES - FACTORY).find(|&k| name == recording_file(k)) {
            Some(k) => places[k] = read(p),
            None => others.push(p),
        }
    }
    let mut others = others.into_iter();
    for slot in places.iter_mut().filter(|s| s.is_none()) {
        *slot = others.by_ref().find_map(read);
    }
    places
}

/// The sounds of the player's own places (empty ones where there is nothing).
pub fn load_folder(dir: &Path) -> Vec<Sample> {
    fill(scan_folder(dir).into_iter().map(|f| f.map(|f| f.sample)).collect())
}

fn fill(places: Vec<Option<Sample>>) -> Vec<Sample> {
    places.into_iter().enumerate().map(|(k, s)| s.unwrap_or_else(|| Sample::new(NAMES[FACTORY + k], Vec::new()))).collect()
}

/// Where the player's samples are: `OP8_SAMPLES`, else `~/.config/op8/samples`.
pub fn default_dir() -> Option<PathBuf> {
    if let Some(d) = std::env::var_os("OP8_SAMPLES") {
        return Some(PathBuf::from(d));
    }
    let base = std::env::var_os("XDG_CONFIG_HOME").map(PathBuf::from).or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))?;
    Some(base.join("op8").join("samples"))
}

// ---- the instrument ----------------------------------------------------------------------------

#[derive(Clone, Copy, Default)]
struct Voice {
    active: bool,
    sample: usize,
    pos: f64,
    step: f64,
    start: f64,
    end: f64,
    looped: bool,
    gain: f32,
    env: f32,
    releasing: bool,
    /// Samples of fade left at the end (one shot) is worked out from `release` each block.
    age: u64,
    note: u8,
}

pub struct Sampler {
    values: [f32; 8],
    bank: Vec<Sample>,
    voices: [Voice; VOICES],
    clock: u64,
    scope: crate::dsp::Scope,
}

impl Default for Sampler {
    fn default() -> Self {
        Self::new()
    }
}

impl Sampler {
    /// The eight that come with the machine and the player's own from the samples folder.
    pub fn new() -> Self {
        let user = default_dir().map(|d| load_folder(&d)).unwrap_or_default();
        Self::with_user(user)
    }

    /// The eight that come with the machine and these as the player's own.
    pub fn with_user(user: Vec<Sample>) -> Self {
        let mut bank = factory_bank();
        bank.extend(user.into_iter().take(PLACES - FACTORY));
        while bank.len() < PLACES {
            bank.push(Sample::new(NAMES[bank.len()], Vec::new()));
        }
        let mut values = [0.0; 8];
        for (i, p) in PARAMS.iter().chain(SECONDARY.iter()).enumerate() {
            values[i] = p.default;
        }
        Self { values, bank, voices: [Voice::default(); VOICES], clock: 0, scope: crate::dsp::Scope::new(4) }
    }

    fn chosen(&self) -> usize {
        choice_index(self.values[0], PLACES)
    }

    /// What the `loop` knob chose: play once, loop, or cut the window into this many slices.
    fn slices(&self) -> Option<usize> {
        match choice_index(self.values[6], LOOP_NAMES.len()) {
            2 => Some(8),
            3 => Some(16),
            _ => None,
        }
    }

    /// `loop` is on `hits`: the window is cut where the sound hits.
    fn hits(&self) -> bool {
        choice_index(self.values[6], LOOP_NAMES.len()) == 4
    }

    /// The knob value that is the same position of `loop` as `v` was when it had four.
    fn loop_from_four(v: f32) -> f32 {
        match choice_index(v, 4) {
            0 => 0.0,
            i => (i as f32 + 0.5) / LOOP_NAMES.len() as f32,
        }
    }

    fn looping(&self) -> bool {
        choice_index(self.values[6], LOOP_NAMES.len()) == 1
    }

    /// The semitones `tune` adds: -24 to +24.
    fn tune_semitones(&self) -> f32 {
        -24.0 + 48.0 * self.values[3]
    }

    /// The window of the chosen sound that plays: `start` to `start + length` of what is left.
    fn window(&self, len: usize) -> (f64, f64) {
        let len = len as f64;
        let start = (self.values[1] as f64 * len).min((len - MIN_WINDOW).max(0.0));
        let end = (start + (0.01 + 0.99 * self.values[2] as f64) * (len - start)).min(len);
        (start, end.max(start + MIN_WINDOW.min(len - start)))
    }

    fn note_on(&mut self, note: u8, velocity: f32) {
        let i = self.chosen();
        let Some(sample) = self.bank.get(i).filter(|s| !s.is_empty()) else { return };
        let (mut start, mut end) = self.window(sample.len());
        let mut st = self.tune_semitones() + (note as f32 - 60.0);
        if let Some(n) = self.slices() {
            // a kit made of the sound: the note chooses the slice and the slice keeps its own pitch
            let k = (note as i32 - 48).rem_euclid(n as i32) as f64;
            let width = (end - start) / n as f64;
            (start, end) = (start + k * width, start + (k + 1.0) * width);
            st = self.tune_semitones();
        } else if self.hits() {
            // the same, cut where the sound hits: a slice runs from one hit to the next
            let edges = sample.hit_edges(start, end);
            let k = (note as i32 - 48).rem_euclid(edges.len() as i32 - 1) as usize;
            (start, end) = (edges[k], edges[k + 1]);
            st = self.tune_semitones();
        }
        self.clock += 1;
        let slot = self.voices.iter().position(|v| !v.active).unwrap_or_else(|| (0..VOICES).min_by_key(|&k| self.voices[k].age).unwrap_or(0));
        self.voices[slot] = Voice {
            active: true,
            sample: i,
            pos: start,
            step: 2f64.powf(st as f64 / 12.0),
            start,
            end,
            looped: self.looping(),
            gain: velocity,
            env: 0.0,
            releasing: false,
            age: self.clock,
            note,
        };
    }

    /// Put a sound in one of the player's eight places (0..8) while the instrument plays.
    pub fn load_place(&mut self, place: usize, data: Vec<f32>) -> bool {
        if place >= PLACES - FACTORY {
            return false;
        }
        let k = FACTORY + place;
        for v in self.voices.iter_mut().filter(|v| v.sample == k) {
            v.active = false;
        }
        self.bank[k] = Sample::new(NAMES[k], data);
        true
    }

    fn note_off(&mut self, note: u8) {
        // a sound that is held in a loop fades when the key is let go; a one shot plays through
        for v in self.voices.iter_mut().filter(|v| v.active && v.looped && v.note == note && !v.releasing) {
            v.releasing = true;
        }
    }
}

impl InstrumentCartridge for Sampler {
    fn info(&self) -> InstrumentInfo {
        InstrumentInfo::new("sampler", "Sampler")
    }

    fn process(&mut self, output_l: &mut [f32], output_r: &mut [f32], events: &[NoteEvent]) {
        let n = output_l.len().min(output_r.len());
        let attack = 1.0 / (0.5 * (2000.0f32 / 0.5).powf(self.values[4]) / 1000.0 * SAMPLE_RATE);
        let release_samples = 5.0 * (4000.0f32 / 5.0).powf(self.values[5]) / 1000.0 * SAMPLE_RATE;
        let fall = 1.0 / release_samples;
        let level = self.values[7] * 0.8;
        let mut ev = events.iter().peekable();
        for k in 0..n {
            while let Some(e) = ev.next_if(|e| e.sample_offset as usize <= k) {
                match e.kind {
                    NoteKind::On => self.note_on(e.note, e.velocity.clamp(0.0, 1.0)),
                    NoteKind::Off => self.note_off(e.note),
                }
            }
            let mut mix = 0.0f32;
            for v in self.voices.iter_mut().filter(|v| v.active) {
                let data = &self.bank[v.sample].data;
                if v.pos >= v.end {
                    if v.looped {
                        v.pos = v.start + (v.pos - v.end) % (v.end - v.start).max(1.0);
                    } else {
                        v.active = false;
                        continue;
                    }
                }
                // a one shot starts to fade `release` before the end of its window
                if !v.looped && !v.releasing && (v.end - v.pos) <= release_samples as f64 * v.step {
                    v.releasing = true;
                }
                if v.releasing {
                    v.env -= fall;
                    if v.env <= 0.0 {
                        v.active = false;
                        continue;
                    }
                } else if v.env < 1.0 {
                    v.env = (v.env + attack).min(1.0);
                }
                let i = v.pos as usize;
                let f = (v.pos - i as f64) as f32;
                let a = data.get(i).copied().unwrap_or(0.0);
                let b = data.get(i + 1).copied().unwrap_or(0.0);
                mix += (a + (b - a) * f) * v.env * v.gain;
                v.pos += v.step;
            }
            let s = (mix * level).tanh();
            output_l[k] = s;
            output_r[k] = s;
            self.scope.push(s);
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
    fn load_sample(&mut self, place: usize, data: Vec<f32>) -> bool {
        self.load_place(place, data)
    }
    fn all_notes_off(&mut self) {
        for v in &mut self.voices {
            v.active = false;
        }
    }

    /// The wave of the chosen sound across the screen, with the part that plays lit and its two ends marked
    /// (`start` in the colour of encoder 6, `length` in that of 7), a mark for the pitch (encoder 8), and a white
    /// line for every voice that is playing. A place with nothing in it is drawn as an empty line.
    fn draw_ui(&self, frame: &mut FrameBuffer480x272, accent_color: Rgb565) {
        use op8_gfx::{knob, paint::*, theme};
        let t = theme();
        let (x0, w, cy, amp) = (30.0f32, 420.0f32, 124.0f32, 62.0f32);
        let i = self.chosen();
        let sample = self.bank.get(i).filter(|s| !s.is_empty());
        let Some(s) = sample else {
            line_aa(frame, x0, cy, x0 + w, cy, 1.0, t.faint);
            ring(frame, 240.0, cy, 26.0, 2.0, t.faint);
            line_aa(frame, 222.0, cy + 18.0, 258.0, cy - 18.0, 2.0, t.faint);
            return;
        };
        let (start, end) = self.window(s.len());
        let (u0, u1) = (start as f32 / s.len() as f32, end as f32 / s.len() as f32);
        let (xs, xe) = (x0 + u0 * w, x0 + u1 * w);
        fill_round_rect(frame, xs, cy - amp - 6.0, (xe - xs).max(2.0), amp * 2.0 + 12.0, 6.0, t.panel);
        line_aa(frame, x0, cy, x0 + w, cy, 1.0, t.faint);
        let col = w / COLUMNS as f32;
        for (c, &(lo, hi)) in s.peaks.iter().enumerate() {
            let x = x0 + c as f32 * col;
            let inside = x + col > xs && x < xe;
            line_aa(frame, x, cy - hi * amp, x, cy - lo * amp - 0.5, 1.6, if inside { accent_color } else { t.faint });
        }
        line_aa(frame, xs, cy - amp - 10.0, xs, cy + amp + 10.0, 2.0, knob(5));
        line_aa(frame, xe, cy - amp - 10.0, xe, cy + amp + 10.0, 2.0, knob(6));
        // the pitch: a mark on a short scale under the wave
        let (px0, px1, py) = (170.0f32, 310.0f32, 206.0f32);
        line_aa(frame, px0, py, px1, py, 1.0, t.faint);
        fill_circle(frame, (px0 + px1) / 2.0, py, 2.0, t.dim);
        fill_circle(frame, px0 + (px1 - px0) * self.values[3], py, 5.0, knob(7));
        // every voice that is playing
        for v in self.voices.iter().filter(|v| v.active && v.sample == i) {
            let x = x0 + (v.pos as f32 / s.len() as f32) * w;
            line_aa(frame, x, cy - amp - 12.0, x, cy + amp + 12.0, 2.0, t.fg);
        }
        if let Some(n) = self.slices() {
            // slices: a tick between each, and the one that sounds is the lit one
            for k in 1..n {
                let x = xs + (xe - xs) * k as f32 / n as f32;
                line_aa(frame, x, cy - amp - 6.0, x, cy + amp + 6.0, 1.0, knob(6));
            }
        }
        if self.hits() {
            // hits: a tick at each place the sound is cut, so the picture says where the notes will start
            let edges = s.hit_edges(start, end);
            for &e in &edges[..edges.len() - 1] {
                let x = x0 + (e as f32 / s.len() as f32) * w;
                line_aa(frame, x, cy - amp - 6.0, x, cy + amp + 6.0, 1.0, knob(6));
            }
        }
        if self.looping() {
            // loop: two small arrows turning round, over the ends of the window
            ring(frame, xe, cy - amp - 16.0, 4.0, 1.5, knob(6));
            ring(frame, xs, cy - amp - 16.0, 4.0, 1.5, knob(5));
        }
        let _ = ui::clock;
    }

    fn save_state(&self) -> Vec<u8> {
        state::encode_f32s(STATE_VERSION, &self.values)
    }
    fn load_state(&mut self, data: &[u8]) {
        if state::decode_f32s(STATE_VERSION, data, &mut self.values) {
            return;
        }
        // version 2: `loop` had four positions and now has five (`hits` is the last): the same position as before
        let mut old = self.values;
        if state::decode_f32s(2, data, &mut old) {
            old[6] = Self::loop_from_four(old[6]);
            self.values = old;
            return;
        }
        // version 1: `tune` was ±12 semitones and `loop` was off or on; the same sound as before
        if state::decode_f32s(1, data, &mut old) {
            old[3] = 0.25 + 0.5 * old[3];
            old[6] = if old[6] >= 0.5 { Self::loop_from_four(0.375) } else { 0.0 };
            self.values = old;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use op8_cartridge::MAX_BLOCK;

    /// Play one note and return the output, `blocks` blocks of 512.
    fn play(s: &mut Sampler, note: u8, off_at: Option<u32>, blocks: usize) -> Vec<f32> {
        let (mut l, mut r) = (vec![0.0; 512], vec![0.0; 512]);
        let mut out = Vec::new();
        for b in 0..blocks {
            let mut ev = Vec::new();
            if b == 0 {
                ev.push(NoteEvent::on(0, note, 1.0));
            }
            if let Some(o) = off_at {
                if (o as usize) / 512 == b {
                    ev.push(NoteEvent::off(o % 512, note));
                }
            }
            s.process(&mut l, &mut r, &ev);
            out.extend_from_slice(&l);
        }
        out
    }

    fn last_sound(out: &[f32]) -> usize {
        out.iter().rposition(|x| x.abs() > 1e-4).unwrap_or(0)
    }

    fn choose(s: &mut Sampler, place: usize) {
        s.set_parameter(0, (place as f32 + 0.5) / PLACES as f32);
    }

    // F-A16
    #[test]
    fn the_eight_that_come_with_the_machine_exist_and_differ() {
        let bank = factory_bank();
        assert_eq!(bank.len(), 8);
        for s in &bank {
            let peak = s.data.iter().fold(0.0f32, |m, x| m.max(x.abs()));
            assert!(s.len() > 4800 && (0.85..=0.91).contains(&peak), "{}: {} samples, peak {peak}", s.name, s.len());
            assert!(s.data.iter().all(|x| x.is_finite()));
        }
        for i in 0..8 {
            for j in i + 1..8 {
                assert!(bank[i].data.len() != bank[j].data.len() || bank[i].data[1000] != bank[j].data[1000], "{} and {}", bank[i].name, bank[j].name);
            }
        }
        assert_eq!(bank, factory_bank(), "made the same way every time");
    }

    // F-A16
    #[test]
    fn a_note_plays_the_sound_once_at_its_own_pitch_and_an_octave_up_is_twice_as_fast() {
        let mut s = Sampler::with_user(vec![]);
        choose(&mut s, 1); // the bell, 2 s
        s.set_parameter(5, 0.0); // a short release
        let plain = play(&mut s, 60, None, 400);
        let len = s.bank[1].len();
        assert!(plain[..2000].iter().any(|x| x.abs() > 0.05), "it sounds");
        let end = last_sound(&plain);
        assert!((end as i64 - len as i64).abs() < 600, "plays for its own length: {end} of {len}");
        let mut s = Sampler::with_user(vec![]);
        choose(&mut s, 1);
        s.set_parameter(5, 0.0);
        let up = play(&mut s, 72, None, 400);
        let end_up = last_sound(&up);
        assert!((end_up as f64 - len as f64 / 2.0).abs() < 600.0, "an octave up takes half the time: {end_up}");
    }

    // F-A16
    #[test]
    fn start_and_length_cut_the_sound() {
        let mut s = Sampler::with_user(vec![]);
        choose(&mut s, 1);
        s.set_parameter(5, 0.0);
        s.set_parameter(1, 0.25); // start a quarter of the way in
        s.set_parameter(2, 0.5); // half of what is left
        let out = play(&mut s, 60, None, 400);
        let len = s.bank[1].len() as f64;
        let want = (len * 0.75) * (0.01 + 0.99 * 0.5);
        assert!((last_sound(&out) as f64 - want).abs() < 800.0, "{} vs {want}", last_sound(&out));
    }

    // F-A16
    #[test]
    fn a_loop_keeps_going_while_the_key_is_held_and_fades_when_it_is_let_go() {
        let mut s = Sampler::with_user(vec![]);
        choose(&mut s, 2); // the pad
        s.set_parameter(6, 0.375); // loop
        s.set_parameter(2, 0.1); // a short window
        s.set_parameter(5, 0.3);
        let out = play(&mut s, 60, Some(60_000), 400);
        assert!(out[40_000..58_000].iter().any(|x| x.abs() > 0.05), "still sounding after 2 s");
        assert!(last_sound(&out) > 60_000, "it fades after the note-off, not before");
        assert!(out[150_000..].iter().all(|x| x.abs() < 1e-3), "and then it is silent");
    }

    // F-A16
    #[test]
    fn a_one_shot_ignores_the_note_off() {
        let mut s = Sampler::with_user(vec![]);
        choose(&mut s, 1);
        s.set_parameter(5, 0.0);
        let out = play(&mut s, 60, Some(1_000), 120);
        assert!(out[20_000..30_000].iter().any(|x| x.abs() > 0.01), "it plays on after the key is let go");
    }

    // F-A16
    #[test]
    fn eight_voices_and_a_ninth_takes_the_oldest() {
        let mut s = Sampler::with_user(vec![]);
        choose(&mut s, 2);
        let (mut l, mut r) = (vec![0.0; MAX_BLOCK], vec![0.0; MAX_BLOCK]);
        let ons: Vec<NoteEvent> = (0..9).map(|i| NoteEvent::on(i, 40 + i as u8, 1.0)).collect();
        s.process(&mut l, &mut r, &ons);
        assert_eq!(s.voices.iter().filter(|v| v.active).count(), 8);
        assert!(!s.voices.iter().any(|v| v.active && v.note == 40), "the first note was taken");
        assert!(l.iter().all(|x| x.is_finite() && x.abs() <= 1.0));
    }

    // F-A16
    #[test]
    fn the_players_files_fill_the_other_eight_places() {
        let dir = std::env::temp_dir().join(format!("op8-samples-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let tone: Vec<f32> = (0..24_000).map(|i| (i as f32 * 0.05).sin()).collect();
        std::fs::write(dir.join("b-second.wav"), crate::wav::encode_16(&tone, 48_000)).unwrap();
        std::fs::write(dir.join("a-first.wav"), crate::wav::encode_16(&tone[..8000], 48_000)).unwrap();
        std::fs::write(dir.join("notes.txt"), b"not a sound").unwrap();
        std::fs::write(dir.join("broken.wav"), b"RIFFxxxxWAVE").unwrap();
        let user = load_folder(&dir);
        assert_eq!(user.len(), 8);
        assert_eq!(user.iter().filter(|s| !s.is_empty()).map(|s| s.name.as_str()).collect::<Vec<_>>(), ["a-first", "b-second"], "in the order of their names; what is not a sound is skipped");
        let mut s = Sampler::with_user(user);
        choose(&mut s, 8);
        let out = play(&mut s, 60, None, 100);
        assert!(out[..2000].iter().any(|x| x.abs() > 0.05), "the first of the player's own sounds plays");
        choose(&mut s, 12); // an empty place
        assert!(play(&mut s, 60, None, 10).iter().all(|x| *x == 0.0), "an empty place is silent");
        assert!(load_folder(&dir.join("nowhere")).iter().all(|s| s.is_empty()));
        let _ = std::fs::remove_dir_all(&dir);
    }

    // F-A17: what SAMPLING saves as rec-N.wav is place N, whatever else is in the folder.
    #[test]
    fn a_recording_goes_to_its_own_place() {
        let dir = std::env::temp_dir().join(format!("op8-rec-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let tone: Vec<f32> = (0..9_600).map(|i| (i as f32 * 0.05).sin()).collect();
        let wav = crate::wav::encode_16(&tone, 48_000);
        std::fs::write(dir.join(recording_file(2)), &wav).unwrap(); // place 3
        std::fs::write(dir.join("a-mine.wav"), &wav).unwrap();
        let user = load_folder(&dir);
        assert!(!user[2].is_empty(), "rec-3 is the third place");
        assert!(!user[0].is_empty() && user[0].name == "a-mine", "her own file takes the first place that is free");
        assert!(user[1].is_empty() && user[3].is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }

    // F-A17: a place can be filled while the instrument plays, and a voice on it stops.
    #[test]
    fn a_place_is_filled_while_it_plays() {
        let mut s = Sampler::with_user(vec![]);
        choose(&mut s, 9);
        assert!(play(&mut s, 60, None, 10).iter().all(|x| *x == 0.0), "empty at first");
        let tone: Vec<f32> = (0..24_000).map(|i| (i as f32 * 0.05).sin()).collect();
        assert!(s.load_sample(1, tone));
        assert!(!s.load_sample(8, vec![0.1; 10]), "there are eight places, not nine");
        let out = play(&mut s, 60, None, 20);
        assert!(out.iter().any(|x| x.abs() > 0.05), "it plays what was put there");
    }

    fn slice_sound(s: &mut Sampler, note: u8) -> Vec<f32> {
        s.set_parameter(5, 0.0); // a short release
        play(s, note, None, 80)
    }

    // F-A22: with `loop` on slices the window is cut in equal parts and a note plays one of them, at its own pitch; C3 is the first, the
    // notes go round after the last, and the length of what sounds is a slice.
    #[test]
    fn slices_make_a_kit_of_the_sound() {
        let mut s = Sampler::with_user(vec![]);
        choose(&mut s, 1); // the bell, 2 s = 96 000 samples
        s.set_parameter(2, 1.0);
        s.set_parameter(6, 0.5); // slices 8
        assert_eq!(s.slices(), Some(8));
        let len = s.window(s.bank[1].len()).1 - s.window(s.bank[1].len()).0;
        let slice = len / 8.0;
        let first = slice_sound(&mut s, 48);
        assert!((last_sound(&first) as f64 - slice).abs() < 700.0, "a slice is an eighth of the window: {} of {slice}", last_sound(&first));
        let mut s2 = Sampler::with_user(vec![]);
        choose(&mut s2, 1);
        s2.set_parameter(2, 1.0);
        s2.set_parameter(6, 0.5);
        let third = slice_sound(&mut s2, 50);
        assert_ne!(first[..4000], third[..4000], "another note is another slice");
        let mut s3 = Sampler::with_user(vec![]);
        choose(&mut s3, 1);
        s3.set_parameter(2, 1.0);
        s3.set_parameter(6, 0.5);
        let again = slice_sound(&mut s3, 56); // the ninth: round to the first
        assert_eq!(first, again, "after the eighth the notes go round");
        // the slices keep their pitch whatever the note: no key tracking
        let mut s4 = Sampler::with_user(vec![]);
        choose(&mut s4, 1);
        s4.set_parameter(2, 1.0);
        s4.set_parameter(6, 0.7); // slices 16
        assert_eq!(s4.slices(), Some(16));
        let a = slice_sound(&mut s4, 48);
        assert!((last_sound(&a) as f64 - len / 16.0).abs() < 700.0, "sixteen: a sixteenth");
    }

    // F-A22: `tune` goes two octaves either way, and a project saved before (tune of ±12, loop on or off) sounds the same.
    #[test]
    fn tune_has_two_octaves_either_way_and_old_states_still_mean_the_same() {
        let mut s = Sampler::with_user(vec![]);
        choose(&mut s, 1);
        s.set_parameter(5, 0.0);
        s.set_parameter(3, 1.0); // +24 semitones: two octaves up: a quarter of the time
        let up = play(&mut s, 60, None, 400);
        assert!((last_sound(&up) as f64 - s.bank[1].len() as f64 / 4.0).abs() < 700.0, "{}", last_sound(&up));
        // version 1: tune 0.75 was +6 semitones, loop 1.0 was on
        let mut old = vec![1u8];
        for v in [0.2f32, 0.0, 1.0, 0.75, 0.0, 0.3, 1.0, 0.8] {
            old.extend_from_slice(&v.to_le_bytes());
        }
        let mut t = Sampler::with_user(vec![]);
        t.load_state(&old);
        assert!((t.tune_semitones() - 6.0).abs() < 1e-4, "the same +6: {}", t.tune_semitones());
        assert!(t.looping() && t.slices().is_none(), "the loop is still a loop");
        assert_eq!(t.save_state()[0], STATE_VERSION, "and it is saved as the new version");
    }

    /// Four noise bursts, 80 ms each, that die away, at 0.1, 0.6, 1.1 and 1.6 s of two seconds of silence.
    fn four_hits() -> Vec<f32> {
        let mut n = Noise(0x9e37_79b9);
        let mut v = vec![0.0f32; 96_000];
        for k in 0..4 {
            let at = 4800 + k * 24_000;
            for i in 0..3840 {
                v[at + i] = n.next() * (-(i as f32) / 900.0).exp();
            }
        }
        v
    }

    // F-A26: a sound is searched for where it hits: a burst that stands out by 9 dB or more from what was just before, 60 ms apart at least;
    // a held tone or a quiet murmur has none.
    #[test]
    fn the_hits_of_a_sound_are_found() {
        let found = find_onsets(&four_hits());
        let at: Vec<usize> = found.iter().map(|o| o.0).collect();
        assert_eq!(at.len(), 4, "{at:?}");
        for (k, p) in at.iter().enumerate() {
            let want = 4800 + k * 24_000;
            assert!(p.abs_diff(want) <= 480, "hit {k} at {p}, want about {want}");
        }
        let tone: Vec<f32> = (0..96_000).map(|i| (i as f32 * 0.05).sin() * 0.5).collect();
        assert!(find_onsets(&tone).is_empty(), "a steady tone has no hits");
        let murmur: Vec<f32> = (0..96_000).map(|i| (i as f32 * 0.3).sin() * 0.001).collect();
        assert!(find_onsets(&murmur).is_empty(), "nor does a murmur");
        assert!(find_onsets(&[0.5; 100]).is_empty(), "nor does a sound too short to tell");
    }

    // F-A26: with `loop` on hits a note plays from one hit to the next (C3 the first; the notes go round after the last), keeps its pitch
    // and a sound with no hits is one slice.
    #[test]
    fn hits_cut_the_window_where_the_sound_hits() {
        let mut s = Sampler::with_user(vec![Sample::new("hits", four_hits())]);
        choose(&mut s, 8); // user 1
        s.set_parameter(2, 1.0);
        s.set_parameter(6, 0.9); // hits
        assert!(s.hits() && s.slices().is_none());
        let sample = s.bank[8].clone();
        let (a, b) = s.window(sample.len());
        let edges = sample.hit_edges(a, b);
        assert_eq!(edges.len(), 5, "four slices: {edges:?}");
        let voice_of = |note: u8| {
            let mut x = Sampler::with_user(vec![Sample::new("hits", four_hits())]);
            choose(&mut x, 8);
            x.set_parameter(2, 1.0);
            x.set_parameter(6, 0.9);
            x.note_on(note, 1.0);
            let v = x.voices.iter().find(|v| v.active).copied().unwrap();
            (v.start, v.end, v.step)
        };
        let (a1, b1, step) = voice_of(48);
        assert_eq!((a1, b1), (edges[0], edges[1]), "C3 runs from the first hit to the second");
        assert!((a1 - 4752.0).abs() <= 480.0, "and the first hit is about where the first burst starts: {a1}");
        assert_eq!(voice_of(50).0, edges[2], "another note is another slice");
        assert_eq!(voice_of(51), (edges[3], edges[4], step), "the last runs to the end of the window");
        assert_eq!(voice_of(52), (a1, b1, step), "after the fourth the notes go round");
        assert_eq!(voice_of(72).2, step, "a slice keeps its pitch whatever the note");
        // a sound with no hits is one slice, the whole window
        let mut t = Sampler::with_user(vec![Sample::new("tone", (0..48_000).map(|i| (i as f32 * 0.05).sin()).collect())]);
        choose(&mut t, 8);
        t.set_parameter(2, 1.0);
        t.set_parameter(6, 0.9);
        assert_eq!(t.bank[8].hit_edges(0.0, 48_000.0), vec![0.0, 48_000.0]);
        // the window limits what is found: a window that starts after the first hit does not have it
        let late = sample.hit_edges(10_000.0, 96_000.0);
        assert_eq!(late.len(), 4, "{late:?}");
    }

    // F-A26: a state saved when `loop` had four positions means the same position now that it has five.
    #[test]
    fn a_state_of_four_loop_positions_means_the_same() {
        for (old, index) in [(0.0f32, 0), (0.375, 1), (0.625, 2), (0.875, 3)] {
            let mut bytes = vec![2u8];
            for v in [0.2f32, 0.0, 1.0, 0.5, 0.0, 0.3, old, 0.8] {
                bytes.extend_from_slice(&v.to_le_bytes());
            }
            let mut t = Sampler::with_user(vec![]);
            t.load_state(&bytes);
            assert_eq!(choice_index(t.values[6], LOOP_NAMES.len()), index, "{old}");
            assert!(!t.hits(), "a saved state never lands on hits");
            assert_eq!(t.save_state()[0], STATE_VERSION);
        }
    }

    // F-A16
    #[test]
    fn the_picture_shows_the_wave_and_the_voices() {
        let mut s = Sampler::with_user(vec![]);
        choose(&mut s, 3);
        let mut fb = FrameBuffer480x272::new();
        s.draw_ui(&mut fb, Rgb565::from_hex(0x30d5c8));
        let still = fb.pixels().iter().filter(|&&p| p != 0).count();
        assert!(still > 800, "the wave is drawn: {still}");
        let _ = play(&mut s, 60, None, 4);
        let mut fb2 = FrameBuffer480x272::new();
        s.draw_ui(&mut fb2, Rgb565::from_hex(0x30d5c8));
        assert_ne!(fb.pixels(), fb2.pixels(), "a playing voice is marked");
        choose(&mut s, 12);
        s.draw_ui(&mut fb2, Rgb565::from_hex(0x30d5c8));
    }
}

impl PartialEq for Sample {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name && self.data == other.data
    }
}
impl std::fmt::Debug for Sample {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Sample({}, {} samples)", self.name, self.data.len())
    }
}
