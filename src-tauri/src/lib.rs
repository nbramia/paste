mod clipboard;
mod config;
mod expander;
mod hotkey;
mod injector;
mod storage;
mod tray;

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! Welcome to Paste.", name)
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![greet])
        .run(tauri::generate_context!())
        .expect("error while running paste");
}
