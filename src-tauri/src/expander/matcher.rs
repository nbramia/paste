use std::collections::HashMap;

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
        candidates.sort_by(|a, b| b.0.len().cmp(&a.0.len()));

        for (abbr, result) in candidates {
            if buffer.ends_with(abbr.as_str()) {
                return Some(result);
            }
        }
        None
    }

    /// Get the number of registered abbreviations.
    pub fn len(&self) -> usize {
        self.abbreviations.len()
    }

    /// Check if there are no registered abbreviations.
    pub fn is_empty(&self) -> bool {
        self.abbreviations.is_empty()
    }
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
}
