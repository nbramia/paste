use evdev::KeyCode;

/// Convert a KeyCode to its character representation.
/// Returns None for non-character keys (modifiers, function keys, etc.).
pub fn keycode_to_char(key: KeyCode, shift: bool) -> Option<char> {
    match key {
        // Letters
        KeyCode::KEY_A => Some(if shift { 'A' } else { 'a' }),
        KeyCode::KEY_B => Some(if shift { 'B' } else { 'b' }),
        KeyCode::KEY_C => Some(if shift { 'C' } else { 'c' }),
        KeyCode::KEY_D => Some(if shift { 'D' } else { 'd' }),
        KeyCode::KEY_E => Some(if shift { 'E' } else { 'e' }),
        KeyCode::KEY_F => Some(if shift { 'F' } else { 'f' }),
        KeyCode::KEY_G => Some(if shift { 'G' } else { 'g' }),
        KeyCode::KEY_H => Some(if shift { 'H' } else { 'h' }),
        KeyCode::KEY_I => Some(if shift { 'I' } else { 'i' }),
        KeyCode::KEY_J => Some(if shift { 'J' } else { 'j' }),
        KeyCode::KEY_K => Some(if shift { 'K' } else { 'k' }),
        KeyCode::KEY_L => Some(if shift { 'L' } else { 'l' }),
        KeyCode::KEY_M => Some(if shift { 'M' } else { 'm' }),
        KeyCode::KEY_N => Some(if shift { 'N' } else { 'n' }),
        KeyCode::KEY_O => Some(if shift { 'O' } else { 'o' }),
        KeyCode::KEY_P => Some(if shift { 'P' } else { 'p' }),
        KeyCode::KEY_Q => Some(if shift { 'Q' } else { 'q' }),
        KeyCode::KEY_R => Some(if shift { 'R' } else { 'r' }),
        KeyCode::KEY_S => Some(if shift { 'S' } else { 's' }),
        KeyCode::KEY_T => Some(if shift { 'T' } else { 't' }),
        KeyCode::KEY_U => Some(if shift { 'U' } else { 'u' }),
        KeyCode::KEY_V => Some(if shift { 'V' } else { 'v' }),
        KeyCode::KEY_W => Some(if shift { 'W' } else { 'w' }),
        KeyCode::KEY_X => Some(if shift { 'X' } else { 'x' }),
        KeyCode::KEY_Y => Some(if shift { 'Y' } else { 'y' }),
        KeyCode::KEY_Z => Some(if shift { 'Z' } else { 'z' }),

        // Numbers / symbols
        KeyCode::KEY_1 => Some(if shift { '!' } else { '1' }),
        KeyCode::KEY_2 => Some(if shift { '@' } else { '2' }),
        KeyCode::KEY_3 => Some(if shift { '#' } else { '3' }),
        KeyCode::KEY_4 => Some(if shift { '$' } else { '4' }),
        KeyCode::KEY_5 => Some(if shift { '%' } else { '5' }),
        KeyCode::KEY_6 => Some(if shift { '^' } else { '6' }),
        KeyCode::KEY_7 => Some(if shift { '&' } else { '7' }),
        KeyCode::KEY_8 => Some(if shift { '*' } else { '8' }),
        KeyCode::KEY_9 => Some(if shift { '(' } else { '9' }),
        KeyCode::KEY_0 => Some(if shift { ')' } else { '0' }),

        // Punctuation
        KeyCode::KEY_MINUS => Some(if shift { '_' } else { '-' }),
        KeyCode::KEY_EQUAL => Some(if shift { '+' } else { '=' }),
        KeyCode::KEY_LEFTBRACE => Some(if shift { '{' } else { '[' }),
        KeyCode::KEY_RIGHTBRACE => Some(if shift { '}' } else { ']' }),
        KeyCode::KEY_SEMICOLON => Some(if shift { ':' } else { ';' }),
        KeyCode::KEY_APOSTROPHE => Some(if shift { '"' } else { '\'' }),
        KeyCode::KEY_GRAVE => Some(if shift { '~' } else { '`' }),
        KeyCode::KEY_BACKSLASH => Some(if shift { '|' } else { '\\' }),
        KeyCode::KEY_COMMA => Some(if shift { '<' } else { ',' }),
        KeyCode::KEY_DOT => Some(if shift { '>' } else { '.' }),
        KeyCode::KEY_SLASH => Some(if shift { '?' } else { '/' }),

        // Whitespace
        KeyCode::KEY_SPACE => Some(' '),
        KeyCode::KEY_TAB => Some('\t'),

        // Everything else returns None
        _ => None,
    }
}

