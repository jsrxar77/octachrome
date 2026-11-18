//! Where the player's own things live: settings, projects, presets and samples, in `<config>/octachrome/`, with `<config>` being
//! `$XDG_CONFIG_HOME` or `~/.config`. The project had another name at first (`op8`), and its folder is moved here the first time it is
//! looked for, with the project files in it taking the new extension, so that nothing a player saved is lost.

use std::path::{Path, PathBuf};

/// The folder that holds every config folder: `$XDG_CONFIG_HOME`, else `~/.config`.
pub fn config_home() -> Option<PathBuf> {
    std::env::var_os("XDG_CONFIG_HOME")
        .filter(|p| !p.is_empty())
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))
}

/// The folder of the player's things (`<config>/octachrome`), after bringing over the one of the old name if it is the only one.
pub fn dir() -> Option<PathBuf> {
    let home = config_home()?;
    migrate(&home);
    Some(home.join("octachrome"))
}

/// Move `<home>/op8` to `<home>/octachrome` when only the old one exists, and give the project files in it (`user-N.op8p`) their new
/// extension (`user-N.ocp`). Returns whether it moved the folder. A folder of the new name is never touched.
pub fn migrate(home: &Path) -> bool {
    let (old, new) = (home.join("op8"), home.join("octachrome"));
    if new.exists() || !old.is_dir() || std::fs::rename(&old, &new).is_err() {
        return false;
    }
    if let Ok(files) = std::fs::read_dir(new.join("projects")) {
        for f in files.flatten() {
            let path = f.path();
            if path.extension().is_some_and(|e| e == "op8p") {
                let _ = std::fs::rename(&path, path.with_extension("ocp"));
            }
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("octachrome-config-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    // F-F19: what a player saved under the first name is found under the new one: the folder moves, the projects take the new extension,
    // their names and the samples and presets go with them, and a folder of the new name is never touched.
    #[test]
    fn the_folder_of_the_old_name_moves_and_loses_nothing() {
        let home = temp("move");
        let old = home.join("op8");
        std::fs::create_dir_all(old.join("projects")).unwrap();
        std::fs::create_dir_all(old.join("samples")).unwrap();
        std::fs::write(old.join("settings"), b"theme=chrome\n").unwrap();
        std::fs::write(old.join("projects/user-1.op8p"), b"project").unwrap();
        std::fs::write(old.join("projects/user-1.name"), b"my song\n").unwrap();
        std::fs::write(old.join("samples/rec-1.wav"), b"wav").unwrap();
        assert!(migrate(&home));
        let new = home.join("octachrome");
        assert!(!old.exists(), "the old folder is gone");
        assert_eq!(std::fs::read(new.join("settings")).unwrap(), b"theme=chrome\n");
        assert_eq!(std::fs::read(new.join("projects/user-1.ocp")).unwrap(), b"project");
        assert!(!new.join("projects/user-1.op8p").exists());
        assert_eq!(std::fs::read(new.join("projects/user-1.name")).unwrap(), b"my song\n");
        assert_eq!(std::fs::read(new.join("samples/rec-1.wav")).unwrap(), b"wav");
        assert!(!migrate(&home), "and it only happens once");
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn a_folder_of_the_new_name_is_never_touched() {
        let home = temp("keep");
        std::fs::create_dir_all(home.join("op8")).unwrap();
        std::fs::create_dir_all(home.join("octachrome")).unwrap();
        std::fs::write(home.join("op8/settings"), b"old").unwrap();
        std::fs::write(home.join("octachrome/settings"), b"new").unwrap();
        assert!(!migrate(&home));
        assert_eq!(std::fs::read(home.join("octachrome/settings")).unwrap(), b"new");
        assert!(home.join("op8/settings").exists(), "the old one stays where it is");
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn nothing_to_move_is_not_an_error() {
        let home = temp("none");
        assert!(!migrate(&home));
        assert!(!home.join("octachrome").exists());
        let _ = std::fs::remove_dir_all(&home);
    }
}
