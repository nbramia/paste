use chrono::Utc;
use rusqlite::params;
use uuid::Uuid;

use super::db::Storage;
use super::error::StorageError;
use super::models::{Clip, ClipFilters, NewClip, StorageStats};

impl Storage {
    /// Insert a new clip into the database.
    ///
    /// Generates a UUID v7 for the id and sets timestamps.
    /// Returns `StorageError::Duplicate` if the content hash matches
    /// the most recent entry (consecutive deduplication).
    pub fn insert_clip(&self, new_clip: &NewClip) -> Result<Clip, StorageError> {
        // Check for consecutive duplicate
        if let Some(last_hash) = self.get_most_recent_hash()? {
            if last_hash == new_clip.content_hash {
                return Err(StorageError::Duplicate);
            }
        }

        let id = Uuid::now_v7().to_string();
        let created_at = Utc::now().to_rfc3339();

        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO clips (
                id, content_type, text_content, html_content, image_path,
                source_app, source_app_icon, content_hash, content_size,
                metadata, is_favorite, created_at, access_count
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, FALSE, ?11, 0)",
            params![
                id,
                new_clip.content_type,
                new_clip.text_content,
                new_clip.html_content,
                new_clip.image_path,
                new_clip.source_app,
                new_clip.source_app_icon,
                new_clip.content_hash,
                new_clip.content_size,
                new_clip.metadata,
                created_at,
            ],
        )?;

        let clip = Clip {
            id,
            content_type: new_clip.content_type.clone(),
            text_content: new_clip.text_content.clone(),
            html_content: new_clip.html_content.clone(),
            image_path: new_clip.image_path.clone(),
            source_app: new_clip.source_app.clone(),
            source_app_icon: new_clip.source_app_icon.clone(),
            content_hash: new_clip.content_hash.clone(),
            content_size: new_clip.content_size,
            metadata: new_clip.metadata.clone(),
            pinboard_id: None,
            is_favorite: false,
            created_at,
            accessed_at: None,
            access_count: 0,
        };

