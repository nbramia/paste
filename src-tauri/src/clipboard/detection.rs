use sha2::{Digest, Sha256};

/// Detected content type for clipboard content.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContentType {
    Text,
    Code,
    Link,
    Image,
    File,
}

impl ContentType {
    pub fn as_str(&self) -> &'static str {
        match self {
            ContentType::Text => "text",
            ContentType::Code => "code",
            ContentType::Link => "link",
            ContentType::Image => "image",
            ContentType::File => "file",
        }
    }
}

/// Detect the content type of plain text content.
pub fn detect_text_content_type(text: &str) -> ContentType {
    let trimmed = text.trim();

    // Check for URLs first
    if is_url(trimmed) {
        return ContentType::Link;
    }

    // Check for file paths
    if is_file_path(trimmed) {
        return ContentType::File;
    }

    // Check for code patterns
    if is_code(trimmed) {
        return ContentType::Code;
    }

    ContentType::Text
}

/// Check if text looks like a URL.
fn is_url(text: &str) -> bool {
    // Only consider single-line content as potential URLs
    if text.contains('\n') {
        return false;
    }
    let t = text.trim();
    t.starts_with("http://") || t.starts_with("https://") || t.starts_with("ftp://")
}

/// Check if text looks like a file path.
fn is_file_path(text: &str) -> bool {
    if text.contains('\n') {
        return false;
    }
    let t = text.trim();
    t.starts_with("file://") || (t.starts_with('/') && !t.contains("  ") && t.len() < 500)
}

/// Check if text looks like source code using heuristic pattern matching.
fn is_code(text: &str) -> bool {
    let lines: Vec<&str> = text.lines().collect();

    // Single line is rarely code unless it has strong indicators
    if lines.len() <= 1 {
        let line = text.trim();
        // Strong single-line code indicators
        return line.contains("fn ") && line.contains('(') && line.contains(')')
            || line.contains("def ") && line.contains(':')
            || line.contains("function ") && line.contains('(')
            || line.starts_with("import ") && (line.contains("from ") || line.contains(';'))
            || line.starts_with("#include ")
            || line.starts_with("package ")
            || line.starts_with("use ") && line.ends_with(';');
    }

    let mut score = 0u32;
    let total_lines = lines.len() as u32;

    // Check for common code patterns
    for line in &lines {
        let trimmed = line.trim();

        // Braces, brackets
        if trimmed.ends_with('{') || trimmed == "}" || trimmed == "};" {
            score += 2;
        }

        // Lines ending with semicolons
        if trimmed.ends_with(';') {
            score += 1;
        }

        // Language keywords
        if trimmed.starts_with("fn ")
            || trimmed.starts_with("pub ")
            || trimmed.starts_with("let ")
            || trimmed.starts_with("const ")
            || trimmed.starts_with("def ")
            || trimmed.starts_with("class ")
            || trimmed.starts_with("import ")
            || trimmed.starts_with("from ")
            || trimmed.starts_with("#include")
            || trimmed.starts_with("//")
            || trimmed.starts_with("/*")
            || trimmed.starts_with("# ") && lines.len() > 3 // avoid markdown confusion
            || trimmed.starts_with("if ") && (trimmed.contains('{') || trimmed.ends_with(':'))
            || trimmed.starts_with("for ") && (trimmed.contains('{') || trimmed.ends_with(':'))
            || trimmed.starts_with("return ")
        {
            score += 2;
        }

        // Indentation (code tends to be indented)
        if line.starts_with("    ") || line.starts_with('\t') {
            score += 1;
        }

        // Arrows and operators
        if trimmed.contains("=>") || trimmed.contains("->") || trimmed.contains("::") {
            score += 1;
        }
    }

    // Need a meaningful ratio of code-like lines
    // Threshold: at least 30% of lines should show code patterns
    let threshold = total_lines * 3 / 10;
    score > threshold.max(3)
}