/// Check if a KeyCode represents a word boundary character.
/// Word boundaries: space, tab, enter, punctuation.
pub fn is_word_boundary_key(key: KeyCode) -> bool {
    matches!(
        key,
        KeyCode::KEY_SPACE
            | KeyCode::KEY_TAB
            | KeyCode::KEY_ENTER
            | KeyCode::KEY_DOT
            | KeyCode::KEY_COMMA
            | KeyCode::KEY_SEMICOLON
            | KeyCode::KEY_APOSTROPHE
            | KeyCode::KEY_SLASH
            | KeyCode::KEY_BACKSLASH
            | KeyCode::KEY_MINUS
            | KeyCode::KEY_EQUAL
            | KeyCode::KEY_LEFTBRACE
            | KeyCode::KEY_RIGHTBRACE
            | KeyCode::KEY_GRAVE
    )
}

/// Check if a KeyCode is a modifier key.
pub fn is_modifier_key(key: KeyCode) -> bool {
    matches!(
        key,
        KeyCode::KEY_LEFTCTRL
            | KeyCode::KEY_RIGHTCTRL
            | KeyCode::KEY_LEFTSHIFT
            | KeyCode::KEY_RIGHTSHIFT
            | KeyCode::KEY_LEFTALT
            | KeyCode::KEY_RIGHTALT
            | KeyCode::KEY_LEFTMETA
            | KeyCode::KEY_RIGHTMETA
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_letters() {
        assert_eq!(keycode_to_char(KeyCode::KEY_A, false), Some('a'));
        assert_eq!(keycode_to_char(KeyCode::KEY_A, true), Some('A'));
        assert_eq!(keycode_to_char(KeyCode::KEY_Z, false), Some('z'));
        assert_eq!(keycode_to_char(KeyCode::KEY_Z, true), Some('Z'));
    }

    #[test]
    fn test_numbers() {
        assert_eq!(keycode_to_char(KeyCode::KEY_1, false), Some('1'));
        assert_eq!(keycode_to_char(KeyCode::KEY_1, true), Some('!'));
        assert_eq!(keycode_to_char(KeyCode::KEY_0, false), Some('0'));
    }

    #[test]
    fn test_punctuation() {
        assert_eq!(keycode_to_char(KeyCode::KEY_SEMICOLON, false), Some(';'));
        assert_eq!(keycode_to_char(KeyCode::KEY_SEMICOLON, true), Some(':'));
        assert_eq!(keycode_to_char(KeyCode::KEY_DOT, false), Some('.'));
    }

    #[test]
    fn test_space() {
        assert_eq!(keycode_to_char(KeyCode::KEY_SPACE, false), Some(' '));
    }

    #[test]
    fn test_non_char_keys() {
        assert_eq!(keycode_to_char(KeyCode::KEY_LEFTCTRL, false), None);
        assert_eq!(keycode_to_char(KeyCode::KEY_F1, false), None);
        assert_eq!(keycode_to_char(KeyCode::KEY_ESC, false), None);
        assert_eq!(keycode_to_char(KeyCode::KEY_ENTER, false), None);
    }

    #[test]
    fn test_word_boundary() {
        assert!(is_word_boundary_key(KeyCode::KEY_SPACE));
        assert!(is_word_boundary_key(KeyCode::KEY_ENTER));
        assert!(is_word_boundary_key(KeyCode::KEY_DOT));
        assert!(is_word_boundary_key(KeyCode::KEY_COMMA));
        assert!(!is_word_boundary_key(KeyCode::KEY_A));
        assert!(!is_word_boundary_key(KeyCode::KEY_1));
    }

    #[test]
    fn test_modifier_key() {
        assert!(is_modifier_key(KeyCode::KEY_LEFTCTRL));
        assert!(is_modifier_key(KeyCode::KEY_LEFTSHIFT));
        assert!(is_modifier_key(KeyCode::KEY_RIGHTALT));
        assert!(is_modifier_key(KeyCode::KEY_LEFTMETA));
        assert!(!is_modifier_key(KeyCode::KEY_A));
        assert!(!is_modifier_key(KeyCode::KEY_SPACE));
    }
}
