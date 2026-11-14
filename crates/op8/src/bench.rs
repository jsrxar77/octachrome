//! Test bench: the whole application on a simulated MK3, driven from code.
//!
//! The bench runs the same [`App`] and [`Engine`] as `op8 run`, on a
//! [`MockSurface`], and advances time the same way: one display frame of input
//! and drawing, then 800 samples of audio. It records everything the hardware
//! would have shown or sent, so tests can assert on it (see
//! `docs/architecture.md` §9 and `docs/features.md`).
//!
//! Setting `OP8_BENCH_DIR` makes [`Bench::capture`] and [`Bench::save_audio`]
//! write PNG screenshots and WAV files there, for looking at by hand.

use crate::{app::App, demo, script::{Action, Script}, wav};
use op8_cartridge::SAMPLE_RATE;
use op8_cartridges::MidiMessage;
use op8_engine::{Engine, EngineHandle, EngineStatus};
use op8_gfx::FrameBuffer480x272;
use op8_hw::{mock::MockSurface, Button, ControlEvent, LedFrame, PadEvent, Screen};
use op8_seq::Project;
use op8_ui::UiState;
use std::{path::{Path, PathBuf}, sync::atomic::Ordering::Relaxed};

/// Audio samples per display frame (60 fps at 48 kHz).
pub const FRAME_SAMPLES: usize = SAMPLE_RATE as usize / 60;
const BLOCK: usize = 100;

pub struct Bench {
    pub engine: Engine,
    pub app: App<MockSurface>,
    midi: rtrb::Consumer<MidiMessage>,
    script: Script,
    samples: u64,
    /// Everything rendered so far, left and right.
    pub audio_l: Vec<f32>,
    pub audio_r: Vec<f32>,
    /// Every MIDI message that left the bridge: (time in ms, bytes).
    pub midi_log: Vec<(u64, Vec<u8>)>,
}

impl Bench {
    /// A bench around an engine, its handle and the MIDI consumer, as returned
    /// by `demo::rig` and `demo::empty_rig`.
    pub fn new(rig: (Engine, EngineHandle, rtrb::Consumer<MidiMessage>)) -> Self {
        let (engine, handle, midi) = rig;
        Self {
            engine,
            app: App::new(MockSurface::new(), handle),
            midi,
            script: Script::default(),
            samples: 0,
            audio_l: Vec::new(),
            audio_r: Vec::new(),
            midi_log: Vec::new(),
        }
    }

    /// The demo project in its four slots (KickSyn, FmBass, MIDI, macro).
    pub fn demo() -> Self {
        Self::new(demo::rig())
    }

    /// The same slots with an empty pattern.
    pub fn empty() -> Self {
        Self::new(demo::empty_rig())
    }

    // ---- time ----------------------------------------------------------------

    /// Elapsed time in milliseconds, from audio samples.
    pub fn now_ms(&self) -> u64 {
        self.samples * 1000 / SAMPLE_RATE as u64
    }

    /// One display frame: scripted input, input handling, drawing, then one
    /// frame of audio and the MIDI that came out of it.
    pub fn tick(&mut self) {
        let now = self.now_ms();
        for action in self.script.due(now) {
            match action {
                Action::Event(e) => self.app.inject(e),
                Action::Shot(name) => self.capture(&name),
            }
        }
        self.app.frame(now).expect("the mock surface never fails");

        let (mut l, mut r) = ([0.0f32; BLOCK], [0.0f32; BLOCK]);
        for _ in 0..FRAME_SAMPLES / BLOCK {
            self.engine.render(&mut l, &mut r);
            self.audio_l.extend_from_slice(&l);
            self.audio_r.extend_from_slice(&r);
        }
        self.samples += FRAME_SAMPLES as u64;

        while let Ok(m) = self.midi.pop() {
            self.midi_log.push((now, m.as_bytes().to_vec()));
        }
    }

    /// Run for `ms` of simulated time.
    pub fn run_ms(&mut self, ms: u64) {
        let end = self.now_ms() + ms;
        while self.now_ms() < end {
            self.tick();
        }
    }

