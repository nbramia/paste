//! X11 clipboard monitoring via XFixes extension.

use std::io::Write;
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use log::{debug, error, info, warn};
use x11rb::connection::Connection;
use x11rb::protocol::xfixes::{ConnectionExt as XFixesExt, SelectionEventMask};
use x11rb::protocol::xproto::{self, Atom, AtomEnum, ConnectionExt, Window};
use x11rb::protocol::Event;
use x11rb::rust_connection::RustConnection;

use super::detection::{compute_hash, detect_text_content_type, ContentType};
use super::types::ClipItem;
use super::{ClipboardBackend, ClipboardError};

/// Check if a source app should be excluded based on a case-insensitive
/// substring match against the exclusion list.
fn is_app_excluded(excluded_apps: &[String], source_app: &Option<String>) -> bool {
    if let Some(ref app) = source_app {
        let app_lower = app.to_lowercase();
        excluded_apps
            .iter()
            .any(|excluded| app_lower.contains(&excluded.to_lowercase()))
    } else {
        false
    }
}

/// X11 clipboard backend using XFixes for event-driven monitoring.
pub struct X11Clipboard {
    excluded_apps: Vec<String>,
    max_content_size_bytes: u64,
    monitor_primary: bool,
}

impl X11Clipboard {
    pub fn new(excluded_apps: Vec<String>, max_content_size_mb: u32, monitor_primary: bool) -> Self {
        Self {
            excluded_apps,
            max_content_size_bytes: max_content_size_mb as u64 * 1024 * 1024,
            monitor_primary,
        }
    }

    /// Check if the source app should be excluded.
    pub fn is_excluded(&self, source_app: &Option<String>) -> bool {
        is_app_excluded(&self.excluded_apps, source_app)
    }
}

impl ClipboardBackend for X11Clipboard {
    fn start_monitoring(&self, tx: mpsc::Sender<ClipItem>) -> Result<(), ClipboardError> {
        info!("Starting X11 clipboard monitoring");

        let excluded = self.excluded_apps.clone();
        let max_size = self.max_content_size_bytes;
        let monitor_primary = self.monitor_primary;

        thread::Builder::new()
            .name("clipboard-x11".into())
            .spawn(move || {
                let monitor = X11ClipboardMonitor {
                    excluded_apps: excluded,
                    max_content_size_bytes: max_size,
                    monitor_primary,
                };
                monitor.run_loop(tx);
            })
            .map_err(ClipboardError::Io)?;

        Ok(())
    }

    fn set_clipboard(&self, content: &str) -> Result<(), ClipboardError> {
        let mut child = Command::new("xclip")
            .args(["-selection", "clipboard"])
            .stdin(Stdio::piped())
            .spawn()
            .map_err(|_| {
                ClipboardError::ToolNotFound(
                    "xclip not found. Install it: sudo apt install xclip".into(),
                )
            })?;

        if let Some(mut stdin) = child.stdin.take() {
            stdin.write_all(content.as_bytes())?;
        }

        child.wait()?;
        Ok(())
    }
}

/// Internal monitor that runs the X11 event loop.
struct X11ClipboardMonitor {
    excluded_apps: Vec<String>,
    max_content_size_bytes: u64,
    monitor_primary: bool,
}

impl X11ClipboardMonitor {
    /// Main loop — connects to X11 and monitors clipboard changes.
    /// Automatically reconnects on failure.
    fn run_loop(&self, tx: mpsc::Sender<ClipItem>) {
        loop {
            info!("Connecting to X11 display");
            match self.monitor(&tx) {
                Ok(()) => {
                    info!("X11 clipboard monitor exited normally");
                    return;
                }
                Err(e) => {
                    error!("X11 clipboard monitor error: {e}");
                    warn!("Reconnecting in 2 seconds...");
                    thread::sleep(Duration::from_secs(2));
                }
            }
        }
    }

