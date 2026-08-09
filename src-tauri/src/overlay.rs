//! Overlay window positioning for the filmstrip.
//!
//! On GNOME Wayland, applications cannot position windows. Instead, we make
//! the window cover the screen with a transparent top area, and CSS pushes
//! all content to the bottom.
//!
//! Maximized, not fullscreen: GNOME composites fullscreen surfaces against
//! an opaque black backdrop, so a transparent fullscreen window shows black
//! instead of the desktop behind it. Maximized windows composite normally.

use log::info;
use tauri::{AppHandle, Manager};

/// Set up the main window as a maximized transparent overlay.
/// Content is bottom-aligned via CSS (flex + margin-top: auto).
pub fn setup_overlay(app: &AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    let window = app
        .get_webview_window("main")
        .ok_or("Main window not found")?;

    let _ = window.set_fullscreen(false);
    let _ = window.maximize();
    let _ = window.set_always_on_top(true);

    info!("Overlay set to maximized mode (CSS handles bottom alignment)");
    Ok(())
}
