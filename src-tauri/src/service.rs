//! Systemd user service management for autostart.

use std::path::PathBuf;
use std::process::Command;

use log::{info, warn};

/// Get the service file path.
fn service_path() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("/tmp"))
        .join("systemd")
        .join("user")
        .join("paste.service")
}

/// Get the desktop entry path.
fn desktop_entry_path() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("/tmp"))
        .join("applications")
        .join("paste.desktop")
}

/// Generate the systemd service file content.
fn service_content() -> String {
    // Try to find our own executable path
    let exec_path = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("/usr/bin/paste"));

    format!(
        r#"[Unit]
Description=Paste — Clipboard Manager
Documentation=https://github.com/nbramia/paste
After=graphical-session.target

[Service]
Type=simple
ExecStart={}
Restart=on-failure
RestartSec=5
Environment=DISPLAY=:0

[Install]
WantedBy=default.target
"#,
        exec_path.display()
    )
}

/// Generate the desktop entry content.
fn desktop_entry_content() -> String {
    let exec_path = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("/usr/bin/paste"));

    format!(
        r#"[Desktop Entry]
Name=Paste
Comment=Clipboard manager with text expansion for Linux
Exec={}
Icon=paste
Terminal=false
Type=Application
Categories=Utility;
StartupNotify=false
"#,
        exec_path.display()
    )
}

/// Check if the service is currently installed and enabled.
pub fn is_service_installed() -> bool {
    service_path().exists()
}

/// Check if the service is currently active (running).
pub fn is_service_active() -> bool {
    Command::new("systemctl")
        .args(["--user", "is-active", "--quiet", "paste.service"])
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Install and enable the systemd user service.
pub fn install_service() -> Result<String, String> {
    let path = service_path();

    // Create parent directory
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("Failed to create directory: {e}"))?;
    }

    // Write service file
    let content = service_content();
    std::fs::write(&path, &content).map_err(|e| format!("Failed to write service file: {e}"))?;
    info!("Service file written to {}", path.display());

    // Reload systemd
    let _ = Command::new("systemctl")
        .args(["--user", "daemon-reload"])
        .status();

    // Enable the service
    let status = Command::new("systemctl")
        .args(["--user", "enable", "paste.service"])
        .status()
        .map_err(|e| format!("Failed to enable service: {e}"))?;

    if !status.success() {
        warn!("systemctl enable exited with {}", status);
    }

    // Install desktop entry
    let desktop_path = desktop_entry_path();
    if let Some(parent) = desktop_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = std::fs::write(&desktop_path, desktop_entry_content());

    info!("Service installed and enabled");
    Ok(format!("Service installed at {}", path.display()))
}

/// Uninstall the systemd user service.
pub fn uninstall_service() -> Result<String, String> {
    // Stop the service
    let _ = Command::new("systemctl")
        .args(["--user", "stop", "paste.service"])
        .status();

    // Disable the service
    let _ = Command::new("systemctl")
        .args(["--user", "disable", "paste.service"])
        .status();

    // Remove service file
    let path = service_path();
    if path.exists() {
        std::fs::remove_file(&path).map_err(|e| format!("Failed to remove service file: {e}"))?;
    }

    // Reload systemd
    let _ = Command::new("systemctl")
        .args(["--user", "daemon-reload"])
        .status();

    // Remove desktop entry
    let desktop_path = desktop_entry_path();
    if desktop_path.exists() {
        let _ = std::fs::remove_file(&desktop_path);
    }

    info!("Service uninstalled");
    Ok("Service removed".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_service_content_has_required_fields() {
        let content = service_content();
        assert!(content.contains("[Unit]"));
        assert!(content.contains("[Service]"));
        assert!(content.contains("[Install]"));
        assert!(content.contains("Type=simple"));
        assert!(content.contains("Restart=on-failure"));
        assert!(content.contains("WantedBy=default.target"));
        assert!(content.contains("ExecStart="));
    }

    #[test]
    fn test_desktop_entry_content() {
        let content = desktop_entry_content();
        assert!(content.contains("[Desktop Entry]"));
        assert!(content.contains("Name=Paste"));
        assert!(content.contains("Type=Application"));
        assert!(content.contains("Terminal=false"));
    }

    #[test]
    fn test_service_path() {
        let path = service_path();
        assert!(path.to_string_lossy().contains("paste.service"));
        assert!(path.to_string_lossy().contains("systemd"));
    }

    #[test]
    fn test_desktop_entry_path() {
        let path = desktop_entry_path();
        assert!(path.to_string_lossy().contains("paste.desktop"));
        assert!(path.to_string_lossy().contains("applications"));
    }
}
