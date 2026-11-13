//! Design tokens from `docs/DESIGN.md` (Tratado de UX/UI).
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
