//! Navigation with the 4D encoder (docs/design.md §4, "Navigation"). Every view that has things to choose lists them as
//! [`NavItem`]s; one of them has the **focus**, always, and is drawn with a ring (a frame, pulsing
//! gently). The 4D turns the focus along the list, its arrows move it across the screens, pushing it activates the item, and an
//! item that holds a value can be grabbed (push), turned, and let go (push). The list and the drawing share this one place, so
//! what can be reached and what is drawn cannot drift apart.

use crate::theme::{FOOTER_COL, FOOTER_Y};
use crate::{Main, Mode, UiState};
use op8_gfx::{
    knob,
    paint::{ring, stroke_round_rect},
    theme, FrameBuffer480x272,
};

/// The cells of the sixteen-place views (FILE, PLUG-IN, BROWSER, SAMPLING): where they are drawn on the left screen.
pub const CELL_W: f32 = 108.0;
pub const CELL_H: f32 = 42.0;
pub const GAP: f32 = 6.0;

/// Items 0..16 of the sixteen-place views are the cells, in reading order (the order of the pads).
pub const CELLS: usize = 16;

/// What an item is, which is also how the focus remembers it when the view changes: a place of a sixteen-place view or a pad of the
/// grid (`Cell`, reading order), a screen button (`Strip` 0..8), a track (`Track` 0..8) or a value under a screen (`Value` 0..8: the
/// knob that edits it).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NavKind {
    Cell(u8),
    Strip(u8),
    Track(u8),
    Value(u8),
}

/// Something that can have the focus: what it is, on which screen (0 left, 1 right) and its box in that screen's pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NavItem {
    pub kind: NavKind,
    pub screen: u8,
    pub rect: (f32, f32, f32, f32),
}

/// The box of cell `i` of the sixteen-place views.
pub fn cell_rect(i: usize) -> (f32, f32, f32, f32) {
    let (col, row) = ((i % 4) as f32, (i / 4) as f32);
    (12.0 + col * (CELL_W + GAP), 44.0 + row * (CELL_H + GAP), CELL_W, CELL_H)
}

/// The four values of SAMPLING (source, start, end, fade) under the right screen, where their knobs are.
pub fn value_rect(k: usize) -> (f32, f32, f32, f32) {
    (60.0 + k as f32 * 120.0 - 52.0, 224.0, 104.0, 42.0)
}

impl Default for NavKind {
    fn default() -> Self {
        NavKind::Track(0)
    }
}

/// A screen button's box: the capsule that is drawn for its label (`strip_capsule`), centred on its physical button.
pub fn strip_rect(n: usize, label: &str) -> (f32, f32, f32, f32) {
    let (x, w, h) = crate::theme::strip_capsule(n % 4, label);
    (x, 1.0, w, h)
}

/// A track's dot in the header of the left screen.
pub fn track_rect(t: usize) -> (f32, f32, f32, f32) {
    (338.0 + t as f32 * 18.0 - 9.0, 29.0, 18.0, 18.0)
}

/// A pad of the grid that mirrors the pads on the left screen.
pub fn grid_rect(c: usize) -> (f32, f32, f32, f32) {
    (262.0 + (c % 4) as f32 * 42.0, 50.0 + (c / 4) as f32 * 42.0, 37.0, 37.0)
}

/// An effect of PERFORM: the sixteen are drawn full width, one per pad, in reading order (cell `c` is the pad that is `c`-th in reading
/// order, like the grid), so it is not where the grid of the other modes is.
pub fn punch_rect(c: usize) -> (f32, f32, f32, f32) {
    (12.0 + (c % 4) as f32 * 116.0, 52.0 + (c / 4) as f32 * 42.0, 108.0, 36.0)
}

/// The effect (the pad number, which is also its slot) that cell `c`, counted in reading order, is in PERFORM: pad 0 is bottom-left.
pub fn punch_slot(c: usize) -> usize {
    (3 - c / 4) * 4 + c % 4
}

/// One of the four values of a footer, under its screen.
pub fn footer_rect(col: usize) -> (f32, f32, f32, f32) {
    (col as f32 * FOOTER_COL as f32 + 6.0, FOOTER_Y as f32 + 3.0, FOOTER_COL as f32 - 12.0, 42.0)
}

