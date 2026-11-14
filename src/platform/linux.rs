//! Linux back end: the same functions the macOS back end offers, built on the
//! kernel's evdev/uinput, the X11 protocol where it is available, and a few
//! standard command-line tools.
//!
//! * **Global shortcuts** read keyboards through evdev (`/dev/input/event*`), so
//!   they work on X11 and on Wayland. The person must be in the `input` group;
//!   `scripts/flowline-setup.sh` does that.
//! * **Pasting into other apps** types Ctrl+V through a uinput virtual keyboard
//!   (needs write access to `/dev/uinput`), or `xdotool`/`wtype` if installed.
//!   Without any of them the text is left on the clipboard.
//! * **The active window** is known on X11 sessions only. Wayland deliberately
//!   hides it, so there the app name is unknown and Flowline plays safe.
//! * There is no tray item. Closing the window hides it, and launching Flowline
//!   again brings it back (see `signal_running_instance`).

use super::hotkey_logic::{self as hk, Chords, Config, Ptt};
use super::shared::*;
use std::io::{Read, Write};
use std::os::unix::io::AsRawFd;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicI32, AtomicU32, AtomicUsize, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};

// ---------------------------------------------------------------------------
// Permissions and what this machine can do
// ---------------------------------------------------------------------------

/// Linux has no microphone prompt of its own.
pub fn mic_authorized() -> bool {
    true
}

/// Mirrors the macOS authorization codes: 3 = authorized.
pub fn mic_status() -> i64 {
    3
}

fn on_path(tool: &str) -> bool {
    std::env::var_os("PATH")
        .map(|paths| std::env::split_paths(&paths).any(|d| d.join(tool).is_file()))
        .unwrap_or(false)
}

fn uinput_writable() -> bool {
    std::fs::OpenOptions::new().write(true).open("/dev/uinput").is_ok()
}

/// Can Flowline press keys in other apps? Answered from a cache refreshed every
/// few seconds, because the interface asks this on every frame.
pub fn accessibility_granted() -> bool {
    static CACHE: Mutex<Option<(Instant, bool)>> = Mutex::new(None);
    let mut cache = CACHE.lock().unwrap();
    if let Some((at, value)) = *cache {
        if at.elapsed() < Duration::from_secs(3) {
            return value;
        }
    }
    let value = uinput_writable() || on_path("xdotool") || on_path("wtype");
    *cache = Some((Instant::now(), value));
    value
}

/// There is no system prompt on Linux; report the current state.
pub fn request_accessibility() -> bool {
    accessibility_granted()
}

/// True once at least one keyboard can be read.
pub fn input_monitoring_granted() -> bool {
    DEVICES.load(Ordering::Relaxed) > 0
}

pub fn request_input_monitoring() -> bool {
    input_monitoring_granted()
}

/// No system settings pane to open; the app shows the setup steps itself.
pub fn open_privacy_pane(_anchor: &str) {}

/// The one-time setup, as a single shell command: join the `input` group (read
/// keyboards) and allow that group to use `/dev/uinput` (paste). The same steps
/// are in `scripts/flowline-setup.sh`.
pub fn setup_command() -> String {
    concat!(
        "sudo modprobe uinput && ",
        "echo uinput | sudo tee /etc/modules-load.d/flowline.conf >/dev/null && ",
        "sudo usermod -aG input \"$USER\" && ",
        "echo 'KERNEL==\"uinput\", GROUP=\"input\", MODE=\"0660\", OPTIONS+=\"static_node=uinput\"' ",
        "| sudo tee /etc/udev/rules.d/99-flowline.rules >/dev/null && ",
        "sudo udevadm control --reload && sudo udevadm trigger"
    )
    .to_string()
}

/// What to tell the person when no keyboard can be read.
pub fn denied_hint() -> &'static str {
    "Flowline cannot read your keyboard yet. Run flowline-setup.sh, then log out and back in."
}

/// Open a file or folder with the desktop's default app.
pub fn open_path(path: &Path) {
    let _ = std::process::Command::new("xdg-open").arg(path).spawn();
}

/// Start the app again in place (used after a permission change).
pub fn restart_bundle() {
    if let Ok(exe) = std::env::current_exe() {
        let _ = std::process::Command::new(exe)
            .args(std::env::args().skip(1))
            .spawn();
    }
    std::process::exit(0);
}

