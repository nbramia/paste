use chrono::{Duration, Utc};
use rusqlite::params;

use super::db::Storage;
use super::error::StorageError;

impl Storage {
    /// Enforce retention policy by deleting old clips.
    ///
    /// Clips that are favorited or assigned to a pinboard are exempt from
    /// deletion.
    ///
    /// - `max_days`: If set, deletes clips older than this many days.
    /// - `max_count`: If set, keeps only the N most recent clips (after
    ///   age-based deletion).
    ///
    /// Returns the total number of clips deleted.
    pub fn enforce_retention(
        &self,
        max_days: Option<u32>,
        max_count: Option<usize>,
    ) -> Result<usize, StorageError> {
        let mut total_deleted = 0usize;

        // Phase 1: Age-based deletion
        if let Some(days) = max_days {
            let cutoff = Utc::now() - Duration::days(i64::from(days));
            let cutoff_str = cutoff.to_rfc3339();

            let conn = self.conn.lock().unwrap();

            // Delete image files for expired image clips before removing rows
            let image_paths: Vec<String> = {
                let mut stmt = conn.prepare(
                    "SELECT image_path FROM clips
                     WHERE created_at < ?1
                       AND pinboard_id IS NULL
                       AND is_favorite = FALSE
                       AND image_path IS NOT NULL",
                )?;
                let rows = stmt
                    .query_map(params![cutoff_str], |row| row.get::<_, String>(0))?
                    .filter_map(|r| r.ok())
                    .collect();
                rows
            };

            for path in &image_paths {
                let p = std::path::Path::new(path);
                if p.exists() {
                    let _ = std::fs::remove_file(p);
                }
            }

            let deleted = conn.execute(
                "DELETE FROM clips
                 WHERE created_at < ?1
                   AND pinboard_id IS NULL
                   AND is_favorite = FALSE",
                params![cutoff_str],
            )?;

            total_deleted += deleted;
        }

        // Phase 2: Count-based deletion
        if let Some(max) = max_count {
            let conn = self.conn.lock().unwrap();

            // Count how many non-exempt clips exist
            let total: i64 = conn.query_row(
                "SELECT COUNT(*) FROM clips
                 WHERE pinboard_id IS NULL AND is_favorite = FALSE",
                [],
                |row| row.get(0),
            )?;
            let total = total as usize;

            if total > max {
                let to_delete = total - max;

                // Delete image files first
                let image_paths: Vec<String> = {
                    let mut stmt = conn.prepare(
                        "SELECT image_path FROM clips
                         WHERE pinboard_id IS NULL
                           AND is_favorite = FALSE
                           AND image_path IS NOT NULL
                         ORDER BY created_at ASC
                         LIMIT ?1",
                    )?;
                    let rows = stmt
                        .query_map(params![to_delete as i64], |row| row.get::<_, String>(0))?
                        .filter_map(|r| r.ok())
                        .collect();
                    rows
                };

                for path in &image_paths {
                    let p = std::path::Path::new(path);
                    if p.exists() {
                        let _ = std::fs::remove_file(p);
                    }
                }

                // Delete oldest non-exempt clips
                let deleted = conn.execute(
                    "DELETE FROM clips WHERE id IN (
                        SELECT id FROM clips
                        WHERE pinboard_id IS NULL
                          AND is_favorite = FALSE
                        ORDER BY created_at ASC
                        LIMIT ?1
                    )",
                    params![to_delete as i64],
                )?;

                total_deleted += deleted;
            }
        }

        Ok(total_deleted)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::models::{ClipFilters, NewClip, NewPinboard};

    fn make_clip(text: &str, hash: &str) -> NewClip {
        NewClip {
            content_type: "text".to_string(),
            text_content: Some(text.to_string()),
            html_content: None,
            image_path: None,
            source_app: None,
            source_app_icon: None,
            content_hash: hash.to_string(),
            content_size: text.len() as i64,
            metadata: None,
        }
    }

    #[test]
    fn test_retention_by_age() {
        let storage = Storage::new_in_memory().unwrap();

        // Insert a clip and manually backdate it
        let clip = storage.insert_clip(&make_clip("old clip", "h1")).unwrap();
        {
            let conn = storage.conn.lock().unwrap();
            let old_date = (Utc::now() - Duration::days(100)).to_rfc3339();
            conn.execute(
                "UPDATE clips SET created_at = ?1 WHERE id = ?2",
                params![old_date, clip.id],
            )
            .unwrap();
        }

        // Insert a recent clip
        storage.insert_clip(&make_clip("new clip", "h2")).unwrap();

        // Enforce 90-day retention
        let deleted = storage.enforce_retention(Some(90), None).unwrap();
        assert_eq!(deleted, 1);

        // Only the new clip should remain
        let remaining = storage
            .get_clips(0, 100, &ClipFilters::default())
            .unwrap();
        assert_eq!(remaining.len(), 1);
        assert_eq!(remaining[0].text_content.as_deref(), Some("new clip"));
    }

