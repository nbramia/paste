mod clipboard;
mod config;
mod expander;
mod hotkey;
mod injector;
mod storage;
mod tray;

use std::sync::Arc;
use storage::{Storage, models::{Clip, ClipFilters, Pinboard, NewPinboard}};
use injector::{select_injector, Injector};
use config::AppConfig;

/// Shared application state managed by Tauri.
pub struct AppState {
    pub storage: Storage,
    pub injector: Arc<dyn Injector>,
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
        ])
        .setup(|app| {
            tray::setup_tray(app.handle())?;
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running paste");
}
