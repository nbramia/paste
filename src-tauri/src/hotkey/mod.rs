//! Global hotkey daemon via evdev.

pub mod daemon;
pub mod keys;

pub use daemon::{HotkeyDaemon, HotkeyEvent, HotkeyAction};
pub use keys::{KeyCombo, Modifiers, parse_hotkey};