    /// Play a script from now. Times in the script are relative to the start
    /// of the bench, so load it right after `new`.
    pub fn load_script(&mut self, text: &str) {
        self.script = Script::parse(text).expect("test script must parse");
    }

    /// Run until the loaded script is finished, plus a short tail.
    pub fn run_script(&mut self) {
        while !self.script.is_done() || self.now_ms() < self.script.end_ms() + 200 {
            self.tick();
        }
    }

    // ---- input, as the MK3 would send it -------------------------------------

    pub fn send(&mut self, e: ControlEvent) {
        self.app.inject(e);
        self.tick();
    }

    pub fn button_down(&mut self, button: Button) {
        self.send(ControlEvent::Button { button, pressed: true });
    }

    pub fn button_up(&mut self, button: Button) {
        self.send(ControlEvent::Button { button, pressed: false });
    }

    /// Press and release a button.
    pub fn press(&mut self, button: Button) {
        self.button_down(button);
        self.button_up(button);
    }

    pub fn strike(&mut self, pad: u8) {
        self.send(ControlEvent::Pad { pad, event: PadEvent::Strike(0.8) });
    }

    pub fn pressure(&mut self, pad: u8, value: f32) {
        self.send(ControlEvent::Pad { pad, event: PadEvent::Pressure(value) });
    }

    pub fn release(&mut self, pad: u8) {
        self.send(ControlEvent::Pad { pad, event: PadEvent::Release });
    }

    /// A quick tap: strike and release after 100 ms.
    pub fn tap(&mut self, pad: u8) {
        self.strike(pad);
        self.run_ms(100);
        self.release(pad);
    }

    /// Hold a pad for `ms` and release it.
    pub fn hold(&mut self, pad: u8, ms: u64) {
        self.strike(pad);
        self.run_ms(ms);
        self.release(pad);
    }

    /// Turn encoder `index` (0..8) by `delta` units (1000 per turn).
    pub fn knob(&mut self, index: u8, delta: i32) {
        self.send(ControlEvent::Knob { index, delta });
    }

    pub fn touch(&mut self, index: u8, touched: bool) {
        self.send(ControlEvent::KnobTouch { index, touched });
    }

    pub fn encoder(&mut self, delta: i32) {
        self.send(ControlEvent::Encoder { delta });
    }

    pub fn strip(&mut self, position: Option<f32>) {
        self.send(ControlEvent::Strip { position });
    }

    // ---- what the device shows and sends -------------------------------------

    pub fn project(&self) -> &Project {
        &self.app.handle.project
    }

    /// Where FILE saves the player's projects (a folder of the test's own).
    pub fn set_projects_dir(&mut self, dir: std::path::PathBuf) {
        self.app.ctl.projects_dir = Some(dir);
    }

    pub fn ui(&self) -> &UiState {
        &self.app.ctl.ui
    }

    pub fn status(&self) -> &EngineStatus {
        &self.app.handle.status
    }

    pub fn leds(&self) -> &LedFrame {
        &self.app.surface.leds
    }

    pub fn screen(&self, screen: Screen) -> &FrameBuffer480x272 {
        &self.app.surface.screens[screen as usize]
    }

    /// Parameter `index` of track `track`'s instrument.
    pub fn param(&self, track: usize, index: usize) -> f32 {
        self.app.handle.param(track, index)
    }

    /// Events the engine could not take (should stay at zero).
    pub fn dropped(&self) -> u32 {
        self.status().dropped_events.load(Relaxed)
    }

    /// Pixels that are not pure black on a screen.
    pub fn lit_pixels(&self, screen: Screen) -> usize {
        self.screen(screen).pixels().iter().filter(|&&p| p != 0).count()
    }

    /// Fraction of pixels that are pure black on a screen.
    pub fn black_fraction(&self, screen: Screen) -> f32 {
        let px = self.screen(screen).pixels();
        px.iter().filter(|&&p| p == 0).count() as f32 / px.len() as f32
    }

