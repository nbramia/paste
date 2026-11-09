use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use evdev::KeyCode;
use log::{debug, info};

use super::buffer::CharBuffer;
use super::keymap::{is_modifier_key, is_word_boundary_key, keycode_to_char};
use super::matcher::AbbreviationMatcher;
use super::template::expand_template;

/// Trigger mode for abbreviation expansion.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TriggerMode {
    /// Expand only when a word boundary (space, punctuation) follows the abbreviation.
    WordBoundary,
    /// Expand immediately when the abbreviation is fully typed.
    Immediate,
}

/// Action to take after processing a keystroke.
#[derive(Debug)]
pub enum ExpanderAction {
    /// No action needed.
    None,
    /// Expand: delete N chars (backspaces), inject text, then re-emit `trigger`.
    Expand {
        /// Characters to delete: the abbreviation, plus the boundary key that
        /// triggered it when one was typed (word-boundary mode).
        backspace_count: usize,
        text: String,
        snippet_id: String,
        /// The character that triggered the expansion, to type after the text.
        /// `None` for Tab (which moves focus) and for immediate mode (no
        /// trigger character was typed).
        trigger: Option<char>,
    },
}

/// True when `text` contains a non-ASCII character. ydotool can only type
/// ASCII, so such text has to go in through the clipboard instead.
pub fn needs_clipboard(text: &str) -> bool {
    !text.is_ascii()
}

/// The text to insert for an expansion: the expanded snippet followed by the
/// character that triggered it, if any.
pub fn with_trigger(text: &str, trigger: Option<char>) -> String {
    let mut full = String::from(text);
    if let Some(ch) = trigger {
        full.push(ch);
    }
    full
}

/// The text expander engine. Processes keystrokes and detects abbreviation matches.
pub struct ExpanderEngine {
    buffer: CharBuffer,
    matcher: Arc<Mutex<AbbreviationMatcher>>,
    enabled: Arc<AtomicBool>,
    trigger_mode: TriggerMode,
    shift_held: bool,
    last_keystroke: Instant,
    timeout_secs: u64,
}

impl ExpanderEngine {
    #[cfg(test)]
    pub fn new(trigger_mode: TriggerMode, timeout_secs: u64) -> Self {
        Self::with_matcher(
            trigger_mode,
            timeout_secs,
            Arc::new(Mutex::new(AbbreviationMatcher::new())),
        )
    }

    /// Build an engine that matches against a matcher shared with the rest of
    /// the app, so snippet changes take effect without a restart.
    pub fn with_matcher(
        trigger_mode: TriggerMode,
        timeout_secs: u64,
        matcher: Arc<Mutex<AbbreviationMatcher>>,
    ) -> Self {
        Self {
            buffer: CharBuffer::new(100),
            matcher,
            enabled: Arc::new(AtomicBool::new(true)),
            trigger_mode,
            shift_held: false,
            last_keystroke: Instant::now(),
            timeout_secs,
        }
    }

    /// Get a reference to the enabled flag for toggling from outside.
    // The expander is toggled through the Tauri command layer, which owns its own
    // AtomicBool; these accessors are covered by tests but unused in the app.
    #[allow(dead_code)]
    pub fn enabled_flag(&self) -> Arc<AtomicBool> {
        self.enabled.clone()
    }

    /// Check if the expander is enabled.
    pub fn is_enabled(&self) -> bool {
        self.enabled.load(Ordering::Relaxed)
    }

    /// Toggle enabled state. Returns new state.
    #[allow(dead_code)]
    pub fn toggle(&self) -> bool {
        let prev = self.enabled.fetch_xor(true, Ordering::Relaxed);
        let new = !prev;
        info!("Text expander {}", if new { "enabled" } else { "disabled" });
        new
    }

