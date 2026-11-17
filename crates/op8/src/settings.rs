//! The machine's settings, kept in a small text file of `key=value` lines.
//!
//! Where: `OP8_CONFIG` if set, else `$XDG_CONFIG_HOME/op8/settings`, else
//! `$HOME/.config/op8/settings`. A missing file, an unknown key or a value that
//! does not parse is ignored and the default is used: a damaged file must never
//! stop the machine from starting.
//!
//! At start-up `-t` / `OP8_THEME` win over the saved theme, which wins over
//! the default (see docs/architecture.md §10).

use op8_gfx::ThemeId;
use std::{
    io,
    path::{Path, PathBuf},
};

/// What the player chose in SETTINGS. A device is `None` for "automatic": the MK3's own if it is there, else the system's.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Settings {
    pub theme: ThemeId,
    /// The audio output, by (a part of) its name.
    pub audio_out: Option<String>,
    /// The audio input SAMPLING listens to.
    pub audio_in: Option<String>,
    /// Frames per audio block: 64, 128 or 256 (`None`: 128, or `OP8_BUFFER`).
    pub buffer: Option<u32>,
    /// The MIDI output port.
    pub midi_out: Option<String>,
    /// The MIDI input port that plays the unit (`None`: no input is open).
    pub midi_in: Option<String>,
    /// What a note that comes in plays: `false`, the selected track; `true`, the track of its MIDI channel (1 to 8 are A to H).
    pub midi_by_channel: bool,
    /// MIDI clock out: start, 24 clocks to the quarter note and stop on the MIDI port while the sequencer runs.
    pub midi_clock: bool,
    /// The screen backlight as a step of what the supply allows: 1 to 4 (25, 50, 75 and 100 %).
    pub screen: u8,
    /// The highest level of the pad LEDs: 1 to 3.
    pub pads: u8,
}

impl Default for Settings {
    fn default() -> Self {
        Self { theme: ThemeId::Chrome, audio_out: None, audio_in: None, buffer: None, midi_out: None, midi_in: None, midi_by_channel: false, midi_clock: true, screen: SCREEN_STEPS, pads: 3 }
    }
}

/// The backlight steps, as shares of what the supply allows: the last is the whole of it, never more.
pub const SCREEN_STEPS: u8 = 4;
/// The audio block sizes the player can choose from.
pub const BUFFERS: [u32; 3] = [64, 128, 256];

/// The share of the allowed backlight for a step (1..=4): a quarter, a half, three quarters and all of it.
pub fn screen_share(step: u8) -> f32 {
    step.clamp(1, SCREEN_STEPS) as f32 / SCREEN_STEPS as f32
}

/// The light to ask of the surface: the theme's, with the player's steps put on it (a share of the backlight and a cap on the pad LEDs).
/// It is a share of what the supply allows, so it can only be less, never more.
pub fn effective_light(theme: op8_gfx::Light, screen: u8, pads: u8) -> op8_gfx::Light {
    op8_gfx::Light { backlight: theme.backlight * screen_share(screen), led_level: theme.led_level.min(pads.clamp(1, 3)), mono: theme.mono }
}

/// The theme a name stands for; the names `-t` and `OP8_THEME` accept.
pub fn theme_from_name(name: &str) -> Option<ThemeId> {
    // The old names (op-1, op-1-field, op-xy) still open files saved before the rename.
    match name.trim() {
        "glossy" | "op-1" | "op1" => Some(ThemeId::Glossy),
        "matte" | "op-1-field" | "op-1 field" | "field" => Some(ThemeId::Matte),
        "chrome" | "op-xy" | "opxy" => Some(ThemeId::Chrome),
        _ => None,
    }
}

/// The name written to the file: the one `-t` documents.
pub fn theme_name(id: ThemeId) -> &'static str {
    match id {
        ThemeId::Glossy => "glossy",
        ThemeId::Matte => "matte",
        ThemeId::Chrome => "chrome",
    }
}