    /// Connect to X11, set up XFixes, and process events.
    fn monitor(&self, tx: &mpsc::Sender<ClipItem>) -> Result<(), ClipboardError> {
        let (conn, screen_num) = RustConnection::connect(None).map_err(|e| {
            ClipboardError::OperationFailed(format!("Failed to connect to X11: {e}"))
        })?;

        let screen = &conn.setup().roots[screen_num];
        let root = screen.root;

        // Initialize XFixes extension
        conn.xfixes_query_version(5, 0)
            .map_err(|e| ClipboardError::OperationFailed(format!("XFixes query failed: {e}")))?
            .reply()
            .map_err(|e| {
                ClipboardError::OperationFailed(format!("XFixes not available: {e}"))
            })?;

        // Create a window to receive events
        let win = conn.generate_id().map_err(|e| {
            ClipboardError::OperationFailed(format!("Failed to generate window ID: {e}"))
        })?;

        conn.create_window(
            0, // depth: copy from parent
            win,
            root,
            0,
            0,
            1,
            1,
            0,
            xproto::WindowClass::INPUT_ONLY,
            0, // visual: copy from parent
            &xproto::CreateWindowAux::new(),
        )
        .map_err(|e| {
            ClipboardError::OperationFailed(format!("Failed to create window: {e}"))
        })?;

        // Intern atoms we need
        let clipboard_atom = intern_atom(&conn, "CLIPBOARD")?;
        let utf8_string_atom = intern_atom(&conn, "UTF8_STRING")?;
        let html_atom = intern_atom(&conn, "text/html")?;
        let paste_prop_atom = intern_atom(&conn, "PASTE_SELECTION")?;
        let primary_atom: Atom = AtomEnum::PRIMARY.into();

        // Subscribe to CLIPBOARD selection changes via XFixes
        let event_mask = SelectionEventMask::SET_SELECTION_OWNER
            | SelectionEventMask::SELECTION_WINDOW_DESTROY
            | SelectionEventMask::SELECTION_CLIENT_CLOSE;

        conn.xfixes_select_selection_input(win, clipboard_atom, event_mask)
            .map_err(|e| {
                ClipboardError::OperationFailed(format!(
                    "Failed to subscribe to CLIPBOARD events: {e}"
                ))
            })?;

        // Optionally subscribe to PRIMARY selection changes
        if self.monitor_primary {
            conn.xfixes_select_selection_input(win, primary_atom, event_mask)
                .map_err(|e| {
                    ClipboardError::OperationFailed(format!(
                        "Failed to subscribe to PRIMARY events: {e}"
                    ))
                })?;
        }

        conn.flush().map_err(|e| {
            ClipboardError::OperationFailed(format!("Failed to flush X11 connection: {e}"))
        })?;

        info!("X11 clipboard monitoring active");

        // Track hashes to deduplicate
        let mut last_clipboard_hash: Option<String> = None;
        let mut last_primary_hash: Option<String> = None;

        loop {
            let event = conn.wait_for_event().map_err(|e| {
                ClipboardError::OperationFailed(format!("X11 event error: {e}"))
            })?;

            // Handle XFixes SelectionNotify — clipboard ownership changed
            if let Event::XfixesSelectionNotify(sel) = event {
                let is_primary = sel.selection == primary_atom;
                let last_hash = if is_primary {
                    &mut last_primary_hash
                } else {
                    &mut last_clipboard_hash
                };

                debug!(
                    "Selection change: {}",
                    if is_primary { "PRIMARY" } else { "CLIPBOARD" }
                );

                // Read the selection content
                match read_selection_text(
                    &conn,
                    win,
                    sel.selection,
                    utf8_string_atom,
                    paste_prop_atom,
                ) {
                    Ok(Some(text)) => {
                        if text.is_empty() {
                            continue;
                        }

                        if text.len() as u64 > self.max_content_size_bytes {
                            debug!("Skipping: content too large ({} bytes)", text.len());
                            continue;
                        }

                        let hash = compute_hash(text.as_bytes());

                        // Deduplicate
                        if last_hash.as_ref() == Some(&hash) {
                            continue;
                        }
                        *last_hash = Some(hash.clone());

                        // Detect source app from selection owner
                        let source_app = get_window_class(&conn, sel.owner);

                        // Check exclusion
                        if is_app_excluded(&self.excluded_apps, &source_app) {
                            debug!("Skipping excluded app: {:?}", source_app);
                            continue;
                        }

                        // Detect content type
                        let content_type = detect_text_content_type(&text);

                        // Try to read HTML representation
                        let html_content = read_selection_text(
                            &conn,
                            win,
                            sel.selection,
                            html_atom,
                            paste_prop_atom,
                        )
                        .ok()
                        .flatten();

                        let metadata = if content_type == ContentType::Link {
                            Some(serde_json::json!({ "url": text.trim() }).to_string())
                        } else {
                            None
                        };

                        let item = ClipItem {
                            content_type: content_type.as_str().to_string(),
                            text_content: Some(text.clone()),
                            html_content,
                            image_path: None,
                            source_app,
                            content_hash: hash,
                            content_size: text.len() as i64,
                            metadata,
                        };

                        debug!(
                            "Captured X11 clip: type={}, size={}",
                            item.content_type, item.content_size
                        );

                        if tx.send(item).is_err() {
                            info!("Channel closed, stopping X11 monitor");
                            return Ok(());
                        }
                    }
                    Ok(None) => {
                        debug!("Selection content was empty or unavailable");
                    }
                    Err(e) => {
                        debug!("Failed to read selection: {e}");
                    }
                }
            }
        }
    }
}

