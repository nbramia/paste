use std::collections::HashMap;
use std::sync::Mutex;

use crate::storage::Storage;

/// Stores abbreviation -> expansion mappings for efficient lookup.
pub struct AbbreviationMatcher {
    /// Map of abbreviation string to (snippet_id, expansion content)
    abbreviations: HashMap<String, MatchResult>,
}

/// Result of a successful abbreviation match.
#[derive(Debug, Clone)]
pub struct MatchResult {
    pub snippet_id: String,
    pub abbreviation: String,
    pub content: String,
    // Carried through from storage; matching does not key off content type yet.
    #[allow(dead_code)]
    pub content_type: String,
}

impl AbbreviationMatcher {
    pub fn new() -> Self {
        Self {
            abbreviations: HashMap::new(),
        }
    }

    /// Load abbreviations from a list of (abbreviation, snippet_id, content, content_type) tuples.
    pub fn load(&mut self, snippets: Vec<(String, String, String, String)>) {
        self.abbreviations.clear();
        for (abbr, id, content, content_type) in snippets {
            self.abbreviations.insert(
                abbr.clone(),
                MatchResult {
                    snippet_id: id,
                    abbreviation: abbr,
                    content,
                    content_type,
                },
            );
        }
    }

    /// Check if the buffer ends with any known abbreviation.
    /// Returns the match result if found.
    pub fn find_match(&self, buffer: &str) -> Option<&MatchResult> {
        // Check all abbreviations to see if the buffer ends with one.
        // Start with longest abbreviations first to prefer longer matches.
        let mut candidates: Vec<(&String, &MatchResult)> = self.abbreviations.iter().collect();
        candidates.sort_by_key(|(abbr, _)| std::cmp::Reverse(abbr.len()));

        for (abbr, result) in candidates {
            if !buffer.ends_with(abbr.as_str()) {
                continue;
            }
            if starts_with_word_char(abbr) {
                // Whole-word rule: the char before the abbreviation must be
                // absent or a non-alphanumeric.
                let before = &buffer[..buffer.len() - abbr.len()];
                if before
                    .chars()
                    .next_back()
                    .is_some_and(char::is_alphanumeric)
                {
                    continue;
                }
            }
            return Some(result);
        }
        None
    }

    /// Get the number of registered abbreviations.
    // Used by this module's unit tests.
    #[allow(dead_code)]
    pub fn len(&self) -> usize {
        self.abbreviations.len()
    }

    /// Check if there are no registered abbreviations.
    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.abbreviations.is_empty()
    }
}

/// True when the abbreviation begins with a letter or digit, which makes it
/// subject to the whole-word rule. Punctuation-led abbreviations (`;sig`,
/// `:fire`, `[]`) match as a pure suffix.
fn starts_with_word_char(abbr: &str) -> bool {
    abbr.chars().next().is_some_and(char::is_alphanumeric)
}

/// Replace the shared matcher's contents with every snippet in storage.
/// Returns the number of snippets loaded.
pub fn reload_from_storage(
    matcher: &Mutex<AbbreviationMatcher>,
    storage: &Storage,
) -> Result<usize, String> {
    let snippets = storage.list_snippets(None).map_err(|e| e.to_string())?;
    let entries: Vec<(String, String, String, String)> = snippets
        .iter()
        .map(|s| {
            (
                s.abbreviation.clone(),
                s.id.clone(),
                s.content.clone(),
                s.content_type.clone(),
            )
        })
        .collect();
    let count = entries.len();
    matcher
        .lock()
        .map_err(|_| "matcher lock poisoned".to_string())?
        .load(entries);
    Ok(count)
}

impl Default for AbbreviationMatcher {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_snippets() -> Vec<(String, String, String, String)> {
        vec![
            (
                ";sig".into(),
                "1".into(),
                "Best regards,\nJohn".into(),
                "plain".into(),
            ),
            (
                ";email".into(),
                "2".into(),
                "john@example.com".into(),
                "plain".into(),
            ),
            (
                "//date".into(),
                "3".into(),
                "2024-01-01".into(),
                "plain".into(),
            ),
            (
                ",,addr".into(),
                "4".into(),
                "123 Main St".into(),
                "plain".into(),
            ),
        ]
    }

    #[test]
    fn test_empty_matcher() {
        let matcher = AbbreviationMatcher::new();
        assert!(matcher.is_empty());
        assert_eq!(matcher.len(), 0);
        assert!(matcher.find_match("hello").is_none());
    }

