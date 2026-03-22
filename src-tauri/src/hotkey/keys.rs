use evdev::KeyCode;
use std::fmt;

/// Modifier key flags.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Modifiers {
    pub ctrl: bool,
    pub shift: bool,
    pub alt: bool,
    pub super_key: bool, // Meta/Super/Windows key
}

impl Modifiers {
    /// Check if all required modifiers in `self` are active in `current`.
    pub fn matches(&self, current: &Modifiers) -> bool {
        (!self.ctrl || current.ctrl)
            && (!self.shift || current.shift)
            && (!self.alt || current.alt)
            && (!self.super_key || current.super_key)
    }

    /// Check if any modifier is set.
    pub fn any(&self) -> bool {
        self.ctrl || self.shift || self.alt || self.super_key
    }
}

impl fmt::Display for Modifiers {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut parts = vec![];
        if self.ctrl {
            parts.push("Ctrl");
        }
        if self.alt {
            parts.push("Alt");
        }
        if self.shift {
            parts.push("Shift");
        }
        if self.super_key {
            parts.push("Super");
        }
        write!(f, "{}", parts.join("+"))
    }
}

/// A key combination: modifiers + a single key.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyCombo {
    pub modifiers: Modifiers,
    pub key: KeyCode,
}

impl fmt::Display for KeyCombo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.modifiers.any() {
            write!(f, "{}+{:?}", self.modifiers, self.key)
        } else {
            write!(f, "{:?}", self.key)
        }
    }
}

/// Parse a hotkey string like "Super+V" or "Ctrl+Shift+C" into a KeyCombo.
///
/// Modifier names (case-insensitive): Ctrl, Control, Alt, Shift, Super, Meta, Win
/// Key names: single letters (A-Z), numbers (0-9), or evdev names (Space, Tab, Escape, etc.)
pub fn parse_hotkey(s: &str) -> Result<KeyCombo, HotkeyParseError> {
    let parts: Vec<&str> = s.split('+').map(|p| p.trim()).collect();

    if parts.is_empty() || (parts.len() == 1 && parts[0].is_empty()) {
        return Err(HotkeyParseError::Empty);
    }

    let mut modifiers = Modifiers::default();
    let mut key_part: Option<&str> = None;

    for (i, part) in parts.iter().enumerate() {
        match part.to_lowercase().as_str() {
            "ctrl" | "control" => modifiers.ctrl = true,
            "alt" => modifiers.alt = true,
            "shift" => modifiers.shift = true,
            "super" | "meta" | "win" | "windows" | "logo" => modifiers.super_key = true,
            _ => {
                // Last non-modifier part is the key
                if i == parts.len() - 1 {
                    key_part = Some(part);
                } else {
                    return Err(HotkeyParseError::InvalidModifier(part.to_string()));
                }
            }
        }
    }

    let key_str = key_part.ok_or(HotkeyParseError::NoKey)?;
    let key = parse_key_name(key_str)?;

    Ok(KeyCombo { modifiers, key })
}