    #[test]
    fn test_retention_by_count() {
        let storage = Storage::new_in_memory().unwrap();

        for i in 0..10 {
            storage
                .insert_clip(&make_clip(&format!("clip {i}"), &format!("h{i}")))
                .unwrap();
        }

        // Keep only 5
        let deleted = storage.enforce_retention(None, Some(5)).unwrap();
        assert_eq!(deleted, 5);

        let remaining = storage
            .get_clips(0, 100, &ClipFilters::default())
            .unwrap();
        assert_eq!(remaining.len(), 5);

        // The remaining should be the 5 most recent (clips 5-9)
        for clip in &remaining {
            let num: usize = clip
                .text_content
                .as_ref()
                .unwrap()
                .strip_prefix("clip ")
                .unwrap()
                .parse()
                .unwrap();
            assert!(num >= 5);
        }
    }

    #[test]
    fn test_retention_exempts_pinboard_items() {
        let storage = Storage::new_in_memory().unwrap();

        let pb = storage
            .create_pinboard(&NewPinboard {
                name: "Saved".to_string(),
                color: "#000".to_string(),
                icon: None,
            })
            .unwrap();

        // Create 5 clips, assign first 2 to pinboard
        for i in 0..5 {
            let clip = storage
                .insert_clip(&make_clip(&format!("clip {i}"), &format!("h{i}")))
                .unwrap();
            if i < 2 {
                storage
                    .update_clip_pinboard(&clip.id, Some(&pb.id))
                    .unwrap();
            }
        }

        // Keep only 1 non-exempt clip
        let deleted = storage.enforce_retention(None, Some(1)).unwrap();
        assert_eq!(deleted, 2); // 3 non-exempt, keep 1, delete 2

        let remaining = storage
            .get_clips(0, 100, &ClipFilters::default())
            .unwrap();
        // 2 pinboard + 1 kept = 3 total
        assert_eq!(remaining.len(), 3);
    }

    #[test]
    fn test_retention_exempts_favorites() {
        let storage = Storage::new_in_memory().unwrap();

        // Create clips and favorite one
        let clip1 = storage
            .insert_clip(&make_clip("fav clip", "hf"))
            .unwrap();
        storage.toggle_favorite(&clip1.id).unwrap();

        for i in 0..5 {
            storage
                .insert_clip(&make_clip(&format!("normal {i}"), &format!("hn{i}")))
                .unwrap();
        }

        // Keep only 2 non-exempt
        let deleted = storage.enforce_retention(None, Some(2)).unwrap();
        assert_eq!(deleted, 3); // 5 non-exempt, keep 2, delete 3

        let remaining = storage
            .get_clips(0, 100, &ClipFilters::default())
            .unwrap();
        // 1 favorite + 2 kept = 3 total
        assert_eq!(remaining.len(), 3);

        // Verify the favorite is still there
        let fav = storage.get_clip_by_id(&clip1.id).unwrap();
        assert!(fav.is_some());
        assert!(fav.unwrap().is_favorite);
    }

    #[test]
    fn test_retention_no_deletion_needed() {
        let storage = Storage::new_in_memory().unwrap();

        for i in 0..3 {
            storage
                .insert_clip(&make_clip(&format!("clip {i}"), &format!("h{i}")))
                .unwrap();
        }

        // Max count higher than total
        let deleted = storage.enforce_retention(None, Some(10)).unwrap();
        assert_eq!(deleted, 0);

        // Max days with no old clips
        let deleted = storage.enforce_retention(Some(90), None).unwrap();
        assert_eq!(deleted, 0);
    }

    #[test]
    fn test_retention_combined_age_and_count() {
        let storage = Storage::new_in_memory().unwrap();

        // Create 10 clips, backdate 5 of them
        for i in 0..10 {
            let clip = storage
                .insert_clip(&make_clip(&format!("clip {i}"), &format!("h{i}")))
                .unwrap();
            if i < 5 {
                let old_date = (Utc::now() - Duration::days(100)).to_rfc3339();
                let conn = storage.conn.lock().unwrap();
                conn.execute(
                    "UPDATE clips SET created_at = ?1 WHERE id = ?2",
                    params![old_date, clip.id],
                )
                .unwrap();
            }
        }

        // Enforce both: 90 days and max 3
        let deleted = storage.enforce_retention(Some(90), Some(3)).unwrap();
        // 5 deleted by age + 2 deleted by count (5 remaining, keep 3)
        assert_eq!(deleted, 7);

        let remaining = storage
            .get_clips(0, 100, &ClipFilters::default())
            .unwrap();
        assert_eq!(remaining.len(), 3);
    }
}
