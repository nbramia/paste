mod clipboard;
mod config;
mod expander;
mod hotkey;
mod injector;
mod storage;
mod tray;

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use storage::{Storage, models::{Clip, ClipFilters, Pinboard, NewPinboard, Snippet, NewSnippet, UpdateSnippet, SnippetGroup, NewSnippetGroup, StorageStats}};
use injector::{select_injector, Injector};
use config::AppConfig;
use clipboard::stack::PasteStack;
use expander::template::{FillInField, parse_template, extract_fill_in_fields, evaluate_tokens, ExpansionContext};
use expander::import::{ImportedSnippet, ImportResult, parse_espanso_dir, default_espanso_path};

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
) -> Result<Vec<Clip>, String> {
    let filters = ClipFilters {
        content_type,
        source_app,
        date_from: None,
        date_to: None,
        pinboard_id,
    };
    state.storage
        .get_clips(offset, limit, &filters)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn paste_clip(
    state: tauri::State<'_, AppState>,
    id: String,
) -> Result<(), String> {
    // Get the clip from storage
    let clip = state.storage
        .get_clip_by_id(&id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Clip not found: {}", id))?;

    // Inject the text content
    if let Some(ref text) = clip.text_content {
        state.injector
            .inject_via_clipboard(text)
            .map_err(|e| e.to_string())?;
    }

    // Increment access count
    state.storage
        .increment_access_count(&id)
        .map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
fn delete_clip(
    state: tauri::State<'_, AppState>,
    id: String,
) -> Result<(), String> {
    state.storage
        .delete_clip(&id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn search_clips(
    state: tauri::State<'_, AppState>,
    query: String,
    content_type: Option<String>,
    source_app: Option<String>,
    date_from: Option<String>,
    date_to: Option<String>,
    pinboard_id: Option<String>,
) -> Result<Vec<Clip>, String> {
    let filters = ClipFilters {
        content_type,
        source_app,
        date_from,
        date_to,
        pinboard_id,
    };
    state.storage
        .search_clips(&query, &filters)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn get_source_apps(
    state: tauri::State<'_, AppState>,
) -> Result<Vec<String>, String> {
    state.storage
        .get_distinct_source_apps()
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn list_pinboards(
    state: tauri::State<'_, AppState>,
) -> Result<Vec<Pinboard>, String> {
    state.storage.list_pinboards().map_err(|e| e.to_string())
}

#[tauri::command]
fn create_pinboard(
    state: tauri::State<'_, AppState>,
    name: String,
    color: String,
) -> Result<Pinboard, String> {
    let new_pb = NewPinboard { name, color, icon: None };
    state.storage.create_pinboard(&new_pb).map_err(|e| e.to_string())
}

#[tauri::command]
fn update_pinboard(
    state: tauri::State<'_, AppState>,
    id: String,
    name: String,
    color: String,
) -> Result<Pinboard, String> {
    state.storage.update_pinboard(&id, &name, &color, None).map_err(|e| e.to_string())
}

#[tauri::command]
fn delete_pinboard(
    state: tauri::State<'_, AppState>,
    id: String,
) -> Result<(), String> {
    state.storage.delete_pinboard(&id).map_err(|e| e.to_string())
}

#[tauri::command]
fn add_clip_to_pinboard(
    state: tauri::State<'_, AppState>,
    clip_id: String,
    pinboard_id: String,
) -> Result<(), String> {
    state.storage.update_clip_pinboard(&clip_id, Some(&pinboard_id)).map_err(|e| e.to_string())
}

#[tauri::command]
fn remove_clip_from_pinboard(
    state: tauri::State<'_, AppState>,
    clip_id: String,
) -> Result<(), String> {
    state.storage.update_clip_pinboard(&clip_id, None).map_err(|e| e.to_string())
}

#[tauri::command]
fn quick_paste(
    state: tauri::State<'_, AppState>,
    n: usize,
) -> Result<(), String> {
    if n == 0 || n > 9 {
        return Err("Quick paste index must be between 1 and 9".into());
    }

    // Get the Nth most recent clip (n=1 means most recent, offset=0)
    let clips = state.storage
        .get_clips(n - 1, 1, &ClipFilters::default())
        .map_err(|e| e.to_string())?;

    let clip = clips.into_iter().next()
        .ok_or_else(|| format!("No clip at position {}", n))?;

    // Inject the text content
    if let Some(ref text) = clip.text_content {
        state.injector
            .inject_via_clipboard(text)
            .map_err(|e| e.to_string())?;
    }

    // Increment access count
    state.storage
        .increment_access_count(&clip.id)
        .map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
fn toggle_paste_stack(
    state: tauri::State<'_, AppState>,
) -> Result<bool, String> {
    let active = state.paste_stack.toggle();
    Ok(active)
}

#[tauri::command]
fn get_paste_stack(
    state: tauri::State<'_, AppState>,
) -> Result<Vec<Clip>, String> {
    Ok(state.paste_stack.get_all())
}

#[tauri::command]
fn get_paste_stack_status(
    state: tauri::State<'_, AppState>,
) -> Result<(bool, usize), String> {
    Ok((state.paste_stack.is_active(), state.paste_stack.len()))
}

#[tauri::command]
fn add_to_paste_stack(
    state: tauri::State<'_, AppState>,
    clip_id: String,
) -> Result<(), String> {
    let clip = state.storage
        .get_clip_by_id(&clip_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Clip not found: {}", clip_id))?;
    state.paste_stack.push(clip);
    Ok(())
}

#[tauri::command]
fn pop_paste_stack(
    state: tauri::State<'_, AppState>,
) -> Result<Option<String>, String> {
    let clip = state.paste_stack.pop_next();

    // If stack is now empty, auto-deactivate
    if state.paste_stack.is_empty() {
        state.paste_stack.deactivate();
    }

    // If we got a clip, inject it
    if let Some(ref clip) = clip {
        if let Some(ref text) = clip.text_content {
            state.injector
                .inject_via_clipboard(text)
                .map_err(|e| e.to_string())?;
        }
        state.storage
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
fn clear_paste_stack(
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    state.paste_stack.deactivate();
    Ok(())
}

#[tauri::command]
fn list_snippets(
    state: tauri::State<'_, AppState>,
    group_id: Option<String>,
) -> Result<Vec<Snippet>, String> {
    state.storage.list_snippets(group_id.as_deref()).map_err(|e| e.to_string())
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
    state.storage.create_snippet(&new).map_err(|e| e.to_string())
}

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
    state.storage.update_snippet(&id, &update).map_err(|e| e.to_string())
}

#[tauri::command]
fn delete_snippet(
    state: tauri::State<'_, AppState>,
    id: String,
) -> Result<(), String> {
    state.storage.delete_snippet(&id).map_err(|e| e.to_string())
}

#[tauri::command]
fn list_snippet_groups(
    state: tauri::State<'_, AppState>,
) -> Result<Vec<SnippetGroup>, String> {
    state.storage.list_snippet_groups().map_err(|e| e.to_string())
}

#[tauri::command]
fn create_snippet_group(
    state: tauri::State<'_, AppState>,
    name: String,
) -> Result<SnippetGroup, String> {
    let new = NewSnippetGroup { name };
    state.storage.create_snippet_group(&new).map_err(|e| e.to_string())
}

#[tauri::command]
fn delete_snippet_group(
    state: tauri::State<'_, AppState>,
    id: String,
) -> Result<(), String> {
    state.storage.delete_snippet_group(&id).map_err(|e| e.to_string())
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
fn preview_espanso_import(
    path: Option<String>,
) -> Result<Vec<ImportedSnippet>, String> {
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
        match state.storage.get_snippet_by_abbreviation(&snippet.abbreviation) {
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
fn get_excluded_apps(
    state: tauri::State<'_, AppState>,
) -> Vec<String> {
    state.excluded_apps.lock().unwrap().clone()
}

#[tauri::command]
fn add_excluded_app(
    state: tauri::State<'_, AppState>,
    app_name: String,
) -> Vec<String> {
    let mut apps = state.excluded_apps.lock().unwrap();
    let lower = app_name.to_lowercase();
    if !apps.iter().any(|a| a.to_lowercase() == lower) {
        apps.push(app_name);
    }
    apps.clone()
}

#[tauri::command]
fn remove_excluded_app(
    state: tauri::State<'_, AppState>,
    app_name: String,
) -> Vec<String> {
    let mut apps = state.excluded_apps.lock().unwrap();
    let lower = app_name.to_lowercase();
    apps.retain(|a| a.to_lowercase() != lower);
    apps.clone()
}

#[tauri::command]
fn get_storage_stats(
    state: tauri::State<'_, AppState>,
) -> Result<StorageStats, String> {
    state.storage.get_storage_stats().map_err(|e| e.to_string())
}

#[tauri::command]
fn run_retention(
    state: tauri::State<'_, AppState>,
) -> Result<usize, String> {
    let max_days = Some(90u32);
    let max_count = Some(10000usize);
    state.storage.enforce_retention(max_days, max_count).map_err(|e| e.to_string())
}

#[tauri::command]
fn clear_all_history(
    state: tauri::State<'_, AppState>,
) -> Result<usize, String> {
    // Delete all non-pinboard, non-favorite clips
    state.storage.enforce_retention(None, Some(0)).map_err(|e| e.to_string())
}

pub fn run() {
    // Load config
    let config = AppConfig::load().unwrap_or_else(|e| {
        eprintln!("Warning: Failed to load config: {}. Using defaults.", e);
        AppConfig::default()
    });

    // Initialize storage
    let storage = Storage::new(Some(config.resolved_db_path()))
        .expect("Failed to initialize storage");

    // Initialize injector
    let injector = select_injector(&config.injection.method)
        .expect("Failed to initialize text injector");

    let app_state = AppState {
        storage,
        injector: Arc::from(injector),
        paste_stack: PasteStack::new(),
        excluded_apps: Mutex::new(config.clipboard.excluded_apps.clone()),
    };

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .manage(app_state)
        .invoke_handler(tauri::generate_handler![
            get_clips,
            paste_clip,
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
            get_excluded_apps,
            add_excluded_app,
            remove_excluded_app,
            get_storage_stats,
            run_retention,
            clear_all_history,
        ])
        .setup(|app| {
            tray::setup_tray(app.handle())?;

            // Run retention on startup and schedule periodic runs
            let app_handle = app.handle().clone();
            std::thread::Builder::new()
                .name("retention-scheduler".into())
                .spawn(move || {
                    // Initial run on startup (small delay to let app settle)
                    std::thread::sleep(std::time::Duration::from_secs(5));

                    loop {
                        // Get storage from app state
                        if let Some(state) = app_handle.try_state::<AppState>() {
                            match state.storage.enforce_retention(Some(90), Some(10000)) {
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

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running paste");
}
