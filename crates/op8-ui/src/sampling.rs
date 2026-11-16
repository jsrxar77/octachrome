//! SAMPLING: the main mode that takes both screens (design.md §4). The right screen shows what was
//! recorded as a wave, the part that will be kept, the level while it records and the four knobs of
//! the take; the left screen is the pads: the top two rows save the part into one of the eight
//! places of the player's own sounds, the bottom two listen to it.

use crate::{files::pick_left, theme::HEADER_CY};
use op8_engine::EngineStatus;
use op8_gfx::{
    knob,
    paint::{fill_circle, fill_round_rect, line_aa},
    text_v, theme, Align, FontSize, FrameBuffer480x272, VAlign,
};

/// Columns of the wave kept for the drawing.
pub const COLUMNS: usize = 240;
pub const SOURCES: [&str; 2] = ["input", "resample"];

/// What SAMPLING shows and edits (the audio itself stays in the engine's `take`).
#[derive(Clone, Debug)]
pub struct SamplingView {
    /// 0 = the audio interface's input, 1 = what OP-8 plays.
    pub source: u8,
    pub recording: bool,
    /// Samples in the take.
    pub len: usize,
    /// How loud the input is right now, 0..1 (it falls away between blocks).
    pub level: f32,
    /// The part that is kept, as fractions of the take.
    pub start: f32,
    pub end: f32,
    /// The fade at both ends of the part, 0..1 of 100 ms.
    pub fade: f32,
    /// The wave: (lowest, highest) of each of `COLUMNS` slices of the take.
    pub peaks: Vec<(f32, f32)>,
    /// Which of the eight places of the player's own have a sound.
    pub saved: [bool; 8],
    pub note: String,
    /// 0..1, going round every two seconds: what moves on the screen while nothing records.
    pub phase: f32,
}

impl Default for SamplingView {
    fn default() -> Self {
        Self {
            source: 1,
            recording: false,
            len: 0,
            level: 0.0,
            start: 0.0,
            end: 1.0,
            fade: 0.05,
            peaks: Vec::new(),
            saved: [false; 8],
            note: String::new(),
            phase: 0.0,
        }
    }
}

impl SamplingView {
    pub fn seconds(&self) -> f32 {
        self.len as f32 / op8_cartridge::SAMPLE_RATE
    }
    /// The kept part, in samples of the take.
    pub fn window(&self) -> (usize, usize) {
        let a = (self.start.clamp(0.0, 1.0) * self.len as f32) as usize;
        let b = (self.end.clamp(0.0, 1.0) * self.len as f32) as usize;
        (a.min(self.len), b.max(a).min(self.len))
    }
    /// The fade at each end, in samples.
    pub fn fade_samples(&self) -> usize {
        (self.fade * 0.1 * op8_cartridge::SAMPLE_RATE) as usize
    }
}

pub fn draw_left(fb: &mut FrameBuffer480x272, ui: &crate::UiState, st: &EngineStatus) {
    let s = &ui.smp;
    let mut cells: Vec<(String, bool)> = (0..8).map(|i| (format!("user {}", i + 1), s.saved[i])).collect();
    cells.extend((0..8).map(|_| ("listen".to_string(), s.len > 0)));
    pick_left(fb, ui, st, "sampling", &cells, None);
}

fn secs(s: f32) -> String {
    format!("{s:.2}s")
}

