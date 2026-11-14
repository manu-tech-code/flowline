//! Stand-in for `native_overlay` on systems without the macOS AppKit panel. The
//! dictation pill is drawn by an egui overlay window instead (`show_overlay`),
//! so nothing here does anything.

pub const WIDTH: f64 = 470.0;
pub const HEIGHT: f64 = 176.0;

#[derive(Clone, Default)]
pub struct Model {
    pub kind: u8,
    pub elapsed: f32,
    pub bands: [f32; 7],
    pub label: String,
    pub text: String,
    pub copied: bool,
    pub head: String,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Stop,
    Copy,
    Dismiss,
}

pub fn install() {}
pub fn place(_x: f64, _y: f64) {}
pub fn show(_model: Model) {}
pub fn hide() {}
pub fn origin() -> Option<(f64, f64)> {
    None
}
pub fn take_actions() -> Vec<Action> {
    Vec::new()
}