// ---------------------------------------------------------------------------
// Global shortcuts (evdev)
// ---------------------------------------------------------------------------

static MONITOR_STARTED: AtomicBool = AtomicBool::new(false);
static DEVICES: AtomicUsize = AtomicUsize::new(0);
static PTT_MODE: AtomicI32 = AtomicI32::new(0);
/// Modifier keys currently down, so a paste can wait for them to come up.
static MODS_HELD: AtomicU32 = AtomicU32::new(0);

pub fn set_hotkey_mode(ptt: &str) {
    PTT_MODE.store(Ptt::from_label(ptt).to_index(), Ordering::Relaxed);
}

pub fn hotkey_running() -> bool {
    MONITOR_STARTED.load(Ordering::Relaxed)
}

pub fn hotkey_active() -> bool {
    DEVICES.load(Ordering::Relaxed) > 0
}

/// The macOS back end swaps a listen-only tap for an active one here; there is
/// nothing to upgrade with evdev.
pub fn upgrade_hotkey_tap(_ctx: eframe::egui::Context) {}

/// macOS opens an extra low-level keyboard source here; evdev already is one.
pub fn start_hid_keyboard_service() {}

pub fn ensure_hotkey_monitor(ctx: eframe::egui::Context) {
    *WAKE.lock().unwrap() = Some(ctx);
    if MONITOR_STARTED.swap(true, Ordering::SeqCst) {
        return;
    }
    std::thread::Builder::new()
        .name("flowline-keys".into())
        .spawn(input_loop)
        .ok();
}

fn is_keyboard(dev: &evdev::Device) -> bool {
    if dev.name().unwrap_or("").starts_with("Flowline") {
        return false; // our own virtual keyboard
    }
    dev.supported_keys().map_or(false, |keys| {
        keys.contains(evdev::KeyCode::KEY_A) && keys.contains(evdev::KeyCode::KEY_ENTER)
    })
}

