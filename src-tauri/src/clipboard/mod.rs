//! Clipboard monitoring for X11 (XFixes) and Wayland (wl-paste).

pub mod detection;
pub mod types;
pub mod wayland;

use std::sync::mpsc;
use types::ClipItem;

/// Display server type detected at runtime.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DisplayServer {
    Wayland,
    X11,
}

/// Detect the active display server from environment variables.
pub fn detect_display_server() -> DisplayServer {
    if std::env::var("WAYLAND_DISPLAY").is_ok() {
        DisplayServer::Wayland
    } else if std::env::var("DISPLAY").is_ok() {
        DisplayServer::X11
    } else {
        // Default to Wayland on modern Linux
        log::warn!("No display server detected via env vars, defaulting to Wayland");
        DisplayServer::Wayland
    }
}

/// Trait for clipboard monitoring backends.
pub trait ClipboardBackend: Send + Sync {
    /// Start monitoring clipboard changes. Sends captured items to `tx`.
    /// This method spawns background threads and returns immediately.
    fn start_monitoring(&self, tx: mpsc::Sender<ClipItem>) -> Result<(), ClipboardError>;

    /// Set the system clipboard to the given content.
    fn set_clipboard(&self, content: &str) -> Result<(), ClipboardError>;
}

/// Errors from clipboard operations.
#[derive(Debug, thiserror::Error)]
pub enum ClipboardError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Clipboard tool not found: {0}")]
    ToolNotFound(String),
    #[error("Clipboard operation failed: {0}")]
    OperationFailed(String),
}