/// Compute SHA-256 hash of content, returning hex string.
pub fn compute_hash(content: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content);
    format!("{:x}", hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- URL detection ---
    #[test]
    fn test_url_detection() {
        assert_eq!(
            detect_text_content_type("https://example.com"),
            ContentType::Link
        );
        assert_eq!(
            detect_text_content_type("http://example.com/path?q=1"),
            ContentType::Link
        );
        assert_eq!(
            detect_text_content_type("ftp://files.example.com"),
            ContentType::Link
        );
    }

    #[test]
    fn test_url_multiline_not_url() {
        assert_ne!(
            detect_text_content_type("https://example.com\nsome other text"),
            ContentType::Link
        );
    }

    #[test]
    fn test_plain_text_not_url() {
        assert_ne!(
            detect_text_content_type("hello world"),
            ContentType::Link
        );
        assert_ne!(
            detect_text_content_type("not a url at all"),
            ContentType::Link
        );
    }

    // --- File path detection ---
    #[test]
    fn test_file_path_detection() {
        assert_eq!(
            detect_text_content_type("file:///home/user/doc.txt"),
            ContentType::File
        );
        assert_eq!(
            detect_text_content_type("/home/user/documents/report.pdf"),
            ContentType::File
        );
    }

    #[test]
    fn test_absolute_path_not_too_long() {
        // Very long "paths" are probably not file paths
        let long = format!("/{}", "a".repeat(600));
        assert_ne!(detect_text_content_type(&long), ContentType::File);
    }

    // --- Code detection ---
    #[test]
    fn test_rust_code() {
        let code = r#"fn main() {
    let x = 42;
    println!("Hello {}", x);
}"#;
        assert_eq!(detect_text_content_type(code), ContentType::Code);
    }

    #[test]
    fn test_python_code() {
        let code = r#"def hello():
    x = 42
    if x > 10:
        print("big")
    return x"#;
        assert_eq!(detect_text_content_type(code), ContentType::Code);
    }

    #[test]
    fn test_javascript_code() {
        let code = r#"function greet(name) {
    const msg = `Hello ${name}`;
    console.log(msg);
    return msg;
}"#;
        assert_eq!(detect_text_content_type(code), ContentType::Code);
    }

    #[test]
    fn test_single_line_code() {
        assert_eq!(
            detect_text_content_type("fn main() {}"),
            ContentType::Code
        );
        assert_eq!(detect_text_content_type("def hello():"), ContentType::Code);
        assert_eq!(
            detect_text_content_type("#include <stdio.h>"),
            ContentType::Code
        );
        assert_eq!(detect_text_content_type("use std::io;"), ContentType::Code);
    }

    #[test]
    fn test_plain_text_not_code() {
        assert_eq!(
            detect_text_content_type("Hello, how are you?"),
            ContentType::Text
        );
        assert_eq!(
            detect_text_content_type("Just some regular text here."),
            ContentType::Text
        );
        assert_eq!(
            detect_text_content_type("Meeting at 3pm tomorrow"),
            ContentType::Text
        );
    }

    #[test]
    fn test_prose_paragraph_not_code() {
        let text = "This is a long paragraph of text that talks about various things. \
                    It doesn't contain any code patterns. Just regular English prose \
                    that someone might copy from a document or email.";
        assert_eq!(detect_text_content_type(text), ContentType::Text);
    }

    // --- Hash computation ---
    #[test]
    fn test_hash_deterministic() {
        let h1 = compute_hash(b"hello world");
        let h2 = compute_hash(b"hello world");
        assert_eq!(h1, h2);
    }

    #[test]
    fn test_hash_different_content() {
        let h1 = compute_hash(b"hello");
        let h2 = compute_hash(b"world");
        assert_ne!(h1, h2);
    }

    #[test]
    fn test_hash_is_hex_string() {
        let h = compute_hash(b"test");
        assert!(h.chars().all(|c| c.is_ascii_hexdigit()));
        assert_eq!(h.len(), 64); // SHA-256 = 32 bytes = 64 hex chars
    }

    // --- Content type string ---
    #[test]
    fn test_content_type_as_str() {
        assert_eq!(ContentType::Text.as_str(), "text");
        assert_eq!(ContentType::Code.as_str(), "code");
        assert_eq!(ContentType::Link.as_str(), "link");
        assert_eq!(ContentType::Image.as_str(), "image");
        assert_eq!(ContentType::File.as_str(), "file");
    }
}
