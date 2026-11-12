use crate::{draw_footer, theme::*, FooterCell, UiState};
use op3_engine::SharedCartridge;
use op3_gfx::{text, Align, FontSize, FrameBuffer480x272};

/// Right screen: header, the cartridge's own UI, and its 4 knobs.
///
/// `params` is the host's view of the slot's 8 normalised values (main page
/// then secondary). If the cartridge is busy on the audio thread the frame
/// keeps its previous instrument area rather than waiting.
pub fn draw_right(
    fb: &mut FrameBuffer480x272,
    ui: &UiState,
    cartridge: Option<&SharedCartridge>,
    params: &[f32; 8],
) {
    let accent = TRACK_COLORS[ui.track];
    let Some(c) = cartridge else {
        fb.clear(BG);
        text(
            fb,
            TRACK_NAMES[ui.track],
            12,
            6,
            FontSize::Large,
            accent,
            Align::Left,
        );
        text(
            fb,
            "EMPTY SLOT",
            240,
            120,
            FontSize::Large,
            DIM,
            Align::Center,
        );
        return;
    };
    let Ok(c) = c.try_lock() else { return };
    fb.clear(BG);
    let info = c.info();
    text(
        fb,
        TRACK_NAMES[ui.track],
        12,
        6,
        FontSize::Large,
        accent,
        Align::Left,
    );
    text(
        fb,
        &info.name.to_uppercase(),
        36,
        6,
        FontSize::Large,
        FG,
        Align::Left,
    );
    text(
        fb,
        &info.category.to_uppercase(),
        472,
        10,
        FontSize::Small,
        DIM,
        Align::Right,
    );
    let page_count = if c.secondary_parameters().is_some() {
        2
    } else {
        1
    };
    if page_count > 1 {
        text(
            fb,
            &format!("PG {}/{}", ui.right_page + 1, page_count),
            400,
            10,
            FontSize::Small,
            DIM,
            Align::Right,
        );
    }
    fb.fill_rect(0, 30, 480, 1, FAINT);

    c.draw_ui(fb, accent);

    let (descs, base) = match (ui.right_page, c.secondary_parameters()) {
        (1, Some(s)) => (s, 4),
        _ => (c.parameters(), 0),
    };
    // Clear the footer band in case draw_ui spilled into it.
    fb.fill_rect(0, FOOTER_Y - 2, 480, 50, BG);
    let cells: [FooterCell; 4] = std::array::from_fn(|i| {
        let v = c.parameter_value(base + i).unwrap_or(params[base + i]);
        FooterCell::new(descs[i].name, descs[i].format(v), Some(v))
    });
    draw_footer(fb, &cells, &ui.knob_touch[4..], accent);
}
