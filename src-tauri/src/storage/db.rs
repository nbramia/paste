use std::path::PathBuf;
use std::sync::Mutex;

use rusqlite::Connection;

use super::error::StorageError;
use super::migrations;

/// Core storage struct wrapping a thread-safe SQLite connection.
pub struct Storage {
    pub(crate) conn: Mutex<Connection>,
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
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")?;

        // Run migrations instead of initialize_schema
        migrations::run_migrations(&conn, Some(&path))?;

        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    /// Create a new in-memory Storage instance (for testing).
    pub fn new_in_memory() -> Result<Self, StorageError> {
        let conn = Connection::open_in_memory()?;
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")?;

        // Run migrations for in-memory DB (no backup needed)
        migrations::run_migrations(&conn, None)?;

        Ok(Self {
            conn: Mutex::new(conn),
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
}
