//! A project on disk (the FILE button): the project itself (`octachrome_seq::io`) and, for each track,
//! the name and saved state of its instrument. Written by hand like the project bytes, with a
//! magic and a version, and read defensively: a damaged file is refused, never half-loaded.

use octachrome_engine::{EngineCommand, EngineHandle, SharedCartridge};
use octachrome_seq::{io, Project, TRACKS};
use std::path::{Path, PathBuf};

const MAGIC: &[u8; 4] = b"OCTP";
/// The magic of the files the project wrote under its first name: they are still read.
const LEGACY_MAGIC: &[u8; 4] = b"OP8P";
const VERSION: u8 = 1;

/// The instrument that hands MIDI to the MIDI thread: it is built once at start-up with its queue,
/// so loading a project keeps the one that is running instead of making another.
const MIDI_BRIDGE: &str = "MidiOutBridge";

/// A project with the instruments of its tracks: name and saved state, `None` = empty slot.
#[derive(Clone, Debug, PartialEq)]
pub struct ProjectFile {
    pub project: Project,
    pub instruments: Vec<Option<(String, Vec<u8>)>>,
}

impl ProjectFile {
    /// What is loaded in the engine now.
    pub fn capture(h: &EngineHandle) -> Self {
        let instruments = h
            .slots
            .iter()
            .map(|s| {
                let c = s.as_ref()?.lock().ok()?;
                Some((c.info().name, c.save_state()))
            })
            .collect();
        Self { project: h.project.clone(), instruments }
    }

    pub fn encode(&self) -> Vec<u8> {
        let mut out = MAGIC.to_vec();
        out.push(VERSION);
        let p = io::encode(&self.project);
        out.extend_from_slice(&(p.len() as u32).to_le_bytes());
        out.extend_from_slice(&p);
        for slot in self.instruments.iter().take(TRACKS) {
            match slot {
                None => out.push(0),
                Some((name, state)) => {
                    out.push(1);
                    out.push(name.len().min(255) as u8);
                    out.extend_from_slice(&name.as_bytes()[..name.len().min(255)]);
                    out.extend_from_slice(&(state.len() as u32).to_le_bytes());
                    out.extend_from_slice(state);
                }
            }
        }
        out
    }

    pub fn decode(data: &[u8]) -> Option<Self> {
        let mut r = data;
        let mut take = |n: usize| -> Option<&[u8]> {
            let (a, b) = r.split_at_checked(n)?;
            r = b;
            Some(a)
        };
        let magic = take(4)?;
        if (magic != MAGIC && magic != LEGACY_MAGIC) || take(1)?[0] != VERSION {
            return None;
        }
        let n = u32::from_le_bytes(take(4)?.try_into().ok()?) as usize;
        let project = io::decode(take(n)?)?;
        let mut instruments = Vec::new();
        for _ in 0..TRACKS {
            if take(1)?[0] == 0 {
                instruments.push(None);
                continue;
            }
            let len = take(1)?[0] as usize;
            let name = String::from_utf8(take(len)?.to_vec()).ok()?;
            let sl = u32::from_le_bytes(take(4)?.try_into().ok()?) as usize;
            instruments.push(Some((name, take(sl)?.to_vec())));
        }
        Some(Self { project, instruments })
    }

    /// Put this project and its instruments into the engine.
    pub fn apply(&self, h: &mut EngineHandle) {
        h.send(EngineCommand::LoadProject(Box::new(self.project.clone())));
        for (t, slot) in self.instruments.iter().enumerate().take(TRACKS) {
            let current_is_bridge = h.slots[t].as_ref().and_then(|c| c.lock().ok().map(|c| c.info().name == MIDI_BRIDGE)).unwrap_or(false);
            match slot {
                Some((name, _)) if name == MIDI_BRIDGE => {} // the running bridge stays
                Some((name, state)) => {
                    if let Some(mut c) = octachrome_cartridges::create(name) {
                        c.load_state(state);
                        h.load_cartridge(t, c);
                    }
                }
                None if current_is_bridge => {}
                None => {
                    h.slots[t] = None;
                    h.send(EngineCommand::LoadCartridge { track: t as u8, cartridge: None::<SharedCartridge> });
                }
            }
        }
    }
}

