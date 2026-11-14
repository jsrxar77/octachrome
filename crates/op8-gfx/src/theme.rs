//! Design tokens from `docs/design.md` (UX/UI treatise).
//!
//! Eight encoders, eight fixed colours: knobs 1–4 drive the sequencer (left
//! screen), 5–8 the instrument (right screen). Whatever an encoder moves is
//! drawn in that encoder's colour. The active theme is global so cartridges
//! can use it from `draw_ui` without changing the cartridge trait.

use crate::Rgb565;
use std::sync::atomic::{AtomicU8, Ordering};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ThemeId {
    Op1 = 0,
    Op1Field = 1,
    OpXy = 2,
}

impl ThemeId {
    pub const ALL: [ThemeId; 3] = [ThemeId::Op1, ThemeId::Op1Field, ThemeId::OpXy];
    pub fn name(self) -> &'static str {
        theme_of(self).name
    }
    pub fn next(self) -> Self {
        Self::ALL[(self as usize + 1) % Self::ALL.len()]
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Theme {
    pub name: &'static str,
    pub bg: Rgb565,
    pub fg: Rgb565,
    /// Context: visible but not editable right now.
    pub dim: Rgb565,
    /// Guides, passive borders, empty steps.
    pub faint: Rgb565,
    /// Keycap and pill surface.
    pub panel: Rgb565,
    /// Encoder colours, knob 1..=8.
    pub knobs: [Rgb565; 8],
    /// The brand mark: eight buttons in rainbow order, the eighth is white
    /// (the theme's own white). See `docs/design.md` §2.3.
    pub brand: [Rgb565; 8],
    /// Only REC, erase and peaks.
    pub rec: Rgb565,
}

const fn c(hex: u32) -> Rgb565 {
    Rgb565::from_hex(hex)
}

pub const THEMES: [Theme; 3] = [
    Theme {
        name: "op-1",
        bg: c(0x000000),
        fg: c(0xF2F4F8),
        dim: c(0x7E8794),
        faint: c(0x232831),
        panel: c(0x0D1118),
        knobs: [
            c(0x0099FF),
            c(0x00E56A),
            c(0xF2F4F8),
            c(0xFF4F00),
            c(0x8B6CFF),
            c(0xFFD21F),
            c(0x2ED3E6),
            c(0xFF4FA0),
        ],
        brand: [
            c(0xFF2D2D),
            c(0xFF8A1F),
            c(0xFFD21F),
            c(0x00E56A),
            c(0x0099FF),
            c(0x4B4BFF),
            c(0xB15CFF),
            c(0xF2F4F8),
        ],
        rec: c(0xFF3B30),
    },
    Theme {
        name: "op-1 field",
        bg: c(0x000000),
        fg: c(0xF4EFE6),
        dim: c(0x8E918F),
        faint: c(0x262A30),
        panel: c(0x12161D),
        knobs: [
            c(0x3B8EBA),
            c(0xD9822B),
            c(0xA9AFB8),
            c(0xF2652A),
            c(0x7FB58A),
            c(0xE3C77A),
            c(0xA88BC7),
            c(0xD9677A),
        ],
        brand: [
            c(0xD9534F),
            c(0xD9822B),
            c(0xE3C77A),
            c(0x7FB58A),
            c(0x3B8EBA),
            c(0x5F6BB5),
            c(0xA88BC7),
            c(0xF4EFE6),
        ],
        rec: c(0xE63946),
    },
    Theme {
        name: "op-xy",
        bg: c(0x000000),
        fg: c(0xEEF0F5),
        dim: c(0x80858F),
        faint: c(0x24262C),
        panel: c(0x0F1014),
        knobs: [
            c(0x94A8C6),
            c(0x00F076),
            c(0xEEF0F5),
            c(0xFF3844),
            c(0xFFD60A),
            c(0x34D3E8),
            c(0x9D7BFF),
            c(0xFF8A1F),
        ],
        brand: [
            c(0xFF3B2F),
            c(0xFF8A1F),
            c(0xFFD60A),
            c(0x00F076),
            c(0x4C8DFF),
            c(0x6A5CFF),
            c(0xB07BFF),
            c(0xEEF0F5),
        ],
        rec: c(0xFF2244),
    },
];

static CURRENT: AtomicU8 = AtomicU8::new(ThemeId::Op1 as u8);

pub fn theme_of(id: ThemeId) -> &'static Theme {
    &THEMES[id as usize]
}

/// The active theme.
pub fn theme() -> &'static Theme {
    &THEMES[(CURRENT.load(Ordering::Relaxed) as usize).min(THEMES.len() - 1)]
}

pub fn theme_id() -> ThemeId {
    ThemeId::ALL[(CURRENT.load(Ordering::Relaxed) as usize).min(THEMES.len() - 1)]
}

pub fn set_theme(id: ThemeId) {
    CURRENT.store(id as u8, Ordering::Relaxed);
}

/// Colour of encoder `i` (0-based: 0..=3 sequencer, 4..=7 instrument).
pub fn knob(i: usize) -> Rgb565 {
    theme().knobs[i % 8]
}

#[cfg(test)]
mod brand_tests {
    use super::*;

    fn hue(c: Rgb565) -> f32 {
        let (r, g, b) = c.to_rgb888();
        let (r, g, b) = (r as f32, g as f32, b as f32);
        let (mx, mn) = (r.max(g).max(b), r.min(g).min(b));
        let d = mx - mn;
        let h = if mx == r { ((g - b) / d).rem_euclid(6.0) } else if mx == g { (b - r) / d + 2.0 } else { (r - g) / d + 4.0 };
        h * 60.0
    }

    // F-D09
    #[test]
    fn f_d09_each_theme_has_eight_brand_colours_in_fixed_rainbow_order_ending_in_white() {
        for t in THEMES.iter() {
            assert_eq!(t.brand[7], t.fg, "{}: the eighth brand button is the theme's white", t.name);
            // The order is fixed: red, orange, yellow, green, blue, indigo, violet, then white.
            // A theme may change the range and the hue a little, never the order.
            const BANDS: [(&str, f32, f32); 7] = [
                ("red", -1.0, 15.0),
                ("orange", 15.0, 42.0),
                ("yellow", 42.0, 75.0),
                ("green", 100.0, 170.0),
                ("blue", 190.0, 228.0),
                ("indigo", 228.0, 258.0),
                ("violet", 258.0, 300.0),
            ];
            for (i, (name, lo, hi)) in BANDS.iter().enumerate() {
                let h = hue(t.brand[i]);
                assert!(h > *lo && h <= *hi, "{}: button {} must be {}, hue {}", t.name, i + 1, name, h);
            }
        }
    }
}
