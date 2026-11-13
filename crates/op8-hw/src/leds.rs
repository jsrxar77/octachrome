//! LED state as the MK3 lays it out: 103 slots (62 buttons, 25 strip, 16 pads).

use crate::Button;

pub const LED_SLOTS: usize = 103;
pub const STRIP_BASE: usize = 62;
pub const STRIP_LEDS: usize = 25;
pub const PAD_BASE: usize = 87;

/// Entries of the MK3's default colour palette (report `0xfe`).
///
/// Entry 1 = red and 2 = orange are confirmed in autotel's protocol notes; the
/// rest follow NI's usual 16-hue order and still need checking on a unit
/// (`mk3-learn palette`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum NiColor {
    Off = 0,
    Red = 1,
    Orange = 2,
    LightOrange = 3,
    WarmYellow = 4,
    Yellow = 5,
    Lime = 6,
    Green = 7,
    Mint = 8,
    Cyan = 9,
    Turquoise = 10,
    Blue = 11,
    Plum = 12,
    Violet = 13,
    Purple = 14,
    Magenta = 15,
    Fuchsia = 16,
    White = 17,
}

impl NiColor {
    pub const HUES: [NiColor; 16] = [
        NiColor::Red,
        NiColor::Orange,
        NiColor::LightOrange,
        NiColor::WarmYellow,
        NiColor::Yellow,
        NiColor::Lime,
        NiColor::Green,
        NiColor::Mint,
        NiColor::Cyan,
        NiColor::Turquoise,
        NiColor::Blue,
        NiColor::Plum,
        NiColor::Violet,
        NiColor::Purple,
        NiColor::Magenta,
        NiColor::Fuchsia,
    ];

    /// The byte a colour LED expects: `(palette << 2) | level`, level 0..=3.
    pub const fn byte(self, level: u8) -> u8 {
        if level == 0 || matches!(self, NiColor::Off) {
            0
        } else {
            ((self as u8) << 2) | (level & 3)
        }
    }
}

/// LED slot for each button (`None` for the ten without an LED).
pub fn button_slot(b: Button) -> Option<usize> {
    use Button::*;
    Some(match b {
        ChannelMidi => 0,
        PlugIn => 1,
        Arranger => 2,
        Mixer => 3,
        Browser => 4,
        Sampling => 5,
        ArrowLeft => 6,
        ArrowRight => 7,
        File => 8,
        Settings => 9,
        Auto => 10,
        Macro => 11,
        Screen(n @ 1..=8) => 11 + n as usize,
        Volume => 20,
        Swing => 21,
        NoteRepeat => 22,
        Tempo => 23,
        Lock => 24,
        Pitch => 25,
        Mod => 26,
        Perform => 27,
        Notes => 28,
        Group(g @ 0..=7) => 29 + g as usize,
        Restart => 37,
        Erase => 38,
        Tap => 39,
        Follow => 40,
        Play => 41,
        Rec => 42,
        Stop => 43,
        Shift => 44,
        FixedVel => 45,
        PadMode => 46,
        Keyboard => 47,
        Chords => 48,
        Step => 49,
        Scene => 50,
        Pattern => 51,
        Events => 52,
        Variation => 53,
        Duplicate => 54,
        Select => 55,
        Solo => 56,
        Mute => 57,
        JogUp => 58,
        JogLeft => 59,
        JogRight => 60,
        JogDown => 61,
        _ => return None,
    })
}

