//! macOS system glue, written against the frameworks directly so the app keeps
//! its tiny dependency list.
//!
//! Three separate permissions are involved and macOS treats them separately:
//!
//! * **Microphone** — granted the first time `audio.rs` opens the input queue.
//! * **Input Monitoring** (`CGPreflightListenEventAccess`) — optional. Only the
//!   listen-only fallback tap and the extra HID Fn source use it, and neither
//!   is started unless it is already granted; Flowline never prompts for it.
//! * **Accessibility** (`AXIsProcessTrusted`) — needed to post ⌘V into another
//!   app. Grabbing the clipboard alone does not require it.

use std::ffi::{c_char, c_void, CString};
use std::sync::atomic::{AtomicBool, AtomicI32, AtomicPtr, Ordering};

use super::shared::*;

type Id = *mut c_void;

// ---------------------------------------------------------------------------
// Framework declarations
// ---------------------------------------------------------------------------

#[link(name = "ApplicationServices", kind = "framework")]
#[link(name = "CoreGraphics", kind = "framework")]
#[link(name = "CoreFoundation", kind = "framework")]
#[link(name = "AppKit", kind = "framework")]
#[link(name = "AVFoundation", kind = "framework")]
#[link(name = "IOKit", kind = "framework")]
extern "C" {
    // Permissions.
    fn AXIsProcessTrusted() -> bool;
    fn AXIsProcessTrustedWithOptions(options: *const c_void) -> bool;
    fn AXUIElementCreateSystemWide() -> Id;
    fn AXUIElementCopyAttributeValue(element: Id, attribute: *const c_void, value: *mut Id) -> i32;
    fn CFBooleanGetValue(value: *const c_void) -> bool;
    fn AXUIElementSetMessagingTimeout(element: Id, seconds: f32) -> i32;
    fn CFGetTypeID(cf: *const c_void) -> usize;
    fn CFStringGetTypeID() -> usize;
    static kAXTrustedCheckOptionPrompt: *const c_void;
    fn CGPreflightListenEventAccess() -> bool;
    fn CGRequestListenEventAccess() -> bool;

    // Core Foundation.
    fn CFStringCreateWithBytes(
        alloc: *const c_void,
        bytes: *const u8,
        len: isize,
        encoding: u32,
        external: bool,
    ) -> *mut c_void;
    fn CFDictionaryCreate(
        alloc: *const c_void,
        keys: *const *const c_void,
        values: *const *const c_void,
        count: isize,
        key_callbacks: *const c_void,
        value_callbacks: *const c_void,
    ) -> *mut c_void;
    fn CFRelease(cf: *mut c_void);
    fn CFArrayGetCount(array: *const c_void) -> isize;
    fn CFArrayGetValueAtIndex(array: *const c_void, index: isize) -> *const c_void;
    fn CFDictionaryGetValue(dict: *const c_void, key: *const c_void) -> *const c_void;
    fn CGWindowListCopyWindowInfo(option: u32, relative: u32) -> *mut c_void;
    static kCGWindowOwnerName: *const c_void;
    static kCFBooleanTrue: *const c_void;
    static kCFTypeDictionaryKeyCallBacks: c_void;
    static kCFTypeDictionaryValueCallBacks: c_void;
    static kCFRunLoopCommonModes: *const c_void;

    // Event tap.
    fn CGEventTapCreate(
        tap: u32,
        place: u32,
        options: u32,
        events_of_interest: u64,
        callback: CGEventTapCallback,
        user_info: *mut c_void,
    ) -> *mut c_void;
    fn CGEventTapEnable(tap: *mut c_void, enable: bool);
    fn CGEventGetFlags(event: *mut c_void) -> u64;
    fn CGEventGetIntegerValueField(event: *mut c_void, field: u32) -> i64;
    fn CGEventCreateKeyboardEvent(source: *mut c_void, key: u16, down: bool) -> *mut c_void;
    fn CGEventSetFlags(event: *mut c_void, flags: u64);
    fn CGEventPost(tap: u32, event: *mut c_void);
    fn CFMachPortCreateRunLoopSource(
        alloc: *const c_void,
        port: *mut c_void,
        order: isize,
    ) -> *mut c_void;
    fn CFRunLoopGetCurrent() -> *mut c_void;
    fn CFRunLoopAddSource(run_loop: *mut c_void, source: *mut c_void, mode: *const c_void);
    fn CFRunLoopRun();

    // Objective-C runtime.
    fn objc_getClass(name: *const c_char) -> Id;
    fn sel_registerName(name: *const c_char) -> *mut c_void;
    fn objc_msgSend();
    fn objc_allocateClassPair(superclass: Id, name: *const c_char, extra: usize) -> Id;
    fn objc_registerClassPair(class: Id);
    fn class_addMethod(class: Id, sel: *mut c_void, imp: *const c_void, types: *const c_char)
        -> bool;
    fn object_setClass(obj: Id, class: Id) -> Id;

    static NSPasteboardTypeString: *mut c_void;

    // Microphone permission (AVFoundation).
    static AVMediaTypeAudio: *mut c_void;

    // IOHID keyboard service: a device-level source for the Fn key.
    fn IOHIDManagerCreate(allocator: *const c_void, options: u32) -> *mut c_void;
    fn IOHIDManagerSetDeviceMatching(manager: *mut c_void, matching: *const c_void);
    fn IOHIDManagerRegisterInputValueCallback(
        manager: *mut c_void,
        callback: IOHIDValueCallback,
        context: *mut c_void,
    );
    fn IOHIDManagerScheduleWithRunLoop(
        manager: *mut c_void,
        run_loop: *mut c_void,
        mode: *const c_void,
    );
    fn IOHIDManagerOpen(manager: *mut c_void, options: u32) -> i32;
    fn IOHIDValueGetElement(value: *mut c_void) -> *mut c_void;
    fn IOHIDElementGetUsagePage(element: *mut c_void) -> u32;
    fn IOHIDElementGetUsage(element: *mut c_void) -> u32;
    fn IOHIDValueGetIntegerValue(value: *mut c_void) -> isize;
    fn CFNumberCreate(allocator: *const c_void, kind: isize, value: *const c_void)
        -> *mut c_void;
    fn CFDictionaryCreateMutable(
        allocator: *const c_void,
        capacity: isize,
        key_callbacks: *const c_void,
        value_callbacks: *const c_void,
    ) -> *mut c_void;
    fn CFDictionarySetValue(dict: *mut c_void, key: *const c_void, value: *const c_void);
}

