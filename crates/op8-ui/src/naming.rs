//! NAME: where the player names a project or a preset on the unit (design.md §4, "Naming"). Both screens.
//!
//! **Left:** the name, twelve places in a row, the one with the cursor outlined, and under them the four pads that are not places of the
//! name (`clear`, `delete`, `insert`, `save`). **Right:** the letters round the one under the cursor, as a strip that turns with the 4D
//! encoder, with the name as it will be saved under it.

use crate::theme::{button_strip, HEADER_CY};
use op8_engine::EngineStatus;
use op8_gfx::{
    knob,
    paint::{fill_circle, fill_convex, fill_round_rect, stroke_round_rect},
    text_v, theme, Align, FontSize, FrameBuffer480x272, VAlign,
};
use std::sync::atomic::Ordering::Relaxed;

/// The characters a name is made of, in the order the 4D encoder turns through them: a space, the small letters, the capitals, the
/// digits and a few marks.
pub const NAME_CHARS: &str = " abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789-.+#";
/// How long a name can be.
pub const NAME_LEN: usize = 12;
/// The pads that are not places of the name: in reading order, the cells 12 to 15.
pub const ACTIONS: [&str; 4] = ["clear", "delete", "insert", "save"];

/// What the naming screens show and the controller keeps.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NameView {
    /// What is being named, for the title: `project user 3`, `preset user 2`.
    pub what: String,
    /// The name, `NAME_LEN` characters (spaces fill what is not used).
    pub chars: [char; NAME_LEN],
    pub cursor: usize,
}

impl Default for NameView {
    fn default() -> Self {
        Self { what: String::new(), chars: [' '; NAME_LEN], cursor: 0 }
    }
}

impl NameView {
    /// A name to start from: its first `NAME_LEN` characters, the cursor on the first.
    pub fn new(what: impl Into<String>, name: &str) -> Self {
        let mut chars = [' '; NAME_LEN];
        for (i, c) in name.chars().filter(|c| NAME_CHARS.contains(*c)).take(NAME_LEN).enumerate() {
            chars[i] = c;
        }
        Self { what: what.into(), chars, cursor: 0 }
    }

    /// The name as it is saved: without the spaces at either end.
    pub fn text(&self) -> String {
        self.chars.iter().collect::<String>().trim().to_string()
    }

    /// Where the character under the cursor is in `NAME_CHARS`.
    fn index(&self) -> usize {
        NAME_CHARS.chars().position(|c| c == self.chars[self.cursor]).unwrap_or(0)
    }

    /// Turn the character under the cursor `n` along the list, round.
    pub fn turn(&mut self, n: i32) {
        let all: Vec<char> = NAME_CHARS.chars().collect();
        let i = (self.index() as i32 + n).rem_euclid(all.len() as i32) as usize;
        self.chars[self.cursor] = all[i];
    }

    pub fn move_cursor(&mut self, d: i32) {
        self.cursor = (self.cursor as i32 + d).clamp(0, NAME_LEN as i32 - 1) as usize;
    }

    pub fn clear(&mut self) {
        self.chars = [' '; NAME_LEN];
        self.cursor = 0;
    }

    /// Take the character under the cursor out, and what follows comes one place to the left.
    pub fn delete(&mut self) {
        for i in self.cursor..NAME_LEN - 1 {
            self.chars[i] = self.chars[i + 1];
        }
        self.chars[NAME_LEN - 1] = ' ';
    }

    /// Open a space under the cursor: what follows goes one place to the right (the last character falls off).
    pub fn insert(&mut self) {
        for i in (self.cursor + 1..NAME_LEN).rev() {
            self.chars[i] = self.chars[i - 1];
        }
        self.chars[self.cursor] = ' ';
    }
}

/// The box of place `i` of the name on the left screen.
pub fn place_rect(i: usize) -> (f32, f32, f32, f32) {
    (12.0 + i as f32 * 38.0, 66.0, 34.0, 52.0)
}

