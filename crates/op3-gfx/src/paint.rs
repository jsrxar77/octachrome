//! Anti-aliased vector primitives on the RGB565 framebuffer.
//!
//! Every shape is drawn from a signed distance per pixel, so edges get
//! fractional coverage instead of stair steps (DESIGN.md §5: no jagged
//! 1-bit lines). Angles follow the screen: 0 = +x, growing clockwise.

use crate::{FrameBuffer480x272, Rgb565, HEIGHT, WIDTH};
use std::f32::consts::{PI, TAU};

/// Blend `c` over the pixel at (x, y) with coverage `a` (0..=1).
#[inline]
pub fn blend(fb: &mut FrameBuffer480x272, x: i32, y: i32, c: Rgb565, a: f32) {
    if a <= 0.0 || x < 0 || y < 0 || x >= WIDTH as i32 || y >= HEIGHT as i32 {
        return;
    }
    if a >= 0.999 {
        fb.put(x, y, c);
    } else {
        let under = fb.get(x as usize, y as usize);
        fb.put(x, y, under.lerp(c, a));
    }
}

#[inline]
fn cov(d: f32) -> f32 {
    (0.5 - d).clamp(0.0, 1.0)
}

/// Pixel bounds of a box around (cx, cy), clipped to the screen.
fn bounds(x0: f32, y0: f32, x1: f32, y1: f32) -> (i32, i32, i32, i32) {
    (
        (x0.floor() as i32 - 1).max(0),
        (y0.floor() as i32 - 1).max(0),
        (x1.ceil() as i32 + 1).min(WIDTH as i32 - 1),
        (y1.ceil() as i32 + 1).min(HEIGHT as i32 - 1),
    )
}

pub fn fill_circle(fb: &mut FrameBuffer480x272, cx: f32, cy: f32, r: f32, c: Rgb565) {
    if r <= 0.0 {
        return;
    }
    let (x0, y0, x1, y1) = bounds(cx - r, cy - r, cx + r, cy + r);
    for y in y0..=y1 {
        for x in x0..=x1 {
            let d = ((x as f32 + 0.5 - cx).hypot(y as f32 + 0.5 - cy)) - r;
            blend(fb, x, y, c, cov(d));
        }
    }
}

fn in_arc(a: f32, a0: f32, a1: f32) -> bool {
    if a1 - a0 >= TAU - 1e-4 {
        return true;
    }
    let span = a1 - a0;
    let rel = (a - a0).rem_euclid(TAU);
    rel <= span
}

/// Stroke an arc from `a0` to `a1` (radians, clockwise), `width` px wide,
/// with round caps. A full circle when `a1 - a0 >= 2π`.
#[allow(clippy::too_many_arguments)]
pub fn arc(
    fb: &mut FrameBuffer480x272,
    cx: f32,
    cy: f32,
    r: f32,
    width: f32,
    a0: f32,
    a1: f32,
    c: Rgb565,
) {
    if a1 <= a0 || r <= 0.0 {
        return;
    }
    let hw = width / 2.0;
    let full = a1 - a0 >= TAU - 1e-4;
    let caps = [
        (cx + a0.cos() * r, cy + a0.sin() * r),
        (cx + a1.cos() * r, cy + a1.sin() * r),
    ];
    let o = r + hw + 1.0;
    let (x0, y0, x1, y1) = bounds(cx - o, cy - o, cx + o, cy + o);
    for y in y0..=y1 {
        let py = y as f32 + 0.5;
        for x in x0..=x1 {
            let px = x as f32 + 0.5;
            let (dx, dy) = (px - cx, py - cy);
            let dist = dx.hypot(dy);
            let d = if full || in_arc(dy.atan2(dx), a0, a1) {
                (dist - r).abs() - hw
            } else {
                caps.iter()
                    .map(|&(qx, qy)| (px - qx).hypot(py - qy))
                    .fold(f32::MAX, f32::min)
                    - hw
            };
            blend(fb, x, y, c, cov(d));
        }
    }
}

/// Full ring.
pub fn ring(fb: &mut FrameBuffer480x272, cx: f32, cy: f32, r: f32, width: f32, c: Rgb565) {
    arc(fb, cx, cy, r, width, 0.0, TAU, c);
}

