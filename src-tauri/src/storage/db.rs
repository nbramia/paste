use std::path::PathBuf;
use std::sync::Mutex;

use rusqlite::Connection;

use super::error::StorageError;

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

        let storage = Self {
            conn: Mutex::new(conn),
        };
        storage.initialize_schema()?;
        Ok(storage)
    }

    /// Create a new in-memory Storage instance (for testing).
    pub fn new_in_memory() -> Result<Self, StorageError> {
        let conn = Connection::open_in_memory()?;
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")?;

        let storage = Self {
            conn: Mutex::new(conn),
        };
        storage.initialize_schema()?;
        Ok(storage)
    }

    /// Initialize the full database schema.
    fn initialize_schema(&self) -> Result<(), StorageError> {
        let conn = self.conn.lock().unwrap();
        conn.execute_batch(
            "
            -- Pinboards must exist before clips (FK reference)
            CREATE TABLE IF NOT EXISTS pinboards (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                color TEXT NOT NULL,
                icon TEXT,
                position INTEGER NOT NULL,
                created_at TEXT NOT NULL
            );

            -- Snippet groups must exist before snippets (FK reference)
            CREATE TABLE IF NOT EXISTS snippet_groups (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                position INTEGER NOT NULL,
                created_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS clips (
                id TEXT PRIMARY KEY,
                content_type TEXT NOT NULL,
                text_content TEXT,
                html_content TEXT,
                image_path TEXT,
                source_app TEXT,
                source_app_icon TEXT,
                content_hash TEXT NOT NULL,
                content_size INTEGER NOT NULL,
                metadata TEXT,
                pinboard_id TEXT REFERENCES pinboards(id) ON DELETE SET NULL,
                is_favorite BOOLEAN DEFAULT FALSE,
                created_at TEXT NOT NULL,
                accessed_at TEXT,
                access_count INTEGER DEFAULT 0
            );

            CREATE VIRTUAL TABLE IF NOT EXISTS clips_fts USING fts5(
                text_content,
                content='clips',
                content_rowid='rowid'
            );

            -- FTS5 sync triggers
            CREATE TRIGGER IF NOT EXISTS clips_ai AFTER INSERT ON clips BEGIN
                INSERT INTO clips_fts(rowid, text_content)
                VALUES (new.rowid, new.text_content);
            END;

            CREATE TRIGGER IF NOT EXISTS clips_ad AFTER DELETE ON clips BEGIN
                INSERT INTO clips_fts(clips_fts, rowid, text_content)
                VALUES('delete', old.rowid, old.text_content);
            END;

            CREATE TRIGGER IF NOT EXISTS clips_au AFTER UPDATE ON clips BEGIN
                INSERT INTO clips_fts(clips_fts, rowid, text_content)
                VALUES('delete', old.rowid, old.text_content);
                INSERT INTO clips_fts(rowid, text_content)
                VALUES (new.rowid, new.text_content);
            END;

            CREATE TABLE IF NOT EXISTS snippets (
                id TEXT PRIMARY KEY,
                abbreviation TEXT NOT NULL UNIQUE,
                name TEXT NOT NULL,
                content TEXT NOT NULL,
                content_type TEXT NOT NULL,
                group_id TEXT REFERENCES snippet_groups(id) ON DELETE SET NULL,
                description TEXT,
                use_count INTEGER DEFAULT 0,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS paste_stack (
                id TEXT PRIMARY KEY,
                clip_id TEXT NOT NULL REFERENCES clips(id) ON DELETE CASCADE,
                position INTEGER NOT NULL,
                created_at TEXT NOT NULL
            );

            -- Indexes
            CREATE INDEX IF NOT EXISTS idx_clips_created_at ON clips(created_at DESC);
            CREATE INDEX IF NOT EXISTS idx_clips_content_type ON clips(content_type);
            CREATE INDEX IF NOT EXISTS idx_clips_source_app ON clips(source_app);
            CREATE INDEX IF NOT EXISTS idx_clips_pinboard_id ON clips(pinboard_id);
            CREATE INDEX IF NOT EXISTS idx_clips_content_hash ON clips(content_hash);
            CREATE INDEX IF NOT EXISTS idx_snippets_abbreviation ON snippets(abbreviation);
            ",
        )?;
        Ok(())
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
    fn test_schema_tables_exist() {
        let storage = Storage::new_in_memory().unwrap();
        let conn = storage.conn.lock().unwrap();

        let tables: Vec<String> = conn
            .prepare("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name")
            .unwrap()
            .query_map([], |row| row.get(0))
            .unwrap()
            .filter_map(|r| r.ok())
            .collect();

        assert!(tables.contains(&"clips".to_string()));
        assert!(tables.contains(&"pinboards".to_string()));
        assert!(tables.contains(&"snippets".to_string()));
        assert!(tables.contains(&"snippet_groups".to_string()));
        assert!(tables.contains(&"paste_stack".to_string()));
    }

    #[test]
    fn test_schema_fts_table_exists() {
        let storage = Storage::new_in_memory().unwrap();
        let conn = storage.conn.lock().unwrap();

        let tables: Vec<String> = conn
            .prepare("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name")
            .unwrap()
            .query_map([], |row| row.get(0))
            .unwrap()
            .filter_map(|r| r.ok())
            .collect();

        assert!(tables.contains(&"clips_fts".to_string()));
    }

    #[test]
    fn test_schema_idempotent() {
        // Creating storage twice on same connection should not fail
        let storage = Storage::new_in_memory().unwrap();
        let result = storage.initialize_schema();
        assert!(result.is_ok());
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
