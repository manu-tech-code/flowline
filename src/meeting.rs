//! Meeting auto-detect for Notetaker.
//!
//! A background thread watches two local signals every two seconds and nothing
//! else leaves the Mac:
//!
//! * whether any app is using the default microphone (CoreAudio's
//!   `DeviceIsRunningSomewhere`), and for how long without a break;
//! * which known meeting apps are running (`ps`), plus Zoom's `CptHost`
//!   helper, which exists only while a Zoom meeting is open.
//!
//! The app decides what to do with that (see `features.rs`). This module
//! never records, reads window titles or looks at audio.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};

#[cfg(target_os = "macos")]
mod coreaudio {
    use std::ffi::c_void;

    #[repr(C)]
    struct PropertyAddress {
        selector: u32,
        scope: u32,
        element: u32,
    }

    #[link(name = "CoreAudio", kind = "framework")]
    extern "C" {
        fn AudioObjectGetPropertyData(
            object: u32,
            address: *const PropertyAddress,
            qualifier_size: u32,
            qualifier: *const c_void,
            size: *mut u32,
            data: *mut c_void,
        ) -> i32;
    }

    const SYSTEM_OBJECT: u32 = 1;
    const SCOPE_GLOBAL: u32 = 0x676C_6F62; // 'glob'
    const DEFAULT_INPUT_DEVICE: u32 = 0x6449_6E20; // 'dIn '
    const IS_RUNNING_SOMEWHERE: u32 = 0x676F_6E65; // 'gone'

    /// True while any process (including this one) has the default input open.
    pub fn mic_in_use() -> bool {
        unsafe {
            let addr = PropertyAddress {
                selector: DEFAULT_INPUT_DEVICE,
                scope: SCOPE_GLOBAL,
                element: 0,
            };
            let mut device: u32 = 0;
            let mut size = std::mem::size_of::<u32>() as u32;
            let status = AudioObjectGetPropertyData(
                SYSTEM_OBJECT,
                &addr,
                0,
                std::ptr::null(),
                &mut size,
                &mut device as *mut u32 as *mut c_void,
            );
            if status != 0 || device == 0 {
                return false;
            }
            let addr = PropertyAddress {
                selector: IS_RUNNING_SOMEWHERE,
                scope: SCOPE_GLOBAL,
                element: 0,
            };
            let mut running: u32 = 0;
            let mut size = std::mem::size_of::<u32>() as u32;
            let status = AudioObjectGetPropertyData(
                device,
                &addr,
                0,
                std::ptr::null(),
                &mut size,
                &mut running as *mut u32 as *mut c_void,
            );
            status == 0 && running != 0
        }
    }
}
#[cfg(target_os = "macos")]
use coreaudio::mic_in_use;

/// True while another app is recording from a microphone. Linux asks the sound
/// server (PulseAudio or PipeWire's `pactl`) for its recording streams and
/// ignores Flowline's own.
#[cfg(target_os = "linux")]
pub fn mic_in_use() -> bool {
    std::process::Command::new("pactl")
        .args(["list", "source-outputs"])
        .output()
        .map(|o| other_app_is_recording(&String::from_utf8_lossy(&o.stdout)))
        .unwrap_or(false)
}

/// Read `pactl list source-outputs`: is any stream that is not Flowline's, and
/// is not paused ("Corked"), recording?
#[cfg_attr(target_os = "macos", allow(dead_code))]
pub fn other_app_is_recording(listing: &str) -> bool {
    listing.split("Source Output #").skip(1).any(|stream| {
        let lower = stream.to_lowercase();
        let paused = lower.contains("corked: yes");
        let ours = lower.contains("flowline");
        // The volume-meter streams a mixer or settings panel opens are not calls.
        let meter = lower.contains("peak detect") || lower.contains("volume meter");
        !paused && !ours && !meter
    })
}

/// A meeting app found running. `in_call` means the app has a helper that
/// exists only during a call, so its disappearance reliably marks the end.
#[derive(Clone, Debug, PartialEq)]
pub struct App {
    pub name: &'static str,
    pub in_call: bool,
}