/// Open every readable keyboard we do not have yet. Returns the new ones and
/// how many event nodes exist but could not be read.
fn scan(open: &[PathBuf]) -> (Vec<(PathBuf, evdev::Device)>, usize) {
    let mut found = Vec::new();
    let mut denied = 0;
    let Ok(dir) = std::fs::read_dir("/dev/input") else {
        return (found, 0);
    };
    for entry in dir.flatten() {
        let path = entry.path();
        let is_event = path
            .file_name()
            .and_then(|n| n.to_str())
            .map_or(false, |n| n.starts_with("event"));
        if !is_event || open.contains(&path) {
            continue;
        }
        match evdev::Device::open(&path) {
            Ok(dev) => {
                if is_keyboard(&dev) && dev.set_nonblocking(true).is_ok() {
                    found.push((path, dev));
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied => denied += 1,
            Err(_) => {}
        }
    }
    (found, denied)
}

fn input_loop() {
    let started = Instant::now();
    let mut chords = Chords::default();
    let mut devices: Vec<(PathBuf, evdev::Device)> = Vec::new();
    let mut last_scan: Option<Instant> = None;
    let mut told_denied = false;

    loop {
        // Look for keyboards now and then, so a plugged-in one is picked up and
        // a permission fix takes effect without restarting.
        if last_scan.map_or(true, |t| t.elapsed() >= Duration::from_secs(3)) {
            last_scan = Some(Instant::now());
            let paths: Vec<PathBuf> = devices.iter().map(|(p, _)| p.clone()).collect();
            let (new, denied) = scan(&paths);
            devices.extend(new);
            DEVICES.store(devices.len(), Ordering::Relaxed);
            if devices.is_empty() && denied > 0 && !told_denied {
                told_denied = true;
                push_event(HotkeyEvent::Denied);
            }
        }
        if devices.is_empty() {
            std::thread::sleep(Duration::from_millis(500));
            continue;
        }

        let mut fds: Vec<libc::pollfd> = devices
            .iter()
            .map(|(_, d)| libc::pollfd { fd: d.as_raw_fd(), events: libc::POLLIN, revents: 0 })
            .collect();
        let ready = unsafe { libc::poll(fds.as_mut_ptr(), fds.len() as libc::nfds_t, 40) };
        let now_ms = started.elapsed().as_millis() as u64;
        let cfg = Config {
            ptt: Ptt::from_index(PTT_MODE.load(Ordering::Relaxed)),
            xform_slots: XFORM_SLOTS.load(Ordering::Relaxed),
            command: COMMAND_ENABLED.load(Ordering::Relaxed),
        };

        let mut out = Vec::new();
        let mut dead = Vec::new();
        if ready > 0 {
            for (i, fd) in fds.iter().enumerate() {
                if fd.revents & (libc::POLLERR | libc::POLLHUP | libc::POLLNVAL) != 0 {
                    dead.push(i);
                } else if fd.revents & libc::POLLIN != 0 {
                    match devices[i].1.fetch_events() {
                        Ok(events) => {
                            for ev in events {
                                if ev.event_type() == evdev::EventType::KEY {
                                    out.extend(chords.key(ev.code(), ev.value(), now_ms, &cfg));
                                }
                            }
                        }
                        Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {}
                        Err(_) => dead.push(i),
                    }
                }
            }
        }
        out.extend(chords.tick(now_ms));
        MODS_HELD.store(chords.held_mask(), Ordering::Relaxed);
        for e in out {
            push_event(e);
        }
        if !dead.is_empty() {
            for i in dead.into_iter().rev() {
                devices.remove(i);
            }
            DEVICES.store(devices.len(), Ordering::Relaxed);
            for e in chords.reset() {
                push_event(e);
            }
            MODS_HELD.store(0, Ordering::Relaxed);
        }
    }
}

// ---------------------------------------------------------------------------
// Pressing keys in other apps
// ---------------------------------------------------------------------------

const KEY_V: u16 = 47;
const KEY_C: u16 = 46;
const KEY_INSERT: u16 = 110;

/// The key chords Flowline sends. Ctrl+Insert / Shift+Insert are the classic
/// copy / paste chords that work in terminals too, where Ctrl+C would
/// interrupt the running program.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Chord {
    Paste,
    PasteTerminal,
    PasteInsert,
    CopyInsert,
}

fn chord_keys(c: Chord) -> (&'static [u16], u16) {
    match c {
        Chord::Paste => (&[hk::KEY_LEFTCTRL], KEY_V),
        Chord::PasteTerminal => (&[hk::KEY_LEFTCTRL, hk::KEY_LEFTSHIFT], KEY_V),
        Chord::PasteInsert => (&[hk::KEY_LEFTSHIFT], KEY_INSERT),
        Chord::CopyInsert => (&[hk::KEY_LEFTCTRL], KEY_INSERT),
    }
}

fn xdotool_name(c: Chord) -> &'static str {
    match c {
        Chord::Paste => "ctrl+v",
        Chord::PasteTerminal => "ctrl+shift+v",
        Chord::PasteInsert => "shift+Insert",
        Chord::CopyInsert => "ctrl+Insert",
    }
}

fn wtype_args(c: Chord) -> &'static [&'static str] {
    match c {
        Chord::Paste => &["-M", "ctrl", "-k", "v", "-m", "ctrl"],
        Chord::PasteTerminal => &["-M", "ctrl", "-M", "shift", "-k", "v", "-m", "shift", "-m", "ctrl"],
        Chord::PasteInsert => &["-M", "shift", "-k", "Insert", "-m", "shift"],
        Chord::CopyInsert => &["-M", "ctrl", "-k", "Insert", "-m", "ctrl"],
    }
}

/// Does this window class belong to a terminal emulator?
pub fn is_terminal_class(class: &str) -> bool {
    const TERMINALS: &[&str] = &[
        "terminal", "konsole", "alacritty", "kitty", "xterm", "urxvt", "rxvt", "tilix",
        "terminator", "wezterm", "foot", "st-256color", "guake", "yakuake", "ptyxis", "blackbox",
        "lxterminal", "terminology", "hyper", "tabby", "contour", "rio",
    ];
    let c = class.to_lowercase();
    TERMINALS.iter().any(|t| c.contains(t))
}

/// The chord that pastes into the app in front: Ctrl+V, Ctrl+Shift+V in a
/// terminal, and Shift+Insert when the app is unknown (Wayland).
fn paste_chord() -> Chord {
    match frontmost_app() {
        Some(class) if is_terminal_class(&class) => Chord::PasteTerminal,
        Some(_) => Chord::Paste,
        None => Chord::PasteInsert,
    }
}