    /// FNV-1a over the pixels, to tell whether a screen changed.
    pub fn screen_hash(&self, screen: Screen) -> u64 {
        self.screen(screen).pixels().iter().fold(0xcbf2_9ce4_8422_2325, |h, &p| {
            (h ^ p as u64).wrapping_mul(0x0100_0000_01b3)
        })
    }

    /// Display frames sent to the surface so far, in pixels.
    pub fn pixels_sent(&self) -> u64 {
        self.app.surface.pixels_sent
    }

    // ---- measurements over a time range [from_ms, to_ms) ---------------------

    fn range(&self, from_ms: u64, to_ms: u64) -> std::ops::Range<usize> {
        let sr = SAMPLE_RATE as u64;
        let len = self.audio_l.len() as u64;
        let a = (from_ms * sr / 1000).min(len) as usize;
        let b = (to_ms * sr / 1000).min(len) as usize;
        a..b.max(a)
    }

    /// RMS of the mono mix, in dBFS. −∞ for silence.
    pub fn rms_db(&self, from_ms: u64, to_ms: u64) -> f32 {
        let r = self.range(from_ms, to_ms);
        let n = r.len().max(1) as f64;
        let sum: f64 = r
            .clone()
            .map(|i| {
                let m = (self.audio_l[i] + self.audio_r[i]) as f64 * 0.5;
                m * m
            })
            .sum();
        to_db((sum / n).sqrt() as f32)
    }

    /// Largest absolute sample on either channel, in dBFS.
    pub fn peak_db(&self, from_ms: u64, to_ms: u64) -> f32 {
        let r = self.range(from_ms, to_ms);
        let peak = r
            .map(|i| self.audio_l[i].abs().max(self.audio_r[i].abs()))
            .fold(0.0f32, f32::max);
        to_db(peak)
    }

    /// Largest absolute sample on either channel, linear.
    pub fn peak(&self, from_ms: u64, to_ms: u64) -> f32 {
        10f32.powf(self.peak_db(from_ms, to_ms) / 20.0)
    }

    /// True when nothing above −90 dBFS came out in the range.
    pub fn silent(&self, from_ms: u64, to_ms: u64) -> bool {
        self.peak_db(from_ms, to_ms) < -90.0
    }

    /// MIDI messages sent in the range.
    pub fn midi_between(&self, from_ms: u64, to_ms: u64) -> Vec<&[u8]> {
        self.midi_log
            .iter()
            .filter(|(t, _)| *t >= from_ms && *t < to_ms)
            .map(|(_, b)| b.as_slice())
            .collect()
    }

    // ---- evidence ------------------------------------------------------------

    /// `OP8_BENCH_DIR`, if set.
    pub fn bench_dir() -> Option<PathBuf> {
        std::env::var_os("OP8_BENCH_DIR").map(PathBuf::from)
    }

    /// Save both screens as PNG, if `OP8_BENCH_DIR` is set.
    pub fn capture(&self, name: &str) {
        let Some(dir) = Self::bench_dir() else { return };
        std::fs::create_dir_all(&dir).expect("bench dir");
        for (screen, side) in [(Screen::Left, "left"), (Screen::Right, "right")] {
            let path = dir.join(format!("{name}-{side}.png"));
            std::fs::write(path, encode_png(self.screen(screen))).expect("write png");
        }
    }

    /// Save the audio rendered so far as WAV, if `OP8_BENCH_DIR` is set.
    pub fn save_audio(&self, name: &str) {
        let Some(dir) = Self::bench_dir() else { return };
        std::fs::create_dir_all(&dir).expect("bench dir");
        let path: &Path = &dir.join(format!("{name}.wav"));
        wav::write_wav(path, &self.audio_l, &self.audio_r, SAMPLE_RATE as u32)
            .expect("write wav");
    }
}

fn to_db(linear: f32) -> f32 {
    if linear <= 1e-9 {
        f32::NEG_INFINITY
    } else {
        20.0 * linear.log10()
    }
}

