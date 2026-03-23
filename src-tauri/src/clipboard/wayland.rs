use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::thread;

use log::{debug, error, info, warn};

use super::detection::{compute_hash, detect_text_content_type, ContentType};
use super::types::ClipItem;
use super::{ClipboardBackend, ClipboardError};

/// Wayland clipboard backend using wl-paste.
pub struct WaylandClipboard {
    excluded_apps: Vec<String>,
    max_content_size_bytes: u64,
}

impl WaylandClipboard {
    pub fn new(excluded_apps: Vec<String>, max_content_size_mb: u32) -> Self {
        Self {
            excluded_apps,
            max_content_size_bytes: max_content_size_mb as u64 * 1024 * 1024,
        }
    }

    /// Try to detect the currently focused application via compositor-specific tools.
    fn detect_source_app() -> Option<String> {
        // Try hyprctl first (Hyprland)
        if let Ok(output) = Command::new("hyprctl")
            .args(["activewindow", "-j"])
            .output()
        {
            if output.status.success() {
                if let Ok(text) = std::str::from_utf8(&output.stdout) {
                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(text) {
                        if let Some(class) = json.get("class").and_then(|v| v.as_str()) {
                            if !class.is_empty() {
                                return Some(class.to_string());
                            }
                        }
                    }
                }
            }
        }

        // Try swaymsg (Sway)
        if let Ok(output) = Command::new("swaymsg")
            .args(["-t", "get_tree", "--raw"])
            .output()
        {
            if output.status.success() {
                if let Ok(text) = std::str::from_utf8(&output.stdout) {
                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(text) {
                        if let Some(app) = find_focused_sway(&json) {
                            return Some(app);
                        }
                    }
                }
            }
        }

        // Try gdbus for GNOME
        if let Ok(output) = Command::new("gdbus")
            .args([
                "call",
                "--session",
                "--dest", "org.gnome.Shell",
                "--object-path", "/org/gnome/Shell",
                "--method", "org.gnome.Shell.Eval",
                "global.display.focus_window ? global.display.focus_window.get_wm_class() : ''",
            ])
            .output()
        {
            if output.status.success() {
                if let Ok(text) = std::str::from_utf8(&output.stdout) {
                    // gdbus returns: (true, 'ClassName')
                    // Extract the class name from between quotes
                    if let Some(start) = text.find('\'') {
                        if let Some(end) = text[start + 1..].find('\'') {
                            let class = &text[start + 1..start + 1 + end];
                            if !class.is_empty() {
                                return Some(class.to_string());
                            }
                        }
                    }
                }
            }
        }

        None
    }

    /// Read the HTML representation of the current clipboard, if available.
    fn read_html_content() -> Option<String> {
        let output = Command::new("wl-paste")
            .args(["--no-newline", "--type", "text/html"])
            .output()
            .ok()?;

        if output.status.success() {
            let html = String::from_utf8_lossy(&output.stdout).to_string();
            if !html.is_empty() {
                return Some(html);
            }
        }
        None
    }

    /// Check if the source app should be excluded.
    fn is_excluded(&self, source_app: &Option<String>) -> bool {
        if let Some(ref app) = source_app {
            let app_lower = app.to_lowercase();
            self.excluded_apps
                .iter()
                .any(|excluded| app_lower.contains(&excluded.to_lowercase()))
        } else {
            false
        }
    }
}

impl ClipboardBackend for WaylandClipboard {
    fn start_monitoring(&self, tx: mpsc::Sender<ClipItem>) -> Result<(), ClipboardError> {
        // Check that wl-paste is available
        if Command::new("wl-paste").arg("--version").output().is_err() {
            return Err(ClipboardError::ToolNotFound(
                "wl-paste not found. Install wl-clipboard: sudo apt install wl-clipboard".into(),
            ));
        }

        info!("Starting Wayland clipboard monitoring");

        let excluded = self.excluded_apps.clone();
        let max_size = self.max_content_size_bytes;

        // Spawn text monitoring thread
        let tx_text = tx.clone();
        thread::Builder::new()
            .name("clipboard-text".into())
            .spawn(move || {
                let monitor = WaylandClipboard {
                    excluded_apps: excluded,
                    max_content_size_bytes: max_size,
                };
                monitor_text_loop(&monitor, tx_text);
            })
            .map_err(ClipboardError::Io)?;

        // Image monitoring disabled — wl-paste --type image/png polling
        // causes desktop side-effects on some compositors.
        // TODO: re-enable with event-driven approach
        drop(tx); // drop the sender clone for images

        Ok(())
    }