/// Where the projects of the player live, if a home or config folder is known.
pub fn default_dir() -> Option<PathBuf> {
    if let Some(p) = std::env::var_os("OCTACHROME_PROJECTS").filter(|p| !p.is_empty()) {
        return Some(PathBuf::from(p));
    }
    crate::settings::default_path().and_then(|p| p.parent().map(|d| d.join("projects")))
}

/// The file of a user slot (0..8) in `dir`.
pub fn user_path(dir: &Path, slot: usize) -> PathBuf {
    dir.join(format!("user-{}.ocp", slot + 1))
}

/// The name of a saved project or preset lives in a small text file next to it (`user-1.ocp` → `user-1.name`): the unit writes
/// one when it saves, and a player can write or change one with any editor. A place without one is called `user N`.
pub fn name_path(path: &Path) -> PathBuf {
    path.with_extension("name")
}

/// The name saved with a file, if it has one: the first line, trimmed, at most 14 characters.
pub fn read_name(path: &Path) -> Option<String> {
    let text = std::fs::read_to_string(name_path(path)).ok()?;
    let name: String = text.lines().next()?.trim().chars().take(14).collect();
    (!name.is_empty()).then_some(name)
}

pub fn write_name(path: &Path, name: &str) {
    let _ = std::fs::write(name_path(path), format!("{name}\n"));
}

/// Erase a saved project or preset and its name. `true` if the file was there and is gone.
pub fn erase(path: &Path) -> bool {
    let _ = std::fs::remove_file(name_path(path));
    std::fs::remove_file(path).is_ok()
}

/// Today as the unit names what it saves: `16 nov` (UTC: the unit has no clock of its own to ask for the local day).
pub fn today() -> String {
    let secs = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs());
    day_label(secs / 86_400)
}

/// `16 nov` for a day counted from 1970-01-01 (civil-from-days, Howard Hinnant's algorithm).
pub fn day_label(days: u64) -> String {
    let z = days as i64 + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let _ = era;
    const MONTHS: [&str; 12] = ["jan", "feb", "mar", "apr", "may", "jun", "jul", "aug", "sep", "oct", "nov", "dec"];
    format!("{day} {}", MONTHS[(month - 1) as usize])
}

pub fn save(path: &Path, f: &ProjectFile) -> std::io::Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    // write next to it and rename, so a crash never leaves half a project
    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, f.encode())?;
    std::fs::rename(&tmp, path)
}

pub fn load(path: &Path) -> Option<ProjectFile> {
    ProjectFile::decode(&std::fs::read(path).ok()?)
}

#[cfg(test)]
mod tests {
    use super::*;

    // F-F02
    #[test]
    fn a_project_file_round_trips_and_damaged_ones_are_refused() {
        let f = ProjectFile {
            project: crate::demo::project(),
            instruments: vec![Some(("drums".into(), vec![1, 2, 3])), None, Some(("KickSyn".into(), vec![])), None, None, None, None, None],
        };
        let bytes = f.encode();
        assert_eq!(ProjectFile::decode(&bytes), Some(f));
        assert!(ProjectFile::decode(&bytes[..bytes.len() - 1]).is_none(), "truncated");
        assert!(ProjectFile::decode(b"nope").is_none());
        let mut other = bytes.clone();
        other[4] = 99;
        assert!(ProjectFile::decode(&other).is_none(), "another version");
    }
}

#[cfg(test)]
mod name_tests {
    use super::*;

    #[test]
    fn days_are_named_as_the_unit_writes_them() {
        assert_eq!(day_label(0), "1 jan");
        assert_eq!(day_label(20_408), "16 nov"); // 2025-11-16
        assert_eq!(day_label(19_782), "29 feb"); // 2024-02-29
        assert_eq!(day_label(20_453), "31 dec"); // 2025-12-31
    }
}