// ---- PNG, with no dependencies: stored (uncompressed) deflate blocks ----------

/// Encode a framebuffer as an RGB PNG.
pub fn encode_png(fb: &FrameBuffer480x272) -> Vec<u8> {
    let (w, h) = (FrameBuffer480x272::WIDTH, FrameBuffer480x272::HEIGHT);
    let px = fb.pixels();
    let mut raw = Vec::with_capacity(h * (1 + 3 * w));
    for y in 0..h {
        raw.push(0); // filter: none
        for &p in &px[y * w..(y + 1) * w] {
            let r = ((p >> 11) & 31) as u8;
            let g = ((p >> 5) & 63) as u8;
            let b = (p & 31) as u8;
            raw.extend_from_slice(&[(r << 3) | (r >> 2), (g << 2) | (g >> 4), (b << 3) | (b >> 2)]);
        }
    }

    let mut z = vec![0x78, 0x01];
    let mut blocks = raw.chunks(65535).peekable();
    while let Some(block) = blocks.next() {
        z.push(blocks.peek().is_none() as u8); // BFINAL, BTYPE = stored
        let n = block.len() as u16;
        z.extend_from_slice(&n.to_le_bytes());
        z.extend_from_slice(&(!n).to_le_bytes());
        z.extend_from_slice(block);
    }
    z.extend_from_slice(&adler32(&raw).to_be_bytes());

    let mut out = vec![0x89, b'P', b'N', b'G', b'\r', b'\n', 0x1a, b'\n'];
    let mut ihdr = Vec::with_capacity(13);
    ihdr.extend_from_slice(&(w as u32).to_be_bytes());
    ihdr.extend_from_slice(&(h as u32).to_be_bytes());
    ihdr.extend_from_slice(&[8, 2, 0, 0, 0]); // 8-bit RGB
    chunk(&mut out, b"IHDR", &ihdr);
    chunk(&mut out, b"IDAT", &z);
    chunk(&mut out, b"IEND", &[]);
    out
}

fn chunk(out: &mut Vec<u8>, kind: &[u8; 4], data: &[u8]) {
    out.extend_from_slice(&(data.len() as u32).to_be_bytes());
    let mut body = Vec::with_capacity(4 + data.len());
    body.extend_from_slice(kind);
    body.extend_from_slice(data);
    out.extend_from_slice(&body);
    out.extend_from_slice(&crc32(&body).to_be_bytes());
}

fn crc32(bytes: &[u8]) -> u32 {
    let mut crc = 0xffff_ffffu32;
    for &b in bytes {
        crc ^= b as u32;
        for _ in 0..8 {
            crc = if crc & 1 != 0 { (crc >> 1) ^ 0xedb8_8320 } else { crc >> 1 };
        }
    }
    !crc
}

fn adler32(bytes: &[u8]) -> u32 {
    let (mut a, mut b) = (1u32, 0u32);
    for &x in bytes {
        a = (a + x as u32) % 65521;
        b = (b + a) % 65521;
    }
    (b << 16) | a
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn png_has_signature_and_size() {
        let png = encode_png(&FrameBuffer480x272::new());
        assert_eq!(&png[..8], &[0x89, b'P', b'N', b'G', b'\r', b'\n', 0x1a, b'\n']);
        // IHDR right after the signature: length 13, then width and height.
        assert_eq!(&png[16..20], &480u32.to_be_bytes());
        assert_eq!(&png[20..24], &272u32.to_be_bytes());
    }

    #[test]
    fn crc_and_adler_match_known_values() {
        assert_eq!(crc32(b"IEND"), 0xae42_6082);
        assert_eq!(adler32(b"Wikipedia"), 0x11e6_0398);
    }

    #[test]
    fn empty_bench_is_silent() {
        let mut b = Bench::empty();
        b.run_ms(200);
        assert!(b.silent(0, 200));
        assert!(b.rms_db(0, 200) == f32::NEG_INFINITY);
    }
}
