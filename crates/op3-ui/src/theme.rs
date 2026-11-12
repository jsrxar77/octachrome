use op3_gfx::Rgb565;

pub const BG: Rgb565 = Rgb565::BLACK;
pub const FG: Rgb565 = Rgb565::WHITE;
pub const DIM: Rgb565 = Rgb565::from_hex(0x6a6a6a);
pub const FAINT: Rgb565 = Rgb565::from_hex(0x262626);

/// Accent per group A–H (matched to the pad palette in the app).
pub const TRACK_COLORS: [Rgb565; 8] = [
    Rgb565::from_hex(0xff3b30), // A red
    Rgb565::from_hex(0xff9500), // B orange
    Rgb565::from_hex(0xffd60a), // C yellow
    Rgb565::from_hex(0x34c759), // D green
    Rgb565::from_hex(0x30d5c8), // E turquoise
    Rgb565::from_hex(0x0a84ff), // F blue
    Rgb565::from_hex(0xbf5af2), // G violet
    Rgb565::from_hex(0xff2d92), // H magenta
];

pub const TRACK_NAMES: [&str; 8] = ["A", "B", "C", "D", "E", "F", "G", "H"];

pub const FOOTER_Y: i32 = 224;
pub const FOOTER_COL: i32 = 120;

pub struct FooterCell {
    pub label: String,
    pub value: String,
    /// 0..=1 bar, or None for no bar.
    pub bar: Option<f32>,
}

impl FooterCell {
    pub fn new(label: impl Into<String>, value: impl Into<String>, bar: Option<f32>) -> Self {
        Self {
            label: label.into(),
            value: value.into(),
            bar,
        }
    }
    pub fn empty() -> Self {
        Self::new("", "", None)
    }
}

/// Draw the four 120 px footer columns. `touched[i]` inverts the label, the
/// visual response to cap-touch on that encoder.
pub fn draw_footer(
    fb: &mut op3_gfx::FrameBuffer480x272,
    cells: &[FooterCell; 4],
    touched: &[bool],
    accent: Rgb565,
) {
    use op3_gfx::{text, value_bar, Align, FontSize};
    fb.fill_rect(0, FOOTER_Y - 2, 480, 1, FAINT);
    for (i, c) in cells.iter().enumerate() {
        let x = i as i32 * FOOTER_COL;
        let t = touched.get(i).copied().unwrap_or(false);
        if t {
            fb.fill_rect(x + 2, FOOTER_Y, FOOTER_COL - 4, 12, accent);
        }
        text(
            fb,
            &c.label,
            x + FOOTER_COL / 2,
            FOOTER_Y + 1,
            FontSize::Small,
            if t { BG } else { DIM },
            Align::Center,
        );
        text(
            fb,
            &c.value,
            x + FOOTER_COL / 2,
            FOOTER_Y + 14,
            FontSize::Medium,
            FG,
            Align::Center,
        );
        if let Some(v) = c.bar {
            value_bar(fb, x + 10, FOOTER_Y + 34, FOOTER_COL - 20, 8, v, accent);
        }
        if i > 0 {
            fb.fill_rect(x, FOOTER_Y + 2, 1, 44, FAINT);
        }
    }
}