    #[test]
    fn test_load_and_match() {
        let mut matcher = AbbreviationMatcher::new();
        matcher.load(make_snippets());
        assert_eq!(matcher.len(), 4);

        let result = matcher.find_match("hello ;sig");
        assert!(result.is_some());
        let result = result.unwrap();
        assert_eq!(result.abbreviation, ";sig");
        assert_eq!(result.content, "Best regards,\nJohn");
    }

    #[test]
    fn test_no_match() {
        let mut matcher = AbbreviationMatcher::new();
        matcher.load(make_snippets());
        assert!(matcher.find_match("hello world").is_none());
    }

    #[test]
    fn test_exact_match() {
        let mut matcher = AbbreviationMatcher::new();
        matcher.load(make_snippets());
        assert!(matcher.find_match(";sig").is_some());
    }

    #[test]
    fn test_prefix_not_match() {
        let mut matcher = AbbreviationMatcher::new();
        matcher.load(make_snippets());
        // ";si" should not match ";sig"
        assert!(matcher.find_match(";si").is_none());
    }

    #[test]
    fn test_longer_match_preferred() {
        let mut matcher = AbbreviationMatcher::new();
        matcher.load(vec![
            (";s".into(), "1".into(), "short".into(), "plain".into()),
            (
                ";sig".into(),
                "2".into(),
                "signature".into(),
                "plain".into(),
            ),
        ]);
        // When buffer ends with ";sig", prefer the longer match
        let result = matcher.find_match("hello ;sig").unwrap();
        assert_eq!(result.abbreviation, ";sig");
        assert_eq!(result.content, "signature");
    }

    #[test]
    fn test_reload() {
        let mut matcher = AbbreviationMatcher::new();
        matcher.load(make_snippets());
        assert_eq!(matcher.len(), 4);

        matcher.load(vec![(
            ";new".into(),
            "5".into(),
            "new content".into(),
            "plain".into(),
        )]);
        assert_eq!(matcher.len(), 1);
        assert!(matcher.find_match(";sig").is_none());
        assert!(matcher.find_match(";new").is_some());
    }

    fn word_matcher() -> AbbreviationMatcher {
        let mut m = AbbreviationMatcher::new();
        m.load(vec![
            (
                "cos".into(),
                "1".into(),
                "Chief of Staff".into(),
                "plain".into(),
            ),
            (":fire".into(), "2".into(), "x".into(), "plain".into()),
            (";sig".into(), "3".into(), "sig".into(), "plain".into()),
        ]);
        m
    }

    #[test]
    fn test_whole_word_alnum_abbreviation() {
        let m = word_matcher();
        assert!(m.find_match("cos").is_some());
        assert!(m.find_match("macos").is_none());
        assert!(m.find_match("co2cos").is_none());
        assert!(m.find_match("(cos").is_some());
        assert!(m.find_match("a,cos").is_some());
    }

    #[test]
    fn test_punctuation_led_abbreviation_matches_as_suffix() {
        let m = word_matcher();
        assert!(m.find_match("hello;sig").is_some());
        assert!(m.find_match("hello:fire").is_some());
    }

    #[test]
    fn test_longest_match_skips_failed_boundary() {
        let mut m = AbbreviationMatcher::new();
        m.load(vec![
            ("macos".into(), "1".into(), "long".into(), "plain".into()),
            ("cos".into(), "2".into(), "short".into(), "plain".into()),
        ]);
        // "macos" is whole-word here, so the longer one wins.
        assert_eq!(m.find_match("macos").unwrap().content, "long");
        // "xmacos": "macos" fails the boundary, and "cos" fails too.
        assert!(m.find_match("xmacos").is_none());
    }

    #[test]
    fn test_reload_from_storage() {
        use crate::storage::models::NewSnippet;
        let storage = Storage::new_in_memory().unwrap();
        let matcher = Mutex::new(AbbreviationMatcher::new());
        assert_eq!(reload_from_storage(&matcher, &storage).unwrap(), 0);
        storage
            .create_snippet(&NewSnippet {
                abbreviation: "omw".into(),
                name: "omw".into(),
                content: "On my way!".into(),
                content_type: "plain".into(),
                group_id: None,
                description: None,
            })
            .unwrap();
        assert!(matcher.lock().unwrap().find_match("omw").is_none());
        assert_eq!(reload_from_storage(&matcher, &storage).unwrap(), 1);
        assert!(matcher.lock().unwrap().find_match("omw").is_some());
    }
}
