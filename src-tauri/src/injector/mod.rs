//! Text injection via xdotool (X11), ydotool (Wayland), or wtype (wlroots).

mod clipboard_inject;
mod xdotool;
mod ydotool;
mod wtype;

use std::process::Command;
use log::{info, warn};

pub use clipboard_inject::ClipboardInjector;
pub use xdotool::XdotoolInjector;
pub use ydotool::YdotoolInjector;
pub use wtype::WtypeInjector;

/// Trait for text injection backends.
pub trait Injector: Send + Sync {
    /// Inject text at the current cursor position by simulating typing.
    fn inject_text(&self, text: &str) -> Result<(), InjectorError>;

    /// Inject text via the clipboard: save current clipboard, set content,
    /// simulate Ctrl+V, restore original clipboard.
    fn inject_via_clipboard(&self, text: &str) -> Result<(), InjectorError>;

    /// Send N backspace key presses (for text expander abbreviation deletion).
    fn send_backspaces(&self, count: usize) -> Result<(), InjectorError>;

    /// The name of this injector backend.
    fn name(&self) -> &'static str;
}

/// Errors from text injection operations.
#[derive(Debug, thiserror::Error)]
pub enum InjectorError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Injection tool not found: {0}")]
    ToolNotFound(String),
    #[error("Injection failed: {0}")]
    Failed(String),
}

/// Check if a command-line tool is available on PATH.
fn is_tool_available(tool: &str) -> bool {
    Command::new("which")
        .arg(tool)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Select the best available injector based on display server and available tools.
///
/// If `method` is not "auto", it forces the specified backend.
pub fn select_injector(method: &str) -> Result<Box<dyn Injector>, InjectorError> {
    match method {
        "xdotool" => {
            if !is_tool_available("xdotool") {
                return Err(InjectorError::ToolNotFound(
                    "xdotool not found. Install: sudo apt install xdotool".into(),
                ));
            }
            info!("Using xdotool injector (forced)");
            Ok(Box::new(XdotoolInjector))
        }
        "ydotool" => {
            if !is_tool_available("ydotool") {
                return Err(InjectorError::ToolNotFound(
                    "ydotool not found. Install: sudo apt install ydotool".into(),
                ));
            }
            info!("Using ydotool injector (forced)");
            Ok(Box::new(YdotoolInjector))
        }
        "wtype" => {
            if !is_tool_available("wtype") {
                return Err(InjectorError::ToolNotFound(
                    "wtype not found. Install: sudo apt install wtype".into(),
                ));
            }
            info!("Using wtype injector (forced)");
            Ok(Box::new(WtypeInjector))
        }
        "clipboard" => {
            info!("Using clipboard injector (forced)");
            Ok(Box::new(ClipboardInjector::auto_detect()))
        }
        "auto" => auto_detect_injector(),
        _ => auto_detect_injector(),
    }
}

/// Auto-detect the best injector based on display server and tool availability.
fn auto_detect_injector() -> Result<Box<dyn Injector>, InjectorError> {
    let is_wayland = std::env::var("WAYLAND_DISPLAY").is_ok();

    if is_wayland {
        // Wayland: prefer wtype (wlroots), fall back to ydotool, then clipboard
        if is_tool_available("wtype") {
            info!("Auto-detected: wtype (Wayland/wlroots)");
            return Ok(Box::new(WtypeInjector));
        }
        if is_tool_available("ydotool") {
            info!("Auto-detected: ydotool (Wayland)");
            return Ok(Box::new(YdotoolInjector));
        }
        warn!("No typing injector available on Wayland, using clipboard fallback");
        Ok(Box::new(ClipboardInjector::wayland()))
    } else {
        // X11: prefer xdotool, fall back to clipboard
        if is_tool_available("xdotool") {
            info!("Auto-detected: xdotool (X11)");
            return Ok(Box::new(XdotoolInjector));
        }
        warn!("xdotool not found on X11, using clipboard fallback");
        Ok(Box::new(ClipboardInjector::x11()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_select_injector_auto() {
        // Auto should succeed (falls back to clipboard if nothing else available)
        let injector = select_injector("auto");
        assert!(injector.is_ok());
    }

    #[test]
    fn test_select_injector_clipboard() {
        let injector = select_injector("clipboard").unwrap();
        assert_eq!(injector.name(), "clipboard");
    }

    #[test]
    fn test_select_injector_unknown_falls_to_auto() {
        // Unknown method falls through to auto
        let injector = select_injector("something_weird");
        assert!(injector.is_ok());
    }

    #[test]
    fn test_is_tool_available_nonexistent() {
        assert!(!is_tool_available("definitely_not_a_real_tool_12345"));
    }

    #[test]
    fn test_is_tool_available_which() {
        // 'which' itself should be available
        assert!(is_tool_available("which"));
    }
}
