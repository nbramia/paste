//! Persistent uinput virtual keyboard for key injection.
//!
//! ydotool without a running ydotoold creates a fresh uinput device on every
//! invocation and emits events before the compositor has finished registering
//! the device, so the injected keystrokes are silently dropped (its own
//! "may have latency+delay issues" warning). Creating one virtual device at
//! startup and keeping it alive avoids that race entirely, with no external
//! tool involved.

use std::io;
use std::sync::{Mutex, OnceLock};
use std::thread;
use std::time::Duration;

use evdev::uinput::VirtualDevice;
use evdev::{AttributeSet, EventType, InputEvent, KeyCode};
use log::{info, warn};

static VIRTUAL_KEYBOARD: OnceLock<Option<Mutex<VirtualDevice>>> = OnceLock::new();

fn device() -> Option<&'static Mutex<VirtualDevice>> {
    VIRTUAL_KEYBOARD
        .get_or_init(|| match build_device() {
            Ok(dev) => {
                info!("Persistent virtual keyboard created for key injection");
                Some(Mutex::new(dev))
            }
            Err(e) => {
                warn!("Could not create uinput virtual keyboard ({e}); key injection will fall back to external tools");
                None
            }
        })
        .as_ref()
}

fn build_device() -> io::Result<VirtualDevice> {
    // Declare the full standard key range. udev only classifies a device as
    // ID_INPUT_KEYBOARD when it advertises a realistic key set (a device
    // with just KEY_LEFTCTRL+KEY_V gets ID_INPUT_KEY only), and libinput
    // ignores key events from devices not classified as keyboards.
    // The hotkey daemon excludes this device by name to avoid feedback.
    let mut keys = AttributeSet::<KeyCode>::new();
    for code in 1..=248u16 {
        keys.insert(KeyCode::new(code));
    }
    // The name serves two filters at once: keymappers built on xwaykeyz
    // (e.g. Toshy) exclusively grab every keyboard-classified device but
    // skip names containing their own "XWayKeyz (virtual)" prefix — without
    // it they grab this device and remap the injected keystrokes. Our own
    // hotkey daemon excludes it by the "paste-injection" token.
    VirtualDevice::builder()?
        .name("XWayKeyz (virtual) paste-injection")
        .with_keys(&keys)?
        .build()
}

/// Create the device eagerly at startup so the compositor has registered it
/// long before the first paste.
pub fn init() {
    let _ = device();
}