/// Line with round caps.
pub fn line_aa(
    fb: &mut FrameBuffer480x272,
    x0: f32,
    y0: f32,
    x1: f32,
    y1: f32,
    width: f32,
    c: Rgb565,
) {
    let hw = width / 2.0;
    let (bx0, by0, bx1, by1) = bounds(
        x0.min(x1) - hw,
        y0.min(y1) - hw,
        x0.max(x1) + hw,
        y0.max(y1) + hw,
    );
    let (vx, vy) = (x1 - x0, y1 - y0);
    let len2 = (vx * vx + vy * vy).max(1e-6);
    for y in by0..=by1 {
        let py = y as f32 + 0.5;
        for x in bx0..=bx1 {
            let px = x as f32 + 0.5;
            let t = (((px - x0) * vx + (py - y0) * vy) / len2).clamp(0.0, 1.0);
            let d = (px - (x0 + vx * t)).hypot(py - (y0 + vy * t)) - hw;
            blend(fb, x, y, c, cov(d));
        }
    }
}

/// Polyline; `closed` joins the last point back to the first.
pub fn path(fb: &mut FrameBuffer480x272, pts: &[(f32, f32)], width: f32, closed: bool, c: Rgb565) {
    for w in pts.windows(2) {
        line_aa(fb, w[0].0, w[0].1, w[1].0, w[1].1, width, c);
    }
    if closed && pts.len() > 2 {
        let (a, b) = (pts[pts.len() - 1], pts[0]);
        line_aa(fb, a.0, a.1, b.0, b.1, width, c);
    }
}

fn round_box_sd(px: f32, py: f32, x: f32, y: f32, w: f32, h: f32, r: f32) -> f32 {
    let (hx, hy) = (w / 2.0, h / 2.0);
    let (qx, qy) = (
        (px - (x + hx)).abs() - (hx - r),
        (py - (y + hy)).abs() - (hy - r),
    );
    qx.max(0.0).hypot(qy.max(0.0)) + qx.max(qy).min(0.0) - r
}

pub fn fill_round_rect(
    fb: &mut FrameBuffer480x272,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    r: f32,
    c: Rgb565,
) {
    let r = r.min(w / 2.0).min(h / 2.0);
    let (x0, y0, x1, y1) = bounds(x, y, x + w, y + h);
    for py in y0..=y1 {
        for px in x0..=x1 {
            let d = round_box_sd(px as f32 + 0.5, py as f32 + 0.5, x, y, w, h, r);
            blend(fb, px, py, c, cov(d));
        }
    }
}

/// Stroke centred on the box edge inset by half the width, so the outline
/// stays inside (x, y, w, h).
#[allow(clippy::too_many_arguments)]
pub fn stroke_round_rect(
    fb: &mut FrameBuffer480x272,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    r: f32,
    width: f32,
    c: Rgb565,
) {
    let hw = width / 2.0;
    let (ix, iy, iw, ih) = (x + hw, y + hw, w - width, h - width);
    let r = (r - hw).max(0.0).min(iw / 2.0).min(ih / 2.0);
    let (x0, y0, x1, y1) = bounds(x, y, x + w, y + h);
    for py in y0..=y1 {
        for px in x0..=x1 {
            let d = round_box_sd(px as f32 + 0.5, py as f32 + 0.5, ix, iy, iw, ih, r).abs() - hw;
            blend(fb, px, py, c, cov(d));
        }
    }
}

