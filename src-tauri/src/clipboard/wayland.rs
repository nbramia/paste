use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::thread;

use log::{debug, error, info, warn};

use super::dedup::{ClipDedup, DedupResult};
use super::detection::{compute_hash, detect_text_content_type, ContentType};
use super::types::ClipItem;
use super::{ClipboardBackend, ClipboardError};
use crate::images;

/// Wayland clipboard backend using wl-paste.
pub struct WaylandClipboard {
    excluded_apps: Vec<String>,
    max_content_size_bytes: u64,
    merge_growing: bool,
    debounce_ms: u32,
    /// Size ceiling for image captures — `storage.max_image_size_mb`. Separate
    /// from the text ceiling because screenshots are routinely larger than any
    /// text anyone copies.
    max_image_size_bytes: u64,
    /// Where originals and thumbnails are written.
    image_dir: PathBuf,
}

impl WaylandClipboard {
    pub fn new(
        excluded_apps: Vec<String>,
        max_content_size_mb: u32,
        merge_growing: bool,
        debounce_ms: u32,
        max_image_size_mb: u32,
        image_dir: PathBuf,
    ) -> Self {
        Self {
            excluded_apps,
            max_content_size_bytes: max_content_size_mb as u64 * 1024 * 1024,
            merge_growing,
            debounce_ms,
            max_image_size_bytes: max_image_size_mb as u64 * 1024 * 1024,
            image_dir,
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
        let max_image_size_bytes = self.max_image_size_bytes;
        let image_dir = self.image_dir.clone();

        // One thread handles both text and images. There is no separate image
        // poller: `xclip -o` serves whatever the selection owner offers, so a
        // copied image arrives on this same read as non-UTF8 bytes (#122).
        // The old second loop is what made the GNOME trash icon bounce.
        thread::Builder::new()
            .name("clipboard-text".into())
            .spawn(move || {
                let monitor = WaylandClipboard {
                    excluded_apps: excluded,
                    max_content_size_bytes: max_size,
                    merge_growing,
                    debounce_ms,
                    max_image_size_bytes,
                    image_dir,
                };
                monitor_text_loop(&monitor, tx);
            })
            .map_err(ClipboardError::Io)?;

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
    /// Hash of the last content seen, not the content itself: an image can sit
    /// on the clipboard for hours and gets re-read every second, so keeping a
    /// copy would mean a multi-megabyte allocation per poll.
    last_seen: Option<String>,
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
        let seen = fingerprint(content);
        let changed = self.last_seen.as_deref() != Some(seen.as_str());
        self.last_seen = Some(seen);

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
                self.last_seen = Some(fingerprint(other));
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

/// Identify clipboard content by hash, ignoring trailing newlines.
fn fingerprint(content: &[u8]) -> String {
    compute_hash(normalized(content))
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
    // Images are deduped on their own hash rather than through ClipDedup,
    // which only reasons about text.
    let mut last_image_hash: Option<String> = None;

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

        // Non-UTF8 bytes mean a binary payload — in practice a copied image.
        // `xclip -o` serves whatever the selection owner offers regardless of
        // the target requested, so a screenshot arrives here rather than as an
        // empty read. Borrow rather than clone so a binary payload sitting on
        // the clipboard is not copied once a second.
        let Ok(text) = std::str::from_utf8(&content) else {
            if let Some(item) = capture_image(monitor, &content, &mut last_image_hash) {
                if tx.send(item).is_err() {
                    info!("Clipboard channel closed, stopping clipboard monitor");
                    return Ok(());
                }
            }
            continue;
        };

        if content.len() as u64 > monitor.max_content_size_bytes {
            continue;
        }

        let hash = compute_hash(&content);

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
        // The clipboard has moved on to text, so the same image copied again
        // later is a genuinely new capture rather than a repeat read.
        last_image_hash = None;

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

/// Capture an image sitting on the clipboard.
///
/// Called with the bytes the poll loop already read, so recognising an image
/// costs no extra subprocess — which is the point. The previous design ran a
/// second loop polling `wl-paste --type image/png` once a second, and that
/// rapid spawning is what made the GNOME trash icon bounce (decision 2 in
/// architecture.md). Capture was disabled outright rather than restructured,
/// so every copied screenshot was silently discarded (#122).
///
/// Returns `None` when the bytes are not an image, when the image is too
/// large, when it came from an excluded app, or when it is the same image the
/// previous poll already stored.
fn capture_image(
    monitor: &WaylandClipboard,
    content: &[u8],
    last_image_hash: &mut Option<String>,
) -> Option<ClipItem> {
    // Sniff the content: not every non-UTF8 selection is an image.
    let kind = images::detect_image_format(content)?;

    if content.len() as u64 > monitor.max_image_size_bytes {
        // Remember the hash anyway so an oversized image is reported once
        // rather than once a second for as long as it sits on the clipboard.
        let hash = compute_hash(content);
        if last_image_hash.as_deref() != Some(hash.as_str()) {
            debug!(
                "Skipping image: {} bytes exceeds storage.max_image_size_mb ({} bytes)",
                content.len(),
                monitor.max_image_size_bytes
            );
            *last_image_hash = Some(hash);
        }
        return None;
    }

    let hash = compute_hash(content);
    if last_image_hash.as_deref() == Some(hash.as_str()) {
        // Same image still on the clipboard; the poll loop sees it every second.
        return None;
    }

    let source_app = WaylandClipboard::detect_source_app();
    if monitor.is_excluded(&source_app) {
        debug!("Skipping image from excluded app: {source_app:?}");
        *last_image_hash = Some(hash);
        return None;
    }

    if let Err(e) = std::fs::create_dir_all(&monitor.image_dir) {
        error!(
            "Failed to create image directory {}: {e}",
            monitor.image_dir.display()
        );
        return None;
    }

    let image_id = uuid::Uuid::now_v7().to_string();
    let image_path = monitor
        .image_dir
        .join(format!("{image_id}.{}", kind.extension));

    if let Err(e) = std::fs::write(&image_path, content) {
        error!("Failed to write image file {}: {e}", image_path.display());
        return None;
    }

    // A thumbnail that fails to generate is not fatal — the original is
    // already stored and pasteable; the card just falls back to its icon.
    let thumb_path = images::thumbnail_path_for(&image_path);
    let dimensions = match images::write_thumbnail(content, &thumb_path) {
        Ok(dims) => Some(dims),
        Err(e) => {
            warn!("Failed to write thumbnail for {image_id}: {e}");
            None
        }
    };

    // Only claim the hash once the capture has actually succeeded, so a
    // transient write failure is retried on the next poll.
    *last_image_hash = Some(hash.clone());

    let mut metadata = serde_json::json!({
        "format": kind.extension,
        "mime": kind.mime,
        "size_bytes": content.len(),
    });
    if let Some((width, height)) = dimensions {
        metadata["width"] = serde_json::json!(width);
        metadata["height"] = serde_json::json!(height);
    }

    debug!(
        "Captured image clip: {} ({} bytes)",
        kind.extension,
        content.len()
    );

    Some(ClipItem {
        content_type: ContentType::Image.as_str().to_string(),
        text_content: None,
        html_content: None,
        image_path: Some(image_path.to_string_lossy().to_string()),
        source_app,
        content_hash: hash,
        content_size: content.len() as i64,
        metadata: Some(metadata.to_string()),
        replaces_previous: false,
    })
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
        let wl = WaylandClipboard::new(vec!["1password".into(), "keepassxc".into()], 10, true, 500, 10, PathBuf::from("/tmp/paste-test-images"));
        assert_eq!(wl.excluded_apps.len(), 2);
        assert_eq!(wl.max_content_size_bytes, 10 * 1024 * 1024);
    }

    #[test]
    fn test_is_excluded() {
        let wl = WaylandClipboard::new(vec!["1password".into(), "keepassxc".into()], 10, true, 500, 10, PathBuf::from("/tmp/paste-test-images"));
        assert!(wl.is_excluded(&Some("1Password".into())));
        assert!(wl.is_excluded(&Some("KeePassXC".into())));
        assert!(wl.is_excluded(&Some("org.keepassxc.KeePassXC".into())));
        assert!(!wl.is_excluded(&Some("firefox".into())));
        assert!(!wl.is_excluded(&None));
    }

    #[test]
    fn test_is_excluded_case_insensitive() {
        let wl = WaylandClipboard::new(vec!["Bitwarden".into()], 10, true, 500, 10, PathBuf::from("/tmp/paste-test-images"));
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

    /// A real 8x8 PNG — `capture_image` sniffs magic bytes and the thumbnail
    /// step actually decodes, so a fake byte string will not do.
    fn tiny_png() -> Vec<u8> {
        let mut buf = Vec::new();
        let img = image::RgbImage::from_fn(8, 8, |x, y| {
            image::Rgb([(x * 30) as u8, (y * 30) as u8, 120])
        });
        image::DynamicImage::ImageRgb8(img)
            .write_to(&mut std::io::Cursor::new(&mut buf), image::ImageFormat::Png)
            .unwrap();
        buf
    }

    fn image_monitor(dir: &std::path::Path, max_image_size_mb: u32) -> WaylandClipboard {
        WaylandClipboard::new(
            vec!["1password".into()],
            10,
            true,
            500,
            max_image_size_mb,
            dir.to_path_buf(),
        )
    }

    fn temp_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "paste-capture-{tag}-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn test_capture_image_stores_original_and_thumbnail() {
        let dir = temp_dir("stores");
        let monitor = image_monitor(&dir, 10);
        let mut last_hash = None;

        let item = capture_image(&monitor, &tiny_png(), &mut last_hash)
            .expect("a valid PNG should be captured");

        assert_eq!(item.content_type, "image");
        assert!(item.text_content.is_none());
        assert_eq!(item.content_size, tiny_png().len() as i64);
        assert!(!item.replaces_previous);

        let original = PathBuf::from(item.image_path.as_ref().unwrap());
        assert!(original.exists(), "original image written");
        assert_eq!(original.extension().unwrap(), "png");
        assert!(
            crate::images::thumbnail_path_for(&original).exists(),
            "thumbnail written beside the original"
        );

        let meta: serde_json::Value =
            serde_json::from_str(item.metadata.as_ref().unwrap()).unwrap();
        assert_eq!(meta["format"], "png");
        assert_eq!(meta["mime"], "image/png");
        assert_eq!(meta["width"], 8);
        assert_eq!(meta["height"], 8);

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_capture_image_dedups_the_same_image_across_polls() {
        let dir = temp_dir("dedup");
        let monitor = image_monitor(&dir, 10);
        let png = tiny_png();
        let mut last_hash = None;

        assert!(capture_image(&monitor, &png, &mut last_hash).is_some());
        // The poll loop sees the same image every second while it sits on the
        // clipboard; only the first poll may store it.
        assert!(capture_image(&monitor, &png, &mut last_hash).is_none());
        assert!(capture_image(&monitor, &png, &mut last_hash).is_none());

        let stored = std::fs::read_dir(&dir).unwrap().count();
        assert_eq!(stored, 2, "one original plus one thumbnail, not three pairs");

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_capture_image_ignores_non_image_bytes() {
        let dir = temp_dir("nonimage");
        let monitor = image_monitor(&dir, 10);
        let mut last_hash = None;

        // Latin-1 text is non-UTF8 and reaches this path, but is not an image.
        assert!(capture_image(&monitor, &[0xE9, 0xE8, 0xFC], &mut last_hash).is_none());
        assert!(last_hash.is_none(), "non-images must not claim the hash slot");
        assert_eq!(std::fs::read_dir(&dir).unwrap().count(), 0);

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_capture_image_respects_the_image_size_limit() {
        let dir = temp_dir("toobig");
        // 0 MB limit: any image is over it.
        let monitor = image_monitor(&dir, 0);
        let mut last_hash = None;

        assert!(capture_image(&monitor, &tiny_png(), &mut last_hash).is_none());
        assert!(
            last_hash.is_some(),
            "oversize image is remembered so it is logged once, not once a second"
        );
        assert_eq!(std::fs::read_dir(&dir).unwrap().count(), 0);

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_capture_image_stores_a_different_image_after_the_first() {
        let dir = temp_dir("second");
        let monitor = image_monitor(&dir, 10);
        let mut last_hash = None;

        assert!(capture_image(&monitor, &tiny_png(), &mut last_hash).is_some());

        let mut other = Vec::new();
        let img = image::RgbImage::from_fn(8, 8, |_, _| image::Rgb([1, 2, 3]));
        image::DynamicImage::ImageRgb8(img)
            .write_to(
                &mut std::io::Cursor::new(&mut other),
                image::ImageFormat::Png,
            )
            .unwrap();

        assert!(
            capture_image(&monitor, &other, &mut last_hash).is_some(),
            "a genuinely different image is a new capture"
        );
        assert_eq!(std::fs::read_dir(&dir).unwrap().count(), 4);

        std::fs::remove_dir_all(&dir).ok();
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