static VIRTUAL_KEYBOARD: Mutex<Option<evdev::uinput::VirtualDevice>> = Mutex::new(None);

fn send_via_uinput(c: Chord) -> bool {
    let (mods, key) = chord_keys(c);
    let mut guard = VIRTUAL_KEYBOARD.lock().unwrap();
    if guard.is_none() {
        let mut keys = evdev::AttributeSet::<evdev::KeyCode>::new();
        for code in [hk::KEY_LEFTCTRL, hk::KEY_LEFTSHIFT, KEY_V, KEY_C, KEY_INSERT] {
            keys.insert(evdev::KeyCode::new(code));
        }
        let built = evdev::uinput::VirtualDevice::builder()
            .and_then(|b| b.name("Flowline virtual keyboard").with_keys(&keys))
            .and_then(|b| b.build());
        match built {
            Ok(dev) => {
                *guard = Some(dev);
                // The desktop needs a moment to notice a new keyboard.
                std::thread::sleep(Duration::from_millis(300));
            }
            Err(_) => return false,
        }
    }
    let dev = guard.as_mut().unwrap();
    let ev = |code: u16, value: i32| {
        evdev::InputEvent::new(evdev::EventType::KEY.0, code, value)
    };
    let sync = evdev::InputEvent::new(evdev::EventType::SYNCHRONIZATION.0, 0, 0);
    let pause = || std::thread::sleep(Duration::from_millis(12));
    let mut ok = true;
    for m in mods {
        ok &= dev.emit(&[ev(*m, 1), sync]).is_ok();
        pause();
    }
    ok &= dev.emit(&[ev(key, 1), sync]).is_ok();
    pause();
    ok &= dev.emit(&[ev(key, 0), sync]).is_ok();
    pause();
    for m in mods.iter().rev() {
        ok &= dev.emit(&[ev(*m, 0), sync]).is_ok();
        pause();
    }
    ok
}

fn send_via_tool(c: Chord) -> bool {
    let run = |cmd: &mut std::process::Command| {
        cmd.stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    };
    if on_path("xdotool")
        && run(std::process::Command::new("xdotool").args(["key", "--clearmodifiers", xdotool_name(c)]))
    {
        return true;
    }
    on_path("wtype") && run(std::process::Command::new("wtype").args(wtype_args(c)))
}

/// Press a chord in whatever app has focus. Waits for the person to let go of
/// any modifier keys first, so they do not join the chord.
fn send_chord(c: Chord) -> bool {
    let deadline = Instant::now() + Duration::from_millis(1500);
    while MODS_HELD.load(Ordering::Relaxed) != 0 && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(20));
    }
    send_via_uinput(c) || send_via_tool(c)
}

// ---------------------------------------------------------------------------
// Clipboard
// ---------------------------------------------------------------------------

static CLIPBOARD: Mutex<Option<arboard::Clipboard>> = Mutex::new(None);

/// One long-lived clipboard: on X11 the data is served by this process, so it
/// must outlive the copy.
fn with_clipboard<R>(f: impl FnOnce(&mut arboard::Clipboard) -> R) -> Option<R> {
    let mut guard = CLIPBOARD.lock().ok()?;
    if guard.is_none() {
        *guard = arboard::Clipboard::new().ok();
    }
    guard.as_mut().map(f)
}

fn pasteboard_string() -> Option<String> {
    with_clipboard(|c| c.get_text().ok()).flatten()
}

/// Put `text` on the clipboard, and on the primary selection too when asked
/// (Shift+Insert pastes the primary selection in some apps).
fn set_pasteboard(text: &str, also_primary: bool) {
    with_clipboard(|c| {
        let _ = c.set_text(text.to_string());
        if also_primary {
            use arboard::SetExtLinux;
            let _ = c
                .set()
                .clipboard(arboard::LinuxClipboardKind::Primary)
                .text(text.to_string());
        }
    });
}

fn put_back(old: Option<String>) {
    match old {
        Some(old) => set_pasteboard(&old, false),
        None => {
            with_clipboard(|c| {
                let _ = c.clear();
            });
        }
    }
}