        Ok(clip)
    }

    /// Get paginated clips with optional filters, ordered by newest first.
    pub fn get_clips(
        &self,
        offset: usize,
        limit: usize,
        filters: &ClipFilters,
    ) -> Result<Vec<Clip>, StorageError> {
        let conn = self.conn.lock().unwrap();

        let mut sql = String::from("SELECT * FROM clips WHERE 1=1");
        let mut param_values: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

        if let Some(ref ct) = filters.content_type {
            sql.push_str(" AND content_type = ?");
            param_values.push(Box::new(ct.clone()));
        }
        if let Some(ref sa) = filters.source_app {
            sql.push_str(" AND source_app = ?");
            param_values.push(Box::new(sa.clone()));
        }
        if let Some(ref df) = filters.date_from {
            sql.push_str(" AND created_at >= ?");
            param_values.push(Box::new(df.clone()));
        }
        if let Some(ref dt) = filters.date_to {
            sql.push_str(" AND created_at <= ?");
            param_values.push(Box::new(dt.clone()));
        }
        if let Some(ref pid) = filters.pinboard_id {
            sql.push_str(" AND pinboard_id = ?");
            param_values.push(Box::new(pid.clone()));
        }

        sql.push_str(" ORDER BY created_at DESC LIMIT ? OFFSET ?");
        param_values.push(Box::new(limit as i64));
        param_values.push(Box::new(offset as i64));

        let params_refs: Vec<&dyn rusqlite::types::ToSql> =
            param_values.iter().map(|p| p.as_ref()).collect();

        let mut stmt = conn.prepare(&sql)?;
        let clips = stmt
            .query_map(params_refs.as_slice(), row_to_clip)?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(clips)
    }

    /// Get a single clip by its id.
    pub fn get_clip_by_id(&self, id: &str) -> Result<Option<Clip>, StorageError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT * FROM clips WHERE id = ?1")?;
        let mut rows = stmt.query_map(params![id], row_to_clip)?;

        match rows.next() {
            Some(Ok(clip)) => Ok(Some(clip)),
            Some(Err(e)) => Err(StorageError::Database(e)),
            None => Ok(None),
        }
    }

    /// Delete a clip by id. Also removes the image file if the clip is an image.
    pub fn delete_clip(&self, id: &str) -> Result<(), StorageError> {
        // First, fetch the clip to check for image_path
        let clip = self.get_clip_by_id(id)?;

        let conn = self.conn.lock().unwrap();
        let affected = conn.execute("DELETE FROM clips WHERE id = ?1", params![id])?;

        if affected == 0 {
            return Err(StorageError::NotFound(format!("clip {id}")));
        }
        drop(conn);

        // Clean up image file if present
        if let Some(clip) = clip {
            if let Some(ref image_path) = clip.image_path {
                let path = std::path::Path::new(image_path);
                if path.exists() {
                    let _ = std::fs::remove_file(path);
                }
            }
        }

        Ok(())
    }

    /// Update the pinboard association for a clip.
    pub fn update_clip_pinboard(
        &self,
        id: &str,
        pinboard_id: Option<&str>,
    ) -> Result<(), StorageError> {
        let conn = self.conn.lock().unwrap();
        let affected = conn.execute(
            "UPDATE clips SET pinboard_id = ?1 WHERE id = ?2",
            params![pinboard_id, id],
        )?;

        if affected == 0 {
            return Err(StorageError::NotFound(format!("clip {id}")));
        }
        Ok(())
    }

    /// Toggle the favorite status of a clip.
    pub fn toggle_favorite(&self, id: &str) -> Result<bool, StorageError> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE clips SET is_favorite = NOT is_favorite WHERE id = ?1",
            params![id],
        )?;

        let is_favorite: bool = conn.query_row(
            "SELECT is_favorite FROM clips WHERE id = ?1",
            params![id],
            |row| row.get(0),
        )?;

        Ok(is_favorite)
    }

    /// Increment the access count and update the accessed_at timestamp.
    pub fn increment_access_count(&self, id: &str) -> Result<(), StorageError> {
        let now = Utc::now().to_rfc3339();
        let conn = self.conn.lock().unwrap();
        let affected = conn.execute(
            "UPDATE clips SET access_count = access_count + 1, accessed_at = ?1 WHERE id = ?2",
            params![now, id],
        )?;

        if affected == 0 {
            return Err(StorageError::NotFound(format!("clip {id}")));
        }
        Ok(())
    }

    /// Get distinct source_app values for filter dropdowns.
    pub fn get_distinct_source_apps(&self) -> Result<Vec<String>, StorageError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT DISTINCT source_app FROM clips WHERE source_app IS NOT NULL ORDER BY source_app"
        )?;
        let apps = stmt
            .query_map([], |row| row.get::<_, String>(0))?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(apps)
    }

    /// Get storage statistics.
    pub fn get_storage_stats(&self) -> Result<StorageStats, StorageError> {
        let conn = self.conn.lock().unwrap();

        let total_clips: i64 = conn.query_row(
            "SELECT COUNT(*) FROM clips",
            [],
            |row| row.get(0),
        )?;

        let total_pinboard: i64 = conn.query_row(
            "SELECT COUNT(*) FROM clips WHERE pinboard_id IS NOT NULL",
            [],
            |row| row.get(0),
        )?;

        let total_favorites: i64 = conn.query_row(
            "SELECT COUNT(*) FROM clips WHERE is_favorite = TRUE",
            [],
            |row| row.get(0),
        )?;

        let total_size: i64 = conn.query_row(
            "SELECT COALESCE(SUM(content_size), 0) FROM clips",
            [],
            |row| row.get(0),
        )?;

        let oldest_clip: Option<String> = conn.query_row(
            "SELECT MIN(created_at) FROM clips",
            [],
            |row| row.get(0),
        ).ok();

        let newest_clip: Option<String> = conn.query_row(
            "SELECT MAX(created_at) FROM clips",
            [],
            |row| row.get(0),
        ).ok();

        Ok(StorageStats {
            total_clips: total_clips as usize,
            pinboard_clips: total_pinboard as usize,
            favorite_clips: total_favorites as usize,
            total_size_bytes: total_size,
            oldest_clip_date: oldest_clip,
            newest_clip_date: newest_clip,
        })
    }

    /// Get the content hash of the most recently created clip.
    pub fn get_most_recent_hash(&self) -> Result<Option<String>, StorageError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt =
            conn.prepare("SELECT content_hash FROM clips ORDER BY created_at DESC LIMIT 1")?;
        let mut rows = stmt.query_map([], |row| row.get::<_, String>(0))?;

        match rows.next() {
            Some(Ok(hash)) => Ok(Some(hash)),
            Some(Err(e)) => Err(StorageError::Database(e)),
            None => Ok(None),
        }
    }
}