    /// Process a key press event. Returns an action if expansion should occur.
    pub fn process_key(&mut self, key: KeyCode, pressed: bool) -> ExpanderAction {
        if !self.is_enabled() {
            return ExpanderAction::None;
        }

        // Track shift state for modifier keys
        if is_modifier_key(key) {
            if key == KeyCode::KEY_LEFTSHIFT || key == KeyCode::KEY_RIGHTSHIFT {
                self.shift_held = pressed;
            }
            return ExpanderAction::None;
        }

        // Only process key presses, not releases
        if !pressed {
            return ExpanderAction::None;
        }

        // Check for timeout — reset buffer if too much time passed
        let now = Instant::now();
        if now.duration_since(self.last_keystroke).as_secs() >= self.timeout_secs {
            self.buffer.clear();
        }
        self.last_keystroke = now;

        // Handle backspace — remove last char from buffer
        if key == KeyCode::KEY_BACKSPACE {
            self.buffer.pop();
            return ExpanderAction::None;
        }

        // Handle Enter — reset buffer (never part of an abbreviation)
        if key == KeyCode::KEY_ENTER {
            // In word boundary mode, check for expansion before clearing
            if self.trigger_mode == TriggerMode::WordBoundary {
                if let Some(action) = self.check_and_expand(Some('\n')) {
                    self.buffer.clear();
                    return action;
                }
            }
            self.buffer.clear();
            return ExpanderAction::None;
        }

        // In WordBoundary mode, word boundary keys (space, tab, punctuation)
        // trigger an expansion check on the current buffer content.
        // If the word boundary character is also a typeable character (e.g., ';', '/'),
        // it gets added to the buffer for future abbreviation matching.
        if self.trigger_mode == TriggerMode::WordBoundary && is_word_boundary_key(key) {
            // Check if buffer ends with an abbreviation BEFORE adding this char
            let trigger = if key == KeyCode::KEY_TAB {
                None
            } else {
                keycode_to_char(key, self.shift_held)
            };
            if let Some(action) = self.check_and_expand(trigger) {
                self.buffer.clear();
                return action;
            }

            // No expansion — add the character to the buffer if it's typeable,
            // since it could be the start of an abbreviation (e.g., ';', '/')
            if let Some(ch) = keycode_to_char(key, self.shift_held) {
                // Space and tab are true word separators — clear buffer and don't add
                if ch == ' ' || ch == '\t' {
                    self.buffer.clear();
                } else {
                    // Punctuation that could be part of an abbreviation prefix
                    self.buffer.push(ch);
                }
            } else {
                self.buffer.clear();
            }
            return ExpanderAction::None;
        }

        // Convert key to character
        let Some(ch) = keycode_to_char(key, self.shift_held) else {
            // Non-character key (Escape, arrows, etc.) — reset buffer
            self.buffer.clear();
            return ExpanderAction::None;
        };

        // Add character to buffer
        self.buffer.push(ch);

        // In immediate mode, check for match after every character
        if self.trigger_mode == TriggerMode::Immediate {
            if let Some(action) = self.immediate_expand() {
                self.buffer.clear();
                return action;
            }
        }

        ExpanderAction::None
    }

    /// Expand on a word-boundary key. The key was typed into the app, so it is
    /// deleted along with the abbreviation and then re-emitted as `trigger`.
    fn check_and_expand(&self, trigger: Option<char>) -> Option<ExpanderAction> {
        self.expand(1, trigger)
    }

    /// Expand as soon as the abbreviation is complete. No boundary key was
    /// typed, so nothing extra is deleted and nothing is re-emitted.
    fn immediate_expand(&self) -> Option<ExpanderAction> {
        self.expand(0, None)
    }

