//! Platform-independent plumbing shared by the macOS and Linux back ends: the
//! queue of global-hotkey events, waking the UI from other threads, and the
//! "show the window / quit" requests from a tray item or a second launch.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HotkeyEvent {
    Start,
    Stop,
    Cancel,
    /// Modifier + digit: run transform number `n` (0-based) on the selection.
    Transform(u8),
    /// Command-mode key pressed / released: speak an edit instruction for the selection.
    CommandStart,
    CommandStop,
    /// The tap could not be created, almost always a permission problem.
    Denied,
}

pub(super) static EVENTS: Mutex<Vec<HotkeyEvent>> = Mutex::new(Vec::new());
pub(super) static WAKE: Mutex<Option<eframe::egui::Context>> = Mutex::new(None);

/// Bit n set = modifier+(n+1) is a live Transforms shortcut. Set by the app each
/// frame; zero (Transforms off or no text model) leaves the keys alone.
pub(super) static XFORM_SLOTS: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
/// Command mode (hold ⌥Space) is live. Set by the app each frame.
pub(super) static COMMAND_ENABLED: AtomicBool = AtomicBool::new(false);
pub fn set_command_enabled(on: bool) {
    COMMAND_ENABLED.store(on, Ordering::Relaxed);
}

pub fn set_transform_slots(mask: u32) {
    XFORM_SLOTS.store(mask, Ordering::Relaxed);
}

/// Ask the UI for a frame, even while the window is hidden in the menu bar.
pub fn wake() {
    if let Some(ctx) = WAKE.lock().ok().and_then(|w| w.clone()) {
        ctx.request_repaint();
    }
}

pub fn push_event(event: HotkeyEvent) {
    if let Ok(mut q) = EVENTS.lock() {
        q.push(event);
    }
    if let Some(ctx) = WAKE.lock().ok().and_then(|w| w.clone()) {
        ctx.request_repaint();
    }
}

pub fn take_events() -> Vec<HotkeyEvent> {
    EVENTS
        .lock()
        .map(|mut q| std::mem::take(&mut *q))
        .unwrap_or_default()
}

pub(super) static SHOW_REQUESTED: AtomicBool = AtomicBool::new(false);
pub(super) static PERMISSIONS_REQUESTED: AtomicBool = AtomicBool::new(false);
pub(super) static QUIT_REQUESTED: AtomicBool = AtomicBool::new(false);

pub fn take_show_request() -> bool {
    SHOW_REQUESTED.swap(false, Ordering::SeqCst)
}

pub fn take_quit_request() -> bool {
    QUIT_REQUESTED.swap(false, Ordering::SeqCst)
}

pub fn take_permissions_request() -> bool {
    PERMISSIONS_REQUESTED.swap(false, Ordering::SeqCst)
}

