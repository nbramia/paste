use chrono::Utc;
use rusqlite::params;
use uuid::Uuid;

use super::db::Storage;
use super::error::StorageError;
use super::models::{NewSnippet, NewSnippetGroup, Snippet, SnippetGroup, UpdateSnippet};

impl Storage {
    // ─── Snippet CRUD ────────────────────────────────────────────

    /// Create a new snippet.
    pub fn create_snippet(&self, new_snippet: &NewSnippet) -> Result<Snippet, StorageError> {
        let id = Uuid::now_v7().to_string();
        let now = Utc::now().to_rfc3339();

        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO snippets (
                id, abbreviation, name, content, content_type,
                group_id, description, use_count, created_at, updated_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 0, ?8, ?9)",
            params![
                id,
                new_snippet.abbreviation,
                new_snippet.name,
                new_snippet.content,
                new_snippet.content_type,
                new_snippet.group_id,
                new_snippet.description,
                now,
                now,
            ],
        )?;

        Ok(Snippet {
            id,
            abbreviation: new_snippet.abbreviation.clone(),
            name: new_snippet.name.clone(),
            content: new_snippet.content.clone(),
            content_type: new_snippet.content_type.clone(),
            group_id: new_snippet.group_id.clone(),
            description: new_snippet.description.clone(),
            use_count: 0,
            created_at: now.clone(),
            updated_at: now,
        })
    }

    /// Get a snippet by its abbreviation (used for text expansion lookup).
    pub fn get_snippet_by_abbreviation(
        &self,
        abbreviation: &str,
    ) -> Result<Option<Snippet>, StorageError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, abbreviation, name, content, content_type,
                    group_id, description, use_count, created_at, updated_at
             FROM snippets WHERE abbreviation = ?1",
        )?;

        let mut rows = stmt.query_map(params![abbreviation], row_to_snippet)?;

        match rows.next() {
            Some(Ok(s)) => Ok(Some(s)),
            Some(Err(e)) => Err(StorageError::Database(e)),
            None => Ok(None),
        }
    }

    /// Get a snippet by its id.
    pub fn get_snippet_by_id(&self, id: &str) -> Result<Option<Snippet>, StorageError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, abbreviation, name, content, content_type,
                    group_id, description, use_count, created_at, updated_at
             FROM snippets WHERE id = ?1",
        )?;

        let mut rows = stmt.query_map(params![id], row_to_snippet)?;

        match rows.next() {
            Some(Ok(s)) => Ok(Some(s)),
            Some(Err(e)) => Err(StorageError::Database(e)),
            None => Ok(None),
        }
    }

    /// List all snippets, optionally filtered by group_id.
    pub fn list_snippets(
        &self,
        group_id: Option<&str>,
    ) -> Result<Vec<Snippet>, StorageError> {
        let conn = self.conn.lock().unwrap();

        let (sql, params): (&str, Vec<Box<dyn rusqlite::types::ToSql>>) = match group_id {
            Some(gid) => (
                "SELECT id, abbreviation, name, content, content_type,
                        group_id, description, use_count, created_at, updated_at
                 FROM snippets WHERE group_id = ?1 ORDER BY name",
                vec![Box::new(gid.to_string())],
            ),
            None => (
                "SELECT id, abbreviation, name, content, content_type,
                        group_id, description, use_count, created_at, updated_at
                 FROM snippets ORDER BY name",
                vec![],
            ),
        };

        let params_refs: Vec<&dyn rusqlite::types::ToSql> =
            params.iter().map(|p| p.as_ref()).collect();

        let mut stmt = conn.prepare(sql)?;
        let snippets = stmt
            .query_map(params_refs.as_slice(), row_to_snippet)?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(snippets)
    }

    /// Update a snippet. Updates the `updated_at` timestamp.
    pub fn update_snippet(
        &self,
        id: &str,
        update: &UpdateSnippet,
    ) -> Result<Snippet, StorageError> {
        let now = Utc::now().to_rfc3339();
        let conn = self.conn.lock().unwrap();

        let affected = conn.execute(
            "UPDATE snippets SET
                abbreviation = ?1, name = ?2, content = ?3,
                content_type = ?4, group_id = ?5, description = ?6,
                updated_at = ?7
             WHERE id = ?8",
            params![
                update.abbreviation,
                update.name,
                update.content,
                update.content_type,
                update.group_id,
                update.description,
                now,
                id
            ],
        )?;

        if affected == 0 {
            return Err(StorageError::NotFound(format!("snippet {id}")));
        }
        drop(conn);

        self.get_snippet_by_id(id)?
            .ok_or_else(|| StorageError::NotFound(format!("snippet {id}")))
    }

    /// Delete a snippet by id.
    pub fn delete_snippet(&self, id: &str) -> Result<(), StorageError> {
        let conn = self.conn.lock().unwrap();
        let affected = conn.execute("DELETE FROM snippets WHERE id = ?1", params![id])?;

        if affected == 0 {
            return Err(StorageError::NotFound(format!("snippet {id}")));
        }
        Ok(())
    }

    /// Increment the use count of a snippet.
    pub fn increment_snippet_use_count(&self, id: &str) -> Result<(), StorageError> {
        let conn = self.conn.lock().unwrap();
        let affected = conn.execute(
            "UPDATE snippets SET use_count = use_count + 1 WHERE id = ?1",
            params![id],
        )?;

        if affected == 0 {
            return Err(StorageError::NotFound(format!("snippet {id}")));
        }
        Ok(())
    }

    // ─── Snippet Group CRUD ─────────────────────────────────────

    /// Create a new snippet group. Position is auto-assigned.
    pub fn create_snippet_group(
        &self,
        new_group: &NewSnippetGroup,
    ) -> Result<SnippetGroup, StorageError> {
        let id = Uuid::now_v7().to_string();
        let created_at = Utc::now().to_rfc3339();

        let conn = self.conn.lock().unwrap();

        let position: i64 = conn
            .query_row(
                "SELECT COALESCE(MAX(position), -1) + 1 FROM snippet_groups",
                [],
                |row| row.get(0),
            )
            .unwrap_or(0);

        conn.execute(
            "INSERT INTO snippet_groups (id, name, position, created_at)
             VALUES (?1, ?2, ?3, ?4)",
            params![id, new_group.name, position, created_at],
        )?;

        Ok(SnippetGroup {
            id,
            name: new_group.name.clone(),
            position,
            created_at,
        })
    }

    /// List all snippet groups ordered by position.
    pub fn list_snippet_groups(&self) -> Result<Vec<SnippetGroup>, StorageError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, name, position, created_at
             FROM snippet_groups ORDER BY position",
        )?;

        let groups = stmt
            .query_map([], |row| {
                Ok(SnippetGroup {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    position: row.get(2)?,
                    created_at: row.get(3)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(groups)
    }

    /// Update a snippet group's name.
    pub fn update_snippet_group(
        &self,
        id: &str,
        name: &str,
    ) -> Result<SnippetGroup, StorageError> {
        let conn = self.conn.lock().unwrap();
        let affected = conn.execute(
            "UPDATE snippet_groups SET name = ?1 WHERE id = ?2",
            params![name, id],
        )?;

        if affected == 0 {
            return Err(StorageError::NotFound(format!("snippet_group {id}")));
        }

        let mut stmt = conn.prepare(
            "SELECT id, name, position, created_at FROM snippet_groups WHERE id = ?1",
        )?;
        let group = stmt.query_row(params![id], |row| {
            Ok(SnippetGroup {
                id: row.get(0)?,
                name: row.get(1)?,
                position: row.get(2)?,
                created_at: row.get(3)?,
            })
        })?;

        Ok(group)
    }

    /// Delete a snippet group. Snippets in this group will have their
    /// `group_id` set to NULL via ON DELETE SET NULL.
    pub fn delete_snippet_group(&self, id: &str) -> Result<(), StorageError> {
        let conn = self.conn.lock().unwrap();
        let affected = conn.execute("DELETE FROM snippet_groups WHERE id = ?1", params![id])?;

        if affected == 0 {
            return Err(StorageError::NotFound(format!("snippet_group {id}")));
        }
        Ok(())
    }
}

/// Map a database row to a Snippet struct.
fn row_to_snippet(row: &rusqlite::Row) -> rusqlite::Result<Snippet> {
    Ok(Snippet {
        id: row.get(0)?,
        abbreviation: row.get(1)?,
        name: row.get(2)?,
        content: row.get(3)?,
        content_type: row.get(4)?,
        group_id: row.get(5)?,
        description: row.get(6)?,
        use_count: row.get(7)?,
        created_at: row.get(8)?,
        updated_at: row.get(9)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_snippet(abbr: &str, name: &str) -> NewSnippet {
        NewSnippet {
            abbreviation: abbr.to_string(),
            name: name.to_string(),
            content: format!("expanded content for {name}"),
            content_type: "plain".to_string(),
            group_id: None,
            description: None,
        }
    }

    #[test]
    fn test_create_snippet() {
        let storage = Storage::new_in_memory().unwrap();
        let snippet = storage
            .create_snippet(&make_snippet(";sig", "Email Signature"))
            .unwrap();

        assert!(!snippet.id.is_empty());
        assert_eq!(snippet.abbreviation, ";sig");
        assert_eq!(snippet.name, "Email Signature");
        assert_eq!(snippet.use_count, 0);
    }

    #[test]
    fn test_abbreviation_uniqueness() {
        let storage = Storage::new_in_memory().unwrap();
        storage
            .create_snippet(&make_snippet(";sig", "Sig 1"))
            .unwrap();

        // Same abbreviation should fail
        let result = storage.create_snippet(&make_snippet(";sig", "Sig 2"));
        assert!(result.is_err());
    }

    #[test]
    fn test_get_snippet_by_abbreviation() {
        let storage = Storage::new_in_memory().unwrap();
        storage
            .create_snippet(&make_snippet(";hello", "Hello"))
            .unwrap();

        let found = storage.get_snippet_by_abbreviation(";hello").unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().abbreviation, ";hello");

        let missing = storage.get_snippet_by_abbreviation(";nope").unwrap();
        assert!(missing.is_none());
    }

    #[test]
    fn test_get_snippet_by_id() {
        let storage = Storage::new_in_memory().unwrap();
        let snippet = storage
            .create_snippet(&make_snippet(";test", "Test"))
            .unwrap();

        let found = storage.get_snippet_by_id(&snippet.id).unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().id, snippet.id);
    }

    #[test]
    fn test_list_snippets() {
        let storage = Storage::new_in_memory().unwrap();
        storage
            .create_snippet(&make_snippet(";a", "Alpha"))
            .unwrap();
        storage
            .create_snippet(&make_snippet(";b", "Beta"))
            .unwrap();

        let all = storage.list_snippets(None).unwrap();
        assert_eq!(all.len(), 2);
    }

    #[test]
    fn test_list_snippets_by_group() {
        let storage = Storage::new_in_memory().unwrap();
        let group = storage
            .create_snippet_group(&NewSnippetGroup {
                name: "Email".to_string(),
            })
            .unwrap();

        let mut s1 = make_snippet(";s1", "Snippet 1");
        s1.group_id = Some(group.id.clone());
        storage.create_snippet(&s1).unwrap();

        storage
            .create_snippet(&make_snippet(";s2", "Snippet 2"))
            .unwrap();

        let grouped = storage.list_snippets(Some(&group.id)).unwrap();
        assert_eq!(grouped.len(), 1);
        assert_eq!(grouped[0].abbreviation, ";s1");

        let all = storage.list_snippets(None).unwrap();
        assert_eq!(all.len(), 2);
    }

    #[test]
    fn test_update_snippet() {
        let storage = Storage::new_in_memory().unwrap();
        let snippet = storage
            .create_snippet(&make_snippet(";old", "Old Name"))
            .unwrap();

        let updated = storage
            .update_snippet(
                &snippet.id,
                &UpdateSnippet {
                    abbreviation: ";new".to_string(),
                    name: "New Name".to_string(),
                    content: "new content".to_string(),
                    content_type: "plain".to_string(),
                    group_id: None,
                    description: Some("updated desc".to_string()),
                },
            )
            .unwrap();

        assert_eq!(updated.abbreviation, ";new");
        assert_eq!(updated.name, "New Name");
        assert_eq!(updated.content, "new content");
        assert_eq!(updated.description.as_deref(), Some("updated desc"));
        assert!(updated.updated_at >= snippet.updated_at);
    }

    #[test]
    fn test_update_nonexistent_snippet() {
        let storage = Storage::new_in_memory().unwrap();
        let result = storage.update_snippet(
            "nope",
            &UpdateSnippet {
                abbreviation: ";x".to_string(),
                name: "x".to_string(),
                content: "x".to_string(),
                content_type: "plain".to_string(),
                group_id: None,
                description: None,
            },
        );
        assert!(matches!(result, Err(StorageError::NotFound(_))));
    }

    #[test]
    fn test_delete_snippet() {
        let storage = Storage::new_in_memory().unwrap();
        let snippet = storage
            .create_snippet(&make_snippet(";del", "To Delete"))
            .unwrap();

        storage.delete_snippet(&snippet.id).unwrap();

        let found = storage.get_snippet_by_id(&snippet.id).unwrap();
        assert!(found.is_none());
    }

    #[test]
    fn test_delete_nonexistent_snippet() {
        let storage = Storage::new_in_memory().unwrap();
        let result = storage.delete_snippet("nope");
        assert!(matches!(result, Err(StorageError::NotFound(_))));
    }

    #[test]
    fn test_increment_snippet_use_count() {
        let storage = Storage::new_in_memory().unwrap();
        let snippet = storage
            .create_snippet(&make_snippet(";cnt", "Counter"))
            .unwrap();
        assert_eq!(snippet.use_count, 0);

        storage.increment_snippet_use_count(&snippet.id).unwrap();
        storage.increment_snippet_use_count(&snippet.id).unwrap();
        storage.increment_snippet_use_count(&snippet.id).unwrap();

        let updated = storage.get_snippet_by_id(&snippet.id).unwrap().unwrap();
        assert_eq!(updated.use_count, 3);
    }

    // ─── Snippet Group Tests ─────────────────────────────────

    #[test]
    fn test_create_snippet_group() {
        let storage = Storage::new_in_memory().unwrap();
        let group = storage
            .create_snippet_group(&NewSnippetGroup {
                name: "Email Templates".to_string(),
            })
            .unwrap();

        assert!(!group.id.is_empty());
        assert_eq!(group.name, "Email Templates");
        assert_eq!(group.position, 0);
    }

    #[test]
    fn test_list_snippet_groups() {
        let storage = Storage::new_in_memory().unwrap();
        storage
            .create_snippet_group(&NewSnippetGroup {
                name: "A".to_string(),
            })
            .unwrap();
        storage
            .create_snippet_group(&NewSnippetGroup {
                name: "B".to_string(),
            })
            .unwrap();

        let groups = storage.list_snippet_groups().unwrap();
        assert_eq!(groups.len(), 2);
        assert_eq!(groups[0].name, "A");
        assert_eq!(groups[1].name, "B");
    }

    #[test]
    fn test_update_snippet_group() {
        let storage = Storage::new_in_memory().unwrap();
        let group = storage
            .create_snippet_group(&NewSnippetGroup {
                name: "Old".to_string(),
            })
            .unwrap();

        let updated = storage
            .update_snippet_group(&group.id, "New Name")
            .unwrap();
        assert_eq!(updated.name, "New Name");
    }

    #[test]
    fn test_delete_snippet_group_cascades() {
        let storage = Storage::new_in_memory().unwrap();
        let group = storage
            .create_snippet_group(&NewSnippetGroup {
                name: "To Delete".to_string(),
            })
            .unwrap();

        // Create a snippet in this group
        let mut s = make_snippet(";grp", "Grouped");
        s.group_id = Some(group.id.clone());
        let snippet = storage.create_snippet(&s).unwrap();
        assert_eq!(snippet.group_id.as_deref(), Some(group.id.as_str()));

        // Delete the group
        storage.delete_snippet_group(&group.id).unwrap();

        // Snippet should still exist but with group_id = NULL
        let updated = storage.get_snippet_by_id(&snippet.id).unwrap().unwrap();
        assert!(updated.group_id.is_none());
    }
}
