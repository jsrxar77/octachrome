//! Small drawing helpers in the OP-XY idiom: black background, 1–2 px white
//! lines, one accent colour. Built on `embedded-graphics` primitives.

use crate::{FrameBuffer480x272, Rgb565};
use embedded_graphics::{
    mono_font::{ascii, MonoTextStyle},
    pixelcolor,
    prelude::*,
    primitives::{Line, PrimitiveStyle, Rectangle},
    text::{Alignment, Baseline, Text, TextStyleBuilder},
};

/// Font sizes used across the UI.
#[derive(Clone, Copy, Debug)]
pub enum FontSize {
    /// 6×10, footer labels.
    Small,
    /// 9×15, values and headers.
    Medium,
    /// 10×20, the big BPM / instrument name.
    Large,
}

impl FontSize {
    fn font(self) -> &'static embedded_graphics::mono_font::MonoFont<'static> {
        match self {
            FontSize::Small => &ascii::FONT_6X10,
            FontSize::Medium => &ascii::FONT_9X15,
            FontSize::Large => &ascii::FONT_10X20,
        }
    }
    pub fn char_width(self) -> i32 {
        self.font().character_size.width as i32
    }
    pub fn height(self) -> i32 {
        self.font().character_size.height as i32
    }
}

#[derive(Clone, Copy, Debug)]
pub enum Align {
    Left,
    Center,
    Right,
}

/// Draw text with its top edge at `y`.
pub fn text(
    fb: &mut FrameBuffer480x272,
    s: &str,
    x: i32,
    y: i32,
    size: FontSize,
    color: Rgb565,
    align: Align,
) {
    let style = MonoTextStyle::new(size.font(), pixelcolor::Rgb565::from(color));
    let ts = TextStyleBuilder::new()
        .baseline(Baseline::Top)
        .alignment(match align {
            Align::Left => Alignment::Left,
            Align::Center => Alignment::Center,
            Align::Right => Alignment::Right,
        })
        .build();
    let _ = Text::with_text_style(s, Point::new(x, y), style, ts).draw(fb);
}

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
    fn text_draws_pixels_inside_bounds() {
        let mut fb = FrameBuffer480x272::new();
        text(
            &mut fb,
            "OP-3",
            10,
            10,
            FontSize::Medium,
            Rgb565::WHITE,
            Align::Left,
        );
        let lit = fb.pixels().iter().filter(|&&p| p != 0).count();
        assert!(lit > 20);
        // nothing far right of the text
        assert!((100..480).all(|x| fb.get(x, 15) == Rgb565::BLACK));
    }

    #[test]
    fn drawing_off_screen_is_clipped() {
        let mut fb = FrameBuffer480x272::new();
        line(&mut fb, -50, -50, 600, 400, 2, Rgb565::WHITE);
        fb.fill_rect(470, 260, 50, 50, Rgb565::WHITE);
        text(
            &mut fb,
            "clipped",
            470,
            265,
            FontSize::Large,
            Rgb565::WHITE,
            Align::Left,
        );
    }
}
