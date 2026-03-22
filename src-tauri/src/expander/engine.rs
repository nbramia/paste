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
    /// Expand: delete N chars (backspaces), then inject text.
    Expand {
        backspace_count: usize,
        text: String,
        snippet_id: String,
    },
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
    pub fn new(trigger_mode: TriggerMode, timeout_secs: u64) -> Self {
        Self {
            buffer: CharBuffer::new(100),
            matcher: Arc::new(Mutex::new(AbbreviationMatcher::new())),
            enabled: Arc::new(AtomicBool::new(true)),
            trigger_mode,
            shift_held: false,
            last_keystroke: Instant::now(),
            timeout_secs,
        }
    }

    /// Get a reference to the enabled flag for toggling from outside.
    pub fn enabled_flag(&self) -> Arc<AtomicBool> {
        self.enabled.clone()
    }

    /// Get a reference to the matcher for reloading abbreviations.
    pub fn matcher(&self) -> Arc<Mutex<AbbreviationMatcher>> {
        self.matcher.clone()
    }

    /// Check if the expander is enabled.
    pub fn is_enabled(&self) -> bool {
        self.enabled.load(Ordering::Relaxed)
    }

    /// Toggle enabled state. Returns new state.
    pub fn toggle(&self) -> bool {
        let prev = self.enabled.fetch_xor(true, Ordering::Relaxed);
        let new = !prev;
        info!(
            "Text expander {}",
            if new { "enabled" } else { "disabled" }
        );
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
                if let Some(action) = self.check_and_expand() {
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
            if let Some(action) = self.check_and_expand() {
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
            if let Some(action) = self.check_and_expand() {
                self.buffer.clear();
                return action;
            }
        }

        ExpanderAction::None
    }

    /// Check if the buffer matches an abbreviation and create an expansion action.
    fn check_and_expand(&self) -> Option<ExpanderAction> {
        let buffer_str = self.buffer.as_str();
        let matcher = self.matcher.lock().unwrap();

        if let Some(result) = matcher.find_match(&buffer_str) {
            let expanded = expand_template(&result.content);
            let backspace_count = result.abbreviation.chars().count();

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
                assert_eq!(*backspace_count, 4); // ";sig" = 4 chars
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
                assert_eq!(*backspace_count, 6); // "//date" = 6 chars
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
}
