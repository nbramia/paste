use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::thread;

use log::{debug, error, info, warn};

use super::dedup::{ClipDedup, DedupResult};
use super::detection::{compute_hash, detect_text_content_type, ContentType};
use super::types::ClipItem;
use super::{ClipboardBackend, ClipboardError};

/// Wayland clipboard backend using wl-paste.
pub struct WaylandClipboard {
    excluded_apps: Vec<String>,
    max_content_size_bytes: u64,
    merge_growing: bool,
    debounce_ms: u32,
}

impl WaylandClipboard {
    pub fn new(
        excluded_apps: Vec<String>,
        max_content_size_mb: u32,
        merge_growing: bool,
        debounce_ms: u32,
    ) -> Self {
        Self {
            excluded_apps,
            max_content_size_bytes: max_content_size_mb as u64 * 1024 * 1024,
            merge_growing,
            debounce_ms,
        }
    }

    /// Try to detect the currently focused application via compositor-specific tools.
    pub(crate) fn detect_source_app() -> Option<String> {
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

        // Try focused-window-dbus GNOME extension
        if let Ok(output) = Command::new("gdbus")
            .args([
                "call",
                "--session",
                "--dest",
                "org.gnome.Shell",
                "--object-path",
                "/org/gnome/shell/extensions/FocusedWindow",
                "--method",
                "org.gnome.shell.extensions.FocusedWindow.Get",
            ])
            .output()
        {
            if output.status.success() {
                if let Ok(text) = std::str::from_utf8(&output.stdout) {
                    // Returns: ('{"wm_class":"Firefox",...}',)
                    // Extract JSON from between single quotes, then parse wm_class
                    if let Some(start) = text.find('\'') {
                        if let Some(end) = text[start + 1..].find('\'') {
                            let json_str = &text[start + 1..start + 1 + end];
                            if let Ok(json) = serde_json::from_str::<serde_json::Value>(json_str) {
                                if let Some(wm_class) =
                                    json.get("wm_class").and_then(|v| v.as_str())
                                {
                                    if !wm_class.is_empty() {
                                        // Clean up the class name (e.g., "dev.warp.Warp" → "Warp")
                                        let name = wm_class.rsplit('.').next().unwrap_or(wm_class);
                                        return Some(name.to_string());
                                    }
                                }
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
        let merge_growing = self.merge_growing;
        let debounce_ms = self.debounce_ms;

        // Spawn text monitoring thread
        let tx_text = tx.clone();
        thread::Builder::new()
            .name("clipboard-text".into())
            .spawn(move || {
                let monitor = WaylandClipboard {
                    excluded_apps: excluded,
                    max_content_size_bytes: max_size,
                    merge_growing,
                    debounce_ms,
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
        let mut child = Command::new("wl-copy").stdin(Stdio::piped()).spawn()?;

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

    // wl-copy forks a daemon to serve the selection and only the foreground
    // process exits, so `wait()` returns while the daemon lives on. Without
    // null stdio that daemon inherits — and holds open — our stdout/stderr
    // for as long as it owns the clipboard.
    match Command::new("wl-copy")
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
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
            .stdout(Stdio::null())
            .stderr(Stdio::null())
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

/// Consecutive unchanged polls before the two readers are compared.
///
/// At a 1s poll interval this is one minute of an apparently idle clipboard,
/// so the cross-check costs at most one extra subprocess per minute and none
/// at all while the user is actively copying.
const STALE_CHECK_POLLS: u32 = 60;

/// A tool that can read the CLIPBOARD selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reader {
    /// `xclip`, reading the X11 selection (real X server or XWayland).
    Xclip,
    /// `wl-paste`, reading the Wayland selection directly.
    WlPaste,
}

impl Reader {
    pub fn tool(self) -> &'static str {
        match self {
            Reader::Xclip => "xclip",
            Reader::WlPaste => "wl-paste",
        }
    }

    /// The reader to compare against when checking for staleness.
    pub fn other(self) -> Reader {
        match self {
            Reader::Xclip => Reader::WlPaste,
            Reader::WlPaste => Reader::Xclip,
        }
    }
}

/// What a cross-check concluded.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CrossCheck {
    /// Both readers agree — the bridge is healthy.
    Agree,
    /// They disagree and we switched to the other reader. Its content is the
    /// real clipboard and should be processed as a capture.
    FailedOver,
    /// They agree again after a failover; switched back to the preferred
    /// reader. Nothing new to capture.
    Recovered,
}

/// Detects the XWayland clipboard bridge wedging, and fails over when it does.
///
/// The failure being guarded against is silent: `xclip` keeps exiting 0 and
/// keeps returning the same bytes while the real clipboard moves on, so every
/// poll looks like a duplicate and capture stops without a single log line
/// (#121). Comparing the two readers is the only way to tell "nobody has
/// copied anything" apart from "we have stopped being able to see copies".
///
/// Failing over is not optional. Substituting the other reader's content for
/// one poll while leaving the stale reader primary makes the next poll see the
/// stale bytes as new again, and the loop alternates between the two values
/// once a second.
pub struct StaleGuard {
    reader: Reader,
    preferred: Reader,
    last_seen: Option<Vec<u8>>,
    unchanged_polls: u32,
    interval: u32,
}

impl StaleGuard {
    pub fn new(preferred: Reader, interval: u32) -> Self {
        Self {
            reader: preferred,
            preferred,
            last_seen: None,
            unchanged_polls: 0,
            interval,
        }
    }

    pub fn reader(&self) -> Reader {
        self.reader
    }

    /// Record this poll's content from the current reader.
    ///
    /// Returns true when the clipboard has looked unchanged for long enough
    /// that it is worth asking the other reader for a second opinion.
    pub fn tick(&mut self, content: &[u8]) -> bool {
        let changed = self.last_seen.as_deref().map(normalized) != Some(normalized(content));
        self.last_seen = Some(content.to_vec());

        if changed {
            self.unchanged_polls = 0;
            return false;
        }

        self.unchanged_polls += 1;
        if self.unchanged_polls >= self.interval {
            self.unchanged_polls = 0;
            true
        } else {
            false
        }
    }

    /// Compare the current reader's content against the other reader's.
    ///
    /// Trailing newlines are ignored: `wl-paste --no-newline` strips one and
    /// `xclip -o` does not, so a bare difference in line endings is not
    /// evidence that anything is wrong.
    pub fn cross_check(&mut self, current: &[u8], other: &[u8]) -> CrossCheck {
        let agree = normalized(current) == normalized(other);

        match (agree, self.reader == self.preferred) {
            // Preferred reader disagrees with the other one: it has gone stale.
            (false, true) => {
                self.reader = self.reader.other();
                self.last_seen = Some(other.to_vec());
                self.unchanged_polls = 0;
                CrossCheck::FailedOver
            }
            // Already failed over and the preferred reader has caught up.
            (true, false) => {
                self.reader = self.preferred;
                self.unchanged_polls = 0;
                CrossCheck::Recovered
            }
            // Healthy, or still wedged and correctly staying failed over.
            _ => CrossCheck::Agree,
        }
    }
}

/// Strip trailing newlines so the two readers' conventions can be compared.
fn normalized(bytes: &[u8]) -> &[u8] {
    let mut end = bytes.len();
    while end > 0 && (bytes[end - 1] == b'\n' || bytes[end - 1] == b'\r') {
        end -= 1;
    }
    &bytes[..end]
}

/// Whether a command-line tool is present and runnable.
fn tool_available(tool: &str, version_flag: &str) -> bool {
    Command::new(tool)
        .arg(version_flag)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Read the CLIPBOARD selection as bytes, or `None` when it holds no text.
fn read_clipboard_text(reader: Reader) -> Option<Vec<u8>> {
    let output = match reader {
        Reader::Xclip => Command::new("xclip")
            .args(["-selection", "clipboard", "-o"])
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .output(),
        Reader::WlPaste => Command::new("wl-paste")
            .args(["--no-newline"])
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .output(),
    }
    .ok()?;

    if !output.status.success() || output.stdout.is_empty() {
        return None;
    }
    Some(output.stdout)
}

/// Poll-based clipboard watcher. Reads clipboard every 1s via `wl-paste`.
/// Only spawns a subprocess when checking — no persistent child process.
fn run_text_watcher(
    monitor: &WaylandClipboard,
    tx: &mpsc::Sender<ClipItem>,
) -> Result<(), ClipboardError> {
    let mut dedup = ClipDedup::new(monitor.merge_growing, monitor.debounce_ms);
    let mut last_content: Option<String> = None;
    let mut last_html: Option<String> = None;

    // xclip via XWayland is preferred — wl-paste subprocess visibility causes
    // desktop side-effects (e.g. trash icon bouncing) when polled at 1Hz.
    let has_xclip = tool_available("xclip", "-version");
    let has_wl_paste = tool_available("wl-paste", "--version");

    let primary = if has_xclip {
        Reader::Xclip
    } else {
        Reader::WlPaste
    };
    let mut guard = StaleGuard::new(primary, STALE_CHECK_POLLS);

    // Cross-checking needs both tools. With only one there is nothing to
    // compare against and the watcher behaves as it always did.
    let can_cross_check = has_xclip && has_wl_paste;
    info!(
        "Clipboard polling started (1s interval, reading via {}; staleness cross-check {})",
        primary.tool(),
        if can_cross_check {
            "enabled"
        } else {
            "unavailable (needs both xclip and wl-paste)"
        }
    );

    loop {
        thread::sleep(std::time::Duration::from_secs(1));

        let Some(mut content) = read_clipboard_text(guard.reader()) else {
            if let Some(ref content) = last_content {
                debug!("Clipboard lost — re-asserting");
                reassert_clipboard(content, last_html.as_deref());
            }
            continue;
        };

        // The bridge that mirrors the Wayland clipboard into XWayland can
        // wedge. When it does, xclip keeps succeeding and keeps returning the
        // same stale bytes while the real clipboard moves on — every poll is
        // then a `Duplicate`, which logs nothing, so capture goes dead for
        // days and the app looks healthy (#121). Detect it by comparing the
        // two readers whenever the current one has gone quiet, and fail over
        // to whichever one is telling the truth.
        if can_cross_check && guard.tick(&content) {
            let other = guard.reader().other();
            if let Some(other_content) = read_clipboard_text(other) {
                match guard.cross_check(&content, &other_content) {
                    CrossCheck::Agree => {}
                    CrossCheck::FailedOver => {
                        error!(
                            "Clipboard bridge stale: {} has been serving unchanged content that \
                             {} disagrees with. Failing over to {}; capture had been silently \
                             dropping every copy.",
                            other.other().tool(),
                            other.tool(),
                            other.tool()
                        );
                        content = other_content;
                    }
                    CrossCheck::Recovered => {
                        info!(
                            "Clipboard bridge recovered; reading via {} again",
                            guard.reader().tool()
                        );
                    }
                }
            }
        }

        if content.len() as u64 > monitor.max_content_size_bytes {
            continue;
        }

        let hash = compute_hash(&content);

        // Only text is handled on this path. Borrow rather than clone so a
        // binary payload sitting on the clipboard is not copied once a second.
        let Ok(text) = std::str::from_utf8(&content) else {
            debug!("Skipping non-UTF8 clipboard content");
            continue;
        };

        // Dedup decides all three dispositions: exact repeats (which every
        // poll sees while the clipboard is unchanged), a growing selection
        // superseding its own partial, and a rapid re-copy.
        let disposition = dedup.check(text);
        if disposition == DedupResult::Duplicate {
            continue;
        }
        let text = text.to_string();

        // Detect source app
        let source_app = WaylandClipboard::detect_source_app();

        // Check excluded apps
        if monitor.is_excluded(&source_app) {
            debug!("Skipping clipboard from excluded app: {:?}", source_app);
            continue;
        }

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
            replaces_previous: disposition == DedupResult::Replace,
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
// Dormant alongside the `Image` content type above.
#[allow(dead_code)]
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
            replaces_previous: false,
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
        let wl = WaylandClipboard::new(vec!["1password".into(), "keepassxc".into()], 10, true, 500);
        assert_eq!(wl.excluded_apps.len(), 2);
        assert_eq!(wl.max_content_size_bytes, 10 * 1024 * 1024);
    }

    #[test]
    fn test_is_excluded() {
        let wl = WaylandClipboard::new(vec!["1password".into(), "keepassxc".into()], 10, true, 500);
        assert!(wl.is_excluded(&Some("1Password".into())));
        assert!(wl.is_excluded(&Some("KeePassXC".into())));
        assert!(wl.is_excluded(&Some("org.keepassxc.KeePassXC".into())));
        assert!(!wl.is_excluded(&Some("firefox".into())));
        assert!(!wl.is_excluded(&None));
    }

    #[test]
    fn test_is_excluded_case_insensitive() {
        let wl = WaylandClipboard::new(vec!["Bitwarden".into()], 10, true, 500);
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
    fn test_normalized_ignores_trailing_newlines() {
        // wl-paste --no-newline strips one, xclip -o does not. A difference in
        // line endings alone must never read as a wedged bridge.
        assert_eq!(normalized(b"hello\n"), normalized(b"hello"));
        assert_eq!(normalized(b"hello\r\n"), normalized(b"hello"));
        assert_eq!(normalized(b"hello\n\n"), normalized(b"hello"));
        // Interior newlines are content and must be preserved.
        assert_ne!(normalized(b"a\nb"), normalized(b"ab"));
        assert_eq!(normalized(b""), b"");
        assert_eq!(normalized(b"\n\n"), b"");
    }

    #[test]
    fn test_reader_other_is_symmetric() {
        assert_eq!(Reader::Xclip.other(), Reader::WlPaste);
        assert_eq!(Reader::WlPaste.other(), Reader::Xclip);
        assert_eq!(Reader::Xclip.other().other(), Reader::Xclip);
    }

    #[test]
    fn test_tick_only_asks_for_a_second_opinion_after_a_quiet_interval() {
        let mut guard = StaleGuard::new(Reader::Xclip, 3);

        // First sighting is a change, not a quiet poll.
        assert!(!guard.tick(b"same"));
        assert!(!guard.tick(b"same"));
        assert!(!guard.tick(b"same"));
        // Third *unchanged* poll reaches the interval.
        assert!(guard.tick(b"same"));
        // Counter resets, so it does not fire again immediately.
        assert!(!guard.tick(b"same"));
    }

    #[test]
    fn test_tick_resets_when_the_clipboard_changes() {
        let mut guard = StaleGuard::new(Reader::Xclip, 3);
        guard.tick(b"a");
        guard.tick(b"a");
        guard.tick(b"a");
        // A change resets the quiet run, so the next poll is not a check.
        assert!(!guard.tick(b"b"));
        assert!(!guard.tick(b"b"));
        assert!(!guard.tick(b"b"));
        assert!(guard.tick(b"b"));
    }

    #[test]
    fn test_cross_check_fails_over_when_readers_disagree() {
        let mut guard = StaleGuard::new(Reader::Xclip, 3);
        assert_eq!(guard.reader(), Reader::Xclip);

        let verdict = guard.cross_check(b"stale value", b"what the user actually copied");
        assert_eq!(verdict, CrossCheck::FailedOver);
        assert_eq!(guard.reader(), Reader::WlPaste);
    }

    #[test]
    fn test_cross_check_is_quiet_while_healthy() {
        let mut guard = StaleGuard::new(Reader::Xclip, 3);
        assert_eq!(guard.cross_check(b"same", b"same"), CrossCheck::Agree);
        assert_eq!(guard.reader(), Reader::Xclip);
        // Newline conventions differ between the tools but mean agreement.
        assert_eq!(guard.cross_check(b"same\n", b"same"), CrossCheck::Agree);
        assert_eq!(guard.reader(), Reader::Xclip);
    }

    #[test]
    fn test_cross_check_stays_failed_over_while_still_wedged() {
        let mut guard = StaleGuard::new(Reader::Xclip, 3);
        guard.cross_check(b"stale", b"fresh");
        assert_eq!(guard.reader(), Reader::WlPaste);

        // wl-paste is primary now; xclip is still serving the stale value.
        let verdict = guard.cross_check(b"fresh", b"stale");
        assert_eq!(verdict, CrossCheck::Agree);
        assert_eq!(guard.reader(), Reader::WlPaste, "must not flap back");
    }

    #[test]
    fn test_cross_check_recovers_when_the_bridge_catches_up() {
        let mut guard = StaleGuard::new(Reader::Xclip, 3);
        guard.cross_check(b"stale", b"fresh");
        assert_eq!(guard.reader(), Reader::WlPaste);

        let verdict = guard.cross_check(b"agreed", b"agreed");
        assert_eq!(verdict, CrossCheck::Recovered);
        assert_eq!(guard.reader(), Reader::Xclip);
    }

    #[test]
    fn test_failover_does_not_ping_pong_between_readers() {
        // The bug this guards: capturing the other reader's value while
        // leaving the stale reader primary makes the stale bytes look new on
        // the very next poll, and the loop alternates once a second.
        let mut guard = StaleGuard::new(Reader::Xclip, 3);
        assert_eq!(
            guard.cross_check(b"stale", b"fresh"),
            CrossCheck::FailedOver
        );

        // After failover the guard's memory is the value it failed over to, so
        // polls of that same value count as quiet. If the failover value were
        // instead seen as a change, the first tick would reset the run and it
        // would take a fourth poll to reach the interval.
        assert!(!guard.tick(b"fresh"));
        assert!(!guard.tick(b"fresh"));
        assert!(
            guard.tick(b"fresh"),
            "failover value must count as unchanged, not as a fresh capture"
        );
        assert_eq!(guard.reader(), Reader::WlPaste);
    }

    #[test]
    fn test_guard_starting_on_wl_paste_treats_it_as_preferred() {
        // On a box without xclip, wl-paste is primary and there is nothing to
        // fail back to.
        let mut guard = StaleGuard::new(Reader::WlPaste, 2);
        assert_eq!(guard.reader(), Reader::WlPaste);
        assert_eq!(
            guard.cross_check(b"stale", b"fresh"),
            CrossCheck::FailedOver
        );
        assert_eq!(guard.reader(), Reader::Xclip);
        assert_eq!(guard.cross_check(b"same", b"same"), CrossCheck::Recovered);
        assert_eq!(guard.reader(), Reader::WlPaste);
    }

    // There is deliberately no test for `reassert_clipboard`. The previous
    // one asserted only "does not panic" while really spawning wl-copy: on any
    // machine with a compositor it replaced the developer's clipboard with
    // "test content" and left a wl-copy daemon holding the harness's stdout,
    // which hangs `cargo test` whenever stdout is a pipe. Per the testing
    // strategy in architecture.md, anything needing real clipboard access is
    // mocked; covering this properly means making the subprocess injectable,
    // which is worth more than the "no panic" assertion was.
}
