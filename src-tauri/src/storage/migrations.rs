//! Database schema migration system.
//!
//! Migrations are versioned SQL scripts that run sequentially on startup.
//! The current schema version is stored in the `schema_version` table.

use log::{info, warn};
use rusqlite::Connection;

use super::error::StorageError;

/// A database migration.
struct Migration {
    version: u32,
    description: &'static str,
    sql: &'static str,
}

/// All migrations in order. Each migration upgrades from version N-1 to N.
/// Migration 1 is the baseline — the full initial schema.
const MIGRATIONS: &[Migration] = &[
    Migration {
        version: 1,
        description: "Initial schema",
        sql: r#"
            -- Pinboards (must exist before clips FK)
            CREATE TABLE IF NOT EXISTS pinboards (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                color TEXT NOT NULL,
                icon TEXT,
                position INTEGER NOT NULL,
                created_at TEXT NOT NULL
            );

            -- Snippet groups (must exist before snippets FK)
            CREATE TABLE IF NOT EXISTS snippet_groups (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                position INTEGER NOT NULL,
                created_at TEXT NOT NULL
            );

            -- Clipboard history
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

            -- FTS5 full-text search
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

            -- Snippets
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

            -- Paste stack
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
        "#,
    },
    // Future migrations go here:
    // Migration {
    //     version: 2,
    //     description: "Add tags column to clips",
    //     sql: "ALTER TABLE clips ADD COLUMN tags TEXT;",
    // },
];

/// The current expected schema version.
pub const CURRENT_VERSION: u32 = 1;

/// Run all pending migrations on the database.
///
/// Creates the `schema_version` table if it doesn't exist,
/// checks the current version, and runs any pending migrations
/// in order within a transaction.
pub fn run_migrations(
    conn: &Connection,
    db_path: Option<&std::path::Path>,
) -> Result<(), StorageError> {
    // Create schema_version table if it doesn't exist
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS schema_version (
            version INTEGER NOT NULL,
            applied_at TEXT NOT NULL
        );",
    )?;

    // Get current version
    let current_version: u32 = conn.query_row(
        "SELECT COALESCE(MAX(version), 0) FROM schema_version",
        [],
        |row| row.get(0),
    )?;

    if current_version >= CURRENT_VERSION {
        return Ok(()); // Already up to date
    }

    info!(
        "Database migration needed: v{} → v{}",
        current_version, CURRENT_VERSION
    );

    // Backup the database file before migrating (if it's a file-based DB)
    if let Some(path) = db_path {
        if path.exists() && current_version > 0 {
            let backup_path = path.with_extension(format!("v{}.bak", current_version));
            match std::fs::copy(path, &backup_path) {
                Ok(_) => info!("Database backed up to {}", backup_path.display()),
                Err(e) => warn!("Failed to backup database: {e}"),
            }
        }
    }

    // Run pending migrations
    for migration in MIGRATIONS {
        if migration.version <= current_version {
            continue; // Already applied
        }

        info!(
            "Applying migration v{}: {}",
            migration.version, migration.description
        );

        // Run migration in a transaction
        let tx = conn.unchecked_transaction()?;

        match tx.execute_batch(migration.sql) {
            Ok(_) => {
                let now = chrono::Utc::now().to_rfc3339();
                tx.execute(
                    "INSERT INTO schema_version (version, applied_at) VALUES (?1, ?2)",
                    rusqlite::params![migration.version, now],
                )?;
                tx.commit()?;
                info!("Migration v{} applied successfully", migration.version);
            }
            Err(e) => {
                // Transaction is rolled back on drop
                return Err(StorageError::Database(e));
            }
        }
    }

    Ok(())
}

/// Get the current schema version.
pub fn get_schema_version(conn: &Connection) -> Result<u32, StorageError> {
    // Check if schema_version table exists
    let exists: bool = conn.query_row(
        "SELECT COUNT(*) > 0 FROM sqlite_master WHERE type='table' AND name='schema_version'",
        [],
        |row| row.get(0),
    )?;

    if !exists {
        return Ok(0);
    }

    let version: u32 = conn.query_row(
        "SELECT COALESCE(MAX(version), 0) FROM schema_version",
        [],
        |row| row.get(0),
    )?;

    Ok(version)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    #[test]
    fn test_fresh_db_migrates_to_current() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("PRAGMA foreign_keys=ON;").unwrap();

        run_migrations(&conn, None).unwrap();

        let version = get_schema_version(&conn).unwrap();
        assert_eq!(version, CURRENT_VERSION);
    }

    #[test]
    fn test_migration_creates_all_tables() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("PRAGMA foreign_keys=ON;").unwrap();
        run_migrations(&conn, None).unwrap();

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
        assert!(tables.contains(&"schema_version".to_string()));
    }

    #[test]
    fn test_migration_is_idempotent() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("PRAGMA foreign_keys=ON;").unwrap();

        run_migrations(&conn, None).unwrap();
        run_migrations(&conn, None).unwrap(); // second run should be no-op

        let version = get_schema_version(&conn).unwrap();
        assert_eq!(version, CURRENT_VERSION);

        // Only one version entry should exist
        let count: u32 = conn
            .query_row("SELECT COUNT(*) FROM schema_version", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn test_get_version_no_table() {
        let conn = Connection::open_in_memory().unwrap();
        let version = get_schema_version(&conn).unwrap();
        assert_eq!(version, 0);
    }

    #[test]
    fn test_get_version_empty_table() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE schema_version (version INTEGER NOT NULL, applied_at TEXT NOT NULL);",
        )
        .unwrap();
        let version = get_schema_version(&conn).unwrap();
        assert_eq!(version, 0);
    }

    #[test]
    fn test_current_version_matches_migrations() {
        assert_eq!(CURRENT_VERSION, MIGRATIONS.last().unwrap().version);
    }

    #[test]
    fn test_migrations_are_sequential() {
        for (i, migration) in MIGRATIONS.iter().enumerate() {
            assert_eq!(
                migration.version,
                (i + 1) as u32,
                "Migration {} has wrong version",
                i
            );
        }
    }
}
