use serde::{Deserialize, Serialize};

/// A captured clipboard item ready for storage.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClipItem {
    pub content_type: String,       // "text", "code", "link", "image", "file"
    pub text_content: Option<String>,
    pub html_content: Option<String>,
    pub image_path: Option<String>,
    pub source_app: Option<String>,
    pub content_hash: String,       // SHA-256 hex
    pub content_size: i64,
    pub metadata: Option<String>,   // JSON string
}