type IOHIDValueCallback = extern "C" fn(
    context: *mut c_void,
    result: i32,
    sender: *mut c_void,
    value: *mut c_void,
);

type CGEventTapCallback =
    extern "C" fn(proxy: *mut c_void, kind: u32, event: *mut c_void, user: *mut c_void) -> *mut c_void;

const FLAGS_CHANGED: u32 = 12;
const KEY_DOWN: u32 = 10;
const KEY_UP: u32 = 11;
const TAP_DISABLED_TIMEOUT: u32 = 0xFFFF_FFFE;
const TAP_DISABLED_USER: u32 = 0xFFFF_FFFF;

const FN_FLAG: u64 = 0x0080_0000;
const CTRL_FLAG: u64 = 0x0004_0000;
const CMD_FLAG: u64 = 0x0010_0000;
const SHIFT_FLAG: u64 = 0x0002_0000;
const OPTION_FLAG: u64 = 0x0008_0000;

const KEY_SPACE: i64 = 49;
/// ANSI `V`, pressed with ⌘ to paste. (⌘ + Space would open Spotlight.)
const KEY_V: i64 = 9;
const KEY_ESCAPE: i64 = 53;
/// ANSI `C`, pressed with ⌘ to copy the current selection.
const KEY_C: i64 = 8;

const CF_STRING_ENCODING_UTF8: u32 = 0x0800_0100;

// ---------------------------------------------------------------------------
// Small Objective-C helpers
// ---------------------------------------------------------------------------

unsafe fn sel(name: &str) -> *mut c_void {
    let c = CString::new(name).unwrap();
    sel_registerName(c.as_ptr())
}

unsafe fn msg0(obj: Id, name: &str) -> Id {
    let f: extern "C" fn(Id, *mut c_void) -> Id = std::mem::transmute(objc_msgSend as *const ());
    f(obj, sel(name))
}

unsafe fn msg1(obj: Id, name: &str, a: Id) -> Id {
    let f: extern "C" fn(Id, *mut c_void, Id) -> Id = std::mem::transmute(objc_msgSend as *const ());
    f(obj, sel(name), a)
}

unsafe fn msg2(obj: Id, name: &str, a: Id, b: Id) -> Id {
    let f: extern "C" fn(Id, *mut c_void, Id, Id) -> Id =
        std::mem::transmute(objc_msgSend as *const ());
    f(obj, sel(name), a, b)
}

unsafe fn msg3(obj: Id, name: &str, a: Id, b: Id, c: Id) -> Id {
    let f: extern "C" fn(Id, *mut c_void, Id, Id, Id) -> Id =
        std::mem::transmute(objc_msgSend as *const ());
    f(obj, sel(name), a, b, c)
}

unsafe fn msg_f64(obj: Id, name: &str, a: f64) -> Id {
    let f: extern "C" fn(Id, *mut c_void, f64) -> Id = std::mem::transmute(objc_msgSend as *const ());
    f(obj, sel(name), a)
}

unsafe fn msg_i64(obj: Id, name: &str, a: i64) {
    let f: extern "C" fn(Id, *mut c_void, i64) = std::mem::transmute(objc_msgSend as *const ());
    f(obj, sel(name), a)
}

unsafe fn msg_bool(obj: Id, name: &str, a: bool) {
    let f: extern "C" fn(Id, *mut c_void, bool) = std::mem::transmute(objc_msgSend as *const ());
    f(obj, sel(name), a)
}

unsafe fn msg_take_i64(obj: Id, name: &str) -> i64 {
    let f: extern "C" fn(Id, *mut c_void) -> i64 = std::mem::transmute(objc_msgSend as *const ());
    f(obj, sel(name))
}

unsafe fn nsstring(s: &str) -> Id {
    CFStringCreateWithBytes(
        std::ptr::null(),
        s.as_ptr(),
        s.len() as isize,
        CF_STRING_ENCODING_UTF8,
        false,
    )
}

unsafe fn nsstring_to_string(s: Id) -> Option<String> {
    if s.is_null() {
        return None;
    }
    let utf8 = msg0(s, "UTF8String") as *const c_char;
    if utf8.is_null() {
        return None;
    }
    Some(
        std::ffi::CStr::from_ptr(utf8)
            .to_string_lossy()
            .into_owned(),
    )
}

// ---------------------------------------------------------------------------
// Permissions
// ---------------------------------------------------------------------------

pub fn accessibility_granted() -> bool {
    unsafe { AXIsProcessTrusted() }
}

/// Ask macOS to add the app to Privacy → Accessibility. Returns the current
/// state; the person still has to flip the switch, so this is usually false.
pub fn request_accessibility() -> bool {
    unsafe {
        let key = kAXTrustedCheckOptionPrompt as *const c_void;
        let value = kCFBooleanTrue as *const c_void;
        let keys = [key];
        let values = [value];
        let dict = CFDictionaryCreate(
            std::ptr::null(),
            keys.as_ptr(),
            values.as_ptr(),
            1,
            &kCFTypeDictionaryKeyCallBacks as *const c_void,
            &kCFTypeDictionaryValueCallBacks as *const c_void,
        );
        let trusted = if dict.is_null() {
            AXIsProcessTrusted()
        } else {
            let t = AXIsProcessTrustedWithOptions(dict);
            CFRelease(dict);
            t
        };
        trusted
    }
}

/// 3 = AVAuthorizationStatusAuthorized.
pub fn mic_authorized() -> bool {
    mic_status() == 3
}

/// 0 not determined, 1 restricted, 2 denied, 3 authorized.
pub fn mic_status() -> i64 {
    unsafe {
        let cls = objc_getClass(c"AVCaptureDevice".as_ptr());
        let f: extern "C" fn(Id, *mut c_void, Id) -> i64 =
            std::mem::transmute(objc_msgSend as *const ());
        f(cls, sel("authorizationStatusForMediaType:"), AVMediaTypeAudio)
    }
}