/// Match `ps -axo comm=` output against known meeting apps. Zoom's `CptHost`
/// wins because it proves a meeting is open, not just that Zoom is running.
pub fn classify(processes: &str) -> Option<App> {
    if processes.contains("/CptHost") {
        return Some(App { name: "Zoom", in_call: true });
    }
    let table: [(&str, &str); 5] = [
        ("MSTeams", "Microsoft Teams"),
        ("Microsoft Teams", "Microsoft Teams"),
        ("/Webex", "Webex"),
        ("/FaceTime.app/", "FaceTime"),
        ("/Discord", "Discord"),
    ];
    for (needle, name) in table {
        if processes.contains(needle) {
            return Some(App { name, in_call: false });
        }
    }
    if processes.contains("/Slack.app/") {
        return Some(App { name: "Slack", in_call: false });
    }
    #[cfg(target_os = "linux")]
    for (needle, name) in LINUX_APPS {
        if processes.lines().any(|l| process_matches(l, needle)) {
            return Some(App { name, in_call: false });
        }
    }
    None
}

/// Linux process names (from `ps -axo args=`, so these are command lines).
#[cfg(target_os = "linux")]
const LINUX_APPS: [(&str, &str); 8] = [
    ("zoom", "Zoom"),
    ("teams-for-linux", "Microsoft Teams"),
    ("teams", "Microsoft Teams"),
    ("webex", "Webex"),
    ("slack", "Slack"),
    ("discord", "Discord"),
    ("skypeforlinux", "Skype"),
    ("signal-desktop", "Signal"),
];

/// Does this command line run the program called `name`? Matches the program's
/// own file name, not an argument that happens to contain it.
#[cfg_attr(target_os = "macos", allow(dead_code))]
fn process_matches(command_line: &str, name: &str) -> bool {
    let program = command_line.split_whitespace().next().unwrap_or("");
    let file = program.rsplit('/').next().unwrap_or("").to_lowercase();
    file == name || file.starts_with(&format!("{name}-")) || file.starts_with(&format!("{name}."))
}

/// Browser names as macOS reports them, and window classes as Linux does.
pub const BROWSERS: [&str; 16] = [
    "Google Chrome",
    "Safari",
    "Arc",
    "Firefox",
    "Microsoft Edge",
    "Brave Browser",
    "google-chrome",
    "chromium",
    "brave-browser",
    "microsoft-edge",
    "vivaldi-stable",
    "opera",
    "epiphany",
    "librewolf",
    "zen",
    "navigator",
];

#[derive(Clone, Default)]
pub struct Snapshot {
    pub app: Option<App>,
    /// How long the microphone has been in use without a break.
    pub mic_secs: f32,
}

static ENABLED: AtomicBool = AtomicBool::new(false);
static STARTED: AtomicBool = AtomicBool::new(false);
static SNAPSHOT: Mutex<Snapshot> = Mutex::new(Snapshot { app: None, mic_secs: 0.0 });

pub fn set_enabled(on: bool) {
    ENABLED.store(on, Ordering::Relaxed);
}

pub fn snapshot() -> Snapshot {
    SNAPSHOT.lock().map(|s| s.clone()).unwrap_or_default()
}

fn running_processes() -> String {
    // macOS `comm` is the full executable path; on Linux it is only the short
    // name, so ask for the whole command line instead.
    let column = if cfg!(target_os = "macos") { "comm=" } else { "args=" };
    std::process::Command::new("ps")
        .args(["-axo", column])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).into_owned())
        .unwrap_or_default()
}

