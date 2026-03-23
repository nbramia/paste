//! Overlay window positioning for the filmstrip.
//!
//! On GNOME Wayland, applications cannot position windows. Instead, we make
//! the window fullscreen with a transparent top area, and CSS pushes all
//! content to the bottom of the screen.

use log::info;
use tauri::{AppHandle, Manager};

/// Set up the main window as a fullscreen overlay.
/// Content is bottom-aligned via CSS (flex + margin-top: auto).
pub fn setup_overlay(app: &AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    let window = app
        .get_webview_window("main")
        .ok_or("Main window not found")?;

    let _ = window.set_fullscreen(true);
    let _ = window.set_always_on_top(true);

    info!("Overlay set to fullscreen mode (CSS handles bottom alignment)");
    Ok(())
}