    fn set_clipboard(&self, content: &str) -> Result<(), ClipboardError> {
        let mut child = Command::new("wl-copy")
            .stdin(Stdio::piped())
            .spawn()?;

        if let Some(mut stdin) = child.stdin.take() {
            use std::io::Write;
            stdin.write_all(content.as_bytes())?;
        }

        child.wait()?;
        Ok(())
    }
}

/// Main loop for monitoring text clipboard changes.
fn monitor_text_loop(monitor: &WaylandClipboard, tx: mpsc::Sender<ClipItem>) {
    loop {
        let result = run_text_watcher(monitor, &tx);
        if let Err(e) = result {
            error!("Clipboard watcher error: {e}. Restarting in 5s...");
            thread::sleep(std::time::Duration::from_secs(5));
        }
    }
}

/// Re-assert clipboard content via wl-copy after the source app closed.
/// This preserves clipboard content on Wayland where closing the owner
/// app normally clears the clipboard.
fn reassert_clipboard(content: &str, html: Option<&str>) {
    use std::io::Write;

    // Re-assert plain text
    match Command::new("wl-copy")
        .stdin(Stdio::piped())
        .spawn()
    {
        Ok(mut child) => {
            if let Some(mut stdin) = child.stdin.take() {
                let _ = stdin.write_all(content.as_bytes());
            }
            let _ = child.wait();
        }
        Err(e) => {
            warn!("Failed to re-assert clipboard text: {e}");
        }
    }

    // Re-assert HTML if available
    if let Some(html_content) = html {
        match Command::new("wl-copy")
            .args(["--type", "text/html"])
            .stdin(Stdio::piped())
            .spawn()
        {
            Ok(mut child) => {
                if let Some(mut stdin) = child.stdin.take() {
                    let _ = stdin.write_all(html_content.as_bytes());
                }
                let _ = child.wait();
            }
            Err(e) => {
                debug!("Failed to re-assert clipboard HTML: {e}");
            }
        }
    }
}

