use tauri::{Listener, Manager};

use std::sync::mpsc;

mod clipboard;
mod config;
mod expander;
mod hotkey;
mod injector;
mod logging;
mod overlay;
mod service;
mod storage;
mod tray;

use clipboard::detection::{compute_hash, detect_text_content_type};
use clipboard::stack::PasteStack;
use clipboard::ClipboardBackend;
use config::AppConfig;
use expander::export::{build_export, has_script_snippets, parse_import, JsonImportResult};
use expander::import::{default_espanso_path, parse_espanso_dir, ImportResult, ImportedSnippet};
use expander::template::{
    evaluate_tokens, extract_fill_in_fields, parse_template, ExpansionContext, FillInField,
};
use injector::{select_injector, Injector, RichContent};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Instant;
use storage::{
    models::{
        Clip, ClipFilters, NewClip, NewPinboard, NewSnippet, NewSnippetGroup, Pinboard, Snippet,
        SnippetGroup, StorageStats, UpdateSnippet,
    },
    Storage,
};

/// Channel sender for showing the overlay from the hotkey thread.
struct ShowOverlaySender(std::sync::Mutex<mpsc::Sender<()>>);

/// Shared application state managed by Tauri.
pub struct AppState {
    pub storage: Storage,
    pub injector: Arc<dyn Injector>,
    pub paste_stack: PasteStack,
    pub excluded_apps: Mutex<Vec<String>>,
}

#[tauri::command]
fn get_clips(
    state: tauri::State<'_, AppState>,
    offset: usize,
    limit: usize,
    content_type: Option<String>,
    source_app: Option<String>,
    pinboard_id: Option<String>,
    is_favorite: Option<bool>,
) -> Result<Vec<Clip>, String> {
    let start = Instant::now();
    let filters = ClipFilters {
        content_type,
        source_app,
        date_from: None,
        date_to: None,
        pinboard_id,
        is_favorite,
    };
    let result = state
        .storage
        .get_clips(offset, limit, &filters)
        .map_err(|e| e.to_string());
    let elapsed = start.elapsed();
    log::debug!(
        "get_clips: {}ms ({} results)",
        elapsed.as_millis(),
        result.as_ref().map(|r| r.len()).unwrap_or(0)
    );
    result
}

/// Log loudly if the database file is no longer receiving our writes.
///
/// SQLite reports success when writing to a file that has been unlinked or
/// replaced underneath it, so this condition is otherwise completely silent —
/// the UI shows the data, the log shows clips being captured, and it is all
/// discarded when the process exits (#113).
fn report_db_health(storage: &Storage) {
    match storage.health_check() {
        storage::DbHealth::Ok | storage::DbHealth::InMemory => {}
        storage::DbHealth::Missing => log::error!(
            "DATABASE FILE IS GONE. Clips captured since startup exist only in this \
             process and will be lost when it exits. Recover before restarting: copy \
             /proc/{}/fd/<db-fd> to a safe location.",
            std::process::id()
        ),
        storage::DbHealth::Replaced => log::error!(
            "DATABASE FILE WAS REPLACED. Clips captured since startup are going to the \
             old file and will be lost when this process exits. Recover before \
             restarting: copy /proc/{}/fd/<db-fd> to a safe location.",
            std::process::id()
        ),
        storage::DbHealth::Unreadable(e) => {
            log::warn!("could not verify database file health: {e}")
        }
    }
}

/// Ask GNOME Shell to focus the overlay.
///
/// Mutter grants focus to the overlay only on its very first map; every later
/// show is a re-map and focus-stealing prevention denies it, because the
/// summoning hotkey comes from evdev and the compositor cannot attribute it to
/// us. Measured on this machine: 1 of 5 shows got focus, always the first after
/// startup. `gtk_window_present_with_time()` does not help — without an
/// xdg-activation token GTK falls back to a plain present.
///
/// The `org.gnome.Shell.Extensions.Windows` interface runs inside GNOME Shell,
/// so its `Activate` is not subject to that restriction. It is provided by
/// extensions such as Window Calls; when absent this is a no-op and we keep
/// the behaviour we had. That matches how the rest of the app treats
/// compositor-specific helpers: use them when present, never require them.
fn activate_overlay_via_shell() {
    use std::process::Command;

    const DEST: &str = "org.gnome.Shell";
    const PATH: &str = "/org/gnome/Shell/Extensions/Windows";

    let listed = Command::new("gdbus")
        .args([
            "call",
            "--session",
            "--dest",
            DEST,
            "--object-path",
            PATH,
            "--method",
            "org.gnome.Shell.Extensions.Windows.List",
        ])
        .output();

    let Ok(listed) = listed else {
        return;
    };
    if !listed.status.success() {
        return;
    }
    let raw = String::from_utf8_lossy(&listed.stdout);

    // gdbus wraps the payload as ('<json>',); the JSON itself may be escaped.
    let (Some(open), Some(close)) = (raw.find('['), raw.rfind(']')) else {
        return;
    };
    let json = raw[open..=close].replace("\\\"", "\"");

    let Ok(windows) = serde_json::from_str::<Vec<serde_json::Value>>(&json) else {
        return;
    };

    // Match on our own pid rather than the window class: the class differs
    // between dev and packaged builds, the pid never lies.
    let me = std::process::id() as u64;
    let Some(id) = windows
        .iter()
        .find(|w| w.get("pid").and_then(|p| p.as_u64()) == Some(me))
        .and_then(|w| w.get("id").and_then(|i| i.as_u64()))
    else {
        log::debug!("show: overlay window not listed by GNOME Shell");
        return;
    };

    let activated = Command::new("gdbus")
        .args([
            "call",
            "--session",
            "--dest",
            DEST,
            "--object-path",
            PATH,
            "--method",
            "org.gnome.Shell.Extensions.Windows.Activate",
            &id.to_string(),
        ])
        .status();
    log::info!("show: shell activate window {id} -> {activated:?}");
}