pub fn clipboard_available() -> bool {
    with_clipboard(|_| ()).is_some()
}

pub fn copy_text(text: &str) {
    set_pasteboard(text, false);
}

pub struct ClipboardRestore {
    old: Option<String>,
    written: String,
}

/// Write the transcript, paste it into the app in front, and remember the old
/// clipboard so it can be restored once the paste has landed.
pub fn paste_transcript(text: &str) -> ClipboardRestore {
    let old = pasteboard_string();
    let chord = paste_chord();
    set_pasteboard(text, chord == Chord::PasteInsert);
    send_chord(chord);
    ClipboardRestore { old, written: text.to_string() }
}

/// Restore the clipboard unless the person has copied something new meanwhile.
pub fn restore_clipboard(restore: ClipboardRestore) {
    if pasteboard_string().as_deref() == Some(restore.written.as_str()) {
        put_back(restore.old);
    }
}

/// The clipboard as it was before Transforms borrowed it.
pub struct SavedClipboard {
    old: Option<String>,
}

/// Copy the selected text with Ctrl+Insert and return it. `None` when nothing
/// was selected. A one-off marker on the clipboard tells "copied the same text
/// again" apart from "nothing was copied". Blocks up to half a second.
pub fn copy_selection() -> Option<(String, SavedClipboard)> {
    let old = pasteboard_string();
    let marker = format!(
        "\u{1}flowline-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    );
    set_pasteboard(&marker, false);
    if !send_chord(Chord::CopyInsert) {
        put_back(old);
        return None;
    }
    for _ in 0..25 {
        std::thread::sleep(Duration::from_millis(20));
        match pasteboard_string() {
            Some(text) if text != marker => {
                if text.trim().is_empty() {
                    put_back(old);
                    return None;
                }
                return Some((text, SavedClipboard { old }));
            }
            _ => {}
        }
    }
    put_back(old);
    None
}

/// Give the clipboard back without pasting (the transform failed).
pub fn restore_saved(saved: SavedClipboard) {
    put_back(saved.old);
}

/// Paste `text` over the selection, then put the person's clipboard back unless
/// they copied something new in the meantime.
pub fn paste_and_restore(text: &str, saved: SavedClipboard) {
    let chord = paste_chord();
    set_pasteboard(text, chord == Chord::PasteInsert);
    send_chord(chord);
    std::thread::sleep(Duration::from_millis(900));
    if pasteboard_string().as_deref() == Some(text) {
        put_back(saved.old);
    }
}

// ---------------------------------------------------------------------------
// The window in front (X11 only)
// ---------------------------------------------------------------------------

/// Window class and title of the focused window, on X11 sessions. Wayland does
/// not tell apps which window has focus, so this is `None` there rather than a
/// stale or wrong answer.
fn active_window() -> Option<(String, String)> {
    use x11rb::connection::Connection;
    use x11rb::protocol::xproto::{AtomEnum, ConnectionExt};

    if std::env::var_os("DISPLAY").is_none()
        || std::env::var("XDG_SESSION_TYPE").map_or(false, |t| t == "wayland")
    {
        return None;
    }
    let (conn, screen) = x11rb::rust_connection::RustConnection::connect(None).ok()?;
    let root = conn.setup().roots[screen].root;
    let atom = |name: &str| -> Option<u32> {
        conn.intern_atom(false, name.as_bytes()).ok()?.reply().ok().map(|r| r.atom)
    };
    let active = conn
        .get_property(false, root, atom("_NET_ACTIVE_WINDOW")?, AtomEnum::WINDOW, 0, 1)
        .ok()?
        .reply()
        .ok()?;
    let window = active.value32()?.next().filter(|w| *w != 0)?;

    let class_reply = conn
        .get_property(false, window, AtomEnum::WM_CLASS, AtomEnum::STRING, 0, 256)
        .ok()?
        .reply()
        .ok()?;
    let parts: Vec<&[u8]> = class_reply.value.split(|b| *b == 0).filter(|p| !p.is_empty()).collect();
    let class = String::from_utf8_lossy(parts.get(1).or(parts.first())?).into_owned();

    let title = (|| {
        let reply = conn
            .get_property(false, window, atom("_NET_WM_NAME")?, atom("UTF8_STRING")?, 0, 512)
            .ok()?
            .reply()
            .ok()?;
        Some(String::from_utf8_lossy(&reply.value).into_owned())
    })()
    .unwrap_or_default();
    Some((class, title))
}

pub fn frontmost_app() -> Option<String> {
    active_window().map(|(class, _)| class)
}

pub fn front_window_title() -> Option<String> {
    active_window().map(|(_, title)| title).filter(|t| !t.is_empty())
}

pub fn is_flowline_frontmost() -> bool {
    frontmost_app().map_or(false, |c| c.eq_ignore_ascii_case("flowline"))
}

/// Reading another app's text field needs the AT-SPI accessibility bus, which
/// Flowline does not use on Linux, so these report "unknown".
pub fn focused_text_input() -> Option<bool> {
    None
}

pub fn read_focused_text() -> Option<String> {
    None
}

// ---------------------------------------------------------------------------
// Window and session
// ---------------------------------------------------------------------------

pub fn set_dock_visible(_visible: bool) {}
pub fn activate_app() {}
pub fn hide_app() {}
pub fn dismiss_search_overlay() {}
pub fn promote_overlay_window() {}

fn autostart_path() -> Option<PathBuf> {
    let config = std::env::var_os("XDG_CONFIG_HOME")
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))?;
    Some(config.join("autostart/flowline.desktop"))
}

