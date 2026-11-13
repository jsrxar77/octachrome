//! The machine's settings, kept in a small text file of `key=value` lines.
//!
//! Where: `OP3_CONFIG` if set, else `$XDG_CONFIG_HOME/op3/settings`, else
//! `$HOME/.config/op3/settings`. A missing file, an unknown key or a value that
//! does not parse is ignored and the default is used: a damaged file must never
//! stop the machine from starting.
//!
//! At start-up `-t` / `OP3_THEME` win over the saved theme, which wins over
//! the default (see docs/architecture.md §10).

use op3_gfx::ThemeId;
use std::{
    io,
    path::{Path, PathBuf},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Settings {
    pub theme: ThemeId,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            theme: ThemeId::Op1,
        }
    }
}

/// The theme a name stands for; the names `-t` and `OP3_THEME` accept.
pub fn theme_from_name(name: &str) -> Option<ThemeId> {
    match name.trim() {
        "op-1" | "op1" => Some(ThemeId::Op1),
        "op-1-field" | "op-1 field" | "field" => Some(ThemeId::Op1Field),
        "op-xy" | "opxy" => Some(ThemeId::OpXy),
        _ => None,
    }
}

/// The name written to the file: the one `-t` documents.
pub fn theme_name(id: ThemeId) -> &'static str {
    match id {
        ThemeId::Op1 => "op-1",
        ThemeId::Op1Field => "op-1-field",
        ThemeId::OpXy => "op-xy",
    }
}

impl Settings {
    pub fn parse(text: &str) -> Self {
        let mut s = Self::default();
        for line in text.lines() {
            let Some((key, value)) = line.split_once('=') else {
                continue;
            };
            if key.trim() == "theme" {
                if let Some(id) = theme_from_name(value) {
                    s.theme = id;
                }
            }
        }
        s
    }

    pub fn render(&self) -> String {
        format!("theme={}\n", theme_name(self.theme))
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
    if let Some(p) = std::env::var_os("OP3_CONFIG").filter(|p| !p.is_empty()) {
        return Some(PathBuf::from(p));
    }
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .filter(|p| !p.is_empty())
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))?;
    Some(base.join("op3").join("settings"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("op3-settings-{}-{name}", std::process::id()))
    }

    // F-C06 (docs/features.md)
    #[test]
    fn round_trips_every_theme() {
        for id in ThemeId::ALL {
            let s = Settings { theme: id };
            assert_eq!(Settings::parse(&s.render()), s, "{id:?}");
        }
    }

    // F-C06
    #[test]
    fn a_damaged_file_falls_back_to_the_default() {
        let d = Settings::default();
        for bad in ["", "garbage", "theme", "theme=", "theme=neon", "= =\n\n==", "colour=op-xy"] {
            assert_eq!(Settings::parse(bad), d, "{bad:?}");
        }
        // Unknown keys and blank lines around a good one are ignored.
        assert_eq!(
            Settings::parse("future=1\n\ntheme = op-xy\nother=x\n").theme,
            ThemeId::OpXy
        );
    }

    // F-C06
    #[test]
    fn names_match_the_command_line() {
        for (name, id) in [
            ("op-1", ThemeId::Op1),
            ("op1", ThemeId::Op1),
            ("op-1-field", ThemeId::Op1Field),
            ("field", ThemeId::Op1Field),
            ("op-xy", ThemeId::OpXy),
            ("opxy", ThemeId::OpXy),
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
        let s = Settings {
            theme: ThemeId::OpXy,
        };
        s.save(&path).expect("save");
        assert_eq!(Settings::load(&path), s);
        assert!(!path.with_extension("tmp").exists(), "temp file left behind");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