/// Parse a key name string into an evdev KeyCode.
fn parse_key_name(name: &str) -> Result<KeyCode, HotkeyParseError> {
    // Single letter A-Z
    if name.len() == 1 {
        let ch = name.chars().next().unwrap().to_ascii_uppercase();
        if ch.is_ascii_uppercase() {
            let key = match ch {
                'A' => KeyCode::KEY_A,
                'B' => KeyCode::KEY_B,
                'C' => KeyCode::KEY_C,
                'D' => KeyCode::KEY_D,
                'E' => KeyCode::KEY_E,
                'F' => KeyCode::KEY_F,
                'G' => KeyCode::KEY_G,
                'H' => KeyCode::KEY_H,
                'I' => KeyCode::KEY_I,
                'J' => KeyCode::KEY_J,
                'K' => KeyCode::KEY_K,
                'L' => KeyCode::KEY_L,
                'M' => KeyCode::KEY_M,
                'N' => KeyCode::KEY_N,
                'O' => KeyCode::KEY_O,
                'P' => KeyCode::KEY_P,
                'Q' => KeyCode::KEY_Q,
                'R' => KeyCode::KEY_R,
                'S' => KeyCode::KEY_S,
                'T' => KeyCode::KEY_T,
                'U' => KeyCode::KEY_U,
                'V' => KeyCode::KEY_V,
                'W' => KeyCode::KEY_W,
                'X' => KeyCode::KEY_X,
                'Y' => KeyCode::KEY_Y,
                'Z' => KeyCode::KEY_Z,
                _ => unreachable!(),
            };
            return Ok(key);
        }
        // Single digit 0-9
        if ch.is_ascii_digit() {
            let key = match ch {
                '0' => KeyCode::KEY_0,
                '1' => KeyCode::KEY_1,
                '2' => KeyCode::KEY_2,
                '3' => KeyCode::KEY_3,
                '4' => KeyCode::KEY_4,
                '5' => KeyCode::KEY_5,
                '6' => KeyCode::KEY_6,
                '7' => KeyCode::KEY_7,
                '8' => KeyCode::KEY_8,
                '9' => KeyCode::KEY_9,
                _ => unreachable!(),
            };
            return Ok(key);
        }
    }

    // Named keys (case-insensitive)
    match name.to_lowercase().as_str() {
        "space" => Ok(KeyCode::KEY_SPACE),
        "enter" | "return" => Ok(KeyCode::KEY_ENTER),
        "tab" => Ok(KeyCode::KEY_TAB),
        "escape" | "esc" => Ok(KeyCode::KEY_ESC),
        "backspace" => Ok(KeyCode::KEY_BACKSPACE),
        "delete" | "del" => Ok(KeyCode::KEY_DELETE),
        "insert" | "ins" => Ok(KeyCode::KEY_INSERT),
        "home" => Ok(KeyCode::KEY_HOME),
        "end" => Ok(KeyCode::KEY_END),
        "pageup" | "pgup" => Ok(KeyCode::KEY_PAGEUP),
        "pagedown" | "pgdn" | "pgdown" => Ok(KeyCode::KEY_PAGEDOWN),
        "up" => Ok(KeyCode::KEY_UP),
        "down" => Ok(KeyCode::KEY_DOWN),
        "left" => Ok(KeyCode::KEY_LEFT),
        "right" => Ok(KeyCode::KEY_RIGHT),
        "f1" => Ok(KeyCode::KEY_F1),
        "f2" => Ok(KeyCode::KEY_F2),
        "f3" => Ok(KeyCode::KEY_F3),
        "f4" => Ok(KeyCode::KEY_F4),
        "f5" => Ok(KeyCode::KEY_F5),
        "f6" => Ok(KeyCode::KEY_F6),
        "f7" => Ok(KeyCode::KEY_F7),
        "f8" => Ok(KeyCode::KEY_F8),
        "f9" => Ok(KeyCode::KEY_F9),
        "f10" => Ok(KeyCode::KEY_F10),
        "f11" => Ok(KeyCode::KEY_F11),
        "f12" => Ok(KeyCode::KEY_F12),
        "minus" => Ok(KeyCode::KEY_MINUS),
        "equal" | "equals" => Ok(KeyCode::KEY_EQUAL),
        "comma" => Ok(KeyCode::KEY_COMMA),
        "period" | "dot" => Ok(KeyCode::KEY_DOT),
        "slash" => Ok(KeyCode::KEY_SLASH),
        "backslash" => Ok(KeyCode::KEY_BACKSLASH),
        "semicolon" => Ok(KeyCode::KEY_SEMICOLON),
        "apostrophe" | "quote" => Ok(KeyCode::KEY_APOSTROPHE),
        "bracketleft" | "lbracket" => Ok(KeyCode::KEY_LEFTBRACE),
        "bracketright" | "rbracket" => Ok(KeyCode::KEY_RIGHTBRACE),
        "grave" | "backtick" => Ok(KeyCode::KEY_GRAVE),
        "printscreen" | "print" => Ok(KeyCode::KEY_PRINT),
        "scrolllock" => Ok(KeyCode::KEY_SCROLLLOCK),
        "pause" => Ok(KeyCode::KEY_PAUSE),
        "capslock" => Ok(KeyCode::KEY_CAPSLOCK),
        "numlock" => Ok(KeyCode::KEY_NUMLOCK),
        _ => Err(HotkeyParseError::UnknownKey(name.to_string())),
    }
}

