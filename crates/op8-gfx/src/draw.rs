//! Integer drawing helpers (hairlines, boxes) on `embedded-graphics`, plus the
//! re-exported text API from [`crate::font`].

use crate::{FrameBuffer480x272, Rgb565};
use embedded_graphics::{
    prelude::*,
    primitives::{Line, PrimitiveStyle, Rectangle},
};

pub use crate::font::{text, text_v, text_width, Align, FontSize, VAlign};

pub fn line(
    fb: &mut FrameBuffer480x272,
    x0: i32,
    y0: i32,
    x1: i32,
    y1: i32,
    width: u32,
    color: Rgb565,
) {
    let _ = Line::new(Point::new(x0, y0), Point::new(x1, y1))
        .into_styled(PrimitiveStyle::with_stroke(color.into(), width))
        .draw(fb);
}

pub fn rect_outline(
    fb: &mut FrameBuffer480x272,
    x: i32,
    y: i32,
    w: u32,
    h: u32,
    width: u32,
    color: Rgb565,
) {
    let _ = Rectangle::new(Point::new(x, y), Size::new(w, h))
        .into_styled(PrimitiveStyle::with_stroke(color.into(), width))
        .draw(fb);
}

/// Polyline through `points`, e.g. an oscilloscope trace.
pub fn polyline(
    fb: &mut FrameBuffer480x272,
    points: impl IntoIterator<Item = (i32, i32)>,
    width: u32,
    color: Rgb565,
) {
    let mut prev: Option<(i32, i32)> = None;
    for p in points {
        if let Some(q) = prev {
            line(fb, q.0, q.1, p.0, p.1, width, color);
        }
        prev = Some(p);
    }
}

/// A horizontal value bar (0..=1) with an outline, used in the footers.
pub fn value_bar(
    fb: &mut FrameBuffer480x272,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    v: f32,
    color: Rgb565,
) {
    rect_outline(fb, x, y, w as u32, h as u32, 1, Rgb565::WHITE.scale(0.5));
    let fill = ((w - 4) as f32 * v.clamp(0.0, 1.0)).round() as i32;
    fb.fill_rect(x + 2, y + 2, fill, h - 4, color);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn drawing_off_screen_is_clipped() {
        let mut fb = FrameBuffer480x272::new();
        line(&mut fb, -50, -50, 600, 400, 2, Rgb565::WHITE);
        fb.fill_rect(470, 260, 50, 50, Rgb565::WHITE);
    }
}
