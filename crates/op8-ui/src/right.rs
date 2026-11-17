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
    draw_right_view(fb, ui, cartridge, params);
    if ui.main == crate::Main::Instrument && ui.held == Some(crate::Held::Volume) {
        return crate::left::mixer_footer(fb, ui, 4);
    }
    if let Some(it) = crate::nav::focused(ui).filter(|it| it.screen == 1 && ui.main == crate::Main::Instrument && ui.held != Some(crate::Held::Volume)) {
        crate::nav::draw_ring(fb, &it, ui);
    }
}

fn draw_right_view(
    fb: &mut FrameBuffer480x272,
    ui: &UiState,
    cartridge: Option<&SharedCartridge>,
    params: &[f32; 8],
) {
    if ui.main == crate::Main::Settings {
        return crate::settings::draw_right(fb, ui);
    }
    if ui.main == crate::Main::Mixer {
        return crate::mixer::draw_right(fb, ui);
    }
    if ui.main == crate::Main::File {
        return crate::files::draw_right(fb, ui);
    }
    if ui.main == crate::Main::Browser {
        let title = format!("browser · {}", ui.browse_inst);
        return crate::files::pick_right(fb, &title, &[("tap a pad", "loads that preset"), ("shift + tap", "saves the knobs into your presets"), ("erase + tap twice", "erases one of yours")], &ui.browse_note);
    }
    if ui.main == crate::Main::Sampling {
        return crate::sampling::draw_right(fb, ui);
    }
    if ui.main == crate::Main::Plugin {
        return crate::files::pick_right(fb, "plug-in", &[("tap a pad", "puts that instrument on the selected track")], &ui.plug_note);
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
    // the pages of the right screen: sound, shape (if the instrument has one), fx and mod
    let page = crate::right_page(ui);
    let mut pages = vec![0usize];
    if c.secondary_parameters().is_some() {
        pages.push(1);
    }
    pages.extend([2, 3]);
    for (i, pg) in pages.iter().enumerate() {
        let col = if *pg == page { t.fg } else { t.faint };
        fill_circle(fb, 456.0 - (pages.len() - 1 - i) as f32 * 12.0, HEADER_CY, 3.0, col);
    }

    button_strip(fb, &crate::strip_labels(ui, 1));
    match page {
        2 => crate::pages::fx_view(fb, ui),
        3 => crate::pages::lfo_view(fb, ui),
        _ => c.draw_ui(fb, accent),
    }

    fb.fill_rect(0, AREA_Y + AREA_H, 480, 272 - (AREA_Y + AREA_H), t.bg);
    if page < 2 {
        crosshairs(fb, 14, 50, 452, 168, t.dim);
    }

    let cells: [FooterCell; 4] = match page {
        2 => crate::fx_footer(ui),
        3 => crate::lfo_footer(ui),
        _ => {
            let (descs, base) = match (page, c.secondary_parameters()) {
                (1, Some(s)) => (s, 4),
                _ => (c.parameters(), 0),
            };
            std::array::from_fn(|i| {
                let v = c.parameter_value(base + i).unwrap_or(params[base + i]);
                FooterCell::new(descs[i].name, descs[i].format(v).to_lowercase(), Some(v))
            })
        }
    };
    draw_footer(fb, &cells, 4);
    touch_overlay(fb, &cells, 4, &ui.anim);
}