/// Hide the overlay and give the compositor time to refocus the previously
/// active window, so injected keystrokes land at the user's cursor rather
/// than the overlay itself. No-op when the overlay is already hidden
/// (e.g. Quick Paste via Super+N).
fn hide_overlay_for_paste(app: &tauri::AppHandle) {
    let Some(win) = app.get_webview_window("main") else {
        return;
    };
    if !win.is_visible().unwrap_or(false) {
        return;
    }

    let _ = win.hide();

    // Wait for the compositor to hand focus back before injecting.
    //
    // A fixed delay was enough while the overlay never took focus in the first
    // place. Now that it does (#114), hiding starts a real focus hand-back that
    // takes the better part of a second here — measured at ~780ms — and a
    // Ctrl+V injected before it finishes lands in the hidden overlay and is
    // lost.
    //
    // `is_focused()` cannot be used to detect this: it reported `true` for a
    // hidden window and `false` for a focused one. The compositor's own idea of
    // the focused window is the only signal that matched reality in testing.
    let deadline = std::time::Instant::now() + std::time::Duration::from_millis(2000);
    let mut focus_returned_to = None;
    while std::time::Instant::now() < deadline {
        let focused = clipboard::wayland::WaylandClipboard::detect_source_app();
        let still_ours = focused
            .as_deref()
            .map(|a| a.eq_ignore_ascii_case("paste"))
            .unwrap_or(false);
        if !still_ours {
            focus_returned_to = focused;
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(40));
    }

    // Settle time for the newly focused window to be ready for input.
    std::thread::sleep(std::time::Duration::from_millis(60));

    log::info!("paste: overlay hidden, focus returned to {focus_returned_to:?}");
}

