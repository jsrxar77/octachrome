//! Shared OP-XY-style drawing for instrument screens: black background,
//! 1–2 px white strokes, one accent colour.
//!
//! The right screen layout (header and footer are drawn by the host):
//! y 0..32 header · y 32..224 instrument area · y 224..272 footer.

use crate::dsp::Scope;
use op3_gfx::{line, polyline, rect_outline, FrameBuffer480x272, Rgb565};

pub const AREA_Y: i32 = 34;
pub const AREA_H: i32 = 188;

/// Oscilloscope in the given box.
pub fn draw_scope(
    fb: &mut FrameBuffer480x272,
    scope: &Scope,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    color: Rgb565,
) {
    let mid = y + h / 2;
    line(fb, x, mid, x + w, mid, 1, Rgb565::WHITE.scale(0.2));
    let samples: Vec<f32> = scope.samples().collect();
    let n = samples.len().max(2) as f32;
    let pts = samples.iter().enumerate().map(|(i, s)| {
        let px = x + (i as f32 / (n - 1.0) * w as f32) as i32;
        let py = mid - (s.clamp(-1.0, 1.0) * (h as f32 / 2.0 - 2.0)) as i32;
        (px, py)
    });
    polyline(fb, pts, 1, color);
}

/// Classic ADSR shape. Times in ms; sustain 0..1.
pub fn draw_adsr(
    fb: &mut FrameBuffer480x272,
    a: f32,
    d: f32,
    s: f32,
    r: f32,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    color: Rgb565,
) {
    rect_outline(fb, x, y, w as u32, h as u32, 1, Rgb565::WHITE.scale(0.25));
    // log-ish time axis so a 2 ms attack is still visible next to a 2 s release
    let t = |ms: f32| (1.0 + ms).ln();
    let hold = t(300.0);
    let total = t(a) + t(d) + hold + t(r);
    let k = (w - 8) as f32 / total;
    let x0 = x + 4;
    let base = y + h - 4;
    let top = y + 4;
    let sy = base - ((base - top) as f32 * s) as i32;
    let xa = x0 + (t(a) * k) as i32;
    let xd = xa + (t(d) * k) as i32;
    let xs = xd + (hold * k) as i32;
    let xr = xs + (t(r) * k) as i32;
    polyline(
        fb,
        [(x0, base), (xa, top), (xd, sy), (xs, sy), (xr, base)],
        2,
        Rgb565::WHITE,
    );
    for px in [xa, xd, xs] {
        fb.fill_rect(px - 2, if px == xa { top } else { sy } - 2, 5, 5, color);
    }
}

/// Generic decaying-curve plot (e.g. a kick's pitch or amp envelope).
pub fn draw_curve(
    fb: &mut FrameBuffer480x272,
    f: impl Fn(f32) -> f32,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    width: u32,
    color: Rgb565,
) {
    let pts = (0..=w / 2).map(|i| {
        let u = i as f32 * 2.0 / w as f32;
        (x + i * 2, y + h - (f(u).clamp(0.0, 1.0) * h as f32) as i32)
    });
    polyline(fb, pts, width, color);
}
