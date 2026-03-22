//! System tray integration using Tauri v2's built-in tray icon support.

use tauri::{
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::TrayIconBuilder,
    AppHandle, Manager,
};
use log::info;

/// Set up the system tray icon and context menu.
///
/// Call this inside `tauri::Builder::setup()`.
pub fn setup_tray(app: &AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    info!("Setting up system tray");

    // Build the context menu
    let quit = MenuItem::with_id(app, "quit", "Quit Paste", true, None::<&str>)?;
    let settings = MenuItem::with_id(app, "settings", "Settings", true, None::<&str>)?;
    let about = MenuItem::with_id(app, "about", "About Paste", true, None::<&str>)?;
    let toggle_expander =
        MenuItem::with_id(app, "toggle_expander", "Text Expander: ON", true, None::<&str>)?;
    let toggle_paste_stack =
        MenuItem::with_id(app, "toggle_paste_stack", "Paste Stack: OFF", true, None::<&str>)?;
    let show_overlay =
        MenuItem::with_id(app, "show_overlay", "Show Clipboard (Super+V)", true, None::<&str>)?;

    let separator1 = PredefinedMenuItem::separator(app)?;
    let separator2 = PredefinedMenuItem::separator(app)?;
    let separator3 = PredefinedMenuItem::separator(app)?;

    let menu = Menu::with_items(app, &[
        &show_overlay,
        &separator1,
        &toggle_paste_stack,
        &toggle_expander,
        &separator2,
        &settings,
        &about,
        &separator3,
        &quit,
    ])?;

    // Build the tray icon using the app's default icon
    let _tray = TrayIconBuilder::new()
        .icon(app.default_window_icon().cloned().expect("no app icon"))
        .menu(&menu)
        .tooltip("Paste — Clipboard Manager")
        .on_menu_event(move |app, event| {
            match event.id.as_ref() {
                "quit" => {
                    info!("Quit requested from tray");
                    app.exit(0);
                }
                "show_overlay" => {
                    info!("Show overlay requested from tray");
                    if let Err(e) = app.emit("tray-show-overlay", ()) {
                        log::error!("Failed to emit show-overlay event: {}", e);
                    }
                }
                "toggle_expander" => {
                    info!("Toggle expander requested from tray");
                    if let Err(e) = app.emit("tray-toggle-expander", ()) {
                        log::error!("Failed to emit toggle-expander event: {}", e);
                    }
                }
                "toggle_paste_stack" => {
                    info!("Toggle paste stack requested from tray");
                    if let Err(e) = app.emit("tray-toggle-paste-stack", ()) {
                        log::error!("Failed to emit toggle-paste-stack event: {}", e);
                    }
                }
                "settings" => {
                    info!("Settings requested from tray");
                    if let Err(e) = app.emit("tray-open-settings", ()) {
                        log::error!("Failed to emit open-settings event: {}", e);
                    }
                }
                "about" => {
                    info!("About requested from tray");
                    // Could open a dialog or log version info
                }
                _ => {
                    log::debug!("Unknown tray menu event: {}", event.id.as_ref());
                }
            }
        })
        .build(app)?;

    info!("System tray initialized");
    Ok(())
}