impl Settings {
    pub fn parse(text: &str) -> Self {
        let mut s = Self::default();
        for line in text.lines() {
            let Some((key, value)) = line.split_once('=') else {
                continue;
            };
            let (key, value) = (key.trim(), value.trim());
            let name = || Some(value.to_string()).filter(|v| !v.is_empty() && v != "auto");
            match key {
                "theme" => {
                    if let Some(id) = theme_from_name(value) {
                        s.theme = id;
                    }
                }
                "audio_out" => s.audio_out = name(),
                "audio_in" => s.audio_in = name(),
                "midi_out" => s.midi_out = name(),
                "midi_in" => s.midi_in = name(),
                "midi_channels" => s.midi_by_channel = value == "on",
                "midi_clock" => {
                    s.midi_clock = match value {
                        "off" | "0" | "false" => false,
                        "on" | "1" | "true" => true,
                        _ => s.midi_clock,
                    }
                }
                "buffer" => s.buffer = value.parse().ok().filter(|b| BUFFERS.contains(b)),
                "screen" => s.screen = value.parse().ok().filter(|v| (1..=SCREEN_STEPS).contains(v)).unwrap_or(s.screen),
                "pads" => s.pads = value.parse().ok().filter(|v| (1..=3).contains(v)).unwrap_or(s.pads),
                _ => {}
            }
        }
        s
    }

    /// The file: the theme always, and the rest only where it is not the default (so a file that is only a theme stays one line).
    pub fn render(&self) -> String {
        let d = Self::default();
        let mut out = format!("theme={}\n", theme_name(self.theme));
        for (key, value) in [("audio_out", &self.audio_out), ("audio_in", &self.audio_in), ("midi_out", &self.midi_out), ("midi_in", &self.midi_in)] {
            if let Some(v) = value {
                out.push_str(&format!("{key}={v}\n"));
            }
        }
        if let Some(b) = self.buffer {
            out.push_str(&format!("buffer={b}\n"));
        }
        if self.midi_by_channel {
            out.push_str("midi_channels=on\n");
        }
        if self.midi_clock != d.midi_clock {
            out.push_str(&format!("midi_clock={}\n", if self.midi_clock { "on" } else { "off" }));
        }
        if self.screen != d.screen {
            out.push_str(&format!("screen={}\n", self.screen));
        }
        if self.pads != d.pads {
            out.push_str(&format!("pads={}\n", self.pads));
        }
        out
    }

    /// Settings from `path`; the defaults if it is missing or unreadable.
    pub fn load(path: &Path) -> Self {
        std::fs::read_to_string(path)
            .map(|t| Self::parse(&t))
            .unwrap_or_default()
    }

    /// Write the file, creating its folder. Written beside it and renamed, so
    /// a power cut leaves the old file or the new one, never half of one.
    pub fn save(&self, path: &Path) -> io::Result<()> {
        if let Some(dir) = path.parent().filter(|d| !d.as_os_str().is_empty()) {
            std::fs::create_dir_all(dir)?;
        }
        let tmp = path.with_extension("tmp");
        std::fs::write(&tmp, self.render())?;
        std::fs::rename(&tmp, path)
    }
}