/// What can be reached in the view that is open, in the order the 4D turns through it. Composing lists the screen buttons that have a
/// name, the tracks, the pads of the grid (when the left screen shows it) and the four values of each footer; the step inspector
/// has its own cursor on the 4D and lists nothing.
pub fn items(ui: &UiState) -> Vec<NavItem> {
    let cell = |i: usize| NavItem { kind: NavKind::Cell(i as u8), screen: 0, rect: cell_rect(i) };
    match ui.main {
        Main::File | Main::Plugin | Main::Browser => (0..CELLS).map(cell).collect(),
        Main::Sampling => (0..CELLS)
            .map(cell)
            .chain((0..4).map(|k| NavItem { kind: NavKind::Value(4 + k as u8), screen: 1, rect: value_rect(k) }))
            .collect(),
        Main::Instrument if ui.inspect_step.is_none() => {
            let buttons = crate::screen_buttons(ui);
            let strip = |range: std::ops::Range<usize>, screen: u8| {
                range
                    .filter_map(|n| buttons[n].map(|(name, _)| NavItem { kind: NavKind::Strip(n as u8), screen, rect: strip_rect(n, name) }))
                    .collect::<Vec<_>>()
            };
            let mut v = strip(0..4, 0);
            v.extend((0..8).map(|t| NavItem { kind: NavKind::Track(t as u8), screen: 0, rect: track_rect(t) }));
            // the pads are on the left screen: the grid of the steps (unless a page of buttons 3 and 4 covers it), the grid of the
            // notes, or the sixteen effects of PERFORM, which are drawn bigger; Brain draws no pads, so it lists none
            match ui.mode {
                Mode::Step if ui.left_page == crate::LeftPage::Steps => {
                    v.extend((0..CELLS).map(|c| NavItem { kind: NavKind::Cell(c as u8), screen: 0, rect: grid_rect(c) }))
                }
                Mode::Pads | Mode::Keys => v.extend((0..CELLS).map(|c| NavItem { kind: NavKind::Cell(c as u8), screen: 0, rect: grid_rect(c) })),
                Mode::Punch => v.extend((0..CELLS).map(|c| NavItem { kind: NavKind::Cell(c as u8), screen: 0, rect: punch_rect(c) })),
                _ => {}
            }
            v.extend((0..4).map(|k| NavItem { kind: NavKind::Value(k as u8), screen: 0, rect: footer_rect(k) }));
            v.extend(strip(4..8, 1));
            v.extend((0..4).map(|k| NavItem { kind: NavKind::Value(4 + k as u8), screen: 1, rect: footer_rect(k) }));
            v
        }
        _ => Vec::new(),
    }
}

/// The position of the focus in the list of the view that is open: where it is remembered, or the first item.
pub fn index(ui: &UiState) -> usize {
    items(ui).iter().position(|it| it.kind == ui.focus).unwrap_or(0)
}

/// The item that has the focus, if the view has any.
pub fn focused(ui: &UiState) -> Option<NavItem> {
    let it = items(ui);
    it.get(it.iter().position(|i| i.kind == ui.focus).unwrap_or(0)).copied()
}

/// Turn the focus along the list, wrapping.
pub fn step(n: usize, cur: usize, delta: i32) -> usize {
    if n == 0 {
        return 0;
    }
    (cur as i32 + delta).rem_euclid(n as i32) as usize
}

/// Move the focus across the screens: to the nearest item in that direction (`dx`, `dy` are -1, 0 or 1). The right screen sits
/// to the right of the left one. Stays where it is if there is nothing that way.
pub fn move_dir(items: &[NavItem], cur: usize, dx: i32, dy: i32) -> usize {
    let at = |it: &NavItem| (it.rect.0 + it.rect.2 / 2.0 + it.screen as f32 * 520.0, it.rect.1 + it.rect.3 / 2.0);
    let Some(from) = items.get(cur).map(at) else { return cur };
    let mut best: Option<(f32, usize)> = None;
    for (i, it) in items.iter().enumerate().filter(|(i, _)| *i != cur) {
        let (x, y) = at(it);
        let (along, across) = if dx != 0 { ((x - from.0) * dx as f32, (y - from.1).abs()) } else { ((y - from.1) * dy as f32, (x - from.0).abs()) };
        if along <= 1.0 {
            continue;
        }
        let cost = along + 2.5 * across;
        if best.is_none_or(|(c, _)| cost < c) {
            best = Some((cost, i));
        }
    }
    best.map_or(cur, |(_, i)| i)
}