pub fn draw_left(fb: &mut FrameBuffer480x272, ui: &crate::UiState, st: &EngineStatus) {
    let t = theme();
    let n = &ui.naming;
    fb.clear(t.bg);
    button_strip(fb, &crate::strip_labels(ui, 0));
    fill_circle(fb, 16.0, HEADER_CY, 6.0, t.fg);
    text_v(fb, "name", 30, HEADER_CY as i32, FontSize::Large, t.fg, Align::Left, VAlign::Middle);
    text_v(fb, &n.what, 150, HEADER_CY as i32, FontSize::Medium, t.dim, Align::Left, VAlign::Middle);
    if st.playing.load(Relaxed) {
        fill_convex(fb, &[(460.0, HEADER_CY - 7.0), (470.0, HEADER_CY - 1.0), (460.0, HEADER_CY + 5.0)], t.fg);
    } else {
        fill_round_rect(fb, 460.0, HEADER_CY - 6.0, 10.0, 10.0, 1.5, t.dim);
    }
    for i in 0..NAME_LEN {
        let (x, y, w, h) = place_rect(i);
        let here = i == n.cursor;
        fill_round_rect(fb, x, y, w, h, 6.0, if here { t.panel } else { t.bg });
        stroke_round_rect(fb, x, y, w, h, 6.0, if here { 2.0 } else { 1.0 }, if here { t.fg } else { t.faint });
        let c = n.chars[i];
        if c != ' ' {
            text_v(fb, &c.to_string(), (x + w / 2.0) as i32, (y + h / 2.0) as i32, FontSize::Value, if here { t.fg } else { t.dim }, Align::Center, VAlign::Middle);
        }
    }
    // the four pads under the name
    for (k, label) in ACTIONS.iter().enumerate() {
        let (x, y, w, h) = crate::nav::cell_rect(12 + k);
        stroke_round_rect(fb, x, y + 4.0, w, h, 8.0, 1.0, t.faint);
        text_v(fb, label, (x + w / 2.0) as i32, (y + 4.0 + h / 2.0) as i32, FontSize::Medium, if k == 3 { t.fg } else { t.dim }, Align::Center, VAlign::Middle);
    }
}

pub fn draw_right(fb: &mut FrameBuffer480x272, ui: &crate::UiState) {
    let t = theme();
    let n = &ui.naming;
    fb.clear(t.bg);
    fill_circle(fb, 16.0, HEADER_CY, 6.0, t.fg);
    text_v(fb, "letter", 30, HEADER_CY as i32, FontSize::Large, t.fg, Align::Left, VAlign::Middle);
    text_v(fb, &format!("place {} of {}", n.cursor + 1, NAME_LEN), 464, HEADER_CY as i32, FontSize::Medium, t.dim, Align::Right, VAlign::Middle);
    // the strip: four characters either side of the one under the cursor, which is large; the ones further away are fainter
    let all: Vec<char> = NAME_CHARS.chars().collect();
    let at = n.index() as i32;
    for d in -4i32..=4 {
        let c = all[(at + d).rem_euclid(all.len() as i32) as usize];
        let x = 240 + d * 46;
        let shown = if c == ' ' { '·' } else { c };
        if d == 0 {
            fill_round_rect(fb, x as f32 - 28.0, 74.0, 56.0, 76.0, 10.0, t.panel);
            stroke_round_rect(fb, x as f32 - 28.0, 74.0, 56.0, 76.0, 10.0, 2.0, knob(0));
            text_v(fb, &shown.to_string(), x, 112, FontSize::Huge, t.fg, Align::Center, VAlign::Middle);
        } else {
            text_v(fb, &shown.to_string(), x, 112, FontSize::Large, if d.abs() <= 2 { t.dim } else { t.faint }, Align::Center, VAlign::Middle);
        }
    }
    // the name as it will be saved
    let text = n.text();
    text_v(fb, if text.is_empty() { "(no name: user N)" } else { &text }, 240, 184, FontSize::Value, if text.is_empty() { t.dim } else { t.fg }, Align::Center, VAlign::Middle);
    text_v(fb, "turn: letter    shift + turn: ten", 240, 226, FontSize::Small, t.dim, Align::Center, VAlign::Middle);
    text_v(fb, "jog left or right: place    push: save    FILE or BROWSER: cancel", 240, 248, FontSize::Small, t.dim, Align::Center, VAlign::Middle);
}

#[cfg(test)]
mod tests {
    use super::*;

    // F-N16
    #[test]
    fn turning_walks_the_letters_and_wraps() {
        let mut n = NameView::default();
        n.turn(1);
        assert_eq!(n.chars[0], 'a');
        n.turn(-2);
        assert_eq!(n.chars[0], '#', "before the space comes the last mark");
        n.turn(1);
        assert_eq!(n.chars[0], ' ');
        n.turn(27);
        assert_eq!(n.chars[0], 'A', "after the 26 small letters come the capitals");
    }

    // F-N16
    #[test]
    fn the_cursor_stays_inside_and_delete_and_insert_move_the_rest() {
        let mut n = NameView::new("x", "my jam");
        assert_eq!(n.text(), "my jam");
        n.move_cursor(-3);
        assert_eq!(n.cursor, 0);
        n.move_cursor(40);
        assert_eq!(n.cursor, NAME_LEN - 1);
        n.cursor = 2; // the space
        n.delete();
        assert_eq!(n.text(), "myjam");
        n.insert();
        n.chars[2] = '-';
        assert_eq!(n.text(), "my-jam");
        n.clear();
        assert_eq!((n.text().as_str(), n.cursor), ("", 0));
        // what the unit cannot show is left out, and a long name is cut
        assert_eq!(NameView::new("x", "añb").text(), "ab");
        assert_eq!(NameView::new("x", "abcdefghijklmnopq").text(), "abcdefghijkl");
    }
}
