//! Overlay window positioning for the filmstrip.
//!
//! Positions the Tauri window at the bottom of the screen as an overlay panel.
//! On Wayland, applies compositor-specific window rules for better integration.
//! On X11, sets EWMH properties for dock/panel behavior.

use std::process::Command;

use log::{debug, info, warn};
use tauri::{AppHandle, Manager, PhysicalPosition, PhysicalSize};

/// Set up the main window as a bottom-edge overlay.
///
/// Call this in Tauri's `.setup()` after the window is created.
pub fn setup_overlay(app: &AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    let window = app
        .get_webview_window("main")
        .ok_or("Main window not found")?;

    // Get the monitor where the window is (or primary monitor)
    let monitor = window
        .current_monitor()?
        .or_else(|| window.primary_monitor().ok().flatten())
        .ok_or("No monitor detected")?;

    let monitor_size = monitor.size();
    let monitor_position = monitor.position();
    let scale_factor = monitor.scale_factor();

    let filmstrip_height: u32 = 350;

    // Calculate position: full width, anchored to bottom
    let width = monitor_size.width;
    let x = monitor_position.x;
    let y = monitor_position.y + (monitor_size.height as i32) - (filmstrip_height as i32);

    info!(
        "Positioning overlay: {}x{} at ({}, {}), monitor: {}x{}, scale: {}",
        width, filmstrip_height, x, y, monitor_size.width, monitor_size.height, scale_factor,
    );

    // Set window size and position
    window.set_size(PhysicalSize::new(width, filmstrip_height))?;
    window.set_position(PhysicalPosition::new(x, y))?;

    // Apply compositor-specific overlay behavior
    let is_wayland = std::env::var("WAYLAND_DISPLAY").is_ok();
    if is_wayland {
        apply_wayland_rules(&window.title().unwrap_or_default());
    } else {
        apply_x11_rules();
    }

    info!("Overlay window positioned");
    Ok(())
}

/// Apply Wayland compositor-specific window rules for overlay behavior.
fn apply_wayland_rules(window_title: &str) {
    // Try Hyprland
    if apply_hyprland_rules(window_title) {
        return;
    }

    // Try Sway
    if apply_sway_rules(window_title) {
        return;
    }

    debug!("No compositor-specific rules applied (GNOME/KDE may handle this natively)");
}

/// Apply Hyprland window rules via hyprctl.
fn apply_hyprland_rules(window_title: &str) -> bool {
    // Check if hyprctl is available
    if Command::new("hyprctl").arg("version").output().is_err() {
        return false;
    }

    info!("Applying Hyprland overlay rules");

    // Set window rules: no border, floating, pin (visible on all workspaces)
    let rules = [
        format!(
            "hyprctl keyword windowrulev2 'float,title:{}'",
            window_title
        ),
        format!("hyprctl keyword windowrulev2 'pin,title:{}'", window_title),
        format!(
            "hyprctl keyword windowrulev2 'noborder,title:{}'",
            window_title
        ),
        format!(
            "hyprctl keyword windowrulev2 'noshadow,title:{}'",
            window_title
        ),
        format!(
            "hyprctl keyword windowrulev2 'noanim,title:{}'",
            window_title
        ),
    ];

    for rule in &rules {
        match Command::new("sh").args(["-c", rule]).status() {
            Ok(s) if s.success() => debug!("Applied rule: {}", rule),
            Ok(s) => warn!("Rule failed ({}): {}", s, rule),
            Err(e) => warn!("Failed to execute hyprctl: {e}"),
        }
    }

    true
}

/// Apply Sway window rules via swaymsg.
fn apply_sway_rules(window_title: &str) -> bool {
    if Command::new("swaymsg")
        .arg("-t get_version")
        .output()
        .is_err()
    {
        return false;
    }

    info!("Applying Sway overlay rules");

    let rules = [
        format!(
            r#"swaymsg 'for_window [title="{}"] floating enable'"#,
            window_title
        ),
        format!(
            r#"swaymsg 'for_window [title="{}"] sticky enable'"#,
            window_title
        ),
        format!(
            r#"swaymsg 'for_window [title="{}"] border none'"#,
            window_title
        ),
    ];

    for rule in &rules {
        match Command::new("sh").args(["-c", rule]).status() {
            Ok(s) if s.success() => debug!("Applied rule: {}", rule),
            Ok(s) => warn!("Rule failed ({}): {}", s, rule),
            Err(e) => warn!("Failed to execute swaymsg: {e}"),
        }
    }

    true
}

/// Apply X11 EWMH properties for dock/panel behavior.
fn apply_x11_rules() {
    info!("Applying X11 overlay rules");

    // Use xprop to set window type to DOCK (appears above other windows, no taskbar)
    // This runs after the window is created, so we need to find it by title
    let result = Command::new("sh")
        .args([
            "-c",
            r#"
            WINDOW_ID=$(xdotool search --name "Paste" | head -1)
            if [ -n "$WINDOW_ID" ]; then
                xprop -id "$WINDOW_ID" -f _NET_WM_WINDOW_TYPE 32a -set _NET_WM_WINDOW_TYPE _NET_WM_WINDOW_TYPE_DOCK
                xprop -id "$WINDOW_ID" -f _NET_WM_STATE 32a -set _NET_WM_STATE _NET_WM_STATE_ABOVE,_NET_WM_STATE_STICKY
            fi
            "#,
        ])
        .status();

    match result {
        Ok(s) if s.success() => debug!("X11 EWMH properties set"),
        Ok(s) => warn!("xprop/xdotool failed: {}", s),
        Err(e) => warn!("Failed to set X11 properties: {e}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_apply_x11_rules_does_not_panic() {
        // Just verify the function doesn't panic when tools aren't available
        apply_x11_rules();
    }

    #[test]
    fn test_apply_wayland_rules_does_not_panic() {
        apply_wayland_rules("TestWindow");
    }
}
