//! Structured logging setup with stderr and file output.
//!
//! - Reads `RUST_LOG` env var for level control (default: `info`)
//! - Writes to stderr (for development)
//! - Writes to `~/.local/share/paste/paste.log` (for production debugging)
//! - Rotates log file at 5 MB

use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::sync::Mutex;

use env_logger::Builder;

/// Global log file handle, initialised once by [`init_logging`].
static LOG_FILE: std::sync::OnceLock<Mutex<fs::File>> = std::sync::OnceLock::new();

/// Initialize logging with both stderr and file output.
///
/// Safe to call only once (env_logger panics on double-init).
pub fn init_logging() {
    let log_path = log_file_path();

    // Ensure log directory exists
    if let Some(parent) = log_path.parent() {
        let _ = fs::create_dir_all(parent);
    }

    // Rotate log if it exceeds 5 MB
    if let Ok(metadata) = fs::metadata(&log_path) {
        if metadata.len() > 5 * 1024 * 1024 {
            let backup = log_path.with_extension("log.old");
            let _ = fs::rename(&log_path, &backup);
        }
    }

    // Open log file for appending
    if let Ok(file) = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)
    {
        let _ = LOG_FILE.set(Mutex::new(file));
    }

    Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .format(|buf, record| {
            let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f");
            let line = format!(
                "{} [{}] {}: {}",
                timestamp,
                record.level(),
                record.target(),
                record.args()
            );

            // Write to stderr via env_logger
            writeln!(buf, "{}", line)?;

            // Write to log file
            if let Some(file_mutex) = LOG_FILE.get() {
                if let Ok(mut file) = file_mutex.lock() {
                    let _ = writeln!(file, "{}", line);
                }
            }

            Ok(())
        })
        .init();

    log::info!("Paste v0.1.0 starting -- log file: {}", log_path.display());
}

/// Return the log file path (`~/.local/share/paste/paste.log`).
fn log_file_path() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("/tmp"))
        .join("paste")
        .join("paste.log")
}
