//! End-to-end input probe.
//!
//! Creates a uinput keyboard and emits real key events, so the whole stack can
//! be exercised without a human at the keyboard: paste's evdev hotkey daemon
//! sees these exactly as it sees a physical keyboard.
//!
//! The device name carries the "XWayKeyz (virtual)" prefix so xwaykeyz-based
//! keymappers (Toshy) leave it alone — otherwise the chord we emit is remapped
//! before paste ever sees it. It deliberately avoids the "paste-injection"
//! token, which is how paste's own daemon excludes its injector device.
//!
//! Usage: cargo run --example e2e_probe -- <chord>...
//!   chords: ctrl-alt-v | ctrl-v | right | left | enter | escape | type:<text>
//!   `type:<text>` types US-layout ASCII (letters, digits, space, punctuation),
//!   holding Shift where the character needs it. PROBE_CHAR_DELAY_MS sets the
//!   gap between characters (default 30).
//! Example: cargo run --example e2e_probe -- ctrl-alt-v right enter
//! Example: cargo run --example e2e_probe -- 'type:cos '

use std::thread;
use std::time::Duration;

use evdev::uinput::VirtualDevice;
use evdev::{AttributeSet, EventType, InputEvent, KeyCode};

fn build_device() -> std::io::Result<VirtualDevice> {
    let mut keys = AttributeSet::<KeyCode>::new();
    // udev only tags a device as a keyboard when it advertises a realistic key
    // range, and libinput ignores key events from untagged devices.
    for code in 1..=248u16 {
        keys.insert(KeyCode::new(code));
    }
    VirtualDevice::builder()?
        .name("XWayKeyz (virtual) e2e-probe")
        .with_keys(&keys)?
        .build()
}

fn emit(dev: &mut VirtualDevice, code: KeyCode, value: i32) {
    let ev = InputEvent::new(EventType::KEY.0, code.0, value);
    if let Err(e) = dev.emit(&[ev]) {
        eprintln!("emit failed: {e}");
    }
    thread::sleep(Duration::from_millis(12));
}

/// US-layout key and Shift state for an ASCII character.
fn key_for_char(ch: char) -> Option<(KeyCode, bool)> {
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
    let (key, shift) = match ch {
        ' ' => (KeyCode::KEY_SPACE, false),
        '-' => (KeyCode::KEY_MINUS, false),
        '_' => (KeyCode::KEY_MINUS, true),
        '=' => (KeyCode::KEY_EQUAL, false),
        '+' => (KeyCode::KEY_EQUAL, true),
        '[' => (KeyCode::KEY_LEFTBRACE, false),
        ']' => (KeyCode::KEY_RIGHTBRACE, false),
        ';' => (KeyCode::KEY_SEMICOLON, false),
        ':' => (KeyCode::KEY_SEMICOLON, true),
        '\'' => (KeyCode::KEY_APOSTROPHE, false),
        ',' => (KeyCode::KEY_COMMA, false),
        '.' => (KeyCode::KEY_DOT, false),
        '/' => (KeyCode::KEY_SLASH, false),
        '?' => (KeyCode::KEY_SLASH, true),
        _ => return None,
    };
    Some((key, shift))
}

fn type_text(dev: &mut VirtualDevice, text: &str) {
    // Gap between characters; PROBE_CHAR_DELAY_MS models human typing speed.
    let gap = std::env::var("PROBE_CHAR_DELAY_MS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(30);
    for ch in text.chars() {
        match key_for_char(ch) {
            Some((key, true)) => chord(dev, &[KeyCode::KEY_LEFTSHIFT], key),
            Some((key, false)) => chord(dev, &[], key),
            None => {
                eprintln!("cannot type {ch:?}");
                std::process::exit(2);
            }
        }
        thread::sleep(Duration::from_millis(gap));
    }
}

fn chord(dev: &mut VirtualDevice, mods: &[KeyCode], key: KeyCode) {
    for m in mods {
        emit(dev, *m, 1);
    }
    emit(dev, key, 1);
    emit(dev, key, 0);
    for m in mods.iter().rev() {
        emit(dev, *m, 0);
    }
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.is_empty() {
        eprintln!("usage: e2e_probe <chord>... (ctrl-alt-v|ctrl-v|right|left|enter|escape)");
        std::process::exit(2);
    }

    let mut dev = match build_device() {
        Ok(d) => d,
        Err(e) => {
            eprintln!("could not create uinput device: {e}");
            std::process::exit(1);
        }
    };

    // Wait out both the compositor's device registration and paste's own
    // hotplug rescan (HOTPLUG_SCAN_INTERVAL, 2s) — events emitted before the
    // daemon has opened this device are simply not seen.
    let warmup: u64 = std::env::var("PROBE_WARMUP_MS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(5000);
    thread::sleep(Duration::from_millis(warmup));

    for arg in &args {
        match arg.as_str() {
            "ctrl-alt-v" => chord(
                &mut dev,
                &[KeyCode::KEY_LEFTCTRL, KeyCode::KEY_LEFTALT],
                KeyCode::KEY_V,
            ),
            "ctrl-v" => chord(&mut dev, &[KeyCode::KEY_LEFTCTRL], KeyCode::KEY_V),
            "right" => chord(&mut dev, &[], KeyCode::KEY_RIGHT),
            "left" => chord(&mut dev, &[], KeyCode::KEY_LEFT),
            "enter" => chord(&mut dev, &[], KeyCode::KEY_ENTER),
            "escape" => chord(&mut dev, &[], KeyCode::KEY_ESC),
            other if other.starts_with("type:") => type_text(&mut dev, &other[5..]),
            other => {
                eprintln!("unknown chord: {other}");
                std::process::exit(2);
            }
        }
        println!("sent {arg}");
        thread::sleep(Duration::from_millis(700));
    }

    // Keep the device alive briefly so trailing events are delivered.
    thread::sleep(Duration::from_millis(400));
}