/// Errors from parsing hotkey strings.
#[derive(Debug, thiserror::Error)]
pub enum HotkeyParseError {
    #[error("Empty hotkey string")]
    Empty,
    #[error("No key specified (only modifiers)")]
    NoKey,
    #[error("Invalid modifier: {0}")]
    InvalidModifier(String),
    #[error("Unknown key name: {0}")]
    UnknownKey(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_super_v() {
        let combo = parse_hotkey("Super+V").unwrap();
        assert!(combo.modifiers.super_key);
        assert!(!combo.modifiers.ctrl);
        assert!(!combo.modifiers.alt);
        assert!(!combo.modifiers.shift);
        assert_eq!(combo.key, KeyCode::KEY_V);
    }

    #[test]
    fn test_parse_ctrl_shift_v() {
        let combo = parse_hotkey("Ctrl+Shift+V").unwrap();
        assert!(combo.modifiers.ctrl);
        assert!(combo.modifiers.shift);
        assert!(!combo.modifiers.alt);
        assert!(!combo.modifiers.super_key);
        assert_eq!(combo.key, KeyCode::KEY_V);
    }

    #[test]
    fn test_parse_ctrl_alt_space() {
        let combo = parse_hotkey("Ctrl+Alt+Space").unwrap();
        assert!(combo.modifiers.ctrl);
        assert!(combo.modifiers.alt);
        assert_eq!(combo.key, KeyCode::KEY_SPACE);
    }

    #[test]
    fn test_parse_super_shift_c() {
        let combo = parse_hotkey("Super+Shift+C").unwrap();
        assert!(combo.modifiers.super_key);
        assert!(combo.modifiers.shift);
        assert_eq!(combo.key, KeyCode::KEY_C);
    }

    #[test]
    fn test_parse_case_insensitive() {
        let combo = parse_hotkey("super+shift+v").unwrap();
        assert!(combo.modifiers.super_key);
        assert!(combo.modifiers.shift);
        assert_eq!(combo.key, KeyCode::KEY_V);
    }

    #[test]
    fn test_parse_meta_alias() {
        let combo = parse_hotkey("Meta+V").unwrap();
        assert!(combo.modifiers.super_key);
        assert_eq!(combo.key, KeyCode::KEY_V);
    }

    #[test]
    fn test_parse_with_spaces() {
        let combo = parse_hotkey("Ctrl + Alt + Space").unwrap();
        assert!(combo.modifiers.ctrl);
        assert!(combo.modifiers.alt);
        assert_eq!(combo.key, KeyCode::KEY_SPACE);
    }

    #[test]
    fn test_parse_function_key() {
        let combo = parse_hotkey("Ctrl+F12").unwrap();
        assert!(combo.modifiers.ctrl);
        assert_eq!(combo.key, KeyCode::KEY_F12);
    }

    #[test]
    fn test_parse_number_key() {
        let combo = parse_hotkey("Super+1").unwrap();
        assert!(combo.modifiers.super_key);
        assert_eq!(combo.key, KeyCode::KEY_1);
    }

    #[test]
    fn test_parse_escape() {
        let combo = parse_hotkey("Escape").unwrap();
        assert!(!combo.modifiers.any());
        assert_eq!(combo.key, KeyCode::KEY_ESC);
    }

    #[test]
    fn test_parse_invalid_empty() {
        assert!(parse_hotkey("").is_err());
    }

    #[test]
    fn test_parse_invalid_unknown_key() {
        assert!(parse_hotkey("Ctrl+FooBar").is_err());
    }

    #[test]
    fn test_parse_only_modifiers() {
        assert!(parse_hotkey("Ctrl+Shift").is_err());
    }

    #[test]
    fn test_modifiers_matches() {
        let required = Modifiers {
            ctrl: true,
            shift: true,
            alt: false,
            super_key: false,
        };
        let current = Modifiers {
            ctrl: true,
            shift: true,
            alt: false,
            super_key: false,
        };
        assert!(required.matches(&current));

        // Extra modifiers in current are OK
        let current_extra = Modifiers {
            ctrl: true,
            shift: true,
            alt: true,
            super_key: false,
        };
        assert!(required.matches(&current_extra));

        // Missing required modifier
        let current_missing = Modifiers {
            ctrl: true,
            shift: false,
            alt: false,
            super_key: false,
        };
        assert!(!required.matches(&current_missing));
    }

    #[test]
    fn test_modifiers_display() {
        let m = Modifiers {
            ctrl: true,
            shift: true,
            alt: false,
            super_key: true,
        };
        let s = m.to_string();
        assert!(s.contains("Ctrl"));
        assert!(s.contains("Shift"));
        assert!(s.contains("Super"));
        assert!(!s.contains("Alt"));
    }

    #[test]
    fn test_key_combo_display() {
        let combo = parse_hotkey("Super+V").unwrap();
        let s = combo.to_string();
        assert!(s.contains("Super"));
        assert!(s.contains("KEY_V"));
    }
}
