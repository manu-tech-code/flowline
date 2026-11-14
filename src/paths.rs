//! Where Flowline keeps its files, per operating system.

use std::path::PathBuf;

/// The folder holding `state.json`, `notes.json`, `models/` and the lock file.
/// macOS: `~/Library/Application Support/Flowline`.
/// Linux: `$XDG_DATA_HOME/flowline`, else `~/.local/share/flowline`.
pub fn data_dir() -> Option<PathBuf> {
    let home = std::env::var_os("HOME").filter(|h| !h.is_empty())?;
    Some(data_dir_from(&PathBuf::from(home), std::env::var_os("XDG_DATA_HOME")))
}

fn data_dir_from(home: &std::path::Path, xdg_data_home: Option<std::ffi::OsString>) -> PathBuf {
    if cfg!(target_os = "macos") {
        home.join("Library/Application Support/Flowline")
    } else {
        let base = xdg_data_home
            .filter(|v| !v.is_empty())
            .map(PathBuf::from)
            .unwrap_or_else(|| home.join(".local/share"));
        base.join("flowline")
    }
}

pub fn models_dir() -> Option<PathBuf> {
    data_dir().map(|d| d.join("models"))
}

/// The data folder as shown to the person, with `~` for the home folder.
pub fn display_data_dir() -> &'static str {
    if cfg!(target_os = "macos") {
        "~/Library/Application Support/Flowline"
    } else {
        "~/.local/share/flowline"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn data_dir_follows_the_platform_convention() {
        let home = Path::new("/home/ada");
        let dir = data_dir_from(home, None);
        if cfg!(target_os = "macos") {
            assert_eq!(dir, Path::new("/home/ada/Library/Application Support/Flowline"));
        } else {
            assert_eq!(dir, Path::new("/home/ada/.local/share/flowline"));
        }
    }

    #[test]
    fn xdg_data_home_is_honoured_off_macos() {
        let dir = data_dir_from(Path::new("/home/ada"), Some("/data".into()));
        if !cfg!(target_os = "macos") {
            assert_eq!(dir, Path::new("/data/flowline"));
        }
        // An empty value counts as unset, as the XDG spec says.
        let dir = data_dir_from(Path::new("/home/ada"), Some("".into()));
        if !cfg!(target_os = "macos") {
            assert_eq!(dir, Path::new("/home/ada/.local/share/flowline"));
        }
    }
}