/// Poll-based clipboard watcher. Reads clipboard every 1s via `wl-paste`.
/// Only spawns a subprocess when checking — no persistent child process.
fn run_text_watcher(
    monitor: &WaylandClipboard,
    tx: &mpsc::Sender<ClipItem>,
) -> Result<(), ClipboardError> {
    let mut last_hash: Option<String> = None;
    let mut last_content: Option<String> = None;
    let mut last_html: Option<String> = None;

    // Use xclip via XWayland — avoids wl-paste subprocess visibility
    // issues that cause desktop side-effects (e.g., trash icon bouncing)
    let use_xclip = Command::new("xclip").arg("-version")
        .stdout(Stdio::null()).stderr(Stdio::null())
        .status().map(|s| s.success()).unwrap_or(false);

    let tool = if use_xclip { "xclip" } else { "wl-paste" };
    info!("Clipboard polling started (1s interval, using {tool})");

    loop {
        thread::sleep(std::time::Duration::from_secs(1));

        let output = if use_xclip {
            match Command::new("xclip")
                .args(["-selection", "clipboard", "-o"])
                .stdout(Stdio::piped())
                .stderr(Stdio::null())
                .output()
            {
                Ok(o) => o,
                Err(_) => continue,
            }
        } else {
            match Command::new("wl-paste")
                .args(["--no-newline"])
                .stdout(Stdio::piped())
                .stderr(Stdio::null())
                .output()
            {
                Ok(o) => o,
                Err(_) => continue,
            }
        };

        if !output.status.success() || output.stdout.is_empty() {
            if let Some(ref content) = last_content {
                debug!("Clipboard lost — re-asserting");
                reassert_clipboard(content, last_html.as_deref());
            }
            continue;
        }

        let content = output.stdout;

        if content.len() as u64 > monitor.max_content_size_bytes {
            continue;
        }

        let hash = compute_hash(&content);

        if last_hash.as_ref() == Some(&hash) {
            continue;
        }

        last_hash = Some(hash.clone());

        // Detect source app
        let source_app = WaylandClipboard::detect_source_app();

        // Check excluded apps
        if monitor.is_excluded(&source_app) {
            debug!("Skipping clipboard from excluded app: {:?}", source_app);
            continue;
        }

        // Convert to string
        let text = match String::from_utf8(content.clone()) {
            Ok(s) => s,
            Err(_) => {
                debug!("Skipping non-UTF8 clipboard content");
                continue;
            }
        };

        // Detect content type
        let content_type = detect_text_content_type(&text);

        // Try to get HTML representation
        let html_content = WaylandClipboard::read_html_content();

        // Update last known content for clipboard persistence
        last_content = Some(text.clone());
        last_html = html_content.clone();

        // Build metadata for links
        let metadata = if content_type == ContentType::Link {
            Some(serde_json::json!({ "url": text.trim() }).to_string())
        } else {
            None
        };

        let item = ClipItem {
            content_type: content_type.as_str().to_string(),
            text_content: Some(text),
            html_content,
            image_path: None,
            source_app,
            content_hash: hash,
            content_size: content.len() as i64,
            metadata,
        };

        debug!(
            "Captured text clip: type={}, size={}",
            item.content_type, item.content_size
        );

        if tx.send(item).is_err() {
            info!("Clipboard channel closed, stopping text monitor");
            return Ok(());
        }
    }
}

/// Main loop for monitoring image clipboard changes.
fn monitor_image_loop(monitor: &WaylandClipboard, tx: mpsc::Sender<ClipItem>) {
    let mut last_hash: Option<String> = None;

    loop {
        // Read current clipboard as image
        let output = match Command::new("wl-paste")
            .args(["--no-newline", "--type", "image/png"])
            .output()
        {
            Ok(o) => o,
            Err(e) => {
                error!("Failed to run wl-paste for image: {e}");
                thread::sleep(std::time::Duration::from_secs(2));
                continue;
            }
        };

        if !output.status.success() || output.stdout.is_empty() {
            thread::sleep(std::time::Duration::from_secs(1));
            last_hash = None;
            continue;
        }

        let content = &output.stdout;

        // Skip images larger than max size
        if content.len() as u64 > monitor.max_content_size_bytes {
            debug!("Skipping image: too large ({} bytes)", content.len());
            thread::sleep(std::time::Duration::from_secs(1));
            continue;
        }

        let hash = compute_hash(content);

        if last_hash.as_ref() == Some(&hash) {
            thread::sleep(std::time::Duration::from_secs(1));
            continue;
        }

        last_hash = Some(hash.clone());

        // Detect source app
        let source_app = WaylandClipboard::detect_source_app();

        if monitor.is_excluded(&source_app) {
            debug!("Skipping image from excluded app: {:?}", source_app);
            continue;
        }

        // Save image to data directory
        let image_dir = dirs::data_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("/tmp"))
            .join("paste")
            .join("images");

        if let Err(e) = std::fs::create_dir_all(&image_dir) {
            error!("Failed to create image directory: {e}");
            continue;
        }

        let image_id = uuid::Uuid::now_v7().to_string();
        let image_path = image_dir.join(format!("{image_id}.png"));

        if let Err(e) = std::fs::write(&image_path, content) {
            error!("Failed to write image file: {e}");
            continue;
        }

        let metadata = serde_json::json!({
            "format": "png",
            "size_bytes": content.len(),
        })
        .to_string();

        let item = ClipItem {
            content_type: "image".to_string(),
            text_content: None,
            html_content: None,
            image_path: Some(image_path.to_string_lossy().to_string()),
            source_app,
            content_hash: hash,
            content_size: content.len() as i64,
            metadata: Some(metadata),
        };

        debug!("Captured image clip: size={}", item.content_size);

        if tx.send(item).is_err() {
            info!("Clipboard channel closed, stopping image monitor");
            return;
        }

        thread::sleep(std::time::Duration::from_secs(1));
    }
}

