//! Clipboard deduplication: growing text detection and rapid copy debounce.

use std::time::{Duration, Instant};

/// Deduplication filter for clipboard captures.
/// Sits between clipboard capture and storage to reduce clutter.
pub struct ClipDedup {
    /// Last captured text content.
    last_text: Option<String>,
    /// Timestamp of the last capture.
    last_time: Option<Instant>,
    /// Whether to merge growing text selections.
    merge_growing: bool,
    /// Debounce window in milliseconds.
    debounce_ms: u64,
}

/// Result of the dedup check.
#[derive(Debug, PartialEq)]
pub enum DedupResult {
    /// Accept this clip as new.
    Accept,
    /// This clip replaces the previous one (growing text or debounce winner).
    Replace,
    /// Skip this clip (exact duplicate of previous).
    Duplicate,
}

impl ClipDedup {
    pub fn new(merge_growing: bool, debounce_ms: u32) -> Self {
        Self {
            last_text: None,
            last_time: None,
            merge_growing,
            debounce_ms: debounce_ms as u64,
        }
    }

    /// Check how a new text clip should be handled.
    ///
    /// Call this before storing a clip. Based on the result:
    /// - `Accept`: store as a new clip
    /// - `Replace`: delete the previous clip, then store this one
    /// - `Duplicate`: skip entirely
    pub fn check(&mut self, text: &str) -> DedupResult {
        let now = Instant::now();

        // Check for exact duplicate
        if let Some(ref last) = self.last_text {
            if last == text {
                return DedupResult::Duplicate;
            }
        }

        let result =
            if let (Some(ref last_text), Some(last_time)) = (&self.last_text, self.last_time) {
                let elapsed = now.duration_since(last_time);

                // Growing text detection: new text contains old text as prefix/subset
                if self.merge_growing
                    && elapsed < Duration::from_secs(2)
                    && is_growing(last_text, text)
                {
                    DedupResult::Replace
                }
                // Rapid copy debounce: within debounce window
                else if elapsed < Duration::from_millis(self.debounce_ms) {
                    DedupResult::Replace
                }
                // Normal new clip
                else {
                    DedupResult::Accept
                }
            } else {
                // First clip ever
                DedupResult::Accept
            };

        // Update state
        self.last_text = Some(text.to_string());
        self.last_time = Some(now);

        result
    }

    /// Reset the dedup state (e.g., after a long pause).
    pub fn reset(&mut self) {
        self.last_text = None;
        self.last_time = None;
    }
}

/// Check if `new_text` is a "growing" version of `old_text`.
/// Returns true if the old text is a prefix of the new text,
/// or if the new text contains the old text as a substring.
fn is_growing(old_text: &str, new_text: &str) -> bool {
    // New text must be longer than old text
    if new_text.len() <= old_text.len() {
        return false;
    }

    // Check prefix (most common: selecting more text extends the selection)
    if new_text.starts_with(old_text) {
        return true;
    }

    // Check suffix (selecting backwards)
    if new_text.ends_with(old_text) {
        return true;
    }

    // Check containment (old text is a substring)
    if new_text.contains(old_text) && old_text.len() > 10 {
        // Only consider containment for substantial text (>10 chars)
        // to avoid false positives with short strings
        return true;
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn test_first_clip_is_accepted() {
        let mut dedup = ClipDedup::new(true, 500);
        assert_eq!(dedup.check("hello"), DedupResult::Accept);
    }

    #[test]
    fn test_exact_duplicate_is_skipped() {
        let mut dedup = ClipDedup::new(true, 500);
        dedup.check("hello");
        assert_eq!(dedup.check("hello"), DedupResult::Duplicate);
    }

    #[test]
    fn test_different_content_after_debounce_is_accepted() {
        let mut dedup = ClipDedup::new(true, 50); // 50ms debounce for fast test
        dedup.check("hello");
        thread::sleep(Duration::from_millis(100)); // Wait past debounce
        assert_eq!(dedup.check("world"), DedupResult::Accept);
    }

    #[test]
    fn test_rapid_copy_replaces() {
        let mut dedup = ClipDedup::new(true, 500);
        dedup.check("first");
        // Immediately copy again (within 500ms)
        assert_eq!(dedup.check("second"), DedupResult::Replace);
    }

    #[test]
    fn test_growing_text_prefix() {
        let mut dedup = ClipDedup::new(true, 50);
        dedup.check("Hello");
        // Growing: "Hello" -> "Hello, world"
        assert_eq!(dedup.check("Hello, world"), DedupResult::Replace);
    }

    #[test]
    fn test_growing_text_suffix() {
        let mut dedup = ClipDedup::new(true, 50);
        dedup.check("world");
        assert_eq!(dedup.check("Hello, world"), DedupResult::Replace);
    }

    #[test]
    fn test_growing_text_containment() {
        let mut dedup = ClipDedup::new(true, 50);
        dedup.check("longer text for containment check");
        assert_eq!(
            dedup.check("Some longer text for containment check here"),
            DedupResult::Replace
        );
    }

    #[test]
    fn test_growing_disabled() {
        let mut dedup = ClipDedup::new(false, 50);
        dedup.check("Hello");
        thread::sleep(Duration::from_millis(100));
        // With merge_growing=false, this should be Accept not Replace
        assert_eq!(dedup.check("Hello, world"), DedupResult::Accept);
    }

    #[test]
    fn test_growing_not_triggered_for_short_containment() {
        let mut dedup = ClipDedup::new(true, 50);
        dedup.check("hi"); // Short text
        thread::sleep(Duration::from_millis(100));
        // "hi" is contained in "this" but it's only 2 chars -- not growing
        assert_eq!(dedup.check("this"), DedupResult::Accept);
    }

    #[test]
    fn test_growing_still_triggers_after_debounce_but_within_window() {
        let mut dedup = ClipDedup::new(true, 50);
        dedup.check("Hello");
        thread::sleep(Duration::from_millis(100)); // Past debounce but within 2s growing window
        // Growing text detection has its own 2-second window, independent of debounce.
        // Since "Hello, world" starts with "Hello" and we're within 2s, this is Replace.
        assert_eq!(dedup.check("Hello, world"), DedupResult::Replace);
    }

    #[test]
    fn test_unrelated_text_after_debounce_is_accepted() {
        let mut dedup = ClipDedup::new(true, 50);
        dedup.check("Hello");
        thread::sleep(Duration::from_millis(100)); // Past debounce
        // Completely different text (not growing) past debounce window -> Accept
        assert_eq!(dedup.check("Goodbye"), DedupResult::Accept);
    }

    #[test]
    fn test_reset() {
        let mut dedup = ClipDedup::new(true, 500);
        dedup.check("hello");
        dedup.reset();
        // After reset, should accept even a duplicate
        assert_eq!(dedup.check("hello"), DedupResult::Accept);
    }

    #[test]
    fn test_is_growing_prefix() {
        assert!(is_growing("Hello", "Hello, world"));
        assert!(!is_growing("Hello, world", "Hello")); // shrinking, not growing
    }

    #[test]
    fn test_is_growing_suffix() {
        assert!(is_growing("world", "Hello world"));
    }

    #[test]
    fn test_is_growing_same_length() {
        assert!(!is_growing("hello", "world")); // same length, not growing
    }
}
