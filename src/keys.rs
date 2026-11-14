//! The names of Flowline's shortcuts, per operating system, so the interface
//! always shows the keys that actually work on this machine.
//!
//! macOS can swallow keys, so it uses Fn, ⌥digit and ⌥Space. Linux reads the
//! keyboard without taking keys away from other apps, so its shortcuts use
//! chords that other apps do not claim.

/// Hold-to-talk choices offered in Settings, first = default.
pub fn ptt_choices() -> &'static [&'static str] {
    if cfg!(target_os = "macos") {
        &["Fn", "Ctrl Space"]
    } else {
        &["Right Ctrl", "Right Alt", "Ctrl Space"]
    }
}

pub fn default_ptt() -> &'static str {
    ptt_choices()[0]
}

/// The next choice after `current`, wrapping; an unknown value gives the default.
pub fn next_ptt(current: &str) -> &'static str {
    let list = ptt_choices();
    match list.iter().position(|c| *c == current) {
        Some(i) => list[(i + 1) % list.len()],
        None => list[0],
    }
}

/// Keycap labels for the hold-to-talk setting (used on the Home hero).
pub fn ptt_caps(ptt: &str) -> (String, Option<String>) {
    match ptt {
        "Ctrl Space" => ("Ctrl".into(), Some("Space".into())),
        other => (other.into(), None),
    }
}

/// How a transform shortcut reads, for slot `n` (1-based).
pub fn transform_key(n: usize) -> String {
    if cfg!(target_os = "macos") {
        format!("⌥{n}")
    } else {
        format!("Ctrl+Alt+Shift+{n}")
    }
}

/// "⌥3–⌥9" / "Ctrl+Alt+Shift+3 to 9".
pub fn transform_custom_range() -> String {
    if cfg!(target_os = "macos") {
        "⌥3–⌥9".to_string()
    } else {
        "Ctrl+Alt+Shift+3 to 9".to_string()
    }
}

pub fn command_key() -> &'static str {
    if cfg!(target_os = "macos") {
        "⌥Space"
    } else {
        "Ctrl+Alt+Space"
    }
}

/// Wording that names the machine, as this system would say it: "this Mac" and
/// "macOS" become "this computer" and "Linux" off macOS.
pub fn local(text: &str) -> String {
    if cfg!(target_os = "macos") {
        text.to_string()
    } else {
        text.replace("this Mac", "this computer").replace("macOS", "Linux")
    }
}

#[cfg(target_os = "macos")]
pub const DELETE_ALL_NOTE: &str = "This deletes every transcript and note insertion from this Mac.";
#[cfg(not(target_os = "macos"))]
pub const DELETE_ALL_NOTE: &str = "This deletes every transcript and note insertion from this computer.";

/// The permission that lets Flowline press keys in other apps.
pub fn paste_permission() -> &'static str {
    if cfg!(target_os = "macos") {
        "Accessibility"
    } else {
        "paste access"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ptt_cycles_through_the_choices_and_recovers_from_unknown() {
        let first = default_ptt();
        let second = next_ptt(first);
        assert_ne!(first, second);
        let mut cur = first;
        for _ in 0..ptt_choices().len() {
            cur = next_ptt(cur);
        }
        assert_eq!(cur, first);
        assert_eq!(next_ptt("nonsense"), first);
    }

    #[test]
    fn wording_names_the_machine_for_this_system() {
        let t = local("Everything stays on this Mac. Built for macOS.");
        if cfg!(target_os = "macos") {
            assert_eq!(t, "Everything stays on this Mac. Built for macOS.");
        } else {
            assert_eq!(t, "Everything stays on this computer. Built for Linux.");
        }
    }

    #[test]
    fn caps_split_a_chord() {
        assert_eq!(ptt_caps("Ctrl Space"), ("Ctrl".to_string(), Some("Space".to_string())));
        assert_eq!(ptt_caps("Right Ctrl"), ("Right Ctrl".to_string(), None));
    }
}