/// Quit and reopen the bundle. TCC sometimes only applies a freshly granted
/// permission to a new process, especially Input Monitoring.
pub fn restart_bundle() {
    let bundle = std::env::current_exe().ok().and_then(|exe| {
        // …/Flowline.app/Contents/MacOS/flowline
        exe.ancestors()
            .find(|p| p.extension().map(|e| e == "app").unwrap_or(false))
            .map(|p| p.to_path_buf())
    });
    let target = bundle
        .map(|p| p.display().to_string())
        .unwrap_or_else(|| "/Applications/Flowline.app".into());
    let _ = std::process::Command::new("sh")
        .arg("-c")
        .arg(format!("sleep 1; open -a '{}'", target.replace('\'', "'\\''")))
        .spawn();
    std::process::exit(0);
}

pub fn input_monitoring_granted() -> bool {
    unsafe { CGPreflightListenEventAccess() }
}

/// Shows the system prompt for Input Monitoring. Must be called before a tap
/// can be created; macOS only shows it once per bundle identity.
pub fn request_input_monitoring() -> bool {
    unsafe { CGRequestListenEventAccess() }
}

pub fn open_privacy_pane(anchor: &str) {
    let url = format!(
        "x-apple.systempreferences:com.apple.preference.security?Privacy_{anchor}"
    );
    let _ = std::process::Command::new("open").arg(url).spawn();
}

// ---------------------------------------------------------------------------
// Clipboard and text insertion
// ---------------------------------------------------------------------------

fn pasteboard() -> Id {
    unsafe { msg0(objc_getClass(c"NSPasteboard".as_ptr()), "generalPasteboard") }
}

unsafe fn pasteboard_string() -> Option<String> {
    let pb = pasteboard();
    nsstring_to_string(msg1(pb, "stringForType:", NSPasteboardTypeString))
}

/// The pieces needed to put the previous clipboard back after the paste.
pub struct ClipboardRestore {
    old: Option<String>,
    count: i64,
    written: String,
}

fn set_pasteboard(text: &str) {
    unsafe {
        let pb = pasteboard();
        msg0(pb, "clearContents");
        let s = nsstring(text);
        msg2(pb, "setString:forType:", s, NSPasteboardTypeString);
        CFRelease(s);
    }
}

/// Write the transcript, press ⌘V into whatever is frontmost, and remember the
/// old clipboard so it can be restored once the paste lands.
pub fn clipboard_available() -> bool {
    !pasteboard().is_null()
}

/// Put text on the clipboard without pressing anything. Used when
/// Accessibility is missing: the transcript is still one ⌘V away.
pub fn copy_text(text: &str) {
    set_pasteboard(text);
}

pub fn paste_transcript(text: &str) -> ClipboardRestore {
    unsafe {
        let old = pasteboard_string();
        set_pasteboard(text);
        let count = msg_take_i64(pasteboard(), "changeCount");
        post_key(KEY_V as u16, CMD_FLAG);
        ClipboardRestore {
            old,
            count,
            written: text.to_string(),
        }
    }
}

/// Restore the clipboard if the person has not copied something new in the
/// meantime.
pub fn restore_clipboard(restore: ClipboardRestore) {
    unsafe {
        let pb = pasteboard();
        let count = msg_take_i64(pb, "changeCount");
        if count != restore.count {
            return;
        }
        if pasteboard_string().as_deref() != Some(restore.written.as_str()) {
            return;
        }
        match &restore.old {
            Some(old) => set_pasteboard(old),
            None => {
                msg0(pb, "clearContents");
            }
        }
    }
}

/// The clipboard as it was before Transforms borrowed it.
pub struct SavedClipboard {
    old: Option<String>,
}

/// Press ⌘C and return the selected text. `None` when nothing was selected
/// (the clipboard did not change). Blocks for up to half a second, so call it
/// off the UI thread.
pub fn copy_selection() -> Option<(String, SavedClipboard)> {
    unsafe {
        let old = pasteboard_string();
        let before = msg_take_i64(pasteboard(), "changeCount");
        post_key(KEY_C as u16, CMD_FLAG);
        for _ in 0..25 {
            std::thread::sleep(std::time::Duration::from_millis(20));
            if msg_take_i64(pasteboard(), "changeCount") != before {
                let text = pasteboard_string()?;
                if text.trim().is_empty() {
                    put_back(old);
                    return None;
                }
                return Some((text, SavedClipboard { old }));
            }
        }
        None
    }
}

fn put_back(old: Option<String>) {
    match old {
        Some(old) => set_pasteboard(&old),
        None => unsafe {
            msg0(pasteboard(), "clearContents");
        },
    }
}

/// Give the clipboard back without pasting (the transform failed).
pub fn restore_saved(saved: SavedClipboard) {
    put_back(saved.old);
}

/// Paste `text` over the selection with ⌘V, then put the person's own
/// clipboard back unless they copied something new in the meantime.
pub fn paste_and_restore(text: &str, saved: SavedClipboard) {
    unsafe {
        set_pasteboard(text);
        let count = msg_take_i64(pasteboard(), "changeCount");
        post_key(KEY_V as u16, CMD_FLAG);
        std::thread::sleep(std::time::Duration::from_millis(900));
        if msg_take_i64(pasteboard(), "changeCount") == count {
            put_back(saved.old);
        }
    }
}

fn post_key(key: u16, flags: u64) {
    unsafe {
        let down = CGEventCreateKeyboardEvent(std::ptr::null_mut(), key, true);
        if down.is_null() {
            return;
        }
        CGEventSetFlags(down, flags);
        CGEventPost(0, down);
        CFRelease(down);
        let up = CGEventCreateKeyboardEvent(std::ptr::null_mut(), key, false);
        if up.is_null() {
            return;
        }
        CGEventSetFlags(up, flags);
        CGEventPost(0, up);
        CFRelease(up);
    }
}

