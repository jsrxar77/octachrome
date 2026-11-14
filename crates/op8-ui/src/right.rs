//! Right screen: the instrument of the selected track. Encoders 5–8.
//!
//! The host draws the top bar, the calibration crosshairs and the footer;
//! the cartridge draws its own animated picture in between with `draw_ui`,
//! using `op8_gfx::knob(4..8)` for what encoders 5–8 move.

use crate::{draw_footer, theme::*, touch_overlay, FooterCell, UiState};
use op8_engine::SharedCartridge;
use op8_gfx::{
    paint::{crosshairs, fill_circle, line_aa, ring},
    text_v, theme, Align, FontSize, FrameBuffer480x272, VAlign,
};

pub fn draw_right(
    fb: &mut FrameBuffer480x272,
    ui: &UiState,
    cartridge: Option<&SharedCartridge>,
    params: &[f32; 8],
) {
    if ui.main == crate::Main::Settings {
        return crate::settings::draw_right(fb);
    }
    let t = theme();
    let accent = track_color(ui.track);
    let Some(c) = cartridge else {
        fb.clear(t.bg);
        fill_circle(fb, 16.0, HEADER_CY, 6.0, accent);
        text_v(
            fb,
            "empty",
            30,
            HEADER_CY as i32,
            FontSize::Large,
            t.dim,
            Align::Left,
            VAlign::Middle,
        );
        ring(fb, 240.0, 126.0, 50.0, 2.0, t.faint);
        line_aa(fb, 205.0, 161.0, 275.0, 91.0, 2.0, t.faint);
        return;
    };
    // If the audio thread holds the cartridge, keep the previous frame.
    let Ok(c) = c.try_lock() else { return };
    fb.clear(t.bg);
    fill_circle(fb, 16.0, HEADER_CY, 6.0, accent);
    text_v(
        fb,
        &c.info().name.to_lowercase(),
        30,
        HEADER_CY as i32,
        FontSize::Large,
        t.fg,
        Align::Left,
        VAlign::Middle,
    );
    let pages = if c.secondary_parameters().is_some() {
        2
    } else {
        1
    };
    if pages > 1 {
        for i in 0..pages {
            let col = if i == ui.right_page { t.fg } else { t.faint };
            fill_circle(fb, 456.0 + i as f32 * 12.0, HEADER_CY, 3.0, col);
        }
    }

    button_strip(fb, &crate::strip_labels(ui, 1));
    c.draw_ui(fb, accent);

    // Keep the cartridge inside its area.
    fb.fill_rect(0, AREA_Y + AREA_H, 480, 272 - (AREA_Y + AREA_H), t.bg);
    crosshairs(fb, 14, 50, 452, 168, t.dim);

    let (descs, base) = match (ui.right_page, c.secondary_parameters()) {
        (1, Some(s)) => (s, 4),
        _ => (c.parameters(), 0),
    };
    let cells: [FooterCell; 4] = std::array::from_fn(|i| {
        let v = c.parameter_value(base + i).unwrap_or(params[base + i]);
        FooterCell::new(descs[i].name, descs[i].format(v).to_lowercase(), Some(v))
    });
    draw_footer(fb, &cells, 4);
    touch_overlay(fb, &cells, 4, &ui.anim);
}
