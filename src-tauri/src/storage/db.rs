use std::path::{Path, PathBuf};
use std::sync::Mutex;

use rusqlite::Connection;

use super::error::StorageError;
use super::migrations;

/// Identity of the database file at the moment it was opened.
///
/// Compared against the path later to notice that the file we are writing to is
/// no longer the file that lives at that path — see [`Storage::health_check`].
#[derive(Debug, Clone)]
pub(crate) struct DbFile {
    path: PathBuf,
    dev: u64,
    ino: u64,
}

/// Result of a database health check.
#[derive(Debug, PartialEq, Eq)]
pub enum DbHealth {
    /// The open file is still the file at the expected path.
    Ok,
    /// In-memory database (tests); nothing to verify.
    InMemory,
    /// Nothing exists at the path any more. Writes are going to an unlinked
    /// inode and will be lost when the process exits.
    Missing,
    /// A different file now occupies the path. Our writes are invisible to
    /// anything reading it, and are lost on exit.
    Replaced,
    /// The path could not be inspected.
    Unreadable(String),
}

impl DbHealth {
    /// Whether this state means writes are being lost.
    pub fn is_losing_writes(&self) -> bool {
        matches!(self, DbHealth::Missing | DbHealth::Replaced)
    }
}

/// Core storage struct wrapping a thread-safe SQLite connection.
pub struct Storage {
    pub(crate) conn: Mutex<Connection>,
    /// `None` for in-memory databases.
    pub(crate) file: Option<DbFile>,
}

impl Storage {
    /// Create a new Storage instance.
    ///
    /// If `db_path` is `None`, the database is created at
    /// `~/.local/share/paste/paste.db`.
    pub fn new(db_path: Option<PathBuf>) -> Result<Self, StorageError> {
        let path = match db_path {
            Some(p) => p,
            None => {
                let data_dir = dirs::data_dir()
                    .expect("could not determine data directory")
                    .join("paste");
                std::fs::create_dir_all(&data_dir)?;
                data_dir.join("paste.db")
            }
        };

        let conn = Connection::open(&path)?;

        // Rollback journal rather than WAL.
        //
        // WAL exists to let readers and writers work concurrently. This app has
        // a single `Mutex<Connection>` and no concurrent access, so it bought
        // nothing — while introducing a silent data-loss mode: if the `-wal`
        // and `-shm` sidecars are removed while the app holds them open, every
        // commit goes to an unlinked inode. SQLite keeps reporting success, the
        // UI keeps showing the data, and it all disappears at exit. That
        // happened in practice and cost two days of clipboard history (#113).
        //
        // With DELETE there are no long-lived sidecar files, and every commit
        // lands in `paste.db` itself.
        let mode: String = conn.query_row("PRAGMA journal_mode=DELETE", [], |r| r.get(0))?;
        if !mode.eq_ignore_ascii_case("delete") {
            log::warn!("database journal mode is {mode}, expected delete");
        }
        conn.execute_batch("PRAGMA foreign_keys=ON;")?;

        // Run migrations instead of initialize_schema
        migrations::run_migrations(&conn, Some(&path))?;

        let file = Self::identify(&path);
        if file.is_none() {
            log::warn!("could not record database file identity; health checks disabled");
        }

        Ok(Self {
            conn: Mutex::new(conn),
            file,
        })
    }

    fn identify(path: &Path) -> Option<DbFile> {
        use std::os::unix::fs::MetadataExt;
        let md = std::fs::metadata(path).ok()?;
        Some(DbFile {
            path: path.to_path_buf(),
            dev: md.dev(),
            ino: md.ino(),
        })
    }

    /// Check that writes are still reaching the database file on disk.
    ///
    /// SQLite happily writes to a file that has been unlinked or replaced
    /// underneath it, reporting success the whole time, so nothing in the
    /// normal code path notices. This compares the identity of the file we
    /// opened against whatever is at the path now.
    pub fn health_check(&self) -> DbHealth {
        use std::os::unix::fs::MetadataExt;

        let Some(ref file) = self.file else {
            return DbHealth::InMemory;
        };

        match std::fs::metadata(&file.path) {
            Ok(md) if md.dev() == file.dev && md.ino() == file.ino => DbHealth::Ok,
            Ok(_) => DbHealth::Replaced,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => DbHealth::Missing,
            Err(e) => DbHealth::Unreadable(e.to_string()),
        }
    }

    /// Create a new in-memory Storage instance (for testing).
    pub fn new_in_memory() -> Result<Self, StorageError> {
        let conn = Connection::open_in_memory()?;
        conn.execute_batch("PRAGMA foreign_keys=ON;")?;

        // Run migrations for in-memory DB (no backup needed)
        migrations::run_migrations(&conn, None)?;

        Ok(Self {
            conn: Mutex::new(conn),
            file: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_in_memory_storage() {
        let storage = Storage::new_in_memory();
        assert!(storage.is_ok());
    }

    #[test]
    fn test_pragmas() {
        let storage = Storage::new_in_memory().unwrap();
        let conn = storage.conn.lock().unwrap();

        let fk_enabled: bool = conn
            .query_row("PRAGMA foreign_keys", [], |row| row.get(0))
            .unwrap();
        assert!(fk_enabled);
    }

    /// Unique scratch directory per test; no tempfile dependency needed.
    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("paste_db_test_{name}_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn test_file_db_uses_rollback_journal() {
        let dir = scratch("journal");
        let storage = Storage::new(Some(dir.join("paste.db"))).unwrap();

        let mode: String = storage
            .conn
            .lock()
            .unwrap()
            .query_row("PRAGMA journal_mode", [], |row| row.get(0))
            .unwrap();

        // WAL would leave -wal/-shm sidecars that can be unlinked underneath a
        // running app, silently discarding every later write (#113).
        assert_eq!(mode.to_lowercase(), "delete");
        assert!(!dir.join("paste.db-wal").exists());

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_health_check_ok_for_untouched_file() {
        let dir = scratch("healthy");
        let storage = Storage::new(Some(dir.join("paste.db"))).unwrap();
        assert_eq!(storage.health_check(), DbHealth::Ok);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_health_check_detects_deleted_database() {
        let dir = scratch("deleted");
        let path = dir.join("paste.db");
        let storage = Storage::new(Some(path.clone())).unwrap();

        std::fs::remove_file(&path).unwrap();

        // The connection still works and reports success — that is precisely
        // why this needs an explicit check.
        assert_eq!(storage.health_check(), DbHealth::Missing);
        assert!(storage.health_check().is_losing_writes());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_health_check_detects_replaced_database() {
        let dir = scratch("replaced");
        let path = dir.join("paste.db");
        let storage = Storage::new(Some(path.clone())).unwrap();

        std::fs::remove_file(&path).unwrap();
        std::fs::write(&path, b"a different file").unwrap();

        assert_eq!(storage.health_check(), DbHealth::Replaced);
        assert!(storage.health_check().is_losing_writes());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_health_check_is_quiet_for_in_memory() {
        let storage = Storage::new_in_memory().unwrap();
        assert_eq!(storage.health_check(), DbHealth::InMemory);
        assert!(!storage.health_check().is_losing_writes());
    }
}
