use embedded_graphics::pixelcolor::{self, RgbColor};

/// A 16-bit 5-6-5 colour, red in the top bits (the MK3 screen's native format).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Rgb565(pub u16);

impl Rgb565 {
    pub const BLACK: Self = Self(0x0000);
    pub const WHITE: Self = Self(0xFFFF);

    /// Pack 8-bit channels.
    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self(((r as u16 >> 3) << 11) | ((g as u16 >> 2) << 5) | (b as u16 >> 3))
    }

    /// Parse `0xRRGGBB`.
    pub const fn from_hex(rgb: u32) -> Self {
        Self::new((rgb >> 16) as u8, (rgb >> 8) as u8, rgb as u8)
    }

    /// Expand back to 8-bit channels (low bits replicated).
    pub const fn to_rgb888(self) -> (u8, u8, u8) {
        let r = ((self.0 >> 11) & 0x1f) as u8;
        let g = ((self.0 >> 5) & 0x3f) as u8;
        let b = (self.0 & 0x1f) as u8;
        (
            (r << 3) | (r >> 2),
            (g << 2) | (g >> 4),
            (b << 3) | (b >> 2),
        )
    }

    /// Linear blend towards `other`; `t` in 0..=1.
    pub fn lerp(self, other: Self, t: f32) -> Self {
        let t = t.clamp(0.0, 1.0);
        let (r0, g0, b0) = self.to_rgb888();
        let (r1, g1, b1) = other.to_rgb888();
        let m = |a: u8, b: u8| (a as f32 + (b as f32 - a as f32) * t).round() as u8;
        Self::new(m(r0, r1), m(g0, g1), m(b0, b1))
    }

    /// Relative luminance (WCAG): 0 black, 1 white.
    pub fn luminance(self) -> f32 {
        let (r, g, b) = self.to_rgb888();
        let lin = |c: u8| {
            let c = c as f32 / 255.0;
            if c <= 0.03928 { c / 12.92 } else { ((c + 0.055) / 1.055).powf(2.4) }
        };
        0.2126 * lin(r) + 0.7152 * lin(g) + 0.0722 * lin(b)
    }

    /// Contrast ratio between two colours (WCAG): 1 for the same, 21 for black on white.
    pub fn contrast(self, other: Self) -> f32 {
        let (a, b) = (self.luminance(), other.luminance());
        (a.max(b) + 0.05) / (a.min(b) + 0.05)
    }

    /// Darken by a factor in 0..=1 (0 = black).
    pub fn scale(self, k: f32) -> Self {
        Self::BLACK.lerp(self, k)
    }
}

impl From<Rgb565> for pixelcolor::Rgb565 {
    fn from(c: Rgb565) -> Self {
        let (r, g, b) = (
            ((c.0 >> 11) & 0x1f) as u8,
            ((c.0 >> 5) & 0x3f) as u8,
            (c.0 & 0x1f) as u8,
        );
        pixelcolor::Rgb565::new(r, g, b)
    }
}

impl From<pixelcolor::Rgb565> for Rgb565 {
    fn from(c: pixelcolor::Rgb565) -> Self {
        Self(((c.r() as u16) << 11) | ((c.g() as u16) << 5) | c.b() as u16)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn packs_primaries() {
        assert_eq!(Rgb565::new(255, 0, 0).0, 0xF800);
        assert_eq!(Rgb565::new(0, 255, 0).0, 0x07E0);
        assert_eq!(Rgb565::new(0, 0, 255).0, 0x001F);
        assert_eq!(Rgb565::from_hex(0xffffff), Rgb565::WHITE);
    }

    #[test]
    fn round_trips_through_embedded_graphics() {
        let c = Rgb565::from_hex(0x3fa9f5);
        let eg: pixelcolor::Rgb565 = c.into();
        assert_eq!(Rgb565::from(eg), c);
    }
}