/// US-layout key and Shift state that produce an ASCII character, or `None`
/// for characters the virtual keyboard cannot type.
pub fn key_for_char(ch: char) -> Option<(KeyCode, bool)> {
    const LETTERS: [KeyCode; 26] = [
        KeyCode::KEY_A,
        KeyCode::KEY_B,
        KeyCode::KEY_C,
        KeyCode::KEY_D,
        KeyCode::KEY_E,
        KeyCode::KEY_F,
        KeyCode::KEY_G,
        KeyCode::KEY_H,
        KeyCode::KEY_I,
        KeyCode::KEY_J,
        KeyCode::KEY_K,
        KeyCode::KEY_L,
        KeyCode::KEY_M,
        KeyCode::KEY_N,
        KeyCode::KEY_O,
        KeyCode::KEY_P,
        KeyCode::KEY_Q,
        KeyCode::KEY_R,
        KeyCode::KEY_S,
        KeyCode::KEY_T,
        KeyCode::KEY_U,
        KeyCode::KEY_V,
        KeyCode::KEY_W,
        KeyCode::KEY_X,
        KeyCode::KEY_Y,
        KeyCode::KEY_Z,
    ];
    const DIGITS: [KeyCode; 10] = [
        KeyCode::KEY_0,
        KeyCode::KEY_1,
        KeyCode::KEY_2,
        KeyCode::KEY_3,
        KeyCode::KEY_4,
        KeyCode::KEY_5,
        KeyCode::KEY_6,
        KeyCode::KEY_7,
        KeyCode::KEY_8,
        KeyCode::KEY_9,
    ];
    const SHIFTED_DIGITS: &str = ")!@#$%^&*(";

    if ch.is_ascii_lowercase() {
        return Some((LETTERS[(ch as u8 - b'a') as usize], false));
    }
    if ch.is_ascii_uppercase() {
        return Some((LETTERS[(ch as u8 - b'A') as usize], true));
    }
    if ch.is_ascii_digit() {
        return Some((DIGITS[(ch as u8 - b'0') as usize], false));
    }
    if let Some(i) = SHIFTED_DIGITS.find(ch) {
        return Some((DIGITS[i], true));
    }
    let mapped = match ch {
        ' ' => (KeyCode::KEY_SPACE, false),
        '\n' => (KeyCode::KEY_ENTER, false),
        '\t' => (KeyCode::KEY_TAB, false),
        '-' => (KeyCode::KEY_MINUS, false),
        '_' => (KeyCode::KEY_MINUS, true),
        '=' => (KeyCode::KEY_EQUAL, false),
        '+' => (KeyCode::KEY_EQUAL, true),
        '[' => (KeyCode::KEY_LEFTBRACE, false),
        '{' => (KeyCode::KEY_LEFTBRACE, true),
        ']' => (KeyCode::KEY_RIGHTBRACE, false),
        '}' => (KeyCode::KEY_RIGHTBRACE, true),
        ';' => (KeyCode::KEY_SEMICOLON, false),
        ':' => (KeyCode::KEY_SEMICOLON, true),
        '\'' => (KeyCode::KEY_APOSTROPHE, false),
        '"' => (KeyCode::KEY_APOSTROPHE, true),
        '`' => (KeyCode::KEY_GRAVE, false),
        '~' => (KeyCode::KEY_GRAVE, true),
        '\\' => (KeyCode::KEY_BACKSLASH, false),
        '|' => (KeyCode::KEY_BACKSLASH, true),
        ',' => (KeyCode::KEY_COMMA, false),
        '<' => (KeyCode::KEY_COMMA, true),
        '.' => (KeyCode::KEY_DOT, false),
        '>' => (KeyCode::KEY_DOT, true),
        '/' => (KeyCode::KEY_SLASH, false),
        '?' => (KeyCode::KEY_SLASH, true),
        _ => return None,
    };
    Some(mapped)
}

/// Emit one key transition, then pause briefly.
fn emit_key(dev: &mut VirtualDevice, code: KeyCode, value: i32) -> bool {
    let event = InputEvent::new(EventType::KEY.0, code.0, value);
    if let Err(e) = dev.emit(&[event]) {
        warn!("Virtual keyboard emit failed: {e}");
        return false;
    }
    thread::sleep(Duration::from_millis(1));
    true
}

/// Outcome of a key sequence sent through the virtual keyboard.
#[derive(Debug, PartialEq, Eq)]
pub enum VkOutcome {
    /// Every key was emitted.
    Done,
    /// Nothing was emitted (no device, or untypeable text); fall back to an
    /// external tool.
    Unavailable,
    /// An emit failed partway. Some keys already reached the focused window,
    /// so retrying the whole sequence elsewhere would duplicate them.
    Failed,
}

/// Press and release `key`, holding Shift around it when asked. On a failed
/// emit, every key this call pressed is released (best effort), so nothing is
/// left held on the persistent device.
fn tap(dev: &mut VirtualDevice, key: KeyCode, shift: bool) -> bool {
    if shift && !emit_key(dev, KeyCode::KEY_LEFTSHIFT, 1) {
        return false;
    }
    let ok = emit_key(dev, key, 1) && emit_key(dev, key, 0);
    if !ok {
        emit_key(dev, key, 0);
    }
    let released = !shift || emit_key(dev, KeyCode::KEY_LEFTSHIFT, 0);
    ok && released
}

