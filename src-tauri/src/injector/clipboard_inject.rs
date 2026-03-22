use std::process::Command;
use std::thread;
use std::time::Duration;
use std::io::Write;

use log::{debug, warn};
use super::{Injector, InjectorError};

/// Rich clipboard content for multi-format paste.
pub struct RichContent {
    pub text: Option<String>,
    pub html: Option<String>,
    pub image_path: Option<String>,
}

/// Clipboard injection fallback.
///
/// Works by: save clipboard -> set clipboard to content -> simulate Ctrl+V -> restore clipboard.
pub struct ClipboardInjector {
    /// "wayland" or "x11"
    pub(crate) display_server: &'static str,
}

impl ClipboardInjector {
    pub fn wayland() -> Self {
        Self { display_server: "wayland" }
    }

    pub fn x11() -> Self {
        Self { display_server: "x11" }
    }

    /// Auto-detect display server for clipboard injector.
    pub fn auto_detect() -> Self {
        if std::env::var("WAYLAND_DISPLAY").is_ok() {
            Self::wayland()
        } else {
            Self::x11()
        }
    }
}

impl Injector for ClipboardInjector {
    fn inject_text(&self, text: &str) -> Result<(), InjectorError> {
        self.inject_via_clipboard(text)
    }

    fn inject_via_clipboard(&self, text: &str) -> Result<(), InjectorError> {
        match self.display_server {
            "wayland" => clipboard_inject_wayland(text, "ydotool"),
            "x11" => clipboard_inject_x11(text),
            _ => Err(InjectorError::Failed("Unknown display server".into())),
        }
    }

    fn send_backspaces(&self, count: usize) -> Result<(), InjectorError> {
        if count == 0 {
            return Ok(());
        }

        match self.display_server {
            "wayland" => {
                // Try wtype first, then ydotool
                if super::is_tool_available("wtype") {
                    for _ in 0..count {
                        Command::new("wtype").args(["-k", "BackSpace"]).status()?;
                    }
                    Ok(())
                } else if super::is_tool_available("ydotool") {
                    for _ in 0..count {
                        Command::new("ydotool").args(["key", "14:1", "14:0"]).status()?;
                    }
                    Ok(())
                } else {
                    Err(InjectorError::ToolNotFound(
                        "No tool available for sending backspaces on Wayland".into(),
                    ))
                }
            }
            "x11" => {
                if super::is_tool_available("xdotool") {
                    for _ in 0..count {
                        Command::new("xdotool")
                            .args(["key", "--clearmodifiers", "BackSpace"])
                            .status()?;
                    }
                    Ok(())
                } else {
                    Err(InjectorError::ToolNotFound(
                        "xdotool not found for sending backspaces on X11".into(),
                    ))
                }
            }
            _ => Err(InjectorError::Failed("Unknown display server".into())),
        }
    }

    fn name(&self) -> &'static str {
        "clipboard"
    }

    fn inject_rich(&self, content: &RichContent) -> Result<(), InjectorError> {
        match self.display_server {
            "wayland" => clipboard_inject_rich_wayland(content, "ydotool"),
            "x11" => clipboard_inject_rich_x11(content),
            _ => Err(InjectorError::Failed("Unknown display server".into())),
        }
    }
}

/// Clipboard injection for Wayland: wl-copy + key simulation via specified tool.
pub(crate) fn clipboard_inject_wayland(text: &str, key_tool: &str) -> Result<(), InjectorError> {
    debug!("Clipboard inject (Wayland): {} chars via {}", text.len(), key_tool);

    // 1. Save current clipboard
    let old_clipboard = Command::new("wl-paste")
        .args(["--no-newline"])
        .output()
        .ok()
        .and_then(|o| if o.status.success() { Some(o.stdout) } else { None });

    // 2. Set clipboard to new content
    let mut child = Command::new("wl-copy")
        .stdin(std::process::Stdio::piped())
        .spawn()?;

    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(text.as_bytes())?;
    }
    child.wait()?;

    // Brief delay for clipboard to settle
    thread::sleep(Duration::from_millis(50));

    // 3. Simulate Ctrl+V
    let status = match key_tool {
        "wtype" => Command::new("wtype")
            .args(["-M", "ctrl", "-k", "v", "-m", "ctrl"])
            .status()?,
        _ => Command::new("ydotool")
            // KEY_LEFTCTRL=29, KEY_V=47
            .args(["key", "29:1", "47:1", "47:0", "29:0"])
            .status()?,
    };

    if !status.success() {
        warn!("Ctrl+V simulation exited with {}", status);
    }

    // Brief delay for paste to complete
    thread::sleep(Duration::from_millis(100));

    // 4. Restore old clipboard
    if let Some(old) = old_clipboard {
        if !old.is_empty() {
            let mut restore = Command::new("wl-copy")
                .stdin(std::process::Stdio::piped())
                .spawn()?;
            if let Some(mut stdin) = restore.stdin.take() {
                let _ = stdin.write_all(&old);
            }
            let _ = restore.wait();
        }
    }

    Ok(())
}