/// The settings file for this machine, if a home or config folder is known.
pub fn default_path() -> Option<PathBuf> {
    if let Some(p) = std::env::var_os("OP8_CONFIG").filter(|p| !p.is_empty()) {
        return Some(PathBuf::from(p));
    }
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .filter(|p| !p.is_empty())
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))?;
    Some(base.join("op8").join("settings"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("op8-settings-{}-{name}", std::process::id()))
    }

    // F-C06 (docs/features.md)
    #[test]
    fn round_trips_every_theme() {
        for id in ThemeId::ALL {
            let s = Settings { theme: id, ..Settings::default() };
            assert_eq!(Settings::parse(&s.render()), s, "{id:?}");
        }
    }

    // F-C06
    #[test]
    fn a_damaged_file_falls_back_to_the_default() {
        let d = Settings::default();
        for bad in ["", "garbage", "theme", "theme=", "theme=neon", "= =\n\n==", "colour=chrome"] {
            assert_eq!(Settings::parse(bad), d, "{bad:?}");
        }
        // Unknown keys and blank lines around a good one are ignored.
        assert_eq!(
            Settings::parse("future=1\n\ntheme = chrome\nother=x\n").theme,
            ThemeId::Chrome
        );
    }

    // F-C06
    #[test]
    fn names_match_the_command_line() {
        for (name, id) in [
            ("glossy", ThemeId::Glossy),
            ("matte", ThemeId::Matte),
            ("chrome", ThemeId::Chrome),
            // saved before the rename
            ("op-1", ThemeId::Glossy),
            ("op-1-field", ThemeId::Matte),
            ("op-xy", ThemeId::Chrome),
        ] {
            assert_eq!(theme_from_name(name), Some(id), "{name}");
        }
        assert_eq!(theme_from_name("nope"), None);
        for id in ThemeId::ALL {
            assert_eq!(theme_from_name(theme_name(id)), Some(id));
        }
    }

    // F-C06
    #[test]
    fn saves_and_loads_through_a_file_and_creates_the_folder() {
        let dir = temp("save");
        let path = dir.join("nested").join("settings");
        let _ = std::fs::remove_dir_all(&dir);
        assert_eq!(Settings::load(&path), Settings::default(), "missing file");
        let s = Settings { theme: ThemeId::Matte, ..Settings::default() };
        s.save(&path).expect("save");
        assert_eq!(Settings::load(&path), s);
        assert!(!path.with_extension("tmp").exists(), "temp file left behind");
        let _ = std::fs::remove_dir_all(&dir);
    }

    // F-D17
    #[test]
    fn matte_is_the_default_and_comes_first() {
        assert_eq!(Settings::default().theme, ThemeId::Chrome);
        assert_eq!(ThemeId::ALL[0], ThemeId::Chrome);
        assert_eq!(ThemeId::Chrome.next(), ThemeId::Matte, "the order is chrome, matte, glossy");
        assert_eq!(ThemeId::Glossy.next(), ThemeId::Chrome);
    }

    // F-C13: the devices, the buffer, the backlight step and the pad level are kept in the file; a file that is only a theme stays one line,
    // "auto" or nothing is automatic, and a value that makes no sense is the default.
    #[test]
    fn the_devices_and_the_light_are_kept_in_the_file() {
        let s = Settings {
            theme: ThemeId::Glossy,
            audio_out: Some("Maschine MK3".into()),
            audio_in: Some("Scarlett 2i2".into()),
            buffer: Some(64),
            midi_out: Some("Maschine".into()),
            midi_in: Some("Keystation".into()),
            midi_by_channel: true,
            midi_clock: false,
            screen: 2,
            pads: 1,
        };
        let text = s.render();
        assert_eq!(Settings::parse(&text), s, "{text}");
        assert_eq!(Settings { theme: ThemeId::Matte, ..Settings::default() }.render(), "theme=matte\n", "only what is not the default is written");
        let d = Settings::default();
        for bad in ["audio_out=\n", "audio_out=auto\n", "buffer=100\n", "buffer=x\n", "midi_clock=maybe\n", "screen=0\n", "screen=9\n", "pads=4\n", "pads=\n"] {
            assert_eq!(Settings::parse(bad), d, "{bad:?}");
        }
        assert_eq!(Settings::parse("screen=3\nbuffer=256\n").screen, 3);
        // the light is the theme's, with the steps put on it, and never more than the theme asked
        let theme = op8_gfx::Light { backlight: 0.8, led_level: 3, mono: 0.9 };
        let l = effective_light(theme, 2, 2);
        assert!((l.backlight - 0.4).abs() < 1e-6 && l.led_level == 2 && l.mono == 0.9);
        assert_eq!(effective_light(theme, SCREEN_STEPS, 3), theme, "the last step is all of it");
        assert_eq!(effective_light(op8_gfx::Light { backlight: 1.0, led_level: 2, mono: 1.0 }, 4, 3).led_level, 2, "a cap is never raised");
    }
}