#[tauri::command]
fn paste_clip(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    id: String,
) -> Result<(), String> {
    let start = Instant::now();

    // Get the clip from storage
    let clip = state
        .storage
        .get_clip_by_id(&id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Clip not found: {}", id))?;

    // Use rich paste when HTML or image content is available
    let rich_content = RichContent {
        text: clip.text_content.clone(),
        html: clip.html_content.clone(),
        image_path: clip.image_path.clone(),
    };

    // Increment access count
    state
        .storage
        .increment_access_count(&id)
        .map_err(|e| e.to_string())?;

    // Hide and inject on a worker thread, then return immediately.
    //
    // A synchronous Tauri command runs on the main thread, which is also the
    // GTK event loop. Waiting here for the compositor to hand focus back is a
    // deadlock: the focus-out event cannot be processed until this returns, so
    // the overlay keeps focus for exactly as long as we wait for it to lose
    // focus, and the injected Ctrl+V lands in the hidden overlay (#114).
    let injector = Arc::clone(&state.injector);
    let app_bg = app.clone();
    std::thread::Builder::new()
        .name("paste-inject".into())
        .spawn(move || {
            hide_overlay_for_paste(&app_bg);
            if let Err(e) = injector.inject_rich(&rich_content) {
                // The content is already on the clipboard either way — the
                // injector sets it before simulating the keystroke — so a
                // failure here costs the automatic paste, not the clip.
                log::error!("paste injection failed: {e}");
            }
        })
        .map_err(|e| e.to_string())?;

    let elapsed = start.elapsed();
    log::debug!(
        "paste_clip: {}ms (injection continues off-thread)",
        elapsed.as_millis()
    );

    Ok(())
}

#[tauri::command]
fn hide_overlay(app: tauri::AppHandle) -> Result<(), String> {
    if let Some(win) = app.get_webview_window("main") {
        log::info!("hide: requested by frontend (hide_overlay command)");
        win.hide().map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
fn copy_to_clipboard(state: tauri::State<'_, AppState>, id: String) -> Result<(), String> {
    let clip = state
        .storage
        .get_clip_by_id(&id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Clip not found: {}", id))?;

    if let Some(ref text) = clip.text_content {
        // Use xclip to set clipboard (doesn't simulate paste, just copies)
        use std::io::Write;
        use std::process::{Command, Stdio};
        let mut child = Command::new("xclip")
            .args(["-selection", "clipboard"])
            .stdin(Stdio::piped())
            .spawn()
            .map_err(|e| e.to_string())?;
        if let Some(mut stdin) = child.stdin.take() {
            stdin
                .write_all(text.as_bytes())
                .map_err(|e| e.to_string())?;
        }
        child.wait().map_err(|e| e.to_string())?;
    }

    state
        .storage
        .increment_access_count(&id)
        .map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
fn delete_clip(state: tauri::State<'_, AppState>, id: String) -> Result<(), String> {
    state.storage.delete_clip(&id).map_err(|e| e.to_string())
}

// Tauri deserializes command arguments positionally from the JS call site,
// so grouping these into a struct would change the IPC contract for every
// caller. The width is inherent to the command surface, not a smell.
#[allow(clippy::too_many_arguments)]
#[tauri::command]
fn search_clips(
    state: tauri::State<'_, AppState>,
    query: String,
    content_type: Option<String>,
    source_app: Option<String>,
    date_from: Option<String>,
    date_to: Option<String>,
    pinboard_id: Option<String>,
    is_favorite: Option<bool>,
) -> Result<Vec<Clip>, String> {
    let start = Instant::now();
    let filters = ClipFilters {
        content_type,
        source_app,
        date_from,
        date_to,
        pinboard_id,
        is_favorite,
    };
    let result = state
        .storage
        .search_clips(&query, &filters)
        .map_err(|e| e.to_string());
    let elapsed = start.elapsed();
    log::debug!(
        "search_clips '{}': {}ms ({} results)",
        query,
        elapsed.as_millis(),
        result.as_ref().map(|r| r.len()).unwrap_or(0)
    );
    result
}

#[tauri::command]
fn get_source_apps(state: tauri::State<'_, AppState>) -> Result<Vec<String>, String> {
    state
        .storage
        .get_distinct_source_apps()
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn list_pinboards(state: tauri::State<'_, AppState>) -> Result<Vec<Pinboard>, String> {
    state.storage.list_pinboards().map_err(|e| e.to_string())
}

#[tauri::command]
fn create_pinboard(
    state: tauri::State<'_, AppState>,
    name: String,
    color: String,
) -> Result<Pinboard, String> {
    let new_pb = NewPinboard {
        name,
        color,
        icon: None,
    };
    state
        .storage
        .create_pinboard(&new_pb)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn update_pinboard(
    state: tauri::State<'_, AppState>,
    id: String,
    name: String,
    color: String,
) -> Result<Pinboard, String> {
    state
        .storage
        .update_pinboard(&id, &name, &color, None)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn delete_pinboard(state: tauri::State<'_, AppState>, id: String) -> Result<(), String> {
    state
        .storage
        .delete_pinboard(&id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn add_clip_to_pinboard(
    state: tauri::State<'_, AppState>,
    clip_id: String,
    pinboard_id: String,
) -> Result<(), String> {
    state
        .storage
        .update_clip_pinboard(&clip_id, Some(&pinboard_id))
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn remove_clip_from_pinboard(
    state: tauri::State<'_, AppState>,
    clip_id: String,
) -> Result<(), String> {
    state
        .storage
        .update_clip_pinboard(&clip_id, None)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn quick_paste(state: tauri::State<'_, AppState>, n: usize) -> Result<(), String> {
    if n == 0 || n > 9 {
        return Err("Quick paste index must be between 1 and 9".into());
    }

    // Get the Nth most recent clip (n=1 means most recent, offset=0)
    let clips = state
        .storage
        .get_clips(n - 1, 1, &ClipFilters::default())
        .map_err(|e| e.to_string())?;

    let clip = clips
        .into_iter()
        .next()
        .ok_or_else(|| format!("No clip at position {}", n))?;

    // Inject the text content
    if let Some(ref text) = clip.text_content {
        state
            .injector
            .inject_via_clipboard(text)
            .map_err(|e| e.to_string())?;
    }

    // Increment access count
    state
        .storage
        .increment_access_count(&clip.id)
        .map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
fn toggle_paste_stack(state: tauri::State<'_, AppState>) -> Result<bool, String> {
    let active = state.paste_stack.toggle();
    Ok(active)
}

#[tauri::command]
fn get_paste_stack(state: tauri::State<'_, AppState>) -> Result<Vec<Clip>, String> {
    Ok(state.paste_stack.get_all())
}

#[tauri::command]
fn get_paste_stack_status(state: tauri::State<'_, AppState>) -> Result<(bool, usize), String> {
    Ok((state.paste_stack.is_active(), state.paste_stack.len()))
}

#[tauri::command]
fn add_to_paste_stack(state: tauri::State<'_, AppState>, clip_id: String) -> Result<(), String> {
    let clip = state
        .storage
        .get_clip_by_id(&clip_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Clip not found: {}", clip_id))?;
    state.paste_stack.push(clip);
    Ok(())
}

#[tauri::command]
fn pop_paste_stack(state: tauri::State<'_, AppState>) -> Result<Option<String>, String> {
    let clip = state.paste_stack.pop_next();

    // If stack is now empty, auto-deactivate
    if state.paste_stack.is_empty() {
        state.paste_stack.deactivate();
    }

    // If we got a clip, inject it
    if let Some(ref clip) = clip {
        if let Some(ref text) = clip.text_content {
            state
                .injector
                .inject_via_clipboard(text)
                .map_err(|e| e.to_string())?;
        }
        state
            .storage
            .increment_access_count(&clip.id)
            .map_err(|e| e.to_string())?;
    }

    Ok(clip.map(|c| c.id))
}

#[tauri::command]
fn remove_from_paste_stack(
    state: tauri::State<'_, AppState>,
    clip_id: String,
) -> Result<(), String> {
    state.paste_stack.remove(&clip_id);
    Ok(())
}

#[tauri::command]
fn reorder_paste_stack(
    state: tauri::State<'_, AppState>,
    from_index: usize,
    to_index: usize,
) -> Result<(), String> {
    if !state.paste_stack.reorder(from_index, to_index) {
        return Err("Invalid reorder indices".into());
    }
    Ok(())
}

#[tauri::command]
fn clear_paste_stack(state: tauri::State<'_, AppState>) -> Result<(), String> {
    state.paste_stack.deactivate();
    Ok(())
}

#[tauri::command]
fn list_snippets(
    state: tauri::State<'_, AppState>,
    group_id: Option<String>,
) -> Result<Vec<Snippet>, String> {
    state
        .storage
        .list_snippets(group_id.as_deref())
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn create_snippet(
    state: tauri::State<'_, AppState>,
    abbreviation: String,
    name: String,
    content: String,
    content_type: String,
    group_id: Option<String>,
    description: Option<String>,
) -> Result<Snippet, String> {
    let new = NewSnippet {
        abbreviation,
        name,
        content,
        content_type,
        group_id,
        description,
    };
    state
        .storage
        .create_snippet(&new)
        .map_err(|e| e.to_string())
}

// Tauri deserializes command arguments positionally from the JS call site,
// so grouping these into a struct would change the IPC contract for every
// caller. The width is inherent to the command surface, not a smell.
#[allow(clippy::too_many_arguments)]
#[tauri::command]
fn update_snippet(
    state: tauri::State<'_, AppState>,
    id: String,
    abbreviation: String,
    name: String,
    content: String,
    content_type: String,
    group_id: Option<String>,
    description: Option<String>,
) -> Result<Snippet, String> {
    let update = UpdateSnippet {
        abbreviation,
        name,
        content,
        content_type,
        group_id,
        description,
    };
    state
        .storage
        .update_snippet(&id, &update)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn delete_snippet(state: tauri::State<'_, AppState>, id: String) -> Result<(), String> {
    state.storage.delete_snippet(&id).map_err(|e| e.to_string())
}

#[tauri::command]
fn list_snippet_groups(state: tauri::State<'_, AppState>) -> Result<Vec<SnippetGroup>, String> {
    state
        .storage
        .list_snippet_groups()
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn create_snippet_group(
    state: tauri::State<'_, AppState>,
    name: String,
) -> Result<SnippetGroup, String> {
    let new = NewSnippetGroup { name };
    state
        .storage
        .create_snippet_group(&new)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn delete_snippet_group(state: tauri::State<'_, AppState>, id: String) -> Result<(), String> {
    state
        .storage
        .delete_snippet_group(&id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn get_fill_in_fields(template: String) -> Vec<FillInField> {
    let tokens = parse_template(&template);
    extract_fill_in_fields(&tokens)
}

#[tauri::command]
fn expand_with_fill_ins(
    template: String,
    fill_values: HashMap<String, String>,
) -> Result<String, String> {
    let tokens = parse_template(&template);
    let ctx = ExpansionContext {
        clipboard_content: String::new(),
        fill_values,
        ..Default::default()
    };
    Ok(evaluate_tokens(&tokens, &ctx).text)
}

#[tauri::command]
fn preview_espanso_import(path: Option<String>) -> Result<Vec<ImportedSnippet>, String> {
    let dir = path
        .map(std::path::PathBuf::from)
        .unwrap_or_else(default_espanso_path);
    parse_espanso_dir(&dir)
}

#[tauri::command]
fn import_espanso(
    state: tauri::State<'_, AppState>,
    path: Option<String>,
) -> Result<ImportResult, String> {
    let dir = path
        .map(std::path::PathBuf::from)
        .unwrap_or_else(default_espanso_path);

    let snippets = parse_espanso_dir(&dir)?;
    let mut imported = 0usize;
    let mut skipped = 0usize;
    let mut errors = Vec::new();

    for snippet in &snippets {
        // Check for duplicate abbreviation
        match state
            .storage
            .get_snippet_by_abbreviation(&snippet.abbreviation)
        {
            Ok(Some(_)) => {
                skipped += 1;
                continue;
            }
            Ok(None) => {}
            Err(e) => {
                errors.push(format!("Error checking '{}': {e}", snippet.abbreviation));
                continue;
            }
        }

        let new_snippet = NewSnippet {
            abbreviation: snippet.abbreviation.clone(),
            name: snippet.name.clone(),
            content: snippet.content.clone(),
            content_type: snippet.content_type.clone(),
            group_id: None,
            description: Some(format!("Imported from espanso: {}", snippet.source_file)),
        };

        match state.storage.create_snippet(&new_snippet) {
            Ok(_) => imported += 1,
            Err(e) => errors.push(format!("Failed to import '{}': {e}", snippet.abbreviation)),
        }
    }

    Ok(ImportResult {
        imported,
        skipped,
        errors,
    })
}

#[tauri::command]
fn export_snippets(state: tauri::State<'_, AppState>) -> Result<String, String> {
    let snippets = state
        .storage
        .list_snippets(None)
        .map_err(|e| e.to_string())?;
    let groups = state
        .storage
        .list_snippet_groups()
        .map_err(|e| e.to_string())?;
    let export = build_export(&snippets, &groups);
    serde_json::to_string_pretty(&export).map_err(|e| e.to_string())
}

#[tauri::command]
fn import_snippets_json(
    state: tauri::State<'_, AppState>,
    json: String,
) -> Result<JsonImportResult, String> {
    let export = parse_import(&json)?;
    let has_scripts = has_script_snippets(&export);

    let mut imported = 0usize;
    let mut skipped = 0usize;
    let mut errors = Vec::new();

    for group in &export.groups {
        // Create or find the group (skip "Ungrouped")
        let group_id = if group.name != "Ungrouped" {
            match state.storage.create_snippet_group(&NewSnippetGroup {
                name: group.name.clone(),
            }) {
                Ok(g) => Some(g.id),
                Err(_) => {
                    // Group might already exist -- find it
                    state
                        .storage
                        .list_snippet_groups()
                        .ok()
                        .and_then(|groups| groups.into_iter().find(|g| g.name == group.name))
                        .map(|g| g.id)
                }
            }
        } else {
            None
        };

        for snippet in &group.snippets {
            // Check for duplicate abbreviation
            match state
                .storage
                .get_snippet_by_abbreviation(&snippet.abbreviation)
            {
                Ok(Some(_)) => {
                    skipped += 1;
                    continue;
                }
                Ok(None) => {}
                Err(e) => {
                    errors.push(format!("Error checking '{}': {e}", snippet.abbreviation));
                    continue;
                }
            }

            let new_snippet = NewSnippet {
                abbreviation: snippet.abbreviation.clone(),
                name: snippet.name.clone(),
                content: snippet.content.clone(),
                content_type: snippet.content_type.clone(),
                group_id: group_id.clone(),
                description: snippet.description.clone(),
            };

            match state.storage.create_snippet(&new_snippet) {
                Ok(_) => imported += 1,
                Err(e) => errors.push(format!("Failed to import '{}': {e}", snippet.abbreviation)),
            }
        }
    }

    Ok(JsonImportResult {
        imported,
        skipped,
        errors,
        has_scripts,
    })
}

#[tauri::command]
fn update_clip_content(
    state: tauri::State<'_, AppState>,
    id: String,
    content: String,
) -> Result<(), String> {
    state
        .storage
        .update_clip_content(&id, &content)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn paste_clips_multi(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    ids: Vec<String>,
) -> Result<(), String> {
    let start = Instant::now();

    // Collect text content from all clips in order
    let mut texts: Vec<String> = Vec::new();
    for id in &ids {
        let clip = state
            .storage
            .get_clip_by_id(id)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| format!("Clip not found: {}", id))?;

        if let Some(text) = clip.text_content {
            texts.push(text);
        }

        // Increment access count for each
        let _ = state.storage.increment_access_count(id);
    }

    if texts.is_empty() {
        return Ok(());
    }

    // Concatenate with newlines and paste
    let combined = texts.join("\n");
    hide_overlay_for_paste(&app);
    state
        .injector
        .inject_via_clipboard(&combined)
        .map_err(|e| e.to_string())?;

    let elapsed = start.elapsed();
    log::debug!(
        "paste_clips_multi: {}ms ({} clips)",
        elapsed.as_millis(),
        ids.len()
    );

    Ok(())
}

#[tauri::command]
fn paste_clip_plain(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    id: String,
) -> Result<(), String> {
    let clip = state
        .storage
        .get_clip_by_id(&id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Clip not found: {}", id))?;

    if let Some(ref text) = clip.text_content {
        hide_overlay_for_paste(&app);
        // Plain paste = text-only clipboard injection (no HTML/image MIME).
        // Clipboard + Ctrl+V is far more reliable than typing simulation.
        state
            .injector
            .inject_via_clipboard(text)
            .map_err(|e| e.to_string())?;
    }

    state
        .storage
        .increment_access_count(&id)
        .map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
fn toggle_favorite(state: tauri::State<'_, AppState>, id: String) -> Result<bool, String> {
    state
        .storage
        .toggle_favorite(&id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn get_excluded_apps(state: tauri::State<'_, AppState>) -> Vec<String> {
    state.excluded_apps.lock().unwrap().clone()
}

#[tauri::command]
fn add_excluded_app(state: tauri::State<'_, AppState>, app_name: String) -> Vec<String> {
    let mut apps = state.excluded_apps.lock().unwrap();
    let lower = app_name.to_lowercase();
    if !apps.iter().any(|a| a.to_lowercase() == lower) {
        apps.push(app_name);
    }
    apps.clone()
}

#[tauri::command]
fn remove_excluded_app(state: tauri::State<'_, AppState>, app_name: String) -> Vec<String> {
    let mut apps = state.excluded_apps.lock().unwrap();
    let lower = app_name.to_lowercase();
    apps.retain(|a| a.to_lowercase() != lower);
    apps.clone()
}

#[tauri::command]
fn get_storage_stats(state: tauri::State<'_, AppState>) -> Result<StorageStats, String> {
    state.storage.get_storage_stats().map_err(|e| e.to_string())
}

#[tauri::command]
fn run_retention(state: tauri::State<'_, AppState>) -> Result<usize, String> {
    let max_days = Some(90u32);
    let max_count = Some(10000usize);
    state
        .storage
        .enforce_retention(max_days, max_count)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn clear_all_history(state: tauri::State<'_, AppState>) -> Result<usize, String> {
    // Delete all non-pinboard, non-favorite clips
    state
        .storage
        .enforce_retention(None, Some(0))
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn get_config() -> Result<AppConfig, String> {
    AppConfig::load().map_err(|e| e.to_string())
}

#[tauri::command]
fn save_config(config: AppConfig) -> Result<(), String> {
    let path = AppConfig::config_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let toml_str = toml::to_string_pretty(&config).map_err(|e| e.to_string())?;
    std::fs::write(&path, toml_str).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
fn reset_config() -> Result<AppConfig, String> {
    let default = AppConfig::default();
    let path = AppConfig::config_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let toml_str = toml::to_string_pretty(&default).map_err(|e| e.to_string())?;
    std::fs::write(&path, toml_str).map_err(|e| e.to_string())?;
    Ok(default)
}

#[tauri::command]
fn create_clip_from_text(
    state: tauri::State<'_, AppState>,
    text: String,
    content_type: Option<String>,
) -> Result<Clip, String> {
    let hash = compute_hash(text.as_bytes());
    let detected_type =
        content_type.unwrap_or_else(|| detect_text_content_type(&text).as_str().to_string());

    let new_clip = NewClip {
        content_type: detected_type,
        text_content: Some(text.clone()),
        html_content: None,
        image_path: None,
        source_app: Some("Paste (drop)".to_string()),
        source_app_icon: None,
        content_hash: hash,
        content_size: text.len() as i64,
        metadata: None,
    };

    state
        .storage
        .insert_clip(&new_clip)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn get_autostart_status() -> Result<bool, String> {
    Ok(service::is_service_installed())
}

#[tauri::command]
fn install_autostart() -> Result<String, String> {
    service::install_service()
}

#[tauri::command]
fn uninstall_autostart() -> Result<String, String> {
    service::uninstall_service()
}

pub fn run() {
    // Initialize logging first — all subsequent log calls go to stderr + file
    logging::init_logging();

    log::info!("=== Paste starting ===");

    // Load config
    let config = AppConfig::load().unwrap_or_else(|e| {
        log::warn!("Failed to load config: {e}. Using defaults.");
        AppConfig::default()
    });

    // Initialize storage with fallback to in-memory on failure
    let storage = match Storage::new(Some(config.resolved_db_path())) {
        Ok(s) => {
            log::info!(
                "Storage initialized at {}",
                config.resolved_db_path().display()
            );
            s
        }
        Err(e) => {
            log::error!("Failed to initialize storage: {e}");
            log::info!("Attempting fallback with in-memory storage");
            Storage::new_in_memory().expect("Failed to create even in-memory storage")
        }
    };

    // Initialize injector with fallback to clipboard on failure
    // Create the persistent virtual keyboard up front so the compositor has
    // registered it long before the first paste (avoids dropped keystrokes).
    injector::virtual_keyboard::init();

    let injector = match select_injector(&config.injection.method) {
        Ok(i) => {
            log::info!("Text injector initialized: {}", i.name());
            i
        }
        Err(e) => {
            log::error!(
                "Failed to initialize injector ({}): {e}",
                config.injection.method
            );
            log::info!("Falling back to clipboard injector");
            select_injector("clipboard").expect("Failed to create clipboard injector")
        }
    };

    let app_state = AppState {
        storage,
        injector: Arc::from(injector),
        paste_stack: PasteStack::new(),
        excluded_apps: Mutex::new(config.clipboard.excluded_apps.clone()),
    };

    log::info!("App state initialized");

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .manage(app_state)
        .invoke_handler(tauri::generate_handler![
            get_clips,
            paste_clip,
            paste_clips_multi,
            paste_clip_plain,
            update_clip_content,
            delete_clip,
            search_clips,
            get_source_apps,
            list_pinboards,
            create_pinboard,
            update_pinboard,
            delete_pinboard,
            add_clip_to_pinboard,
            remove_clip_from_pinboard,
            quick_paste,
            toggle_paste_stack,
            get_paste_stack,
            get_paste_stack_status,
            add_to_paste_stack,
            pop_paste_stack,
            remove_from_paste_stack,
            reorder_paste_stack,
            clear_paste_stack,
            list_snippets,
            create_snippet,
            update_snippet,
            delete_snippet,
            list_snippet_groups,
            create_snippet_group,
            delete_snippet_group,
            get_fill_in_fields,
            expand_with_fill_ins,
            preview_espanso_import,
            import_espanso,
            export_snippets,
            import_snippets_json,
            toggle_favorite,
            get_excluded_apps,
            add_excluded_app,
            remove_excluded_app,
            get_storage_stats,
            run_retention,
            clear_all_history,
            get_config,
            save_config,
            reset_config,
            get_autostart_status,
            install_autostart,
            uninstall_autostart,
            create_clip_from_text,
            copy_to_clipboard,
            hide_overlay,
        ])
        .setup(|app| {
            if let Err(e) = tray::setup_tray(app.handle()) {
                log::error!("Failed to setup tray: {e}");
                // Continue without tray — not fatal
            }

            // Position window as bottom-edge overlay
            if let Err(e) = overlay::setup_overlay(app.handle()) {
                log::error!("Failed to setup overlay positioning: {e}");
            }

            // Open the web inspector at startup when requested — the overlay
            // auto-hides on focus loss, so the right-click route can't reach it.
            #[cfg(feature = "devtools")]
            if std::env::var("PASTE_DEVTOOLS").is_ok() {
                if let Some(win) = app.get_webview_window("main") {
                    win.open_devtools();
                }
            }

            // Auto-hide when window loses focus (click outside)
            // Use a flag to avoid hiding immediately after showing
            if let Some(win) = app.get_webview_window("main") {
                let win_clone = win.clone();
                let showing = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
                let showing_clone = showing.clone();

                win.on_window_event(move |event| {
                    match event {
                        tauri::WindowEvent::Focused(true) => {
                            // Window got focus — clear the "just shown" flag
                            showing_clone.store(false, std::sync::atomic::Ordering::Relaxed);
                        }
                        // Only auto-hide if we're not in the middle of showing
                        tauri::WindowEvent::Focused(false)
                            if !showing_clone.load(std::sync::atomic::Ordering::Relaxed) =>
                        {
                            log::info!("hide: focus lost (auto-hide)");
                            let _ = win_clone.hide();
                        }
                        _ => {}
                    }
                });

                // Helper function to show the overlay properly
                let show_overlay = {
                    let app_handle = app.handle().clone();
                    let showing = showing.clone();
                    move || {
                        showing.store(true, std::sync::atomic::Ordering::Relaxed);
                        let _ = overlay::setup_overlay(&app_handle);
                        if let Some(win) = app_handle.get_webview_window("main") {
                            // The overlay silently failing to appear is invisible
                            // in logs otherwise: show() errors were discarded and
                            // nothing recorded whether the window actually mapped.
                            let before = win.is_visible();
                            let shown = win.show();
                            let focused = win.set_focus();
                            let after = win.is_visible();
                            log::info!(
                                "show: show={shown:?} set_focus={focused:?} visible={after:?} focused={:?}",
                                win.is_focused()
                            );
                            let _ = before;

                            // Tauri's set_focus() maps to gtk_window_present(),
                            // which mutter declines: the overlay is summoned by
                            // an evdev hotkey the compositor never saw, so there
                            // is no interaction it can attribute the request to.
                            // Presenting with an explicit timestamp gives the
                            // focus-stealing check something to compare against
                            // (#114).
                            activate_overlay_via_shell();
                        } else {
                            log::warn!("show: main window not found");
                        }
                        // Clear the flag after a short delay
                        let showing2 = showing.clone();
                        std::thread::spawn(move || {
                            std::thread::sleep(std::time::Duration::from_millis(200));
                            showing2.store(false, std::sync::atomic::Ordering::Relaxed);
                        });
                    }
                };

                // Handle tray "Show overlay" directly in Rust
                let show_overlay_tray = show_overlay.clone();
                app.listen("tray-show-overlay", move |_| {
                    log::info!("Showing overlay from tray");
                    show_overlay_tray();
                });

                // Store the show function for the hotkey handler
                // We use a channel since we can't easily share closures across threads
                let (show_tx, show_rx) = std::sync::mpsc::channel::<()>();
                let show_overlay_hotkey = show_overlay.clone();
                std::thread::Builder::new()
                    .name("overlay-show".into())
                    .spawn(move || {
                        for () in show_rx {
                            show_overlay_hotkey();
                        }
                    })
                    .ok();

                // Make show_tx available for the hotkey handler via app state
                // Store it as managed state
                app.manage(ShowOverlaySender(std::sync::Mutex::new(show_tx)));
            }

            // Start clipboard monitoring
            {
                let app_handle = app.handle().clone();
                let excluded_apps = if let Some(state) = app_handle.try_state::<AppState>() {
                    state.excluded_apps.lock().unwrap().clone()
                } else {
                    vec![]
                };

                let (tx, rx) = mpsc::channel::<clipboard::types::ClipItem>();

                // Start Wayland clipboard monitor. These four knobs come from
                // [clipboard] in config.toml — max_content_size_mb used to be
                // hardcoded to 10 and the dedup pair was never read at all.
                let clip_cfg = config::AppConfig::load().unwrap_or_default().clipboard;
                let monitor = clipboard::wayland::WaylandClipboard::new(
                    excluded_apps,
                    clip_cfg.max_content_size_mb,
                    clip_cfg.merge_growing,
                    clip_cfg.debounce_ms,
                );
                match monitor.start_monitoring(tx) {
                    Ok(()) => log::info!("Clipboard monitoring started"),
                    Err(e) => log::error!("Failed to start clipboard monitor: {e}"),
                }

                // Spawn a thread to receive captured clips and insert into storage
                let app_handle2 = app.handle().clone();
                std::thread::Builder::new()
                    .name("clipboard-receiver".into())
                    .spawn(move || {
                        use storage::models::NewClip;
                        loop {
                            match rx.recv() {
                                Ok(item) => {
                                    if let Some(state) = app_handle2.try_state::<AppState>() {
                                        let replaces_previous = item.replaces_previous;
                                        let new_clip = NewClip {
                                            content_type: item.content_type,
                                            text_content: item.text_content,
                                            html_content: item.html_content,
                                            image_path: item.image_path,
                                            source_app: item.source_app,
                                            source_app_icon: None,
                                            content_hash: item.content_hash,
                                            content_size: item.content_size,
                                            metadata: item.metadata,
                                        };
                                        // A superseding capture (grown selection or
                                        // rapid re-copy) replaces the previous clip
                                        // instead of stacking on top of it.
                                        let stored = if replaces_previous {
                                            state
                                                .storage
                                                .replace_latest_clip(&new_clip)
                                                .map(|(clip, replaced)| {
                                                    if replaced {
                                                        log::debug!("Clip superseded previous");
                                                    }
                                                    clip
                                                })
                                        } else {
                                            state.storage.insert_clip(&new_clip)
                                        };
                                        match stored {
                                            Ok(clip) => {
                                                log::debug!(
                                                    "Clip captured: {} ({} bytes)",
                                                    clip.content_type, clip.content_size
                                                );
                                                // Notify frontend to reload clips
                                                use tauri::Emitter;
                                                let _ = app_handle2.emit("clip-added", &clip.id);
                                            }
                                            Err(storage::StorageError::Duplicate) => {
                                                log::debug!("Duplicate clip skipped");
                                            }
                                            Err(e) => log::error!("Failed to store clip: {e}"),
                                        }
                                    }
                                }
                                Err(_) => {
                                    log::info!("Clipboard channel closed, stopping receiver");
                                    break;
                                }
                            }
                        }
                    })
                    .ok();
            }

            // Start hotkey daemon + text expander
            {
                use hotkey::daemon::{HotkeyDaemon, HotkeyAction};
                use expander::engine::{ExpanderEngine, TriggerMode, ExpanderAction};

                let hk_config = config::AppConfig::load().unwrap_or_default();

                match HotkeyDaemon::new(
                    &hk_config.hotkeys.toggle_overlay,
                    &hk_config.hotkeys.paste_stack_mode,
                    &hk_config.hotkeys.quick_copy_to_pinboard,
                    &hk_config.hotkeys.toggle_expander,
                ) {
                    Ok(daemon) => {
                        let (hotkey_tx, hotkey_rx) = mpsc::channel();
                        let (keystroke_tx, keystroke_rx) = mpsc::channel();

                        match daemon.start(hotkey_tx, Some(keystroke_tx)) {
                            Ok(()) => log::info!("Hotkey daemon started"),
                            Err(e) => log::error!("Failed to start hotkey daemon: {e}"),
                        }

                        // Handle hotkey events (toggle overlay, etc.)
                        let app_handle_hk = app.handle().clone();
                        std::thread::Builder::new()
                            .name("hotkey-handler".into())
                            .spawn(move || {
                                for event in hotkey_rx {
                                    match event.action {
                                        HotkeyAction::ToggleOverlay => {
                                            if let Some(win) = app_handle_hk.get_webview_window("main") {
                                                if win.is_visible().unwrap_or(false) {
                                                    log::info!("toggle: window reported visible -> hiding");
                                                    let _ = win.hide();
                                                } else {
                                                    log::info!("toggle: window reported hidden -> showing");
                                                    // Use the ShowOverlaySender for proper show with focus handling
                                                    if let Some(sender) = app_handle_hk.try_state::<ShowOverlaySender>() {
                                                        let _ = sender.0.lock().unwrap().send(());
                                                    }
                                                }
                                            }
                                        }
                                        HotkeyAction::ToggleExpander => {
                                            log::info!("Text expander toggled");
                                        }
                                        HotkeyAction::QuickPaste(n) => {
                                            if let Some(state) = app_handle_hk.try_state::<AppState>() {
                                                let clips = state.storage.get_clips(
                                                    (n as usize) - 1, 1, &ClipFilters::default()
                                                );
                                                if let Ok(clips) = clips {
                                                    if let Some(clip) = clips.first() {
                                                        if let Some(ref text) = clip.text_content {
                                                            let _ = state.injector.inject_via_clipboard(text);
                                                            let _ = state.storage.increment_access_count(&clip.id);
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                        _ => {
                                            log::debug!("Hotkey action: {:?}", event.action);
                                        }
                                    }
                                }
                            })
                            .ok();

                        // Text expander: process keystrokes
                        let app_handle_exp = app.handle().clone();
                        std::thread::Builder::new()
                            .name("text-expander".into())
                            .spawn(move || {
                                let trigger = if hk_config.expander.trigger == "immediate" {
                                    TriggerMode::Immediate
                                } else {
                                    TriggerMode::WordBoundary
                                };
                                let mut engine = ExpanderEngine::new(trigger, 5);

                                // Load snippets into matcher
                                if let Some(state) = app_handle_exp.try_state::<AppState>() {
                                    if let Ok(snippets) = state.storage.list_snippets(None) {
                                        let entries: Vec<(String, String, String, String)> = snippets
                                            .iter()
                                            .map(|s| (
                                                s.abbreviation.clone(),
                                                s.id.clone(),
                                                s.content.clone(),
                                                s.content_type.clone(),
                                            ))
                                            .collect();
                                        engine.matcher().lock().unwrap().load(entries);
                                        log::info!("Text expander loaded {} snippets", snippets.len());
                                    }
                                }

                                for keystroke in keystroke_rx {
                                    let action = engine.process_key(keystroke.key, keystroke.pressed);
                                    match action {
                                        ExpanderAction::Expand { backspace_count, text, snippet_id } => {
                                            // +1 backspace for the trigger character (space/punctuation)
                                            let total_backspaces = backspace_count + 1;
                                            log::debug!("Expanding snippet {snippet_id}: {total_backspaces} backspaces + {} chars", text.len());
                                            if let Some(state) = app_handle_exp.try_state::<AppState>() {
                                                // Delete abbreviation + trigger char
                                                let _ = state.injector.send_backspaces(total_backspaces);
                                                std::thread::sleep(std::time::Duration::from_millis(50));
                                                // Insert expansion
                                                let _ = state.injector.inject_text(&text);
                                                // Increment use count
                                                let _ = state.storage.increment_snippet_use_count(&snippet_id);
                                            }
                                        }
                                        ExpanderAction::None => {}
                                    }
                                }
                            })
                            .ok();
                    }
                    Err(e) => {
                        log::error!("Failed to create hotkey daemon: {e}");
                        log::info!("Hotkeys and text expander will not be available");
                    }
                }
            }

            // Run retention on startup and schedule periodic runs
            let app_handle = app.handle().clone();
            // Retention limits come from [storage] in config.toml. They used to
            // be hardcoded to (90, 10000) here, so setting max_history_days = 0
            // — the documented "unlimited" — deleted history anyway (#120).
            let retention_cfg = config::AppConfig::load().unwrap_or_default().storage;
            let retention_days = retention_cfg.retention_max_days();
            let retention_count = retention_cfg.retention_max_count();
            log::info!(
                "Retention policy: max_days={}, max_count={}",
                retention_days.map_or("unlimited".to_string(), |d| d.to_string()),
                retention_count.map_or("unlimited".to_string(), |c| c.to_string()),
            );
            std::thread::Builder::new()
                .name("retention-scheduler".into())
                .spawn(move || {
                    // Initial run on startup (small delay to let app settle)
                    std::thread::sleep(std::time::Duration::from_secs(5));

                    loop {
                        // Get storage from app state
                        if let Some(state) = app_handle.try_state::<AppState>() {
                            report_db_health(&state.storage);
                            match state
                                .storage
                                .enforce_retention(retention_days, retention_count)
                            {
                                Ok(deleted) => {
                                    if deleted > 0 {
                                        log::info!("Retention: deleted {deleted} clips");
                                    }
                                }
                                Err(e) => log::error!("Retention failed: {e}"),
                            }
                        }

                        // Sleep for 1 hour
                        std::thread::sleep(std::time::Duration::from_secs(3600));
                    }
                })
                .ok();

                    // One check at startup so a database that went missing while the app
            // was stopped is reported immediately, not an hour later.
            if let Some(state) = app.handle().try_state::<AppState>() {
                report_db_health(&state.storage);
            }

    log::info!("App setup complete");
            Ok(())
        })
        .run(tauri::generate_context!())
        .unwrap_or_else(|e| {
            log::error!("Fatal: Tauri runtime error: {e}");
            eprintln!("Fatal error: {e}");
            std::process::exit(1);
        });
}