/// Clipboard injection for X11: xclip + xdotool Ctrl+V.
pub(crate) fn clipboard_inject_x11(text: &str) -> Result<(), InjectorError> {
    debug!("Clipboard inject (X11): {} chars", text.len());

    // 1. Save current clipboard
    let old_clipboard = Command::new("xclip")
        .args(["-selection", "clipboard", "-o"])
        .output()
        .ok()
        .and_then(|o| if o.status.success() { Some(o.stdout) } else { None });

    // 2. Set clipboard to new content
    let mut child = Command::new("xclip")
        .args(["-selection", "clipboard"])
        .stdin(std::process::Stdio::piped())
        .spawn()?;

    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(text.as_bytes())?;
    }
    child.wait()?;

    // Brief delay
    thread::sleep(Duration::from_millis(50));

    // 3. Simulate Ctrl+V
    let status = Command::new("xdotool")
        .args(["key", "--clearmodifiers", "ctrl+v"])
        .status()?;

    if !status.success() {
        warn!("xdotool Ctrl+V exited with {}", status);
    }

    // Brief delay for paste to complete
    thread::sleep(Duration::from_millis(100));

    // 4. Restore old clipboard
    if let Some(old) = old_clipboard {
        if !old.is_empty() {
            let mut restore = Command::new("xclip")
                .args(["-selection", "clipboard"])
                .stdin(std::process::Stdio::piped())
                .spawn()?;
            if let Some(mut stdin) = restore.stdin.take() {
                let _ = stdin.write_all(&old);
            }
            let _ = restore.wait();
        }
    }

    Ok(())
}

/// Paste rich content via clipboard on Wayland.
/// Sets multiple MIME types so target apps get the best format.
pub(crate) fn clipboard_inject_rich_wayland(content: &RichContent, key_tool: &str) -> Result<(), InjectorError> {
    debug!("Rich clipboard inject (Wayland)");

    // 1. Save current clipboard
    let old_clipboard = Command::new("wl-paste")
        .args(["--no-newline"])
        .output()
        .ok()
        .and_then(|o| if o.status.success() { Some(o.stdout) } else { None });

    // 2. Set clipboard with appropriate MIME type
    if let Some(ref image_path) = content.image_path {
        // Image paste: set clipboard to image data
        let path = std::path::Path::new(image_path);
        if path.exists() {
            let status = Command::new("wl-copy")
                .args(["--type", "image/png"])
                .stdin(std::process::Stdio::from(
                    std::fs::File::open(path).map_err(|e| InjectorError::Failed(format!("Open image: {e}")))?
                ))
                .status()?;
            if !status.success() {
                warn!("wl-copy image failed");
            }
        }
    } else if let Some(ref html) = content.html {
        // HTML: set text/html which most apps prefer for rich content
        let mut child = Command::new("wl-copy")
            .args(["--type", "text/html"])
            .stdin(std::process::Stdio::piped())
            .spawn()?;
        if let Some(mut stdin) = child.stdin.take() {
            stdin.write_all(html.as_bytes())?;
        }
        child.wait()?;
    } else if let Some(ref text) = content.text {
        // Plain text only
        let mut child = Command::new("wl-copy")
            .stdin(std::process::Stdio::piped())
            .spawn()?;
        if let Some(mut stdin) = child.stdin.take() {
            stdin.write_all(text.as_bytes())?;
        }
        child.wait()?;
    } else {
        return Ok(()); // Nothing to paste
    }

    // 3. Brief delay for clipboard to settle
    thread::sleep(Duration::from_millis(50));

    // 4. Simulate Ctrl+V
    let status = match key_tool {
        "wtype" => Command::new("wtype")
            .args(["-M", "ctrl", "-k", "v", "-m", "ctrl"])
            .status()?,
        "ydotool" | _ => Command::new("ydotool")
            .args(["key", "29:1", "47:1", "47:0", "29:0"])
            .status()?,
    };
    if !status.success() {
        warn!("Ctrl+V simulation exited with {}", status);
    }

    // 5. Brief delay for paste to complete
    thread::sleep(Duration::from_millis(100));

    // 6. Restore old clipboard
    if let Some(old) = old_clipboard {
        if !old.is_empty() {
            let mut restore = Command::new("wl-copy")
                .stdin(std::process::Stdio::piped())
                .spawn()?;
            if let Some(mut stdin) = restore.stdin.take() {
                let _ = stdin.write_all(&old);
            }
            let _ = restore.wait();
        }
    }

    Ok(())
}

