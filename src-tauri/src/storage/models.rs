use serde::{Deserialize, Serialize};

/// Represents a clipboard history entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Clip {
    pub id: String,
    pub content_type: String,
    pub text_content: Option<String>,
    pub html_content: Option<String>,
    pub image_path: Option<String>,
    pub source_app: Option<String>,
    pub source_app_icon: Option<String>,
    pub content_hash: String,
    pub content_size: i64,
    pub metadata: Option<String>,
    pub pinboard_id: Option<String>,
    pub is_favorite: bool,
    pub created_at: String,
    pub accessed_at: Option<String>,
    pub access_count: i64,
}

/// Input struct for creating a new clip (no id/timestamps).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewClip {
    pub content_type: String,
    pub text_content: Option<String>,
    pub html_content: Option<String>,
    pub image_path: Option<String>,
    pub source_app: Option<String>,
    pub source_app_icon: Option<String>,
    pub content_hash: String,
    pub content_size: i64,
    pub metadata: Option<String>,
}

/// Filters for querying clips.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ClipFilters {
    pub content_type: Option<String>,
    pub source_app: Option<String>,
    pub date_from: Option<String>,
    pub date_to: Option<String>,
    pub pinboard_id: Option<String>,
}

/// Represents a pinboard (named collection).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pinboard {
    pub id: String,
    pub name: String,
    pub color: String,
    pub icon: Option<String>,
    pub position: i64,
    pub created_at: String,
}

/// Input struct for creating a new pinboard.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewPinboard {
    pub name: String,
    pub color: String,
    pub icon: Option<String>,
}

/// Represents a text expander snippet.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Snippet {
    pub id: String,
    pub abbreviation: String,
    pub name: String,
    pub content: String,
    pub content_type: String,
    pub group_id: Option<String>,
    pub description: Option<String>,
    pub use_count: i64,
    pub created_at: String,
    pub updated_at: String,
}

/// Input struct for creating a new snippet.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewSnippet {
    pub abbreviation: String,
    pub name: String,
    pub content: String,
    pub content_type: String,
    pub group_id: Option<String>,
    pub description: Option<String>,
}

/// Input struct for updating an existing snippet.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateSnippet {
    pub abbreviation: String,
    pub name: String,
    pub content: String,
    pub content_type: String,
    pub group_id: Option<String>,
    pub description: Option<String>,
}

/// Represents a snippet group.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnippetGroup {
    pub id: String,
    pub name: String,
    pub position: i64,
    pub created_at: String,
}

/// Input struct for creating a new snippet group.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewSnippetGroup {
    pub name: String,
}

/// Represents an item on the paste stack.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PasteStackItem {
    pub id: String,
    pub clip_id: String,
    pub position: i64,
    pub created_at: String,
}

/// Storage statistics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageStats {
    pub total_clips: usize,
    pub pinboard_clips: usize,
    pub favorite_clips: usize,
    pub total_size_bytes: i64,
    pub oldest_clip_date: Option<String>,
    pub newest_clip_date: Option<String>,
}
