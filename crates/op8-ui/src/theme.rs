//! Screen furniture shared by both screens: track colours, the four-dial
//! footer and the cap-touch overlay (design.md §4–§5). Colours come from the
//! active [`op8_gfx::Theme`]; knob `i` always draws in `knob(i)`.

use crate::Anim;
use op8_gfx::{
    knob,
    paint::{arc, fill_circle, ring, DIAL_A0, DIAL_A1},
    text, text_v, theme, Align, FontSize, FrameBuffer480x272, Rgb565, VAlign,
};
use op8_seq::StepComponent;

/// Accent of group A–H in the active theme (matched to the pad LEDs in the
/// app). Used only to identify the track: dots, rings, pad fills (design.md rule 10).
pub fn track_color(i: usize) -> Rgb565 {
    op8_gfx::theme().tracks[i]
}

pub const TRACK_NAMES: [&str; 8] = ["A", "B", "C", "D", "E", "F", "G", "H"];

/// Height of the strip that names the 4 screen buttons above each screen.
pub const STRIP_H: i32 = 24;
/// Centre of each of the 4 screen buttons, in screen pixels (measured on the unit: about 140 px apart).
pub const BUTTON_CX: [i32; 4] = [24, 164, 304, 444];
/// Vertical centre of the header items (name, tempo, chord, track dots), below the strip.
pub const HEADER_CY: f32 = 37.0;

pub const FOOTER_Y: i32 = 224;
pub const FOOTER_COL: i32 = 120;
/// Drawing area between the top bar and the footer.
pub const AREA_Y: i32 = 32;
pub const AREA_H: i32 = 186;

/// A small picture that replaces the dial for some parameters.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum FooterGlyph {
    /// Track length as dots on a ring.
    Length(u8),
    /// A step component.
    Component(StepComponent),
}

#[derive(Clone, Debug, PartialEq)]
pub struct FooterCell {
    pub label: String,
    pub value: String,
    /// 0..=1 dial position, or None for no dial.
    pub bar: Option<f32>,
    pub glyph: Option<FooterGlyph>,
}

impl FooterCell {
    pub fn new(label: impl Into<String>, value: impl Into<String>, bar: Option<f32>) -> Self {
        Self {
            label: label.into().to_lowercase(),
            value: value.into(),
            bar,
            glyph: None,
        }
    }
    pub fn with_glyph(mut self, g: FooterGlyph) -> Self {
        self.glyph = Some(g);
        self
    }
    pub fn empty() -> Self {
        Self::new("", "", None)
    }
}

/// 270° dial in the encoder's colour.
pub fn dial(fb: &mut FrameBuffer480x272, cx: f32, cy: f32, r: f32, v: f32, c: Rgb565) {
    let t = theme();
    arc(fb, cx, cy, r, 3.0, DIAL_A0, DIAL_A1, t.faint);
    let a = DIAL_A0 + (DIAL_A1 - DIAL_A0) * v.clamp(0.0, 1.0);
    if a > DIAL_A0 + 0.01 {
        arc(fb, cx, cy, r, 3.0, DIAL_A0, a, c);
    }
    fill_circle(fb, cx + a.cos() * r, cy + a.sin() * r, 3.0, c);
}

