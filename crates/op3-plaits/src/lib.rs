//! Safe Rust wrapper around the vendored macro-oscillator voice
//! (`vendor/README.md`: MIT, Emilie Gillet).
//!
//! One [`MacroVoice`] is a monophonic voice with 24 synthesis engines, each
//! shaped by three continuous controls (harmonics, timbre, morph) and an
//! internal low-pass-gate envelope (decay, colour). Renders at 48 kHz.

use std::ffi::c_void;

#[repr(C)]
struct RawPatch {
    note: f32,
    harmonics: f32,
    timbre: f32,
    morph: f32,
    engine: i32,
    decay: f32,
    lpg_colour: f32,
    trigger: f32,
}

extern "C" {
    fn op3_macro_new() -> *mut c_void;
    fn op3_macro_free(p: *mut c_void);
    fn op3_macro_render(
        p: *mut c_void,
        patch: *const RawPatch,
        out: *mut f32,
        aux: *mut f32,
        size: usize,
    );
    fn op3_macro_active_engine(p: *const c_void) -> i32;
}

/// Number of synthesis engines.
pub const ENGINES: usize = 24;
/// Largest block `render` accepts in one call.
pub const MAX_BLOCK: usize = 24;

/// Short lowercase names (our own wording, DESIGN.md rule 8).
pub const ENGINE_NAMES: [&str; ENGINES] = [
    "va filter",
    "phase dist",
    "fm 6op a",
    "fm 6op b",
    "fm 6op c",
    "terrain",
    "string mach",
    "chiptune",
    "va",
    "waveshape",
    "fm",
    "grain",
    "additive",
    "wavetable",
    "chords",
    "speech",
    "swarm",
    "noise",
    "particle",
    "string",
    "modal",
    "kick",
    "snare",
    "hihat",
];

/// Controls for one render call. All 0..=1 except `note` (MIDI) and `engine`.
#[derive(Clone, Copy, Debug)]
pub struct MacroPatch {
    pub note: f32,
    pub engine: usize,
    pub harmonics: f32,
    pub timbre: f32,
    pub morph: f32,
    pub decay: f32,
    pub colour: f32,
    /// Rising edge (0 → 1) strikes the internal envelope.
    pub trigger: bool,
}

impl Default for MacroPatch {
    fn default() -> Self {
        Self {
            note: 48.0,
            engine: 8,
            harmonics: 0.5,
            timbre: 0.5,
            morph: 0.5,
            decay: 0.5,
            colour: 0.5,
            trigger: false,
        }
    }
}

pub struct MacroVoice {
    ptr: *mut c_void,
}

// The C++ object is only touched through &mut self (or &self for a read).
unsafe impl Send for MacroVoice {}
unsafe impl Sync for MacroVoice {}

impl MacroVoice {
    /// Allocates the voice and its 32 KB work buffer (do this off the audio thread).
    pub fn new() -> Self {
        let ptr = unsafe { op3_macro_new() };
        assert!(!ptr.is_null(), "out of memory creating macro voice");
        Self { ptr }
    }

    /// Render up to [`MAX_BLOCK`] samples; `out` and `aux` must have equal length.
    pub fn render(&mut self, p: &MacroPatch, out: &mut [f32], aux: &mut [f32]) {
        let n = out.len().min(aux.len()).min(MAX_BLOCK);
        let raw = RawPatch {
            note: p.note,
            harmonics: p.harmonics.clamp(0.0, 1.0),
            timbre: p.timbre.clamp(0.0, 1.0),
            morph: p.morph.clamp(0.0, 1.0),
            engine: p.engine.min(ENGINES - 1) as i32,
            decay: p.decay.clamp(0.0, 1.0),
            lpg_colour: p.colour.clamp(0.0, 1.0),
            trigger: if p.trigger { 1.0 } else { 0.0 },
        };
        unsafe { op3_macro_render(self.ptr, &raw, out.as_mut_ptr(), aux.as_mut_ptr(), n) }
    }

    pub fn active_engine(&self) -> usize {
        unsafe { op3_macro_active_engine(self.ptr) as usize }
    }
}

impl Default for MacroVoice {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for MacroVoice {
    fn drop(&mut self) {
        unsafe { op3_macro_free(self.ptr) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_engine_makes_sound_when_struck() {
        let mut v = MacroVoice::new();
        let (mut out, mut aux) = ([0.0f32; 12], [0.0f32; 12]);
        for e in 0..ENGINES {
            let mut p = MacroPatch {
                engine: e,
                ..Default::default()
            };
            let mut energy = 0.0;
            for b in 0..400 {
                p.trigger = b % 100 < 5;
                v.render(&p, &mut out, &mut aux);
                energy += out.iter().map(|x| x * x).sum::<f32>();
            }
            assert_eq!(v.active_engine(), e);
            assert!(energy > 1e-3, "engine {e} silent");
            assert!(out.iter().all(|x| x.is_finite() && x.abs() <= 1.0));
        }
    }
}
