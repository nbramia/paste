//! Clipboard monitoring.
//!
//! One backend serves both display servers: `wayland` polls the CLIPBOARD
//! selection through xclip, which reads identically under X11 and XWayland.
//! An event-driven XFixes backend for native X11 existed here but was never
//! constructed and has been removed (#103); `git log` has it if the sub-second
//! capture latency it offered is ever wanted.

pub mod dedup;
pub mod detection;
pub mod stack;
pub mod types;
pub mod wayland;

use std::sync::mpsc;
use types::ClipItem;

/// Trait for clipboard monitoring backends.
///
/// Only `WaylandClipboard` implements this today. The trait stays because it
/// pins down what a backend owes the rest of the app, which is what a second
/// one would have to satisfy.
pub trait ClipboardBackend: Send + Sync {
    /// Start monitoring clipboard changes. Sends captured items to `tx`.
    /// This method spawns background threads and returns immediately.
    fn start_monitoring(&self, tx: mpsc::Sender<ClipItem>) -> Result<(), ClipboardError>;

    /// Set the system clipboard to the given content.
    // Backends only capture today; re-copying goes through the `copy_to_clipboard`
    // command, which shells out to xclip directly.
    #[allow(dead_code)]
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
    // Reserved for backends that report operation failures.
    #[allow(dead_code)]
    OperationFailed(String),
}