/// Name of the frontmost app, e.g. `"Safari"`. Used to label history and to
/// avoid pasting into the hub window.
pub fn frontmost_app() -> Option<String> {
    unsafe {
        let workspace = msg0(objc_getClass(c"NSWorkspace".as_ptr()), "sharedWorkspace");
        let app = msg0(workspace, "frontmostApplication");
        nsstring_to_string(msg0(app, "localizedName"))
    }
}

pub fn is_flowline_frontmost() -> bool {
    frontmost_app()
        .map(|n| n == "Flowline" || n == "flowline")
        .unwrap_or(false)
}

// ---------------------------------------------------------------------------
// AppKit: dock icon, activation, launch at login
// ---------------------------------------------------------------------------

pub fn set_dock_visible(visible: bool) {
    unsafe {
        let app = msg0(objc_getClass(c"NSApplication".as_ptr()), "sharedApplication");
        // 0 = NSApplicationActivationPolicyRegular, 1 = Accessory (no Dock tile).
        msg_i64(app, "setActivationPolicy:", if visible { 0 } else { 1 });
    }
}

pub fn activate_app() {
    unsafe {
        let app = msg0(objc_getClass(c"NSApplication".as_ptr()), "sharedApplication");
        let _ = msg_bool(app, "activateIgnoringOtherApps:", true);
    }
}

pub fn hide_app() {
    unsafe {
        let app = msg0(objc_getClass(c"NSApplication".as_ptr()), "sharedApplication");
        msg1(app, "hide:", std::ptr::null_mut());
    }
}

pub fn launch_agent_installed() -> bool {
    launch_agent_path().map(|p| p.exists()).unwrap_or(false)
}

fn launch_agent_path() -> Option<std::path::PathBuf> {
    let home = std::env::var_os("HOME")?;
    Some(
        std::path::PathBuf::from(home)
            .join("Library/LaunchAgents/com.flowline.app.plist"),
    )
}

/// Install or remove a per-user LaunchAgent that runs the current executable at
/// login. The bundle identifier is stable so toggling replaces the old entry.
pub fn set_launch_at_login(enabled: bool) -> Result<(), String> {
    let Some(path) = launch_agent_path() else {
        return Err("no HOME directory".into());
    };
    let _ = std::process::Command::new("launchctl")
        .args(["unload", "-w"])
        .arg(&path)
        .status();
    if !enabled {
        if path.exists() {
            std::fs::remove_file(&path).map_err(|e| e.to_string())?;
        }
        return Ok(());
    }
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let Some(dir) = path.parent() else {
        return Err("bad LaunchAgents path".into());
    };
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let log = dir.join("com.flowline.app.log");
    let plist = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>Label</key><string>com.flowline.app</string>
  <key>ProgramArguments</key>
  <array><string>{}</string></array>
  <key>RunAtLoad</key><true/>
  <key>StandardOutPath</key><string>{}</string>
  <key>StandardErrorPath</key><string>{}</string>
</dict>
</plist>
"#,
        xml_escape(&exe.to_string_lossy()),
        xml_escape(&log.to_string_lossy()),
        xml_escape(&log.to_string_lossy()),
    );
    std::fs::write(&path, plist).map_err(|e| e.to_string())?;
    let status = std::process::Command::new("launchctl")
        .args(["load", "-w"])
        .arg(&path)
        .status()
        .map_err(|e| e.to_string())?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("launchctl exited with {status}"))
    }
}

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

// ---------------------------------------------------------------------------
// Global hotkey
// ---------------------------------------------------------------------------


extern "C" fn hid_input_value(
    _context: *mut c_void,
    _result: i32,
    _sender: *mut c_void,
    value: *mut c_void,
) {
    if value.is_null() {
        return;
    }
    unsafe {
        let element = IOHIDValueGetElement(value);
        if element.is_null() {
            return;
        }
        let page = IOHIDElementGetUsagePage(element);
        let usage = IOHIDElementGetUsage(element);
        // Apple keyboards expose the Fn key as vendor page 0xFF usage 3.
        if page == 0xFF && usage == 3 {
            let down = IOHIDValueGetIntegerValue(value) != 0;
            if std::env::var_os("FLOWLINE_DEBUG").is_some() {
                eprintln!("hid fn down={down}");
            }
            push_event(if down {
                HotkeyEvent::Start
            } else {
                HotkeyEvent::Stop
            });
        }
    }
}

/// Device-level keyboard service: IOHIDManager sees Fn even when the keyboard
/// does not deliver a usable CGEvent flag.
pub fn start_hid_keyboard_service() {
    // Opening an IOHIDManager on keyboards makes macOS ask for Input
    // Monitoring. Only check the status, never ask: run this extra Fn source
    // only when access is already granted. The event tap (Accessibility) is the
    // main path.
    if !input_monitoring_granted() || HID_STARTED.swap(true, Ordering::SeqCst) {
        return;
    }
    std::thread::spawn(|| unsafe {
        let manager = IOHIDManagerCreate(std::ptr::null(), 0);
        if manager.is_null() {
            return;
        }
        let matching = CFDictionaryCreateMutable(
            std::ptr::null(),
            0,
            &kCFTypeDictionaryKeyCallBacks as *const c_void,
            &kCFTypeDictionaryValueCallBacks as *const c_void,
        );
        let mut page: i32 = 1; // Generic Desktop
        let mut usage: i32 = 6; // Keyboard
        let page_num = CFNumberCreate(std::ptr::null(), 9, &mut page as *mut i32 as *const c_void);
        let usage_num = CFNumberCreate(std::ptr::null(), 9, &mut usage as *mut i32 as *const c_void);
        if !matching.is_null() {
            let page_key = nsstring("DeviceUsagePage");
            let usage_key = nsstring("DeviceUsage");
            CFDictionarySetValue(matching, page_key, page_num);
            CFDictionarySetValue(matching, usage_key, usage_num);
            CFRelease(page_key);
            CFRelease(usage_key);
            IOHIDManagerSetDeviceMatching(manager, matching);
            CFRelease(matching);
        }
        let _ = (page_num, usage_num);
        IOHIDManagerRegisterInputValueCallback(manager, hid_input_value, std::ptr::null_mut());
        let run_loop = CFRunLoopGetCurrent();
        IOHIDManagerScheduleWithRunLoop(manager, run_loop, kCFRunLoopCommonModes);
        let opened = IOHIDManagerOpen(manager, 0);
        if std::env::var_os("FLOWLINE_DEBUG").is_some() {
            eprintln!("hid: manager opened={} (0 = ok)", opened);
        }
        CFRunLoopRun();
    });
}