/// Map a database row to a Clip struct.
fn row_to_clip(row: &rusqlite::Row) -> rusqlite::Result<Clip> {
    Ok(Clip {
        id: row.get("id")?,
        content_type: row.get("content_type")?,
        text_content: row.get("text_content")?,
        html_content: row.get("html_content")?,
        image_path: row.get("image_path")?,
        source_app: row.get("source_app")?,
        source_app_icon: row.get("source_app_icon")?,
        content_hash: row.get("content_hash")?,
        content_size: row.get("content_size")?,
        metadata: row.get("metadata")?,
        pinboard_id: row.get("pinboard_id")?,
        is_favorite: row.get("is_favorite")?,
        created_at: row.get("created_at")?,
        accessed_at: row.get("accessed_at")?,
        access_count: row.get("access_count")?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_new_clip(text: &str, hash: &str) -> NewClip {
        NewClip {
            content_type: "text".to_string(),
            text_content: Some(text.to_string()),
            html_content: None,
            image_path: None,
            source_app: Some("test-app".to_string()),
            source_app_icon: None,
            content_hash: hash.to_string(),
            content_size: text.len() as i64,
            metadata: None,
        }
    }

    #[test]
    fn test_insert_and_get_clip() {
        let storage = Storage::new_in_memory().unwrap();
        let new_clip = make_new_clip("hello world", "hash1");
        let clip = storage.insert_clip(&new_clip).unwrap();

        assert!(!clip.id.is_empty());
        assert_eq!(clip.content_type, "text");
        assert_eq!(clip.text_content.as_deref(), Some("hello world"));
        assert_eq!(clip.content_hash, "hash1");
        assert!(!clip.is_favorite);
        assert_eq!(clip.access_count, 0);

        let fetched = storage.get_clip_by_id(&clip.id).unwrap();
        assert!(fetched.is_some());
        let fetched = fetched.unwrap();
        assert_eq!(fetched.id, clip.id);
        assert_eq!(fetched.text_content, clip.text_content);
    }

    #[test]
    fn test_duplicate_detection() {
        let storage = Storage::new_in_memory().unwrap();
        let new_clip = make_new_clip("hello", "same_hash");
        storage.insert_clip(&new_clip).unwrap();

        // Second insert with same hash should fail
        let result = storage.insert_clip(&new_clip);
        assert!(matches!(result, Err(StorageError::Duplicate)));
    }

    #[test]
    fn test_no_duplicate_after_different_clip() {
        let storage = Storage::new_in_memory().unwrap();

        let clip1 = make_new_clip("hello", "hash_a");
        storage.insert_clip(&clip1).unwrap();

        let clip2 = make_new_clip("world", "hash_b");
        storage.insert_clip(&clip2).unwrap();

        // Same hash as clip1 but not as the most recent — should succeed
        let clip3 = make_new_clip("hello again", "hash_a");
        let result = storage.insert_clip(&clip3);
        assert!(result.is_ok());
    }

    #[test]
    fn test_get_clips_pagination() {
        let storage = Storage::new_in_memory().unwrap();

        for i in 0..10 {
            let clip = make_new_clip(&format!("clip {i}"), &format!("hash_{i}"));
            storage.insert_clip(&clip).unwrap();
        }

        let page1 = storage
            .get_clips(0, 5, &ClipFilters::default())
            .unwrap();
        assert_eq!(page1.len(), 5);

        let page2 = storage
            .get_clips(5, 5, &ClipFilters::default())
            .unwrap();
        assert_eq!(page2.len(), 5);

        // Pages should not overlap
        let ids1: Vec<&str> = page1.iter().map(|c| c.id.as_str()).collect();
        let ids2: Vec<&str> = page2.iter().map(|c| c.id.as_str()).collect();
        for id in &ids2 {
            assert!(!ids1.contains(id));
        }
    }

    #[test]
    fn test_get_clips_with_filters() {
        let storage = Storage::new_in_memory().unwrap();

        let mut clip_text = make_new_clip("some text", "h1");
        clip_text.content_type = "text".to_string();
        clip_text.source_app = Some("firefox".to_string());
        storage.insert_clip(&clip_text).unwrap();

        let mut clip_code = make_new_clip("fn main() {}", "h2");
        clip_code.content_type = "code".to_string();
        clip_code.source_app = Some("vscode".to_string());
        storage.insert_clip(&clip_code).unwrap();

        // Filter by content_type
        let filters = ClipFilters {
            content_type: Some("code".to_string()),
            ..Default::default()
        };
        let results = storage.get_clips(0, 100, &filters).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].content_type, "code");

        // Filter by source_app
        let filters = ClipFilters {
            source_app: Some("firefox".to_string()),
            ..Default::default()
        };
        let results = storage.get_clips(0, 100, &filters).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].source_app.as_deref(), Some("firefox"));
    }

    #[test]
    fn test_delete_clip() {
        let storage = Storage::new_in_memory().unwrap();
        let new_clip = make_new_clip("to delete", "hash_del");
        let clip = storage.insert_clip(&new_clip).unwrap();

        storage.delete_clip(&clip.id).unwrap();

        let fetched = storage.get_clip_by_id(&clip.id).unwrap();
        assert!(fetched.is_none());
    }

    #[test]
    fn test_delete_nonexistent_clip() {
        let storage = Storage::new_in_memory().unwrap();
        let result = storage.delete_clip("nonexistent");
        assert!(matches!(result, Err(StorageError::NotFound(_))));
    }

    #[test]
    fn test_update_clip_pinboard() {
        let storage = Storage::new_in_memory().unwrap();

        // Create a pinboard first
        let pb = storage
            .create_pinboard(&super::super::models::NewPinboard {
                name: "Work".to_string(),
                color: "#ff0000".to_string(),
                icon: None,
            })
            .unwrap();

        let new_clip = make_new_clip("clip for pinboard", "hash_pb");
        let clip = storage.insert_clip(&new_clip).unwrap();
        assert!(clip.pinboard_id.is_none());

        // Assign to pinboard
        storage
            .update_clip_pinboard(&clip.id, Some(&pb.id))
            .unwrap();
        let updated = storage.get_clip_by_id(&clip.id).unwrap().unwrap();
        assert_eq!(updated.pinboard_id.as_deref(), Some(pb.id.as_str()));

        // Remove from pinboard
        storage.update_clip_pinboard(&clip.id, None).unwrap();
        let updated = storage.get_clip_by_id(&clip.id).unwrap().unwrap();
        assert!(updated.pinboard_id.is_none());
    }

    #[test]
    fn test_increment_access_count() {
        let storage = Storage::new_in_memory().unwrap();
        let new_clip = make_new_clip("access me", "hash_access");
        let clip = storage.insert_clip(&new_clip).unwrap();

        assert_eq!(clip.access_count, 0);
        assert!(clip.accessed_at.is_none());

        storage.increment_access_count(&clip.id).unwrap();
        let updated = storage.get_clip_by_id(&clip.id).unwrap().unwrap();
        assert_eq!(updated.access_count, 1);
        assert!(updated.accessed_at.is_some());

        storage.increment_access_count(&clip.id).unwrap();
        let updated = storage.get_clip_by_id(&clip.id).unwrap().unwrap();
        assert_eq!(updated.access_count, 2);
    }

    #[test]
    fn test_toggle_favorite() {
        let storage = Storage::new_in_memory().unwrap();
        let new_clip = make_new_clip("fav me", "hash_fav");
        let clip = storage.insert_clip(&new_clip).unwrap();
        assert!(!clip.is_favorite);

        let is_fav = storage.toggle_favorite(&clip.id).unwrap();
        assert!(is_fav);

        let is_fav = storage.toggle_favorite(&clip.id).unwrap();
        assert!(!is_fav);
    }

    #[test]
    fn test_get_most_recent_hash() {
        let storage = Storage::new_in_memory().unwrap();

        assert!(storage.get_most_recent_hash().unwrap().is_none());

        let clip1 = make_new_clip("first", "first_hash");
        storage.insert_clip(&clip1).unwrap();
        assert_eq!(
            storage.get_most_recent_hash().unwrap().as_deref(),
            Some("first_hash")
        );

        let clip2 = make_new_clip("second", "second_hash");
        storage.insert_clip(&clip2).unwrap();
        assert_eq!(
            storage.get_most_recent_hash().unwrap().as_deref(),
            Some("second_hash")
        );
    }

    #[test]
    fn test_uuid_v7_is_time_ordered() {
        let storage = Storage::new_in_memory().unwrap();

        let clip1 = storage
            .insert_clip(&make_new_clip("a", "ha"))
            .unwrap();
        let clip2 = storage
            .insert_clip(&make_new_clip("b", "hb"))
            .unwrap();

        // UUID v7 is time-ordered, so clip2.id > clip1.id lexicographically
        assert!(clip2.id > clip1.id);
    }
}