/// Paste rich content via clipboard on X11.
pub(crate) fn clipboard_inject_rich_x11(content: &RichContent) -> Result<(), InjectorError> {
    debug!("Rich clipboard inject (X11)");

    // 1. Save current clipboard
    let old_clipboard = Command::new("xclip")
        .args(["-selection", "clipboard", "-o"])
        .output()
        .ok()
        .and_then(|o| if o.status.success() { Some(o.stdout) } else { None });

    // 2. Set clipboard with appropriate type
    if let Some(ref image_path) = content.image_path {
        let path = std::path::Path::new(image_path);
        if path.exists() {
            let status = Command::new("xclip")
                .args(["-selection", "clipboard", "-target", "image/png", "-i", image_path])
                .status()?;
            if !status.success() {
                warn!("xclip image failed");
            }
        }
    } else if let Some(ref html) = content.html {
        // Set HTML via xclip
        let mut child = Command::new("xclip")
            .args(["-selection", "clipboard", "-target", "text/html"])
            .stdin(std::process::Stdio::piped())
            .spawn()?;
        if let Some(mut stdin) = child.stdin.take() {
            stdin.write_all(html.as_bytes())?;
        }
        child.wait()?;
    } else if let Some(ref text) = content.text {
        let mut child = Command::new("xclip")
            .args(["-selection", "clipboard"])
            .stdin(std::process::Stdio::piped())
            .spawn()?;
        if let Some(mut stdin) = child.stdin.take() {
            stdin.write_all(text.as_bytes())?;
        }
        child.wait()?;
    } else {
        return Ok(());
    }

    // 3. Simulate Ctrl+V
    thread::sleep(Duration::from_millis(50));
    let status = Command::new("xdotool")
        .args(["key", "--clearmodifiers", "ctrl+v"])
        .status()?;
    if !status.success() {
        warn!("xdotool Ctrl+V exited with {}", status);
    }

    // 4. Restore clipboard
    thread::sleep(Duration::from_millis(100));
    if let Some(old) = old_clipboard {
        if !old.is_empty() {
            let mut restore = Command::new("xclip")
                .args(["-selection", "clipboard"])
                .stdin(std::process::Stdio::piped())
                .spawn()?;
            if let Some(mut stdin) = restore.stdin.take() {
                let _ = stdin.write_all(&old);
            }
            let _ = restore.wait();
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clipboard_injector_wayland() {
        let injector = ClipboardInjector::wayland();
        assert_eq!(injector.name(), "clipboard");
        assert_eq!(injector.display_server, "wayland");
    }

    #[test]
    fn test_clipboard_injector_x11() {
        let injector = ClipboardInjector::x11();
        assert_eq!(injector.name(), "clipboard");
        assert_eq!(injector.display_server, "x11");
    }

    #[test]
    fn test_clipboard_injector_auto() {
        let injector = ClipboardInjector::auto_detect();
        assert_eq!(injector.name(), "clipboard");
        // display_server should be one of "wayland" or "x11"
        assert!(injector.display_server == "wayland" || injector.display_server == "x11");
    }

    #[test]
    fn test_send_zero_backspaces() {
        let injector = ClipboardInjector::wayland();
        // Zero backspaces should succeed (no-op)
        assert!(injector.send_backspaces(0).is_ok());
    }
}
