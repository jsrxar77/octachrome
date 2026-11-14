//! Smoothed animation state. The screens never jump: playheads glide, notes
//! flash and decay, the touch overlay fades (design.md §6). Call
//! [`animate`] once per frame before drawing.

use crate::UiState;
use op8_engine::EngineStatus;
use op8_seq::{Project, TRACKS};
use std::sync::atomic::Ordering::Relaxed;

#[derive(Clone, Debug, Default)]
pub struct Anim {
    /// Smoothed playhead per track, in steps.
    pub ph: [f32; TRACKS],
    /// Note flash per track, 1 on a new note, decaying.
    pub hit: [f32; TRACKS],
    last_count: [u32; TRACKS],
    /// Cap-touch fade per encoder.
    pub touch: [f32; 8],
    /// 1 when the Brain chord changes, decaying.
    pub chord_pulse: f32,
    last_chord: (u8, u16),
}

fn ease(cur: f32, target: f32, k: f32) -> f32 {
    cur + (target - cur) * k
}

pub fn animate(ui: &mut UiState, p: &Project, st: &EngineStatus) {
    let a = &mut ui.anim;
    for t in 0..TRACKS {
        let n = st.note_count[t].load(Relaxed);
        if n != a.last_count[t] {
            a.last_count[t] = n;
            a.hit[t] = 1.0;
        }
        a.hit[t] *= 0.86;
        let len = p.tracks[t].len() as f32;
        let target = st.playhead[t].load(Relaxed) as f32;
        let mut d = target - a.ph[t];
        if d < -len / 2.0 {
            d += len;
        }
        if d > len / 2.0 {
            d -= len;
        }
        a.ph[t] = (a.ph[t] + d * 0.35).rem_euclid(len);
    }
    for k in 0..8 {
        a.touch[k] = ease(a.touch[k], if ui.knob_touch[k] { 1.0 } else { 0.0 }, 0.3);
        if a.touch[k] < 0.01 {
            a.touch[k] = 0.0;
        }
    }
    let ch = st.chord();
    if (ch.root, ch.mask) != a.last_chord {
        a.last_chord = (ch.root, ch.mask);
        a.chord_pulse = 1.0;
    }
    a.chord_pulse *= 0.9;
}