static HID_STARTED: AtomicBool = AtomicBool::new(false);
static TAP: AtomicPtr<c_void> = AtomicPtr::new(std::ptr::null_mut());
static MONITOR_STARTED: AtomicBool = AtomicBool::new(false);
static TAP_OK: AtomicBool = AtomicBool::new(false);
/// 0 = Fn, 1 = ctrl-space; mirrors `Settings::ptt`.
static HOTKEY_MODE: AtomicI32 = AtomicI32::new(0);
static FN_DOWN: AtomicBool = AtomicBool::new(false);
static SPACE_DOWN: AtomicBool = AtomicBool::new(false);
/// True when the tap was created with `kCGEventTapOptionDefault`, which lets
/// it swallow the Fn flags change so macOS does not also open Spotlight/emoji.
static TAP_SUPPRESSES_FN: AtomicBool = AtomicBool::new(false);

/// Digit key whose key-down we swallowed, so its key-up is swallowed too.
static XFORM_KEY_HELD: std::sync::atomic::AtomicI64 = std::sync::atomic::AtomicI64::new(-1);

static COMMAND_HELD: AtomicBool = AtomicBool::new(false);

/// ANSI digit key codes 1…9 in order of the number row.
pub fn digit_slot(keycode: i64) -> Option<u32> {
    match keycode {
        18 => Some(0),
        19 => Some(1),
        20 => Some(2),
        21 => Some(3),
        23 => Some(4),
        22 => Some(5),
        26 => Some(6),
        28 => Some(7),
        25 => Some(8),
        _ => None,
    }
}

pub fn hotkey_running() -> bool {
    TAP_OK.load(Ordering::Relaxed)
}

/// True only when the tap can swallow the Fn flags change (Accessibility).
pub fn hotkey_active() -> bool {
    TAP_SUPPRESSES_FN.load(Ordering::Relaxed)
}

/// A listen-only tap created before Accessibility was granted can never
/// suppress the Globe action. Replace it with an active tap as soon as the
/// permission appears, without requiring an app restart.
pub fn upgrade_hotkey_tap(ctx: eframe::egui::Context) {
    if TAP_SUPPRESSES_FN.load(Ordering::Relaxed) || !accessibility_granted() {
        return;
    }
    if std::env::var_os("FLOWLINE_DEBUG").is_some() {
        eprintln!("hotkey: upgrading listen-only tap to active");
    }
    let old = TAP.swap(std::ptr::null_mut(), Ordering::SeqCst);
    if !old.is_null() {
        unsafe { CGEventTapEnable(old, false) };
    }
    MONITOR_STARTED.store(false, Ordering::SeqCst);
    ensure_hotkey_monitor(ctx);
}

/// Does the frontmost app have a text field focused? `None` means Accessibility
/// could not answer (some apps hide it), in which case Flowline pastes anyway.
unsafe fn ax_string(element: Id, name: &str) -> Option<String> {
    let attr = nsstring(name);
    let mut value: Id = std::ptr::null_mut();
    let err = AXUIElementCopyAttributeValue(element, attr, &mut value);
    CFRelease(attr);
    if err != 0 || value.is_null() {
        return None;
    }
    let out = if CFGetTypeID(value) == CFStringGetTypeID() {
        nsstring_to_string(value)
    } else {
        None
    };
    CFRelease(value);
    out
}

/// The text of the field that has keyboard focus, for My Voice's correction
/// learner. Returns `None` for password fields (never read), for fields that
/// do not expose text, and for anything over 20,000 characters.
pub fn read_focused_text() -> Option<String> {
    unsafe {
        let system = AXUIElementCreateSystemWide();
        if system.is_null() {
            return None;
        }
        let attr = nsstring("AXFocusedUIElement");
        let mut focused: Id = std::ptr::null_mut();
        let err = AXUIElementCopyAttributeValue(system, attr, &mut focused);
        CFRelease(system);
        CFRelease(attr);
        if err != 0 || focused.is_null() {
            return None;
        }
        let role = ax_string(focused, "AXRole").unwrap_or_default();
        let subrole = ax_string(focused, "AXSubrole").unwrap_or_default();
        let secure = role == "AXSecureTextField" || subrole == "AXSecureTextField";
        let text = if secure { None } else { ax_string(focused, "AXValue") };
        CFRelease(focused);
        text.filter(|t| t.chars().count() <= 20_000)
    }
}

/// Title of the window that has focus, e.g. a browser tab's title. Used only to
/// tell Gmail from Slack from WhatsApp Web when picking a tone; never stored.
pub fn front_window_title() -> Option<String> {
    unsafe {
        let system = AXUIElementCreateSystemWide();
        if system.is_null() {
            return None;
        }
        // A hung app must not freeze Flowline: give up after half a second.
        AXUIElementSetMessagingTimeout(system, 0.5);
        let attr = nsstring("AXFocusedUIElement");
        let mut focused: Id = std::ptr::null_mut();
        let err = AXUIElementCopyAttributeValue(system, attr, &mut focused);
        CFRelease(system);
        CFRelease(attr);
        if err != 0 || focused.is_null() {
            return None;
        }
        let win_attr = nsstring("AXWindow");
        let mut window: Id = std::ptr::null_mut();
        let err = AXUIElementCopyAttributeValue(focused, win_attr, &mut window);
        CFRelease(win_attr);
        CFRelease(focused);
        if err != 0 || window.is_null() {
            return None;
        }
        let title = ax_string(window, "AXTitle");
        CFRelease(window);
        title
    }
}