pub fn launch_agent_installed() -> bool {
    autostart_path().map_or(false, |p| p.is_file())
}

/// The autostart entry, as the XDG autostart spec describes it.
fn desktop_entry(exe: &Path) -> String {
    let exe = exe.to_string_lossy().replace('\\', "\\\\").replace('"', "\\\"");
    format!(
        "[Desktop Entry]\nType=Application\nName=Flowline\nComment=Hold-to-talk dictation\nExec=\"{exe}\" --background\nTerminal=false\nX-GNOME-Autostart-enabled=true\n"
    )
}

pub fn set_launch_at_login(enabled: bool) -> Result<(), String> {
    let path = autostart_path().ok_or_else(|| "no home folder".to_string())?;
    if !enabled {
        let _ = std::fs::remove_file(&path);
        return Ok(());
    }
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    std::fs::write(&path, desktop_entry(&exe)).map_err(|e| e.to_string())
}

// ---------------------------------------------------------------------------
// One running copy: launching again shows the window
// ---------------------------------------------------------------------------

fn socket_path() -> Option<PathBuf> {
    std::env::var_os("XDG_RUNTIME_DIR")
        .filter(|v| !v.is_empty())
        .map(|d| PathBuf::from(d).join("flowline.sock"))
        .or_else(|| crate::paths::data_dir().map(|d| d.join("ipc.sock")))
}

/// With no tray to click, a second launch is how the person reopens the window:
/// tell the running copy to show itself. Returns whether one answered.
pub fn signal_running_instance() -> bool {
    let Some(path) = socket_path() else { return false };
    match std::os::unix::net::UnixStream::connect(path) {
        Ok(mut s) => s.write_all(b"show\n").is_ok(),
        Err(_) => false,
    }
}