/// Filled convex polygon (clockwise or counter-clockwise).
pub fn fill_convex(fb: &mut FrameBuffer480x272, pts: &[(f32, f32)], c: Rgb565) {
    if pts.len() < 3 {
        return;
    }
    let (mut minx, mut miny, mut maxx, mut maxy) = (f32::MAX, f32::MAX, f32::MIN, f32::MIN);
    for &(x, y) in pts {
        minx = minx.min(x);
        miny = miny.min(y);
        maxx = maxx.max(x);
        maxy = maxy.max(y);
    }
    let n = pts.len() as f32;
    let (gx, gy) = pts
        .iter()
        .fold((0.0, 0.0), |acc, p| (acc.0 + p.0 / n, acc.1 + p.1 / n));
    // Outward unit normal and offset per edge.
    let edges: Vec<(f32, f32, f32)> = (0..pts.len())
        .map(|i| {
            let (a, b) = (pts[i], pts[(i + 1) % pts.len()]);
            let (ex, ey) = (b.0 - a.0, b.1 - a.1);
            let len = ex.hypot(ey).max(1e-6);
            let (mut nx, mut ny) = (ey / len, -ex / len);
            if nx * (gx - a.0) + ny * (gy - a.1) > 0.0 {
                nx = -nx;
                ny = -ny;
            }
            (nx, ny, nx * a.0 + ny * a.1)
        })
        .collect();
    let (x0, y0, x1, y1) = bounds(minx, miny, maxx, maxy);
    for y in y0..=y1 {
        let py = y as f32 + 0.5;
        for x in x0..=x1 {
            let px = x as f32 + 0.5;
            let d = edges
                .iter()
                .map(|&(nx, ny, o)| nx * px + ny * py - o)
                .fold(f32::MIN, f32::max);
            blend(fb, x, y, c, cov(d));
        }
    }
}

/// Calibration crosshairs `+` at the four corners of a technical view.
pub fn crosshairs(fb: &mut FrameBuffer480x272, x: i32, y: i32, w: i32, h: i32, c: Rgb565) {
    for (px, py) in [(x, y), (x + w, y), (x, y + h), (x + w, y + h)] {
        fb.fill_rect(px - 3, py, 7, 1, c);
        fb.fill_rect(px, py - 3, 1, 7, c);
    }
}

/// Angle helpers for the 270° dials (start bottom-left, end bottom-right).
pub const DIAL_A0: f32 = PI * 0.75;
pub const DIAL_A1: f32 = PI * 2.25;

#[cfg(test)]
mod tests {
    use super::*;

    fn lit(fb: &FrameBuffer480x272) -> usize {
        fb.pixels().iter().filter(|&&p| p != 0).count()
    }

    #[test]
    fn circle_area_is_about_pi_r_squared() {
        let mut fb = FrameBuffer480x272::new();
        fill_circle(&mut fb, 100.0, 100.0, 20.0, Rgb565::WHITE);
        let n = lit(&fb) as f32;
        assert!((n - PI * 400.0).abs() < 120.0, "{n}");
        // edge pixels are partially covered (grey), not all white
        assert!(fb.pixels().iter().any(|&p| p != 0 && p != 0xFFFF));
    }

    #[test]
    fn half_arc_only_covers_its_half() {
        let mut fb = FrameBuffer480x272::new();
        arc(&mut fb, 100.0, 100.0, 30.0, 3.0, 0.0, PI, Rgb565::WHITE);
        // clockwise from +x through +y (down): bottom half lit, top empty
        assert_ne!(fb.get(100, 130), Rgb565::BLACK);
        assert_eq!(fb.get(100, 70), Rgb565::BLACK);
    }

    #[test]
    fn triangle_fills_inside() {
        let mut fb = FrameBuffer480x272::new();
        fill_convex(
            &mut fb,
            &[(10.0, 10.0), (30.0, 20.0), (10.0, 30.0)],
            Rgb565::WHITE,
        );
        assert_eq!(fb.get(15, 20), Rgb565::WHITE);
        assert_eq!(fb.get(29, 12), Rgb565::BLACK);
    }

    #[test]
    fn shapes_off_screen_are_clipped() {
        let mut fb = FrameBuffer480x272::new();
        fill_circle(&mut fb, -10.0, -10.0, 30.0, Rgb565::WHITE);
        line_aa(&mut fb, -100.0, 300.0, 600.0, -20.0, 2.0, Rgb565::WHITE);
        stroke_round_rect(&mut fb, 460.0, 260.0, 60.0, 40.0, 6.0, 2.0, Rgb565::WHITE);
        ring(&mut fb, 479.0, 271.0, 50.0, 3.0, Rgb565::WHITE);
    }
}