/// Draw the four footer columns for encoders `base + 0..4`.
pub fn draw_footer(fb: &mut FrameBuffer480x272, cells: &[FooterCell; 4], base: usize) {
    let t = theme();
    for (i, c) in cells.iter().enumerate() {
        if c.label.is_empty() && c.value.is_empty() {
            continue;
        }
        let x = i as i32 * FOOTER_COL;
        let col = knob(base + i);
        let (cx, cy) = (x as f32 + 26.0, 247.0);
        match c.glyph {
            Some(FooterGlyph::Length(len)) => {
                ring(fb, cx, cy, 13.0, 1.0, t.faint);
                for s in 0..len {
                    let a = -std::f32::consts::FRAC_PI_2
                        + s as f32 / len as f32 * std::f32::consts::TAU;
                    fill_circle(fb, cx + a.cos() * 13.0, cy + a.sin() * 13.0, 2.0, col);
                }
            }
            Some(FooterGlyph::Component(comp)) if comp != StepComponent::None => {
                crate::left::comp_glyph(fb, comp, cx - 17.0, cy - 11.0, col)
            }
            _ => {
                if let Some(v) = c.bar {
                    dial(fb, cx, cy, 14.0, v, col)
                }
            }
        }
        text(
            fb,
            &c.value,
            x + 48,
            233,
            FontSize::Medium,
            t.fg,
            Align::Left,
        );
        text(
            fb,
            &c.label,
            x + 48,
            253,
            FontSize::Small,
            col.scale(0.85),
            Align::Left,
        );
    }
}

/// "What am I touching": fades in while an encoder of this screen is touched.
pub fn touch_overlay(
    fb: &mut FrameBuffer480x272,
    cells: &[FooterCell; 4],
    base: usize,
    anim: &Anim,
) {
    let (mut best, mut v) = (None, 0.05);
    for i in 0..4 {
        if anim.touch[base + i] > v {
            v = anim.touch[base + i];
            best = Some(i);
        }
    }
    let Some(i) = best else { return };
    let c = &cells[i];
    if c.label.is_empty() {
        return;
    }
    let t = theme();
    let col = knob(base + i);
    // darken the drawing area
    let k = 1.0 - 0.88 * v;
    for y in AREA_Y..AREA_Y + AREA_H {
        for x in 0..480 {
            let p = fb.get(x as usize, y as usize);
            fb.put(x, y, p.scale(k));
        }
    }
    let fade = |c: Rgb565| Rgb565::BLACK.lerp(c, v);
    text_v(
        fb,
        &c.label,
        240,
        64,
        FontSize::Large,
        fade(col),
        Align::Center,
        VAlign::Middle,
    );
    text_v(
        fb,
        &c.value,
        240,
        128,
        FontSize::Huge,
        fade(t.fg),
        Align::Center,
        VAlign::Middle,
    );
    if let Some(b) = c.bar {
        fb.fill_rect(120, 182, 240, 3, fade(t.faint));
        fb.fill_rect(120, 182, (240.0 * b.clamp(0.0, 1.0)) as i32, 3, fade(col));
    }
}

/// The strip along the top of a screen: the name of what each of the 4 buttons
/// above it does (rule 11), in a capsule centred on the button. A button with no
/// function shows nothing; the page that is open has an `fg` outline and `fg`
/// text, the others a `faint` outline and `dim` text.
pub fn button_strip(fb: &mut FrameBuffer480x272, labels: &[(&str, bool); 4]) {
    use op8_gfx::{paint::fill_round_rect, text_width, FontSize};
    let t = op8_gfx::theme();
    for (i, (name, on)) in labels.iter().enumerate() {
        if name.is_empty() {
            continue;
        }
        // Whole pixels, so the 1 px outline falls exactly on pixel rows and columns and stays crisp.
        let w = (text_width(name, FontSize::Label) + 28.0).round();
        let x = (BUTTON_CX[i] as f32 - w / 2.0).round().clamp(1.0, 479.0 - w);
        let h = (STRIP_H - 2) as f32;
        // The outline is a 1 px band between two filled shapes on whole pixels, so it is
        // the same width all the way round, on the curves as on the straight sides.
        fill_round_rect(fb, x, 1.0, w, h, h / 2.0, if *on { t.fg } else { t.faint });
        fill_round_rect(fb, x + 1.0, 2.0, w - 2.0, h - 2.0, (h - 2.0) / 2.0, t.panel);
        // Centred as the eye does (design.md §3): ink horizontally, x-height vertically.
        op8_gfx::text_centered(
            fb,
            name,
            x + w / 2.0,
            1.0 + h / 2.0,
            FontSize::Label,
            if *on { t.fg } else { t.dim },
        );
    }
}