/// Listen for those requests. Called where macOS installs its menu-bar item.
pub fn install_status_item(ctx: eframe::egui::Context) {
    *WAKE.lock().unwrap() = Some(ctx);
    let Some(path) = socket_path() else { return };
    let _ = std::fs::remove_file(&path);
    let Ok(listener) = std::os::unix::net::UnixListener::bind(&path) else { return };
    std::thread::Builder::new()
        .name("flowline-ipc".into())
        .spawn(move || {
            for stream in listener.incoming().flatten() {
                let mut stream = stream;
                let mut buf = [0u8; 16];
                let n = stream.read(&mut buf).unwrap_or(0);
                if buf[..n].starts_with(b"show") {
                    SHOW_REQUESTED.store(true, Ordering::SeqCst);
                    wake();
                }
            }
        })
        .ok();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn terminals_are_recognised_by_window_class() {
        for t in ["Gnome-terminal", "konsole", "Alacritty", "kitty", "XTerm", "org.wezterm", "foot"] {
            assert!(is_terminal_class(t), "{t}");
        }
        for t in ["firefox", "Slack", "thunderbird", "code", "Google-chrome"] {
            assert!(!is_terminal_class(t), "{t}");
        }
    }

    #[test]
    fn copy_and_paste_use_chords_that_never_interrupt_a_terminal() {
        // Ctrl+C would send SIGINT to a program running in a terminal.
        assert_ne!(chord_keys(Chord::CopyInsert).1, KEY_C);
        assert_eq!(chord_keys(Chord::Paste).1, KEY_V);
        assert_eq!(chord_keys(Chord::PasteTerminal).0, &[hk::KEY_LEFTCTRL, hk::KEY_LEFTSHIFT]);
    }

    #[test]
    fn autostart_entry_quotes_the_path() {
        let entry = desktop_entry(Path::new("/opt/Flowline App/flowline"));
        assert!(entry.contains("Exec=\"/opt/Flowline App/flowline\" --background"));
        assert!(entry.starts_with("[Desktop Entry]\n"));
    }

    #[test]
    fn setup_command_grants_both_keyboard_and_paste_access() {
        let c = setup_command();
        assert!(c.contains("usermod -aG input"));
        assert!(c.contains("/dev/uinput") || c.contains("KERNEL==\"uinput\""));
        assert!(c.contains("udevadm control --reload"));
    }

    #[test]
    fn clipboard_round_trips_when_a_display_exists() {
        // Needs an X server or a Wayland compositor; elsewhere there is nothing to test.
        if std::env::var_os("DISPLAY").is_none() && std::env::var_os("WAYLAND_DISPLAY").is_none() {
            return;
        }
        assert!(clipboard_available());
        let old = pasteboard_string();
        copy_text("flowline clipboard test ✓");
        assert_eq!(pasteboard_string().as_deref(), Some("flowline clipboard test ✓"));
        // Transforms borrows the clipboard and must give it back.
        let saved = SavedClipboard { old: Some("the person's own text".into()) };
        restore_saved(saved);
        assert_eq!(pasteboard_string().as_deref(), Some("the person's own text"));
        put_back(old);
    }

    #[test]
    fn a_second_launch_can_ask_the_first_to_show_its_window() {
        let dir = std::env::temp_dir().join(format!("flowline-ipc-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        // Safe here: this is the only test that touches XDG_RUNTIME_DIR.
        std::env::set_var("XDG_RUNTIME_DIR", &dir);
        assert!(!signal_running_instance(), "nobody is listening yet");
        install_status_item(eframe::egui::Context::default());
        std::thread::sleep(Duration::from_millis(150));
        assert!(signal_running_instance());
        let end = Instant::now() + Duration::from_secs(2);
        while !take_show_request() {
            assert!(Instant::now() < end, "the running copy never got the request");
            std::thread::sleep(Duration::from_millis(20));
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn modifiers_do_not_linger_in_the_cache_of_a_fresh_start() {
        assert_eq!(MODS_HELD.load(Ordering::Relaxed), 0);
    }
}

/// End-to-end tests against the real kernel input layer. They create virtual
/// keyboards through `/dev/uinput`, so they are ignored unless asked for:
///
/// ```sh
/// docker run --rm --privileged -v /dev:/dev -v "$PWD":/src \
///   -v flowline-cargo-registry:/usr/local/cargo/registry \
///   -v flowline-linux-target:/target -e CARGO_TARGET_DIR=/target \
///   flowline-linux-build cargo test --release loopback -- --ignored --nocapture
/// ```
#[cfg(test)]
mod loopback {
    use super::*;
    use evdev::{uinput::VirtualDevice, AttributeSet, EventType, InputEvent, KeyCode};

    fn key(code: u16, value: i32) -> [InputEvent; 2] {
        [
            InputEvent::new(EventType::KEY.0, code, value),
            InputEvent::new(EventType::SYNCHRONIZATION.0, 0, 0),
        ]
    }

    fn virtual_keyboard(name: &str) -> VirtualDevice {
        let mut keys = AttributeSet::<KeyCode>::new();
        for code in [1u16, 2, 3, 28, 29, 30, 42, 46, 56, 57, 97, 100] {
            keys.insert(KeyCode::new(code));
        }
        VirtualDevice::builder()
            .unwrap()
            .name(name)
            .with_keys(&keys)
            .unwrap()
            .build()
            .expect("could not create a virtual keyboard: is /dev/uinput writable?")
    }

    fn wait_until(what: &str, secs: u64, mut done: impl FnMut() -> bool) {
        let end = Instant::now() + Duration::from_secs(secs);
        while !done() {
            assert!(Instant::now() < end, "timed out waiting for {what}");
            std::thread::sleep(Duration::from_millis(50));
        }
    }

    #[test]
    #[ignore = "needs /dev/uinput and /dev/input (run in a privileged container)"]
    fn kernel_keyboard_drives_the_hotkey_monitor() {
        let mut kb = virtual_keyboard("Loopback test keyboard");
        set_hotkey_mode("Right Ctrl");
        ensure_hotkey_monitor(eframe::egui::Context::default());
        // The monitor looks for new keyboards every few seconds.
        wait_until("the monitor to find the virtual keyboard", 15, hotkey_active);
        take_events();

        // Right Ctrl held alone: starts after its hold time, stops on release.
        kb.emit(&key(97, 1)).unwrap();
        std::thread::sleep(Duration::from_millis(400));
        kb.emit(&key(97, 0)).unwrap();
        wait_until("Start and Stop", 3, || take_events_peek() >= 2);
        assert_eq!(take_events(), vec![HotkeyEvent::Start, HotkeyEvent::Stop]);

        // Right Ctrl in a chord (Right Ctrl + C) never dictates.
        kb.emit(&key(97, 1)).unwrap();
        kb.emit(&key(46, 1)).unwrap();
        std::thread::sleep(Duration::from_millis(400));
        kb.emit(&key(46, 0)).unwrap();
        kb.emit(&key(97, 0)).unwrap();
        std::thread::sleep(Duration::from_millis(300));
        assert_eq!(take_events(), vec![], "a chord must not start dictation");

        // Esc cancels.
        kb.emit(&key(1, 1)).unwrap();
        kb.emit(&key(1, 0)).unwrap();
        wait_until("Cancel", 3, || take_events_peek() >= 1);
        assert_eq!(take_events(), vec![HotkeyEvent::Cancel]);
    }

    /// How many events are waiting, without taking them.
    fn take_events_peek() -> usize {
        EVENTS.lock().map(|q| q.len()).unwrap_or(0)
    }

    #[test]
    #[ignore = "needs /dev/uinput and /dev/input (run in a privileged container)"]
    fn paste_chord_reaches_the_kernel_as_ctrl_v() {
        // The first send creates Flowline's virtual keyboard.
        assert!(send_via_uinput(Chord::Paste), "uinput paste failed");
        let node = std::fs::read_dir("/dev/input")
            .unwrap()
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.to_string_lossy().contains("event"))
            .find_map(|p| {
                let d = evdev::Device::open(&p).ok()?;
                (d.name() == Some("Flowline virtual keyboard")).then_some(p)
            })
            .expect("Flowline's virtual keyboard did not appear in /dev/input");
        let mut reader = evdev::Device::open(node).unwrap();
        reader.set_nonblocking(true).unwrap();

        let mut seen: Vec<(u16, i32)> = Vec::new();
        assert!(send_via_uinput(Chord::Paste));
        let end = Instant::now() + Duration::from_secs(3);
        while seen.len() < 4 && Instant::now() < end {
            if let Ok(events) = reader.fetch_events() {
                seen.extend(
                    events
                        .filter(|e| e.event_type() == EventType::KEY)
                        .map(|e| (e.code(), e.value())),
                );
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        // Ctrl down, V down, V up, Ctrl up.
        assert_eq!(seen, vec![(29, 1), (47, 1), (47, 0), (29, 0)]);

        // The terminal chord adds Shift; copy uses Ctrl+Insert, never Ctrl+C.
        seen.clear();
        assert!(send_via_uinput(Chord::CopyInsert));
        let end = Instant::now() + Duration::from_secs(3);
        while seen.len() < 4 && Instant::now() < end {
            if let Ok(events) = reader.fetch_events() {
                seen.extend(
                    events
                        .filter(|e| e.event_type() == EventType::KEY)
                        .map(|e| (e.code(), e.value())),
                );
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        assert_eq!(seen, vec![(29, 1), (110, 1), (110, 0), (29, 0)]);
    }
}