/// Whether a button's LED is a colour LED (palette encoding) rather than a
/// plain 0..127 brightness.
pub fn button_is_colour(b: Button) -> bool {
    matches!(
        b,
        Button::Sampling
            | Button::Group(_)
            | Button::Play
            | Button::Rec
            | Button::JogUp
            | Button::JogLeft
            | Button::JogRight
            | Button::JogDown
    )
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LedFrame {
    pub slots: [u8; LED_SLOTS],
}

impl Default for LedFrame {
    fn default() -> Self {
        Self {
            slots: [0; LED_SLOTS],
        }
    }
}

impl LedFrame {
    pub fn clear(&mut self) {
        self.slots = [0; LED_SLOTS];
    }

    /// Monochrome button: brightness 0..=1. Colour button: white at that level.
    pub fn set_button(&mut self, b: Button, brightness: f32) {
        if let Some(s) = button_slot(b) {
            self.slots[s] = if button_is_colour(b) {
                NiColor::White.byte((brightness.clamp(0.0, 1.0) * 3.0).round() as u8)
            } else {
                (brightness.clamp(0.0, 1.0) * 127.0) as u8
            };
        }
    }

    /// Colour button (groups, play, rec…).
    pub fn set_button_color(&mut self, b: Button, color: NiColor, level: u8) {
        if let Some(s) = button_slot(b) {
            self.slots[s] = color.byte(level);
        }
    }

    /// Pad 0..16 from the bottom-left. `level` 0..=3 (0 = off).
    pub fn set_pad(&mut self, pad: u8, color: NiColor, level: u8) {
        if let Some(h) = pad_to_hid(pad) {
            self.slots[PAD_BASE + h as usize] = color.byte(level);
        }
    }

    pub fn pad(&self, pad: u8) -> u8 {
        pad_to_hid(pad).map_or(0, |h| self.slots[PAD_BASE + h as usize])
    }

    /// Raw strip LED value (the strip's colour encoding is still unknown).
    pub fn set_strip(&mut self, i: usize, value: u8) {
        if i < STRIP_LEDS {
            self.slots[STRIP_BASE + i] = value;
        }
    }
}

/// Bottom-left numbering (silkscreen − 1) → HID numbering (top-left = 0).
pub const fn pad_to_hid(pad: u8) -> Option<u8> {
    if pad >= 16 {
        return None;
    }
    Some((3 - pad / 4) * 4 + pad % 4)
}

/// HID numbering → bottom-left numbering. The mapping is its own inverse.
pub const fn hid_to_pad(hid: u8) -> Option<u8> {
    pad_to_hid(hid)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pad_numbering_matches_silkscreen() {
        // HID 0 is the top-left pad, silkscreened 13 → our pad 12.
        assert_eq!(hid_to_pad(0), Some(12));
        assert_eq!(hid_to_pad(15), Some(3));
        assert_eq!(pad_to_hid(0), Some(12)); // bottom-left
        for p in 0..16 {
            assert_eq!(hid_to_pad(pad_to_hid(p).unwrap()), Some(p));
        }
    }

    #[test]
    fn colour_byte_encoding() {
        assert_eq!(NiColor::Red.byte(3), 0b0000_0111);
        assert_eq!(NiColor::Orange.byte(0), 0);
        assert_eq!(NiColor::White.byte(3), (17 << 2) | 3);
    }

    #[test]
    fn every_ledded_button_has_a_unique_slot() {
        use Button::*;
        let mut all = vec![
            ChannelMidi,
            PlugIn,
            Arranger,
            Mixer,
            Browser,
            Sampling,
            ArrowLeft,
            ArrowRight,
            File,
            Settings,
            Auto,
            Macro,
            Volume,
            Swing,
            NoteRepeat,
            Tempo,
            Lock,
            Pitch,
            Mod,
            Perform,
            Notes,
            Restart,
            Erase,
            Tap,
            Follow,
            Play,
            Rec,
            Stop,
            Shift,
            FixedVel,
            PadMode,
            Keyboard,
            Chords,
            Step,
            Scene,
            Pattern,
            Events,
            Variation,
            Duplicate,
            Select,
            Solo,
            Mute,
            JogUp,
            JogLeft,
            JogRight,
            JogDown,
        ];
        all.extend((1..=8).map(Screen));
        all.extend((0..8).map(Group));
        let mut slots: Vec<usize> = all.iter().map(|&b| button_slot(b).unwrap()).collect();
        slots.sort();
        assert_eq!(slots, (0..62).collect::<Vec<_>>());
    }
}