pub fn draw_right(fb: &mut FrameBuffer480x272, ui: &crate::UiState) {
    let t = theme();
    let s = &ui.smp;
    fb.clear(t.bg);
    // the bar: a dot that pulses while it records, the title, the source and the length
    let pulse = if s.recording { 0.5 + 0.5 * (s.phase * std::f32::consts::TAU * 2.0).sin() } else { 0.0 };
    fill_circle(fb, 16.0, HEADER_CY, 6.0 + 2.0 * pulse, if s.recording { t.fg } else { t.dim });
    text_v(fb, if s.recording { "recording" } else { "sampling" }, 30, HEADER_CY as i32, FontSize::Large, t.fg, Align::Left, VAlign::Middle);
    text_v(fb, &secs(s.seconds()), 464, HEADER_CY as i32, FontSize::Large, t.dim, Align::Right, VAlign::Middle);

    // the wave
    let (x0, w, cy, amp) = (30.0f32, 420.0f32, 108.0f32, 52.0f32);
    line_aa(fb, x0, cy, x0 + w, cy, 1.0, t.faint);
    if s.len == 0 {
        // nothing yet: a dot goes along the line, waiting
        fill_circle(fb, x0 + s.phase * w, cy, 4.0, t.dim);
    } else {
        let (xs, xe) = (x0 + s.start * w, x0 + s.end.max(s.start) * w);
        fill_round_rect(fb, xs, cy - amp - 6.0, (xe - xs).max(2.0), amp * 2.0 + 12.0, 6.0, t.panel);
        let col = w / COLUMNS as f32;
        for (c, &(lo, hi)) in s.peaks.iter().enumerate() {
            let x = x0 + c as f32 * col;
            let inside = x + col > xs && x < xe;
            line_aa(fb, x, cy - hi * amp, x, cy - lo * amp - 0.5, 1.6, if inside { t.fg } else { t.faint });
        }
        line_aa(fb, xs, cy - amp - 10.0, xs, cy + amp + 10.0, 2.0, knob(5));
        line_aa(fb, xe, cy - amp - 10.0, xe, cy + amp + 10.0, 2.0, knob(6));
        if s.recording {
            let x = x0 + w * (s.len as f32 / op8_engine::MAX_RECORD as f32).min(1.0);
            line_aa(fb, x, cy - amp - 12.0, x, cy + amp + 12.0, 2.0, t.fg);
        }
    }

    // the level
    let (ly, lw) = (186.0f32, 420.0f32);
    fill_round_rect(fb, x0, ly, lw, 6.0, 3.0, t.faint);
    let lv = s.level.clamp(0.0, 1.0).sqrt();
    if lv > 0.0 {
        fill_round_rect(fb, x0, ly, (lw * lv).max(6.0), 6.0, 3.0, if s.level > 0.95 { t.fg } else { t.dim });
    }
    if !s.note.is_empty() {
        text_v(fb, &s.note, 30, 208, FontSize::Medium, t.fg, Align::Left, VAlign::Middle);
    }

    // the four knobs under the screen, in their colours
    let (a, b) = s.window();
    let sr = op8_cartridge::SAMPLE_RATE;
    let values = [
        ("source", SOURCES[s.source.min(1) as usize].to_string()),
        ("start", secs(a as f32 / sr)),
        ("end", secs(b as f32 / sr)),
        ("fade", format!("{:.0}ms", s.fade * 100.0)),
    ];
    for (i, (name, v)) in values.iter().enumerate() {
        let x = 60 + i as i32 * 120;
        text_v(fb, name, x, 236, FontSize::Small, t.dim, Align::Center, VAlign::Middle);
        text_v(fb, v, x, 254, FontSize::Medium, knob(4 + i), Align::Center, VAlign::Middle);
    }
    if let Some(it) = crate::nav::focused(ui).filter(|it| it.screen == 1) {
        crate::nav::draw_ring(fb, &it, ui);
    }
}

/// The wave of `data` as `COLUMNS` (lowest, highest) pairs.
pub fn peaks_of(data: &[f32]) -> Vec<(f32, f32)> {
    let n = data.len().max(1);
    (0..COLUMNS)
        .map(|c| {
            let (a, b) = (c * n / COLUMNS, ((c + 1) * n / COLUMNS).max(c * n / COLUMNS + 1).min(n));
            data.get(a..b).unwrap_or(&[]).iter().fold((0.0f32, 0.0f32), |(lo, hi), &x| (lo.min(x), hi.max(x)))
        })
        .collect()
}