/// The ring of the focus: a frame round the item in the colour of knob 1; grabbed, it is white and thicker. It does not move by itself:
/// a still screen sends nothing to the MK3.
pub fn draw_ring(fb: &mut FrameBuffer480x272, it: &NavItem, ui: &UiState) {
    let (col, wd) = ring_style(it, ui);
    // a track is a small round capsule: the ring hugs its edge (the chosen one is bigger)
    if let NavKind::Track(t) = it.kind {
        let (x, y, w, h) = it.rect;
        let r = if t as usize == ui.track { 10.0 } else { 8.0 };
        return ring(fb, x + w / 2.0, y + h / 2.0, r, wd, col);
    }
    let (x, y, w, h, r) = ring_box(it);
    stroke_round_rect(fb, x, y, w, h, r, wd, col);
}

/// Colour and width of the ring: a grabbed value, or the effect of PERFORM that is sounding on this very place, is `fg` and thick.
fn ring_style(it: &NavItem, ui: &UiState) -> (op8_gfx::Rgb565, f32) {
    let sounding = matches!((ui.mode, it.kind, ui.punch), (Mode::Punch, NavKind::Cell(c), Some((s, _))) if s == punch_slot(c as usize));
    if ui.grabbed || sounding {
        (theme().fg, 3.5)
    } else {
        (knob(0), 2.5)
    }
}

/// The box the ring is stroked in (the stroke stays inside it), and its corner radius. A screen button is a capsule at the very top of
/// the screen, so its ring is drawn on the capsule's own edge and not outside it; any other box gets the ring 3 px outside, pushed in
/// where that would leave the screen. Nothing of the ring is ever off the screen.
pub fn ring_box(it: &NavItem) -> (f32, f32, f32, f32, f32) {
    let (x, y, w, h) = it.rect;
    if matches!(it.kind, NavKind::Strip(_)) {
        return (x, y, w, h, h / 2.0);
    }
    let (bx, by, bw, bh) = (x - 3.0, y - 3.0, w + 6.0, h + 6.0);
    let (sw, sh) = (op8_gfx::WIDTH as f32, op8_gfx::HEIGHT as f32);
    let (nx, ny) = (bx.clamp(0.0, sw - bw), by.clamp(0.0, sh - bh));
    (nx, ny, bw, bh, 11.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_focus_wraps_and_moves_across_the_grid() {
        assert_eq!(step(16, 15, 1), 0);
        assert_eq!(step(16, 0, -1), 15);
        let ui = UiState { main: Main::File, ..Default::default() };
        let it = items(&ui);
        assert_eq!(it.len(), 16);
        assert_eq!(move_dir(&it, 5, 1, 0), 6);
        assert_eq!(move_dir(&it, 5, -1, 0), 4);
        assert_eq!(move_dir(&it, 5, 0, 1), 9);
        assert_eq!(move_dir(&it, 5, 0, -1), 1);
        assert_eq!(move_dir(&it, 3, 1, 0), 3, "nothing to the right of the last column");
    }

    #[test]
    fn the_right_screen_is_to_the_right_of_the_left_one() {
        let ui = UiState { main: Main::Sampling, ..Default::default() };
        let it = items(&ui);
        assert_eq!(it.len(), 20);
        let to_the_right = move_dir(&it, 15, 1, 0);
        assert!(to_the_right >= 16, "from the last cell, right goes to the right screen: {to_the_right}");
        assert!(move_dir(&it, 16, -1, 0) < 16, "and back");
    }

    #[test]
    fn composing_lists_the_buttons_that_have_a_name_the_tracks_the_pads_and_the_values() {
        let ui = UiState { main: Main::Instrument, has_second: true, ..Default::default() };
        let it = items(&ui);
        let count = |f: fn(&NavKind) -> bool| it.iter().filter(|i| f(&i.kind)).count();
        assert_eq!(count(|k| matches!(k, NavKind::Strip(_))), 8, "all eight screen buttons");
        assert_eq!(count(|k| matches!(k, NavKind::Track(_))), 8);
        assert_eq!(count(|k| matches!(k, NavKind::Cell(_))), 16);
        assert_eq!(count(|k| matches!(k, NavKind::Value(_))), 8);
        let one_page = UiState { main: Main::Instrument, has_second: false, ..Default::default() };
        assert_eq!(items(&one_page).iter().filter(|i| matches!(i.kind, NavKind::Strip(_))).count(), 7, "no shape button without a second page");
        let open = UiState { main: Main::Instrument, inspect_step: Some(0), ..Default::default() };
        assert!(items(&open).is_empty(), "the step inspector has its own cursor");
        let players = UiState { main: Main::Instrument, left_page: crate::LeftPage::Players, ..Default::default() };
        assert_eq!(items(&players).iter().filter(|i| matches!(i.kind, NavKind::Cell(_))).count(), 0, "a page that covers the grid lists no pads");
    }
}