pub fn focused_text_input() -> Option<bool> {
    unsafe {
        let system = AXUIElementCreateSystemWide();
        if system.is_null() {
            return None;
        }
        let focused_attr = nsstring("AXFocusedUIElement");
        let mut focused: Id = std::ptr::null_mut();
        let err = AXUIElementCopyAttributeValue(system, focused_attr, &mut focused);
        CFRelease(system);
        CFRelease(focused_attr);
        if err != 0 || focused.is_null() {
            return None;
        }
        let role_attr = nsstring("AXRole");
        let mut role_value: Id = std::ptr::null_mut();
        let role_ok = AXUIElementCopyAttributeValue(focused, role_attr, &mut role_value) == 0;
        CFRelease(role_attr);
        let role = if role_ok {
            nsstring_to_string(role_value).unwrap_or_default()
        } else {
            String::new()
        };
        if role_ok {
            CFRelease(role_value);
        }
        let editable_attr = nsstring("AXEditable");
        let mut editable_value: Id = std::ptr::null_mut();
        let editable_ok =
            AXUIElementCopyAttributeValue(focused, editable_attr, &mut editable_value) == 0;
        CFRelease(editable_attr);
        let editable = editable_ok
            && !editable_value.is_null()
            && CFBooleanGetValue(editable_value);
        if editable_ok {
            CFRelease(editable_value);
        }
        CFRelease(focused);
        let text_role = matches!(
            role.as_str(),
            "AXTextField" | "AXTextArea" | "AXComboBox" | "AXSearchField" | "AXSecureTextField"
        ) || role.contains("Text");
        Some(text_role || editable)
    }
}

/// The tap suppresses the Globe action; when macOS still opens Siri's "Search
/// or Ask" panel after a Fn release, close it before the
/// transcript is pasted so the keystroke lands in the app, not in Siri.
pub fn dismiss_search_overlay() {
    if !accessibility_granted() {
        return;
    }
    unsafe {
        let list = CGWindowListCopyWindowInfo(2 /* OnScreenOnly */, 0);
        if list.is_null() {
            return;
        }
        let count = CFArrayGetCount(list);
        let mut search_open = false;
        for i in 0..count {
            let info = CFArrayGetValueAtIndex(list, i);
            if info.is_null() {
                continue;
            }
            let owner = CFDictionaryGetValue(info, kCGWindowOwnerName);
            if owner.is_null() {
                continue;
            }
            let name = nsstring_to_string(owner as Id).unwrap_or_default();
            if name == "Siri AI" || name == "Siri" || name == "Spotlight" || name == "Search" {
                search_open = true;
                break;
            }
        }
        CFRelease(list);
        if !search_open {
            return;
        }
        for _ in 0..2 {
            let down = CGEventCreateKeyboardEvent(std::ptr::null_mut(), 53, true);
            if !down.is_null() {
                CGEventPost(0, down);
                CFRelease(down);
            }
            let up = CGEventCreateKeyboardEvent(std::ptr::null_mut(), 53, false);
            if !up.is_null() {
                CGEventPost(0, up);
                CFRelease(up);
            }
            std::thread::sleep(std::time::Duration::from_millis(90));
        }
    }
}

/// Keep the floating dictation window above fullscreen apps. egui creates a
/// plain window; without these flags it lives in the desktop Space and is
/// invisible while another app is fullscreen.
pub fn promote_overlay_window() {
    unsafe {
        let app = msg0(objc_getClass(c"NSApplication".as_ptr()), "sharedApplication");
        let windows = msg0(app, "windows");
        if windows.is_null() {
            return;
        }
        let count: usize = {
            let f: extern "C" fn(Id, *mut c_void) -> usize =
                std::mem::transmute(objc_msgSend as *const ());
            f(windows, sel("count"))
        };
        let at: extern "C" fn(Id, *mut c_void, usize) -> Id =
            std::mem::transmute(objc_msgSend as *const ());
        for i in 0..count {
            let w = at(windows, sel("objectAtIndex:"), i);
            if w.is_null() {
                continue;
            }
            let title = nsstring_to_string(msg0(w, "title")).unwrap_or_default();
            if title != "Flowline Dictation" {
                continue;
            }
            // Floating level, every Space, alongside fullscreen apps.
            msg_i64(w, "setLevel:", 3);
            let set_behavior: extern "C" fn(Id, *mut c_void, usize) =
                std::mem::transmute(objc_msgSend as *const ());
            set_behavior(w, sel("setCollectionBehavior:"), 1 | 16 | 256);
            msg_bool(w, "setHidesOnDeactivate:", false);
            msg_bool(w, "setCanHide:", false);
            msg_bool(w, "setMovableByWindowBackground:", false);
            msg0(w, "orderFrontRegardless");
            return;
        }
    }
}


pub fn set_hotkey_mode(ptt: &str) {
    HOTKEY_MODE.store(if ptt == "Fn" { 0 } else { 1 }, Ordering::Relaxed);
}

/// Start the listen-only keyboard tap on its own run-loop thread. Safe to call
/// once; extra calls just update the wake context.
pub fn ensure_hotkey_monitor(ctx: eframe::egui::Context) {
    *WAKE.lock().unwrap() = Some(ctx);
    if MONITOR_STARTED.swap(true, Ordering::SeqCst) {
        return;
    }
    std::thread::spawn(|| unsafe { run_event_tap() });
}