/// Recursively find the focused window class in a Sway tree JSON.
fn find_focused_sway(node: &serde_json::Value) -> Option<String> {
    if node.get("focused").and_then(|v| v.as_bool()) == Some(true) {
        if let Some(app_id) = node.get("app_id").and_then(|v| v.as_str()) {
            if !app_id.is_empty() {
                return Some(app_id.to_string());
            }
        }
        if let Some(props) = node.get("window_properties") {
            if let Some(class) = props.get("class").and_then(|v| v.as_str()) {
                return Some(class.to_string());
            }
        }
    }

    if let Some(nodes) = node.get("nodes").and_then(|v| v.as_array()) {
        for child in nodes {
            if let Some(app) = find_focused_sway(child) {
                return Some(app);
            }
        }
    }
    if let Some(nodes) = node.get("floating_nodes").and_then(|v| v.as_array()) {
        for child in nodes {
            if let Some(app) = find_focused_sway(child) {
                return Some(app);
            }
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wayland_clipboard_new() {
        let wl = WaylandClipboard::new(vec!["1password".into(), "keepassxc".into()], 10);
        assert_eq!(wl.excluded_apps.len(), 2);
        assert_eq!(wl.max_content_size_bytes, 10 * 1024 * 1024);
    }

    #[test]
    fn test_is_excluded() {
        let wl = WaylandClipboard::new(vec!["1password".into(), "keepassxc".into()], 10);
        assert!(wl.is_excluded(&Some("1Password".into())));
        assert!(wl.is_excluded(&Some("KeePassXC".into())));
        assert!(wl.is_excluded(&Some("org.keepassxc.KeePassXC".into())));
        assert!(!wl.is_excluded(&Some("firefox".into())));
        assert!(!wl.is_excluded(&None));
    }

    #[test]
    fn test_is_excluded_case_insensitive() {
        let wl = WaylandClipboard::new(vec!["Bitwarden".into()], 10);
        assert!(wl.is_excluded(&Some("bitwarden".into())));
        assert!(wl.is_excluded(&Some("BITWARDEN".into())));
        assert!(wl.is_excluded(&Some("Bitwarden".into())));
    }

    #[test]
    fn test_find_focused_sway_simple() {
        let json: serde_json::Value = serde_json::json!({
            "focused": true,
            "app_id": "firefox",
            "nodes": [],
            "floating_nodes": []
        });
        assert_eq!(find_focused_sway(&json), Some("firefox".into()));
    }

    #[test]
    fn test_find_focused_sway_nested() {
        let json: serde_json::Value = serde_json::json!({
            "focused": false,
            "nodes": [
                {
                    "focused": false,
                    "nodes": [
                        {
                            "focused": true,
                            "app_id": "kitty",
                            "nodes": [],
                            "floating_nodes": []
                        }
                    ],
                    "floating_nodes": []
                }
            ],
            "floating_nodes": []
        });
        assert_eq!(find_focused_sway(&json), Some("kitty".into()));
    }

    #[test]
    fn test_find_focused_sway_not_found() {
        let json: serde_json::Value = serde_json::json!({
            "focused": false,
            "nodes": [],
            "floating_nodes": []
        });
        assert_eq!(find_focused_sway(&json), None);
    }

    #[test]
    fn test_find_focused_sway_window_properties() {
        let json: serde_json::Value = serde_json::json!({
            "focused": true,
            "window_properties": {
                "class": "Google-chrome"
            },
            "nodes": [],
            "floating_nodes": []
        });
        assert_eq!(find_focused_sway(&json), Some("Google-chrome".into()));
    }

    #[test]
    fn test_reassert_clipboard_does_not_panic() {
        // Verify the function doesn't panic with valid input.
        // wl-copy may not be available in CI, but the function
        // handles spawn failures gracefully via warn!/debug! logs.
        reassert_clipboard("test content", None);
        reassert_clipboard("test content", Some("<b>test</b>"));
    }
}
