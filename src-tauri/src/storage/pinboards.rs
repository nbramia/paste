use chrono::Utc;
use rusqlite::params;
use uuid::Uuid;

use super::db::Storage;
use super::error::StorageError;
use super::models::{NewPinboard, Pinboard};

impl Storage {
    /// Create a new pinboard. Position is auto-assigned as the next available.
    pub fn create_pinboard(&self, new_pinboard: &NewPinboard) -> Result<Pinboard, StorageError> {
        let id = Uuid::now_v7().to_string();
        let created_at = Utc::now().to_rfc3339();

        let conn = self.conn.lock().unwrap();

        // Get the next position
        let position: i64 = conn
            .query_row(
                "SELECT COALESCE(MAX(position), -1) + 1 FROM pinboards",
                [],
                |row| row.get(0),
            )
            .unwrap_or(0);

        conn.execute(
            "INSERT INTO pinboards (id, name, color, icon, position, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                id,
                new_pinboard.name,
                new_pinboard.color,
                new_pinboard.icon,
                position,
                created_at,
            ],
        )?;

        Ok(Pinboard {
            id,
            name: new_pinboard.name.clone(),
            color: new_pinboard.color.clone(),
            icon: new_pinboard.icon.clone(),
            position,
            created_at,
        })
    }

    /// List all pinboards ordered by position.
    pub fn list_pinboards(&self) -> Result<Vec<Pinboard>, StorageError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, name, color, icon, position, created_at
             FROM pinboards ORDER BY position",
        )?;

        let pinboards = stmt
            .query_map([], |row| {
                Ok(Pinboard {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    color: row.get(2)?,
                    icon: row.get(3)?,
                    position: row.get(4)?,
                    created_at: row.get(5)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(pinboards)
    }

    /// Get a single pinboard by id.
    pub fn get_pinboard_by_id(&self, id: &str) -> Result<Option<Pinboard>, StorageError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, name, color, icon, position, created_at
             FROM pinboards WHERE id = ?1",
        )?;
        let mut rows = stmt.query_map(params![id], |row| {
            Ok(Pinboard {
                id: row.get(0)?,
                name: row.get(1)?,
                color: row.get(2)?,
                icon: row.get(3)?,
                position: row.get(4)?,
                created_at: row.get(5)?,
            })
        })?;

        match rows.next() {
            Some(Ok(pb)) => Ok(Some(pb)),
            Some(Err(e)) => Err(StorageError::Database(e)),
            None => Ok(None),
        }
    }

    /// Update a pinboard's name, color, and icon.
    pub fn update_pinboard(
        &self,
        id: &str,
        name: &str,
        color: &str,
        icon: Option<&str>,
    ) -> Result<Pinboard, StorageError> {
        let conn = self.conn.lock().unwrap();
        let affected = conn.execute(
            "UPDATE pinboards SET name = ?1, color = ?2, icon = ?3 WHERE id = ?4",
            params![name, color, icon, id],
        )?;

        if affected == 0 {
            return Err(StorageError::NotFound(format!("pinboard {id}")));
        }
        drop(conn);

        self.get_pinboard_by_id(id)?
            .ok_or_else(|| StorageError::NotFound(format!("pinboard {id}")))
    }

    /// Delete a pinboard. Clips assigned to this pinboard will have their
    /// `pinboard_id` set to NULL via ON DELETE SET NULL.
    pub fn delete_pinboard(&self, id: &str) -> Result<(), StorageError> {
        let conn = self.conn.lock().unwrap();
        let affected = conn.execute("DELETE FROM pinboards WHERE id = ?1", params![id])?;

        if affected == 0 {
            return Err(StorageError::NotFound(format!("pinboard {id}")));
        }
        Ok(())
    }

    /// Reorder a pinboard to a new position, shifting others accordingly.
    pub fn reorder_pinboard(&self, id: &str, new_position: i64) -> Result<(), StorageError> {
        let conn = self.conn.lock().unwrap();

        // Get current position
        let current_position: i64 = conn
            .query_row(
                "SELECT position FROM pinboards WHERE id = ?1",
                params![id],
                |row| row.get(0),
            )
            .map_err(|_| StorageError::NotFound(format!("pinboard {id}")))?;

        if current_position == new_position {
            return Ok(());
        }

        if new_position > current_position {
            // Moving down: shift items in between up
            conn.execute(
                "UPDATE pinboards SET position = position - 1
                 WHERE position > ?1 AND position <= ?2",
                params![current_position, new_position],
            )?;
        } else {
            // Moving up: shift items in between down
            conn.execute(
                "UPDATE pinboards SET position = position + 1
                 WHERE position >= ?1 AND position < ?2",
                params![new_position, current_position],
            )?;
        }

        conn.execute(
            "UPDATE pinboards SET position = ?1 WHERE id = ?2",
            params![new_position, id],
        )?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_pinboard(name: &str, color: &str) -> NewPinboard {
        NewPinboard {
            name: name.to_string(),
            color: color.to_string(),
            icon: None,
        }
    }

    #[test]
    fn test_create_pinboard() {
        let storage = Storage::new_in_memory().unwrap();
        let pb = storage
            .create_pinboard(&make_pinboard("Work", "#ff0000"))
            .unwrap();

        assert!(!pb.id.is_empty());
        assert_eq!(pb.name, "Work");
        assert_eq!(pb.color, "#ff0000");
        assert_eq!(pb.position, 0);
    }

    #[test]
    fn test_auto_increment_position() {
        let storage = Storage::new_in_memory().unwrap();

        let pb1 = storage
            .create_pinboard(&make_pinboard("First", "#111"))
            .unwrap();
        let pb2 = storage
            .create_pinboard(&make_pinboard("Second", "#222"))
            .unwrap();
        let pb3 = storage
            .create_pinboard(&make_pinboard("Third", "#333"))
            .unwrap();

        assert_eq!(pb1.position, 0);
        assert_eq!(pb2.position, 1);
        assert_eq!(pb3.position, 2);
    }

    #[test]
    fn test_list_pinboards() {
        let storage = Storage::new_in_memory().unwrap();

        storage
            .create_pinboard(&make_pinboard("A", "#aaa"))
            .unwrap();
        storage
            .create_pinboard(&make_pinboard("B", "#bbb"))
            .unwrap();

        let pinboards = storage.list_pinboards().unwrap();
        assert_eq!(pinboards.len(), 2);
        assert_eq!(pinboards[0].name, "A");
        assert_eq!(pinboards[1].name, "B");
    }

    #[test]
    fn test_get_pinboard_by_id() {
        let storage = Storage::new_in_memory().unwrap();
        let pb = storage
            .create_pinboard(&make_pinboard("Work", "#ff0000"))
            .unwrap();

        let fetched = storage.get_pinboard_by_id(&pb.id).unwrap();
        assert!(fetched.is_some());
        assert_eq!(fetched.unwrap().name, "Work");

        let missing = storage.get_pinboard_by_id("nonexistent").unwrap();
        assert!(missing.is_none());
    }

    #[test]
    fn test_update_pinboard() {
        let storage = Storage::new_in_memory().unwrap();
        let pb = storage
            .create_pinboard(&make_pinboard("Old Name", "#000"))
            .unwrap();

        let updated = storage
            .update_pinboard(&pb.id, "New Name", "#fff", Some("star"))
            .unwrap();
        assert_eq!(updated.name, "New Name");
        assert_eq!(updated.color, "#fff");
        assert_eq!(updated.icon.as_deref(), Some("star"));
    }

    #[test]
    fn test_update_nonexistent_pinboard() {
        let storage = Storage::new_in_memory().unwrap();
        let result = storage.update_pinboard("nope", "name", "#000", None);
        assert!(matches!(result, Err(StorageError::NotFound(_))));
    }

    #[test]
    fn test_delete_pinboard() {
        let storage = Storage::new_in_memory().unwrap();
        let pb = storage
            .create_pinboard(&make_pinboard("To Delete", "#000"))
            .unwrap();

        storage.delete_pinboard(&pb.id).unwrap();

        let fetched = storage.get_pinboard_by_id(&pb.id).unwrap();
        assert!(fetched.is_none());
    }

    #[test]
    fn test_delete_pinboard_cascades_to_clips() {
        let storage = Storage::new_in_memory().unwrap();
        let pb = storage
            .create_pinboard(&make_pinboard("Cascade", "#000"))
            .unwrap();

        // Create a clip and assign to pinboard
        let new_clip = crate::storage::models::NewClip {
            content_type: "text".to_string(),
            text_content: Some("pinned item".to_string()),
            html_content: None,
            image_path: None,
            source_app: None,
            source_app_icon: None,
            content_hash: "cascade_hash".to_string(),
            content_size: 11,
            metadata: None,
        };
        let clip = storage.insert_clip(&new_clip).unwrap();
        storage
            .update_clip_pinboard(&clip.id, Some(&pb.id))
            .unwrap();

        // Delete pinboard
        storage.delete_pinboard(&pb.id).unwrap();

        // Clip should still exist but with pinboard_id = NULL
        let clip = storage.get_clip_by_id(&clip.id).unwrap().unwrap();
        assert!(clip.pinboard_id.is_none());
    }

    #[test]
    fn test_delete_nonexistent_pinboard() {
        let storage = Storage::new_in_memory().unwrap();
        let result = storage.delete_pinboard("nonexistent");
        assert!(matches!(result, Err(StorageError::NotFound(_))));
    }

    #[test]
    fn test_reorder_pinboard() {
        let storage = Storage::new_in_memory().unwrap();

        let pb1 = storage
            .create_pinboard(&make_pinboard("A", "#aaa"))
            .unwrap();
        let pb2 = storage
            .create_pinboard(&make_pinboard("B", "#bbb"))
            .unwrap();
        let pb3 = storage
            .create_pinboard(&make_pinboard("C", "#ccc"))
            .unwrap();

        // Move A (pos 0) to position 2
        storage.reorder_pinboard(&pb1.id, 2).unwrap();

        let pinboards = storage.list_pinboards().unwrap();
        assert_eq!(pinboards[0].name, "B");
        assert_eq!(pinboards[0].position, 0);
        assert_eq!(pinboards[1].name, "C");
        assert_eq!(pinboards[1].position, 1);
        assert_eq!(pinboards[2].name, "A");
        assert_eq!(pinboards[2].position, 2);

        // Move A (pos 2) back to position 0
        storage.reorder_pinboard(&pb1.id, 0).unwrap();

        let pinboards = storage.list_pinboards().unwrap();
        assert_eq!(pinboards[0].name, "A");
        assert_eq!(pinboards[1].name, "B");
        assert_eq!(pinboards[2].name, "C");

        // Verify pb3 id is still valid
        let _ = storage.get_pinboard_by_id(&pb3.id).unwrap().unwrap();
        let _ = storage.get_pinboard_by_id(&pb2.id).unwrap().unwrap();
    }
}