    fn expand(&self, extra_backspaces: usize, trigger: Option<char>) -> Option<ExpanderAction> {
        let buffer_str = self.buffer.as_str();
        let matcher = match self.matcher.lock() {
            Ok(m) => m,
            Err(_) => {
                log::error!("abbreviation matcher lock poisoned; skipping expansion");
                return None;
            }
        };

        if let Some(result) = matcher.find_match(&buffer_str) {
            let expanded = expand_template(&result.content);
            let backspace_count = result.abbreviation.chars().count() + extra_backspaces;

            debug!(
                "Expanding '{}' ({} backspaces) -> '{}' (snippet {})",
                result.abbreviation,
                backspace_count,
                if expanded.len() > 50 {
                    format!("{}...", &expanded[..50])
                } else {
                    expanded.clone()
                },
                result.snippet_id,
            );

            Some(ExpanderAction::Expand {
                backspace_count,
                text: expanded,
                snippet_id: result.snippet_id.clone(),
                trigger,
            })
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup_engine(mode: TriggerMode) -> ExpanderEngine {
        let engine = ExpanderEngine::new(mode, 5);
        let snippets = vec![
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
        ];
        engine.matcher.lock().unwrap().load(snippets);
        engine
    }

    fn type_string(engine: &mut ExpanderEngine, s: &str) -> Vec<ExpanderAction> {
        let mut actions = Vec::new();
        for ch in s.chars() {
            let key = char_to_keycode(ch);
            if let Some((keycode, shift)) = key {
                if shift {
                    engine.process_key(KeyCode::KEY_LEFTSHIFT, true);
                }
                let action = engine.process_key(keycode, true);
                if shift {
                    engine.process_key(KeyCode::KEY_LEFTSHIFT, false);
                }
                actions.push(action);
            }
        }
        actions
    }

    /// Reverse mapping for tests: char -> (KeyCode, needs_shift)
    fn char_to_keycode(ch: char) -> Option<(KeyCode, bool)> {
        match ch {
            'a'..='z' => {
                let key = match ch {
                    'a' => KeyCode::KEY_A,
                    'b' => KeyCode::KEY_B,
                    'c' => KeyCode::KEY_C,
                    'd' => KeyCode::KEY_D,
                    'e' => KeyCode::KEY_E,
                    'f' => KeyCode::KEY_F,
                    'g' => KeyCode::KEY_G,
                    'h' => KeyCode::KEY_H,
                    'i' => KeyCode::KEY_I,
                    'j' => KeyCode::KEY_J,
                    'k' => KeyCode::KEY_K,
                    'l' => KeyCode::KEY_L,
                    'm' => KeyCode::KEY_M,
                    'n' => KeyCode::KEY_N,
                    'o' => KeyCode::KEY_O,
                    'p' => KeyCode::KEY_P,
                    'q' => KeyCode::KEY_Q,
                    'r' => KeyCode::KEY_R,
                    's' => KeyCode::KEY_S,
                    't' => KeyCode::KEY_T,
                    'u' => KeyCode::KEY_U,
                    'v' => KeyCode::KEY_V,
                    'w' => KeyCode::KEY_W,
                    'x' => KeyCode::KEY_X,
                    'y' => KeyCode::KEY_Y,
                    'z' => KeyCode::KEY_Z,
                    _ => return None,
                };
                Some((key, false))
            }
            'A'..='Z' => {
                let lower = ch.to_ascii_lowercase();
                char_to_keycode(lower).map(|(k, _)| (k, true))
            }
            ';' => Some((KeyCode::KEY_SEMICOLON, false)),
            ':' => Some((KeyCode::KEY_SEMICOLON, true)),
            '/' => Some((KeyCode::KEY_SLASH, false)),
            ',' => Some((KeyCode::KEY_COMMA, false)),
            '.' => Some((KeyCode::KEY_DOT, false)),
            ' ' => Some((KeyCode::KEY_SPACE, false)),
            _ => None,
        }
    }

    #[test]
    fn test_word_boundary_expansion() {
        let mut engine = setup_engine(TriggerMode::WordBoundary);
        let actions = type_string(&mut engine, ";sig ");
        // The space triggers the expansion
        let last = actions.last().unwrap();
        match last {
            ExpanderAction::Expand {
                backspace_count,
                text,
                ..
            } => {
                assert_eq!(*backspace_count, 5); // ";sig" + the typed space
                assert_eq!(text, "Best regards,\nJohn");
            }
            _ => panic!("Expected Expand action"),
        }
    }

    #[test]
    fn test_no_expansion_without_boundary() {
        let mut engine = setup_engine(TriggerMode::WordBoundary);
        let actions = type_string(&mut engine, ";sig");
        // No word boundary — no expansion
        for action in &actions {
            assert!(matches!(action, ExpanderAction::None));
        }
    }

    #[test]
    fn test_immediate_expansion() {
        let mut engine = setup_engine(TriggerMode::Immediate);
        let actions = type_string(&mut engine, ";sig");
        // Last character should trigger expansion immediately
        let last = actions.last().unwrap();
        match last {
            ExpanderAction::Expand {
                backspace_count,
                text,
                ..
            } => {
                assert_eq!(*backspace_count, 4);
                assert_eq!(text, "Best regards,\nJohn");
            }
            _ => panic!("Expected Expand action"),
        }
    }

    #[test]
    fn test_no_match() {
        let mut engine = setup_engine(TriggerMode::WordBoundary);
        let actions = type_string(&mut engine, "hello ");
        for action in &actions {
            assert!(matches!(action, ExpanderAction::None));
        }
    }

    #[test]
    fn test_disabled_engine() {
        let mut engine = setup_engine(TriggerMode::Immediate);
        engine.toggle(); // disable
        assert!(!engine.is_enabled());
        let actions = type_string(&mut engine, ";sig");
        for action in &actions {
            assert!(matches!(action, ExpanderAction::None));
        }
    }

    #[test]
    fn test_toggle() {
        let engine = ExpanderEngine::new(TriggerMode::WordBoundary, 5);
        assert!(engine.is_enabled());
        let new_state = engine.toggle();
        assert!(!new_state);
        assert!(!engine.is_enabled());
        let new_state = engine.toggle();
        assert!(new_state);
        assert!(engine.is_enabled());
    }

    #[test]
    fn test_backspace_removes_from_buffer() {
        let mut engine = setup_engine(TriggerMode::WordBoundary);
        type_string(&mut engine, ";si");
        // Backspace should remove 'i'
        engine.process_key(KeyCode::KEY_BACKSPACE, true);
        // Now type 'g' + space — buffer has ";sg" not ";sig"
        let actions = type_string(&mut engine, "g ");
        let last = actions.last().unwrap();
        // ";sg" is not an abbreviation
        assert!(matches!(last, ExpanderAction::None));
    }

    #[test]
    fn test_slash_abbreviation() {
        let mut engine = setup_engine(TriggerMode::WordBoundary);
        let actions = type_string(&mut engine, "//date ");
        let last = actions.last().unwrap();
        match last {
            ExpanderAction::Expand {
                backspace_count,
                text,
                ..
            } => {
                assert_eq!(*backspace_count, 7); // "//date" + the typed space
                assert_eq!(text, "2024-01-01");
            }
            _ => panic!("Expected Expand action"),
        }
    }

    #[test]
    fn test_case_sensitive() {
        let mut engine = setup_engine(TriggerMode::Immediate);
        // ";SIG" should NOT match ";sig"
        let actions = type_string(&mut engine, ";SIG");
        for action in &actions {
            assert!(matches!(action, ExpanderAction::None));
        }
    }

    #[test]
    fn test_non_char_key_resets_buffer() {
        let mut engine = setup_engine(TriggerMode::Immediate);
        type_string(&mut engine, ";si");
        // Press Escape (non-character key) — resets buffer
        engine.process_key(KeyCode::KEY_ESC, true);
        // Now type "g" — buffer only has "g", not ";sig"
        let actions = type_string(&mut engine, "g");
        let last = actions.last().unwrap();
        assert!(matches!(last, ExpanderAction::None));
    }

    fn expand_of(action: &ExpanderAction) -> (usize, &str, Option<char>) {
        match action {
            ExpanderAction::Expand {
                backspace_count,
                text,
                trigger,
                ..
            } => (*backspace_count, text.as_str(), *trigger),
            ExpanderAction::None => panic!("Expected Expand action"),
        }
    }

    fn setup_words(mode: TriggerMode) -> ExpanderEngine {
        let engine = ExpanderEngine::new(mode, 5);
        engine.matcher.lock().unwrap().load(vec![(
            "cos".into(),
            "9".into(),
            "Chief of Staff".into(),
            "plain".into(),
        )]);
        engine
    }

    #[test]
    fn test_trigger_space() {
        let mut engine = setup_engine(TriggerMode::WordBoundary);
        let actions = type_string(&mut engine, ";sig ");
        assert_eq!(expand_of(actions.last().unwrap()).2, Some(' '));
    }

    #[test]
    fn test_trigger_punctuation() {
        let mut engine = setup_words(TriggerMode::WordBoundary);
        let actions = type_string(&mut engine, "cos,");
        let (bs, text, trigger) = expand_of(actions.last().unwrap());
        assert_eq!((bs, text, trigger), (4, "Chief of Staff", Some(',')));

        let mut engine = setup_words(TriggerMode::WordBoundary);
        let actions = type_string(&mut engine, "cos.");
        assert_eq!(expand_of(actions.last().unwrap()).2, Some('.'));
    }

    #[test]
    fn test_trigger_enter() {
        let mut engine = setup_words(TriggerMode::WordBoundary);
        type_string(&mut engine, "cos");
        let action = engine.process_key(KeyCode::KEY_ENTER, true);
        assert_eq!(expand_of(&action).2, Some('\n'));
    }

    #[test]
    fn test_trigger_tab_is_none() {
        let mut engine = setup_words(TriggerMode::WordBoundary);
        type_string(&mut engine, "cos");
        let action = engine.process_key(KeyCode::KEY_TAB, true);
        let (bs, _, trigger) = expand_of(&action);
        assert_eq!(trigger, None);
        assert_eq!(bs, 4); // the typed Tab is still deleted
    }

    #[test]
    fn test_immediate_has_no_trigger_and_no_extra_backspace() {
        let mut engine = setup_engine(TriggerMode::Immediate);
        let actions = type_string(&mut engine, ";sig");
        let (bs, _, trigger) = expand_of(actions.last().unwrap());
        assert_eq!((bs, trigger), (4, None));
    }

    #[test]
    fn test_whole_word_in_engine() {
        let mut engine = setup_words(TriggerMode::WordBoundary);
        let actions = type_string(&mut engine, "macos ");
        assert!(matches!(actions.last().unwrap(), ExpanderAction::None));

        let mut engine = setup_words(TriggerMode::WordBoundary);
        let actions = type_string(&mut engine, "cos ");
        assert!(matches!(
            actions.last().unwrap(),
            ExpanderAction::Expand { .. }
        ));
    }

    #[test]
    fn test_shared_matcher_reload_is_seen_by_engine() {
        let shared = Arc::new(Mutex::new(AbbreviationMatcher::new()));
        let mut engine = ExpanderEngine::with_matcher(TriggerMode::WordBoundary, 5, shared.clone());
        let actions = type_string(&mut engine, "omw ");
        assert!(matches!(actions.last().unwrap(), ExpanderAction::None));

        shared.lock().unwrap().load(vec![(
            "omw".into(),
            "1".into(),
            "On my way!".into(),
            "plain".into(),
        )]);
        let actions = type_string(&mut engine, "omw ");
        assert_eq!(expand_of(actions.last().unwrap()).1, "On my way!");
    }

    #[test]
    fn test_needs_clipboard() {
        assert!(!needs_clipboard("On my way!"));
        assert!(!needs_clipboard(""));
        assert!(needs_clipboard("hot \u{1F525}"));
        assert!(needs_clipboard("Zo\u{eb}"));
    }

    #[test]
    fn test_with_trigger() {
        assert_eq!(with_trigger("hi", Some(' ')), "hi ");
        assert_eq!(with_trigger("hi", Some('\n')), "hi\n");
        assert_eq!(with_trigger("hi", None), "hi");
    }
}