unsafe fn run_event_tap() {
    let mask = (1u64 << FLAGS_CHANGED)
        | (1u64 << KEY_DOWN)
        | (1u64 << KEY_UP)
        | (1u64 << 14); // NX_SYSDEFINED (Globe/media keys)
    // A session-level active tap (tap location 1, option 0) can swallow the Fn
    // flags change. Fall back to listen-only when Accessibility is missing.
    let mut tap = CGEventTapCreate(1, 0, 0, mask, tap_callback, std::ptr::null_mut());
    TAP_SUPPRESSES_FN.store(!tap.is_null(), Ordering::SeqCst);
    // The listen-only fallback needs Input Monitoring and creating it without
    // that access pops the system prompt, so only try it when already granted.
    if tap.is_null() && input_monitoring_granted() {
        tap = CGEventTapCreate(1, 0, 1, mask, tap_callback, std::ptr::null_mut());
    }
    if std::env::var_os("FLOWLINE_DEBUG").is_some() {
        eprintln!(
            "hotkey: tap active={} at {}",
            TAP_SUPPRESSES_FN.load(Ordering::SeqCst),
            if TAP_SUPPRESSES_FN.load(Ordering::SeqCst) { "HID/session" } else { "listen-only" }
        );
    }
    if tap.is_null() {
        TAP_OK.store(false, Ordering::SeqCst);
        MONITOR_STARTED.store(false, Ordering::SeqCst);
        push_event(HotkeyEvent::Denied);
        return;
    }
    TAP.store(tap, Ordering::SeqCst);
    let source = CFMachPortCreateRunLoopSource(std::ptr::null(), tap, 0);
    if source.is_null() {
        TAP_OK.store(false, Ordering::SeqCst);
        MONITOR_STARTED.store(false, Ordering::SeqCst);
        push_event(HotkeyEvent::Denied);
        return;
    }
    CFRunLoopAddSource(CFRunLoopGetCurrent(), source, kCFRunLoopCommonModes);
    CGEventTapEnable(tap, true);
    TAP_OK.store(true, Ordering::SeqCst);
    if std::env::var_os("FLOWLINE_DEBUG_TRAY").is_some() {
        eprintln!(
            "hotkey: tap ready (suppress_fn={})",
            TAP_SUPPRESSES_FN.load(Ordering::SeqCst)
        );
    }
    CFRunLoopRun();
}

extern "C" fn tap_callback(
    _proxy: *mut c_void,
    kind: u32,
    event: *mut c_void,
    _user: *mut c_void,
) -> *mut c_void {
    unsafe {
        if kind == TAP_DISABLED_TIMEOUT || kind == TAP_DISABLED_USER {
            let tap = TAP.load(Ordering::SeqCst);
            if !tap.is_null() {
                CGEventTapEnable(tap, true);
            }
            return event;
        }
        if std::env::var_os("FLOWLINE_DEBUG").is_some() {
            eprintln!(
                "tap kind={kind} key={} flags={:#x}",
                CGEventGetIntegerValueField(event, 9),
                CGEventGetFlags(event)
            );
        }
        let mode = HOTKEY_MODE.load(Ordering::Relaxed);
        let flags = CGEventGetFlags(event);
        if kind == KEY_DOWN && CGEventGetIntegerValueField(event, 9) == KEY_ESCAPE {
            push_event(HotkeyEvent::Cancel);
        }
        // Option + digit runs a transform on the selected text. Only claimed
        // while Transforms is on, and only with the active tap, which is the
        // one that can swallow the key so no ¡™£ is typed.
        if (kind == KEY_DOWN || kind == KEY_UP) && TAP_SUPPRESSES_FN.load(Ordering::Relaxed) {
            let key = CGEventGetIntegerValueField(event, 9);
            if let Some(slot) = digit_slot(key) {
                if kind == KEY_DOWN {
                    let only_option =
                        flags & OPTION_FLAG != 0 && flags & (CMD_FLAG | CTRL_FLAG | SHIFT_FLAG) == 0;
                    if only_option && XFORM_SLOTS.load(Ordering::Relaxed) & (1 << slot) != 0 {
                        XFORM_KEY_HELD.store(key, Ordering::SeqCst);
                        let repeat = CGEventGetIntegerValueField(event, 8) != 0;
                        if !repeat {
                            push_event(HotkeyEvent::Transform(slot as u8));
                        }
                        return std::ptr::null_mut();
                    }
                } else if XFORM_KEY_HELD
                    .compare_exchange(key, -1, Ordering::SeqCst, Ordering::SeqCst)
                    .is_ok()
                {
                    return std::ptr::null_mut();
                }
            }
        }
        // Hold ⌥Space for command mode. Swallowed so no non-breaking space is
        // typed; the key-up ends the command even if Option came up first.
        if (kind == KEY_DOWN || kind == KEY_UP) && TAP_SUPPRESSES_FN.load(Ordering::Relaxed) {
            if CGEventGetIntegerValueField(event, 9) == KEY_SPACE {
                if kind == KEY_DOWN {
                    let only_option =
                        flags & OPTION_FLAG != 0 && flags & (CMD_FLAG | CTRL_FLAG | SHIFT_FLAG) == 0;
                    if only_option && COMMAND_ENABLED.load(Ordering::Relaxed) {
                        if CGEventGetIntegerValueField(event, 8) == 0
                            && !COMMAND_HELD.swap(true, Ordering::SeqCst)
                        {
                            push_event(HotkeyEvent::CommandStart);
                        }
                        return std::ptr::null_mut();
                    }
                } else if COMMAND_HELD.swap(false, Ordering::SeqCst) {
                    push_event(HotkeyEvent::CommandStop);
                    return std::ptr::null_mut();
                }
            }
        }
        match mode {
            0 => {
                if kind == FLAGS_CHANGED {
                    let down = flags & FN_FLAG != 0;
                    let was = FN_DOWN.swap(down, Ordering::SeqCst);
                    if down && !was {
                        push_event(HotkeyEvent::Start);
                    } else if !down && was {
                        push_event(HotkeyEvent::Stop);
                    }
                    // Swallow only Fn-only flags changes, leaving every other
                    // modifier transition to the system and other apps.
                    if TAP_SUPPRESSES_FN.load(Ordering::Relaxed)
                        && (down || was)
                        && flags & (CMD_FLAG | SHIFT_FLAG | OPTION_FLAG | CTRL_FLAG) == 0
                    {
                        return std::ptr::null_mut();
                    }
                }
            }
            _ => {
                if kind == KEY_DOWN || kind == KEY_UP {
                    let key = CGEventGetIntegerValueField(event, 9); // keyboard keycode
                    if key == KEY_SPACE {
                        let plain = flags & (CMD_FLAG | SHIFT_FLAG | OPTION_FLAG) == 0
                            && flags & CTRL_FLAG != 0;
                        if kind == KEY_DOWN && plain && !SPACE_DOWN.swap(true, Ordering::SeqCst) {
                            push_event(HotkeyEvent::Start);
                        } else if kind == KEY_UP && SPACE_DOWN.swap(false, Ordering::SeqCst) {
                            push_event(HotkeyEvent::Stop);
                        }
                    }
                }
            }
        }
    }
    event
}

// ---------------------------------------------------------------------------
// Menu-bar item
// ---------------------------------------------------------------------------

