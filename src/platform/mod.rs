//! System glue. Each operating system has its own back end behind the same set
//! of functions; everything the app calls lives here under one name.

mod shared;
pub use shared::*;

// Pure logic, compiled everywhere so it is tested on every platform.
#[allow(dead_code)]
mod hotkey_logic;

#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "macos")]
pub use macos::*;

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "linux")]
pub use linux::*;
