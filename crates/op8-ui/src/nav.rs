//! Navigation with the 4D encoder (docs/design.md §4, "Navigation"). Every view that has things to choose lists them as
//! [`NavItem`]s; one of them has the **focus**, always, and is drawn with a ring (a frame and a dot at its corner, pulsing
//! gently). The 4D turns the focus along the list, its arrows move it across the screens, pushing it activates the item, and an
//! item that holds a value can be grabbed (push), turned, and let go (push). The list and the drawing share this one place, so
//! what can be reached and what is drawn cannot drift apart.

use crate::{Main, UiState};
use op8_gfx::{
    knob,
    paint::{fill_circle, stroke_round_rect},
    theme, FrameBuffer480x272,
};

/// The cells of the sixteen-place views (FILE, PLUG-IN, BROWSER, SAMPLING): where they are drawn on the left screen.
pub const CELL_W: f32 = 108.0;
pub const CELL_H: f32 = 42.0;
pub const GAP: f32 = 6.0;

/// Items 0..16 of the sixteen-place views are the cells, in reading order (the order of the pads).
pub const CELLS: usize = 16;

/// Something that can have the focus: on which screen (0 left, 1 right) and its box in that screen's pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NavItem {
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

/// What can be reached in the view that is open, in the order the 4D turns through it.
pub fn items(ui: &UiState) -> Vec<NavItem> {
    let cells = (0..CELLS).map(|i| NavItem { screen: 0, rect: cell_rect(i) });
    match ui.main {
        Main::File | Main::Plugin | Main::Browser => cells.collect(),
        Main::Sampling => cells.chain((0..4).map(|k| NavItem { screen: 1, rect: value_rect(k) })).collect(),
        _ => Vec::new(),
    }
}

/// The item that has the focus, if the view has any.
pub fn focused(ui: &UiState) -> Option<NavItem> {
    items(ui).get(ui.focus).copied()
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

/// The ring of the focus: a frame round the item and a dot at its corner, pulsing between two colours; grabbed, it is thicker.
pub fn draw_ring(fb: &mut FrameBuffer480x272, it: &NavItem, grabbed: bool, phase: f32) {
    let t = theme();
    let pulse = 0.5 + 0.5 * (phase * std::f32::consts::TAU).sin();
    let col = knob(0).lerp(t.fg, pulse);
    let (x, y, w, h) = it.rect;
    let wd = if grabbed { 3.5 } else { 2.0 };
    stroke_round_rect(fb, x - 3.0, y - 3.0, w + 6.0, h + 6.0, 11.0, wd, col);
    fill_circle(fb, x - 1.0, y - 1.0, if grabbed { 5.5 } else { 4.0 }, col);
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
}
