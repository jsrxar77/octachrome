//! Jost, the single typeface of OP-3 (DESIGN.md §3), rasterised with
//! `fontdue` and alpha-blended onto the RGB565 framebuffer.
//!
//! Glyphs are cached per (char, size, weight) on first use, so steady-state
//! drawing is a blit. Fonts: SIL Open Font License 1.1 (`assets/fonts/OFL.txt`).

use crate::{paint::blend, FrameBuffer480x272, Rgb565};
use fontdue::{Font, FontSettings, Metrics};
use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::OnceLock;

static REGULAR: &[u8] = include_bytes!("../../../assets/fonts/Jost-Regular.ttf");
static LIGHT: &[u8] = include_bytes!("../../../assets/fonts/Jost-Light.ttf");

fn fonts() -> &'static [Font; 2] {
    static F: OnceLock<[Font; 2]> = OnceLock::new();
    F.get_or_init(|| {
        let load = |b: &[u8]| Font::from_bytes(b, FontSettings::default()).expect("bundled Jost");
        [load(REGULAR), load(LIGHT)]
    })
}

/// The fixed type scale (DESIGN.md §3).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FontSize {
    /// 11 px, parameter names under the dials.
    Small,
    /// 15 px, values under the dials.
    Medium,
    /// 20 px, inspector values.
    Value,
    /// 21 px, instrument name, chord, track letter.
    Large,
    /// 34 px light, step number.
    Big,
    /// 62 px light, value while an encoder is touched.
    Huge,
}

impl FontSize {
    /// (pixel size, light weight?, letter spacing px)
    const fn spec(self) -> (f32, bool, f32) {
        match self {
            FontSize::Small => (11.0, false, 0.6),
            FontSize::Medium => (15.0, false, 0.3),
            FontSize::Value => (20.0, false, 0.2),
            FontSize::Large => (21.0, false, 0.2),
            FontSize::Big => (34.0, true, 0.0),
            FontSize::Huge => (62.0, true, -1.0),
        }
    }
    pub fn px(self) -> f32 {
        self.spec().0
    }
    /// Height of a line box (ascent + descent), in pixels.
    pub fn height(self) -> i32 {
        let (px, light, _) = self.spec();
        let m = fonts()[light as usize]
            .horizontal_line_metrics(px)
            .expect("metrics");
        (m.ascent - m.descent).ceil() as i32
    }
    fn ascent(self) -> f32 {
        let (px, light, _) = self.spec();
        fonts()[light as usize]
            .horizontal_line_metrics(px)
            .expect("metrics")
            .ascent
    }
    /// Height of a capital letter, used to centre text vertically.
    fn cap(self) -> f32 {
        self.px() * 0.70
    }
}

#[derive(Clone, Copy, Debug)]
pub enum Align {
    Left,
    Center,
    Right,
}

#[derive(Clone, Copy, Debug)]
pub enum VAlign {
    /// `y` is the top of the line box.
    Top,
    /// `y` is the middle of the capitals.
    Middle,
}

type Glyph = (Metrics, Vec<u8>);

thread_local! {
    static CACHE: RefCell<HashMap<(char, FontSize), Glyph>> = RefCell::new(HashMap::new());
}

fn with_glyph<R>(ch: char, size: FontSize, f: impl FnOnce(&Glyph) -> R) -> R {
    CACHE.with(|c| {
        let mut c = c.borrow_mut();
        let g = c.entry((ch, size)).or_insert_with(|| {
            let (px, light, _) = size.spec();
            fonts()[light as usize].rasterize(ch, px)
        });
        f(g)
    })
}

/// Advance width of `s` in pixels, letter spacing included.
pub fn text_width(s: &str, size: FontSize) -> f32 {
    let sp = size.spec().2;
    let mut w = 0.0;
    let mut n = 0;
    for ch in s.chars() {
        w += with_glyph(ch, size, |g| g.0.advance_width);
        n += 1;
    }
    w + sp * (n.max(1) - 1) as f32
}

/// Draw `s` at (x, y). Returns the x after the last glyph.
#[allow(clippy::too_many_arguments)]
pub fn text_v(
    fb: &mut FrameBuffer480x272,
    s: &str,
    x: i32,
    y: i32,
    size: FontSize,
    color: Rgb565,
    align: Align,
    valign: VAlign,
) -> i32 {
    let w = text_width(s, size);
    let mut pen = match align {
        Align::Left => x as f32,
        Align::Center => x as f32 - w / 2.0,
        Align::Right => x as f32 - w,
    };
    let baseline = match valign {
        VAlign::Top => y as f32 + size.ascent(),
        VAlign::Middle => y as f32 + size.cap() / 2.0,
    }
    .round();
    let sp = size.spec().2;
    for ch in s.chars() {
        let adv = with_glyph(ch, size, |(m, bmp)| {
            let gx = pen.round() as i32 + m.xmin;
            let gy = baseline as i32 - m.height as i32 - m.ymin;
            for row in 0..m.height {
                for col in 0..m.width {
                    let a = bmp[row * m.width + col];
                    if a > 0 {
                        // slight gamma lift so thin strokes survive on black
                        let a = (a as f32 / 255.0).powf(0.85);
                        blend(fb, gx + col as i32, gy + row as i32, color, a);
                    }
                }
            }
            m.advance_width
        });
        pen += adv + sp;
    }
    pen.round() as i32
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
    text_v(fb, s, x, y, size, color, align, VAlign::Top);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_is_antialiased_and_bounded() {
        let mut fb = FrameBuffer480x272::new();
        text(
            &mut fb,
            "fmbass 126ms",
            10,
            10,
            FontSize::Medium,
            Rgb565::WHITE,
            Align::Left,
        );
        let lit = fb.pixels().iter().filter(|&&p| p != 0).count();
        assert!(lit > 100, "{lit}");
        assert!(fb.pixels().iter().any(|&p| p != 0 && p != 0xFFFF), "no AA");
        let w = text_width("fmbass 126ms", FontSize::Medium) as usize;
        assert!((10 + w + 2..480).all(|x| (0..40).all(|y| fb.get(x, y) == Rgb565::BLACK)));
    }

    #[test]
    fn sizes_grow() {
        let ws: Vec<f32> = [
            FontSize::Small,
            FontSize::Medium,
            FontSize::Large,
            FontSize::Huge,
        ]
        .iter()
        .map(|&s| text_width("122", s))
        .collect();
        assert!(ws.windows(2).all(|w| w[0] < w[1]), "{ws:?}");
    }

    #[test]
    fn clipped_text_does_not_panic() {
        let mut fb = FrameBuffer480x272::new();
        text(
            &mut fb,
            "clipped",
            470,
            265,
            FontSize::Huge,
            Rgb565::WHITE,
            Align::Left,
        );
        text_v(
            &mut fb,
            "x",
            -20,
            -20,
            FontSize::Big,
            Rgb565::WHITE,
            Align::Center,
            VAlign::Middle,
        );
    }
}