static TRAY_TARGET: AtomicPtr<c_void> = AtomicPtr::new(std::ptr::null_mut());
static TRAY_ITEM: AtomicPtr<c_void> = AtomicPtr::new(std::ptr::null_mut());

extern "C" fn tray_action(_this: Id, _cmd: *mut c_void, sender: Id) {
    unsafe {
        let tag = msg_take_i64(sender, "tag");
        match tag {
            1 => {
                SHOW_REQUESTED.store(true, Ordering::SeqCst);
            }
            2 => {
                QUIT_REQUESTED.store(true, Ordering::SeqCst);
            }
            3 => {
                PERMISSIONS_REQUESTED.store(true, Ordering::SeqCst);
            }
            _ => {}
        }
    }
    if let Some(ctx) = WAKE.lock().ok().and_then(|w| w.clone()) {
        ctx.request_repaint();
    }
}

/// Add the waveform status item with an Open / Quit menu. Must run on the main
/// thread once the NSApplication exists (eframe has created it by `App::new`).
pub fn install_status_item(ctx: eframe::egui::Context) {
    *WAKE.lock().unwrap() = Some(ctx);
    if !TRAY_ITEM.load(Ordering::SeqCst).is_null() {
        return;
    }
    unsafe {
        let cls = objc_allocateClassPair(
            objc_getClass(c"NSObject".as_ptr()),
            c"FlowlineTrayTarget".as_ptr(),
            0,
        );
        if cls.is_null() {
            return;
        }
        class_addMethod(
            cls,
            sel("flowlineAction:"),
            tray_action as *const c_void,
            c"v@:@".as_ptr(),
        );
        objc_registerClassPair(cls);
        let target = msg0(msg0(cls, "alloc"), "init");
        TRAY_TARGET.store(target, Ordering::SeqCst);

        let bar = msg0(objc_getClass(c"NSStatusBar".as_ptr()), "systemStatusBar");
        // NSVariableStatusItemLength == -1
        let item = msg_f64(bar, "statusItemWithLength:", -1.0);
        if std::env::var_os("FLOWLINE_DEBUG_TRAY").is_some() {
            eprintln!("tray: bar={bar:p} item={item:p}");
        }
        msg0(item, "retain");
        let button = msg0(item, "button");

        let image = msg2(
            objc_getClass(c"NSImage".as_ptr()),
            "imageWithSystemSymbolName:accessibilityDescription:",
            nsstring("waveform"),
            nsstring("Flowline"),
        );
        if image.is_null() {
            msg1(button, "setTitle:", nsstring("◉"));
        } else {
            msg_bool(image, "setTemplate:", true);
            msg1(button, "setImage:", image);
        }
        msg1(button, "setToolTip:", nsstring("Flowline — hold Fn to dictate"));
        if std::env::var_os("FLOWLINE_DEBUG_TRAY").is_some() {
            eprintln!("tray: button={button:p} image={image:p} visible={:?}", {
                let f: extern "C" fn(Id, *mut c_void) -> bool =
                    std::mem::transmute(objc_msgSend as *const ());
                f(button, sel("isHidden"))
            });
        }

        let menu = msg0(msg0(objc_getClass(c"NSMenu".as_ptr()), "alloc"), "init");
        let open = make_menu_item("Open Flowline", 1, target);
        let quit = make_menu_item("Quit Flowline", 2, target);
        let perms = make_menu_item("Permissions…", 3, target);
        msg1(menu, "addItem:", open);
        msg1(menu, "addItem:", perms);
        let sep = msg0(objc_getClass(c"NSMenuItem".as_ptr()), "separatorItem");
        msg1(menu, "addItem:", sep);
        msg1(menu, "addItem:", quit);
        msg1(item, "setMenu:", menu);

        TRAY_ITEM.store(item, Ordering::SeqCst);
    }
}

unsafe fn make_menu_item(title: &str, tag: i64, target: Id) -> Id {
    let cls = objc_getClass(c"NSMenuItem".as_ptr());
    let alloc = msg0(cls, "alloc");
    let item = msg3(
        alloc,
        "initWithTitle:action:keyEquivalent:",
        nsstring(title),
        sel("flowlineAction:"),
        nsstring(""),
    );
    msg1(item, "setTarget:", target);
    msg_i64(item, "setTag:", tag);
    item
}

#[cfg(test)]
mod tests {
    #[test]
    fn digit_row_maps_to_slots_in_order() {
        // 1,2,3,4,5,6,7,8,9 on an ANSI keyboard.
        let keys = [18, 19, 20, 21, 23, 22, 26, 28, 25];
        for (i, k) in keys.iter().enumerate() {
            assert_eq!(digit_slot(*k), Some(i as u32));
        }
        assert_eq!(digit_slot(KEY_V), None);
        assert_eq!(digit_slot(KEY_SPACE), None);
    }

    #[test]
    fn paste_presses_command_v_not_command_space() {
        assert_eq!(KEY_V, 9);
        assert_ne!(KEY_V, KEY_SPACE);
    }

    use super::*;

    #[test]
    fn xml_escaping_covers_plist_specials() {
        assert_eq!(xml_escape("a&b<c>d"), "a&amp;b&lt;c&gt;d");
        assert_eq!(xml_escape("/Applications/Flowline.app"), "/Applications/Flowline.app");
    }

    #[test]
    fn hotkey_mode_maps_to_the_two_choices() {
        set_hotkey_mode("Fn");
        assert_eq!(HOTKEY_MODE.load(Ordering::Relaxed), 0);
        set_hotkey_mode("Ctrl Space");
        assert_eq!(HOTKEY_MODE.load(Ordering::Relaxed), 1);
        set_hotkey_mode("Fn");
    }
}

/// What to tell the person when the global key cannot be captured.
pub fn denied_hint() -> &'static str {
    "Allow Accessibility for Flowline to use Fn anywhere"
}

/// Open a file or folder in Finder.
pub fn open_path(path: &std::path::Path) {
    let _ = std::process::Command::new("open").arg(path).spawn();
}

/// macOS reopens the window from the menu-bar item, not by launching again.
pub fn signal_running_instance() -> bool {
    false
}
