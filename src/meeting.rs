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

use std::ffi::c_void;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};

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
    None
}

pub const BROWSERS: [&str; 6] = [
    "Google Chrome",
    "Safari",
    "Arc",
    "Firefox",
    "Microsoft Edge",
    "Brave Browser",
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
    std::process::Command::new("ps")
        .args(["-axo", "comm="])
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

    #[test]
    fn zoom_is_only_a_meeting_when_cpthost_runs() {
        let idle = "/Applications/zoom.us.app/Contents/MacOS/zoom.us\n/usr/bin/foo";
        assert_eq!(classify(idle), None);
        let live = "/Applications/zoom.us.app/Contents/MacOS/zoom.us\n/Applications/zoom.us.app/Contents/Frameworks/CptHost.app/Contents/MacOS/CptHost";
        assert_eq!(classify(live), Some(App { name: "Zoom", in_call: true }));
    }

    #[test]
    fn other_apps_are_recognised_but_not_in_call() {
        let teams = "/Applications/Microsoft Teams.app/Contents/MacOS/MSTeams";
        assert_eq!(classify(teams), Some(App { name: "Microsoft Teams", in_call: false }));
        assert_eq!(classify("/System/Applications/Notes.app/Contents/MacOS/Notes"), None);
    }

    #[test]
    fn microphone_probe_returns_without_crashing() {
        let _ = mic_in_use();
    }
}
