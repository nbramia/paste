use super::db::Storage;
use super::error::StorageError;
use super::models::{Clip, ClipFilters};

impl Storage {
    /// Search clips using substring matching with optional filters.
    ///
    /// Uses LIKE for intuitive substring matching (typing "pas" finds "paste").
    /// Results are ordered by newest first.
    pub fn search_clips(
        &self,
        query: &str,
        filters: &ClipFilters,
    ) -> Result<Vec<Clip>, StorageError> {
        let conn = self.conn.lock().unwrap();

        let trimmed = query.trim();
        if trimmed.is_empty() {
            return Ok(vec![]);
        }

        // Use LIKE for substring matching — case insensitive by default in SQLite
        let like_pattern = format!("%{trimmed}%");

        let mut sql = String::from(
            "SELECT c.id, c.content_type, c.text_content, c.html_content,
                    c.image_path, c.source_app, c.source_app_icon,
                    c.content_hash, c.content_size, c.metadata,
                    c.pinboard_id, c.is_favorite, c.created_at,
                    c.accessed_at, c.access_count
             FROM clips c
             WHERE (c.text_content LIKE ? OR c.source_app LIKE ?)",
        );

        let mut param_values: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();
        param_values.push(Box::new(like_pattern.clone()));
        param_values.push(Box::new(like_pattern));

        if let Some(ref ct) = filters.content_type {
            sql.push_str(" AND c.content_type = ?");
            param_values.push(Box::new(ct.clone()));
        }
        if let Some(ref sa) = filters.source_app {
            sql.push_str(" AND c.source_app = ?");
            param_values.push(Box::new(sa.clone()));
        }
        if let Some(ref df) = filters.date_from {
            sql.push_str(" AND c.created_at >= ?");
            param_values.push(Box::new(df.clone()));
        }
        if let Some(ref dt) = filters.date_to {
            sql.push_str(" AND c.created_at <= ?");
            param_values.push(Box::new(dt.clone()));
        }
        if let Some(ref pid) = filters.pinboard_id {
            sql.push_str(" AND c.pinboard_id = ?");
            param_values.push(Box::new(pid.clone()));
        }
        if let Some(fav) = filters.is_favorite {
            sql.push_str(" AND c.is_favorite = ?");
            param_values.push(Box::new(fav));
        }

        sql.push_str(" ORDER BY c.created_at DESC LIMIT 100");

        let params_refs: Vec<&dyn rusqlite::types::ToSql> =
            param_values.iter().map(|p| p.as_ref()).collect();

        let mut stmt = conn.prepare(&sql)?;
        let clips = stmt
            .query_map(params_refs.as_slice(), |row| {
                Ok(Clip {
                    id: row.get(0)?,
                    content_type: row.get(1)?,
                    text_content: row.get(2)?,
                    html_content: row.get(3)?,
                    image_path: row.get(4)?,
                    source_app: row.get(5)?,
                    source_app_icon: row.get(6)?,
                    content_hash: row.get(7)?,
                    content_size: row.get(8)?,
                    metadata: row.get(9)?,
                    pinboard_id: row.get(10)?,
                    is_favorite: row.get(11)?,
                    created_at: row.get(12)?,
                    accessed_at: row.get(13)?,
                    access_count: row.get(14)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(clips)
    }
}

/// Sanitize a user query for FTS5.
///
/// Splits the query into whitespace-delimited tokens and wraps each in
/// double quotes so that special FTS5 characters (*, -, etc.) are treated
/// as literals. Tokens are joined with spaces, which FTS5 interprets as
/// implicit AND.
fn sanitize_fts_query(query: &str) -> String {
    let terms: Vec<String> = query
        .split_whitespace()
        .filter(|t| !t.is_empty())
        .map(|t| {
            // Escape any internal double quotes
            let escaped = t.replace('"', "\"\"");
            format!("\"{escaped}\"")
        })
        .collect();

    terms.join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::models::NewClip;

    fn make_text_clip(text: &str, hash: &str) -> NewClip {
        NewClip {
            content_type: "text".to_string(),
            text_content: Some(text.to_string()),
            html_content: None,
            image_path: None,
            source_app: Some("test".to_string()),
            source_app_icon: None,
            content_hash: hash.to_string(),
            content_size: text.len() as i64,
            metadata: None,
        }
    }

    #[test]
    fn test_basic_search() {
        let storage = Storage::new_in_memory().unwrap();

        storage
            .insert_clip(&make_text_clip("the quick brown fox jumps", "h1"))
            .unwrap();
        storage
            .insert_clip(&make_text_clip("lazy dog sleeps all day", "h2"))
            .unwrap();
        storage
            .insert_clip(&make_text_clip("the fox and the hound", "h3"))
            .unwrap();

        let results = storage
            .search_clips("fox", &ClipFilters::default())
            .unwrap();
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_search_no_results() {
        let storage = Storage::new_in_memory().unwrap();

        storage
            .insert_clip(&make_text_clip("hello world", "h1"))
            .unwrap();

        let results = storage
            .search_clips("nonexistent", &ClipFilters::default())
            .unwrap();
        assert_eq!(results.len(), 0);
    }

    #[test]
    fn test_search_empty_query() {
        let storage = Storage::new_in_memory().unwrap();

        storage
            .insert_clip(&make_text_clip("hello world", "h1"))
            .unwrap();

        let results = storage
            .search_clips("", &ClipFilters::default())
            .unwrap();
        assert_eq!(results.len(), 0);

        let results = storage
            .search_clips("   ", &ClipFilters::default())
            .unwrap();
        assert_eq!(results.len(), 0);
    }

    #[test]
    fn test_search_with_content_type_filter() {
        let storage = Storage::new_in_memory().unwrap();

        let mut clip1 = make_text_clip("rust programming language", "h1");
        clip1.content_type = "text".to_string();
        storage.insert_clip(&clip1).unwrap();

        let mut clip2 = make_text_clip("rust fn main() { println!(\"hello\"); }", "h2");
        clip2.content_type = "code".to_string();
        storage.insert_clip(&clip2).unwrap();

        let filters = ClipFilters {
            content_type: Some("code".to_string()),
            ..Default::default()
        };
        let results = storage.search_clips("rust", &filters).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].content_type, "code");
    }

    #[test]
    fn test_search_with_source_app_filter() {
        let storage = Storage::new_in_memory().unwrap();

        let mut clip1 = make_text_clip("meeting notes for today", "h1");
        clip1.source_app = Some("firefox".to_string());
        storage.insert_clip(&clip1).unwrap();

        let mut clip2 = make_text_clip("meeting agenda items", "h2");
        clip2.source_app = Some("vscode".to_string());
        storage.insert_clip(&clip2).unwrap();

        let filters = ClipFilters {
            source_app: Some("firefox".to_string()),
            ..Default::default()
        };
        let results = storage.search_clips("meeting", &filters).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].source_app.as_deref(), Some("firefox"));
    }

    #[test]
    fn test_search_multi_word() {
        let storage = Storage::new_in_memory().unwrap();

        storage
            .insert_clip(&make_text_clip("the quick brown fox", "h1"))
            .unwrap();
        storage
            .insert_clip(&make_text_clip("quick silver lining", "h2"))
            .unwrap();
        storage
            .insert_clip(&make_text_clip("brown sugar", "h3"))
            .unwrap();

        // Both terms must match (implicit AND)
        let results = storage
            .search_clips("quick brown", &ClipFilters::default())
            .unwrap();
        assert_eq!(results.len(), 1);
        assert!(results[0]
            .text_content
            .as_ref()
            .unwrap()
            .contains("quick brown fox"));
    }

    #[test]
    fn test_sanitize_fts_query() {
        assert_eq!(sanitize_fts_query("hello"), "\"hello\"");
        assert_eq!(sanitize_fts_query("hello world"), "\"hello\" \"world\"");
        assert_eq!(sanitize_fts_query(""), "");
        assert_eq!(sanitize_fts_query("   "), "");
        assert_eq!(sanitize_fts_query("a*b"), "\"a*b\"");
    }
}