/// Read text content from an X11 selection.
///
/// Sends a `ConvertSelection` request, then waits for the `SelectionNotify`
/// response event. On success, reads the property data from the window.
fn read_selection_text(
    conn: &RustConnection,
    win: Window,
    selection: Atom,
    target: Atom,
    property: Atom,
) -> Result<Option<String>, ClipboardError> {
    // Request the selection be converted to our target type
    conn.convert_selection(
        win,
        selection,
        target,
        property,
        xproto::Time::CURRENT_TIME,
    )
    .map_err(|e| {
        ClipboardError::OperationFailed(format!("ConvertSelection failed: {e}"))
    })?;
    conn.flush().map_err(|e| {
        ClipboardError::OperationFailed(format!("Flush failed: {e}"))
    })?;

    // Wait for the SelectionNotify response (with timeout via polling)
    let deadline = std::time::Instant::now() + Duration::from_secs(2);
    loop {
        if std::time::Instant::now() > deadline {
            debug!("Timeout waiting for selection response");
            return Ok(None);
        }

        let event = conn
            .poll_for_event()
            .map_err(|e| {
                ClipboardError::OperationFailed(format!("Poll event error: {e}"))
            })?;

        match event {
            Some(Event::SelectionNotify(notify)) => {
                if notify.property == 0 {
                    // Selection was refused (property is NONE/0)
                    return Ok(None);
                }

                // Read the property
                let reply = conn
                    .get_property(
                        true, // delete after reading
                        win,
                        property,
                        AtomEnum::ANY,
                        0,
                        u32::MAX / 4, // max length in 32-bit units
                    )
                    .map_err(|e| {
                        ClipboardError::OperationFailed(format!("GetProperty failed: {e}"))
                    })?
                    .reply()
                    .map_err(|e| {
                        ClipboardError::OperationFailed(format!(
                            "GetProperty reply failed: {e}"
                        ))
                    })?;

                if reply.value.is_empty() {
                    return Ok(None);
                }

                match String::from_utf8(reply.value) {
                    Ok(s) => return Ok(Some(s)),
                    Err(_) => return Ok(None),
                }
            }
            Some(_) => {
                // Not the event we're looking for, keep polling
            }
            None => {
                // No event yet, brief sleep before retrying
                thread::sleep(Duration::from_millis(10));
            }
        }
    }
}

/// Get the WM_CLASS property of a window for source app detection.
fn get_window_class(conn: &RustConnection, window: Window) -> Option<String> {
    if window == 0 {
        return None;
    }

    let reply = conn
        .get_property(
            false,
            window,
            AtomEnum::WM_CLASS,
            AtomEnum::STRING,
            0,
            1024,
        )
        .ok()?
        .reply()
        .ok()?;

    if reply.value.is_empty() {
        return None;
    }

    // WM_CLASS contains two null-separated strings: instance name and class name.
    // We want the class name (second one).
    let parts: Vec<&[u8]> = reply.value.split(|&b| b == 0).collect();
    if parts.len() >= 2 && !parts[1].is_empty() {
        String::from_utf8(parts[1].to_vec()).ok()
    } else if !parts.is_empty() && !parts[0].is_empty() {
        String::from_utf8(parts[0].to_vec()).ok()
    } else {
        None
    }
}

/// Intern an X11 atom by name.
fn intern_atom(conn: &RustConnection, name: &str) -> Result<Atom, ClipboardError> {
    conn.intern_atom(false, name.as_bytes())
        .map_err(|e| ClipboardError::OperationFailed(format!("InternAtom failed: {e}")))?
        .reply()
        .map(|r| r.atom)
        .map_err(|e| ClipboardError::OperationFailed(format!("InternAtom reply failed: {e}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_x11_clipboard_new() {
        let x11 = X11Clipboard::new(
            vec!["1password".into(), "keepassxc".into()],
            10,
            true,
        );
        assert_eq!(x11.excluded_apps.len(), 2);
        assert_eq!(x11.max_content_size_bytes, 10 * 1024 * 1024);
        assert!(x11.monitor_primary);
    }

    #[test]
    fn test_x11_clipboard_no_primary() {
        let x11 = X11Clipboard::new(vec![], 5, false);
        assert!(!x11.monitor_primary);
        assert_eq!(x11.max_content_size_bytes, 5 * 1024 * 1024);
    }

    #[test]
    fn test_is_excluded() {
        let x11 = X11Clipboard::new(
            vec!["1password".into(), "keepassxc".into()],
            10,
            true,
        );
        assert!(x11.is_excluded(&Some("1Password".into())));
        assert!(x11.is_excluded(&Some("KeePassXC".into())));
        assert!(x11.is_excluded(&Some("org.keepassxc.KeePassXC".into())));
        assert!(!x11.is_excluded(&Some("firefox".into())));
        assert!(!x11.is_excluded(&None));
    }

    #[test]
    fn test_is_excluded_case_insensitive() {
        let x11 = X11Clipboard::new(vec!["Bitwarden".into()], 10, true);
        assert!(x11.is_excluded(&Some("bitwarden".into())));
        assert!(x11.is_excluded(&Some("BITWARDEN".into())));
    }

    #[test]
    fn test_monitor_new_with_defaults() {
        let monitor = X11ClipboardMonitor {
            excluded_apps: vec!["test".into()],
            max_content_size_bytes: 1024,
            monitor_primary: false,
        };
        assert!(!monitor.monitor_primary);
        assert_eq!(monitor.max_content_size_bytes, 1024);
    }

    #[test]
    fn test_is_app_excluded_function() {
        let excluded = vec!["firefox".into(), "chrome".into()];
        assert!(is_app_excluded(&excluded, &Some("Firefox".into())));
        assert!(is_app_excluded(&excluded, &Some("Google-chrome".into())));
        assert!(!is_app_excluded(&excluded, &Some("alacritty".into())));
        assert!(!is_app_excluded(&excluded, &None));
    }
}