/// Type ASCII text through the persistent virtual keyboard, assuming a US
/// layout. Returns `Unavailable` without emitting anything when the device is
/// missing or the text holds a character it cannot type.
///
/// Typing this way takes a few milliseconds where spawning `ydotool type`
/// takes hundreds. That matters for the text expander, because keys the user
/// types while an expansion is still being injected land in the middle of it.
pub fn type_text(text: &str) -> VkOutcome {
    let Some(keys) = text.chars().map(key_for_char).collect::<Option<Vec<_>>>() else {
        return VkOutcome::Unavailable;
    };
    let Some(dev) = device() else {
        return VkOutcome::Unavailable;
    };
    let mut dev = match dev.lock() {
        Ok(d) => d,
        Err(poisoned) => poisoned.into_inner(),
    };
    for (key, shift) in keys {
        if !tap(&mut dev, key, shift) {
            return VkOutcome::Failed;
        }
    }
    VkOutcome::Done
}

/// Emit `count` Backspace presses through the persistent virtual keyboard.
///
/// This is the only backspace path that reaches native Wayland windows:
/// `xdotool` talks to XWayland and its keys never arrive in a Wayland client.
pub fn backspaces(count: usize) -> VkOutcome {
    let Some(dev) = device() else {
        return VkOutcome::Unavailable;
    };
    let mut dev = match dev.lock() {
        Ok(d) => d,
        Err(poisoned) => poisoned.into_inner(),
    };
    for _ in 0..count {
        if !tap(&mut dev, KeyCode::KEY_BACKSPACE, false) {
            return VkOutcome::Failed;
        }
    }
    VkOutcome::Done
}

/// Emit Ctrl+V through the persistent virtual keyboard.
/// Returns false when the device is unavailable — the caller should fall
/// back to an external tool.
pub fn ctrl_v() -> bool {
    let Some(dev) = device() else {
        return false;
    };
    let mut dev = match dev.lock() {
        Ok(d) => d,
        Err(poisoned) => poisoned.into_inner(),
    };

    let seq: [(KeyCode, i32); 4] = [
        (KeyCode::KEY_LEFTCTRL, 1),
        (KeyCode::KEY_V, 1),
        (KeyCode::KEY_V, 0),
        (KeyCode::KEY_LEFTCTRL, 0),
    ];
    for (code, value) in seq {
        let event = InputEvent::new(EventType::KEY.0, code.0, value);
        if let Err(e) = dev.emit(&[event]) {
            warn!("Virtual keyboard emit failed: {e}");
            return false;
        }
        // Small gap between key transitions, mirroring real typing.
        thread::sleep(Duration::from_millis(5));
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_letters_with_shift_for_uppercase() {
        assert_eq!(key_for_char('a'), Some((KeyCode::KEY_A, false)));
        assert_eq!(key_for_char('Z'), Some((KeyCode::KEY_Z, true)));
    }

    #[test]
    fn maps_digits_and_shifted_digits() {
        assert_eq!(key_for_char('0'), Some((KeyCode::KEY_0, false)));
        assert_eq!(key_for_char('!'), Some((KeyCode::KEY_1, true)));
        assert_eq!(key_for_char('('), Some((KeyCode::KEY_9, true)));
        assert_eq!(key_for_char(')'), Some((KeyCode::KEY_0, true)));
    }

    #[test]
    fn maps_punctuation_and_whitespace() {
        assert_eq!(key_for_char('@'), Some((KeyCode::KEY_2, true)));
        assert_eq!(key_for_char(':'), Some((KeyCode::KEY_SEMICOLON, true)));
        assert_eq!(key_for_char('\''), Some((KeyCode::KEY_APOSTROPHE, false)));
        assert_eq!(key_for_char('\n'), Some((KeyCode::KEY_ENTER, false)));
        assert_eq!(key_for_char(' '), Some((KeyCode::KEY_SPACE, false)));
    }

    #[test]
    fn every_printable_ascii_char_is_typeable() {
        for b in 0x20u8..0x7f {
            assert!(key_for_char(b as char).is_some(), "{:?}", b as char);
        }
    }

    #[test]
    fn non_ascii_is_not_typeable() {
        assert_eq!(key_for_char('ë'), None);
        assert_eq!(key_for_char('🔥'), None);
    }
}