/// Start the watcher once. It idles (no `ps`, no CoreAudio calls) while
/// auto-detect is off. `wake` is called after each poll so a hidden window
/// still gets a frame to act on the result.
pub fn start(wake: impl Fn() + Send + 'static) {
    if STARTED.swap(true, Ordering::SeqCst) {
        return;
    }
    std::thread::spawn(move || {
        let mut mic_since: Option<Instant> = None;
        loop {
            std::thread::sleep(Duration::from_secs(2));
            if !ENABLED.load(Ordering::Relaxed) {
                mic_since = None;
                if let Ok(mut s) = SNAPSHOT.lock() {
                    *s = Snapshot::default();
                }
                continue;
            }
            if mic_in_use() {
                mic_since.get_or_insert_with(Instant::now);
            } else {
                mic_since = None;
            }
            let snap = Snapshot {
                app: classify(&running_processes()),
                mic_secs: mic_since.map(|t| t.elapsed().as_secs_f32()).unwrap_or(0.0),
            };
            if let Ok(mut s) = SNAPSHOT.lock() {
                *s = snap;
            }
            wake();
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(target_os = "macos")]
    #[test]
    fn zoom_is_only_a_meeting_when_cpthost_runs() {
        let idle = "/Applications/zoom.us.app/Contents/MacOS/zoom.us\n/usr/bin/foo";
        assert_eq!(classify(idle), None);
        let live = "/Applications/zoom.us.app/Contents/MacOS/zoom.us\n/Applications/zoom.us.app/Contents/Frameworks/CptHost.app/Contents/MacOS/CptHost";
        assert_eq!(classify(live), Some(App { name: "Zoom", in_call: true }));
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn zoom_on_linux_is_a_weak_signal_needing_the_microphone() {
        // No call-only helper process exists to prove a meeting, so a running
        // Zoom only counts once the microphone is busy too (in_call is false).
        let zoom = "/opt/zoom/zoom --url=zoommtg://example\n/usr/bin/foo";
        assert_eq!(classify(zoom), Some(App { name: "Zoom", in_call: false }));
        assert_eq!(classify("/usr/bin/foo\n/usr/bin/bar"), None);
        // A file that merely mentions zoom is not Zoom.
        assert_eq!(classify("/usr/bin/vim zoom-notes.txt"), None);
    }

    #[test]
    fn other_apps_are_recognised_but_not_in_call() {
        let teams = "/Applications/Microsoft Teams.app/Contents/MacOS/MSTeams";
        assert_eq!(classify(teams), Some(App { name: "Microsoft Teams", in_call: false }));
        assert_eq!(classify("/System/Applications/Notes.app/Contents/MacOS/Notes"), None);
    }

    #[test]
    fn pactl_listing_tells_other_apps_from_flowline_and_paused_streams() {
        let none = "";
        assert!(!other_app_is_recording(none));
        let zoom = "Source Output #12\n\tCorked: no\n\tProperties:\n\t\tapplication.name = \"ZOOM VoiceEngine\"\n";
        assert!(other_app_is_recording(zoom));
        let ours = "Source Output #3\n\tCorked: no\n\tProperties:\n\t\tapplication.name = \"Flowline\"\n";
        assert!(!other_app_is_recording(ours));
        let paused = "Source Output #4\n\tCorked: yes\n\tProperties:\n\t\tapplication.name = \"Firefox\"\n";
        assert!(!other_app_is_recording(paused));
        let meter = "Source Output #5\n\tCorked: no\n\tProperties:\n\t\tapplication.name = \"PulseAudio Volume Control\"\n\t\tmedia.name = \"Peak detect\"\n";
        assert!(!other_app_is_recording(meter));
        let mixed = format!("{ours}{zoom}");
        assert!(other_app_is_recording(&mixed), "a real call next to our own stream still counts");
    }

    #[test]
    fn process_names_are_matched_by_program_not_by_argument() {
        assert!(process_matches("/opt/zoom/zoom --url=zoommtg://x", "zoom"));
        assert!(process_matches("/usr/bin/teams-for-linux", "teams-for-linux"));
        assert!(process_matches("/usr/lib/slack/slack --type=renderer", "slack"));
        assert!(!process_matches("/usr/bin/vim notes-about-zoom.txt", "zoom"));
        assert!(!process_matches("/usr/bin/grep slack", "slack"));
    }

    #[test]
    fn microphone_probe_returns_without_crashing() {
        let _ = mic_in_use();
    }
}
