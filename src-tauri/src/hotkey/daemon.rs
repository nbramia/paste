use std::sync::mpsc;
use std::thread;

use evdev::{Device, EventSummary, EventType, KeyCode};
use log::{debug, info, warn};

use super::keys::{KeyCombo, Modifiers, parse_hotkey};

/// Actions that can be triggered by hotkeys.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HotkeyAction {
    ToggleOverlay,
    PasteStackMode,
    QuickCopyToPinboard,
    ToggleExpander,
    QuickPaste(u8),  // 1-9: paste Nth most recent clip
}

impl std::fmt::Display for HotkeyAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HotkeyAction::ToggleOverlay => write!(f, "ToggleOverlay"),
            HotkeyAction::PasteStackMode => write!(f, "PasteStackMode"),
            HotkeyAction::QuickCopyToPinboard => write!(f, "QuickCopyToPinboard"),
            HotkeyAction::ToggleExpander => write!(f, "ToggleExpander"),
            HotkeyAction::QuickPaste(n) => write!(f, "QuickPaste({})", n),
        }
    }
}

/// An event emitted when a hotkey is triggered.
#[derive(Debug, Clone)]
pub struct HotkeyEvent {
    pub action: HotkeyAction,
}

/// A keystroke event forwarded to the text expander.
#[derive(Debug, Clone)]
pub struct KeystrokeEvent {
    pub key: KeyCode,
    pub pressed: bool,
}

/// A registered hotkey binding: combo -> action.
#[derive(Debug, Clone)]
struct HotkeyBinding {
    combo: KeyCombo,
    action: HotkeyAction,
}

/// The global hotkey daemon. Monitors all keyboard devices via evdev.
#[derive(Debug)]
pub struct HotkeyDaemon {
    bindings: Vec<HotkeyBinding>,
}

/// Error type for hotkey daemon operations.
#[derive(Debug, thiserror::Error)]
pub enum HotkeyError {
    #[error("Failed to parse hotkey '{hotkey}': {source}")]
    Parse {
        hotkey: String,
        source: super::keys::HotkeyParseError,
    },
    #[error("No keyboard devices found. Is the user in the 'input' group? Run: sudo usermod -aG input $USER")]
    NoDevices,
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

impl HotkeyDaemon {
    /// Create a new daemon from config hotkey strings.
    ///
    /// Takes the four hotkey config strings and parses them into bindings.
    pub fn new(
        toggle_overlay: &str,
        paste_stack_mode: &str,
        quick_copy_to_pinboard: &str,
        toggle_expander: &str,
    ) -> Result<Self, HotkeyError> {
        let bindings = vec![
            HotkeyBinding {
                combo: parse_hotkey(toggle_overlay).map_err(|e| HotkeyError::Parse {
                    hotkey: toggle_overlay.into(),
                    source: e,
                })?,
                action: HotkeyAction::ToggleOverlay,
            },
            HotkeyBinding {
                combo: parse_hotkey(paste_stack_mode).map_err(|e| HotkeyError::Parse {
                    hotkey: paste_stack_mode.into(),
                    source: e,
                })?,
                action: HotkeyAction::PasteStackMode,
            },
            HotkeyBinding {
                combo: parse_hotkey(quick_copy_to_pinboard).map_err(|e| HotkeyError::Parse {
                    hotkey: quick_copy_to_pinboard.into(),
                    source: e,
                })?,
                action: HotkeyAction::QuickCopyToPinboard,
            },
            HotkeyBinding {
                combo: parse_hotkey(toggle_expander).map_err(|e| HotkeyError::Parse {
                    hotkey: toggle_expander.into(),
                    source: e,
                })?,
                action: HotkeyAction::ToggleExpander,
            },
        ];

        info!(
            "Registered {} hotkey bindings: {}",
            bindings.len(),
            bindings
                .iter()
                .map(|b| format!("{} -> {}", b.combo, b.action))
                .collect::<Vec<_>>()
                .join(", ")
        );

        Ok(Self { bindings })
    }

    /// Start the daemon. Spawns a thread per keyboard device.
    ///
    /// - `hotkey_tx`: channel for hotkey events
    /// - `keystroke_tx`: optional channel for all keystrokes (for text expander)
    ///
    /// Returns immediately. Threads run in the background.
    pub fn start(
        &self,
        hotkey_tx: mpsc::Sender<HotkeyEvent>,
        keystroke_tx: Option<mpsc::Sender<KeystrokeEvent>>,
    ) -> Result<(), HotkeyError> {
        let devices = find_keyboard_devices()?;

        if devices.is_empty() {
            return Err(HotkeyError::NoDevices);
        }

        info!("Found {} keyboard device(s)", devices.len());

        for device in devices {
            let name = device.name().unwrap_or("unknown").to_string();
            info!("Monitoring keyboard: {}", name);

            let bindings = self.bindings.clone();
            let htx = hotkey_tx.clone();
            let ktx = keystroke_tx.clone();

            thread::Builder::new()
                .name(format!(
                    "hotkey-{}",
                    name.chars().take(20).collect::<String>()
                ))
                .spawn(move || {
                    monitor_device(device, &bindings, htx, ktx);
                })
                .map_err(HotkeyError::Io)?;
        }

        Ok(())
    }
}

/// Find all keyboard devices that support key events.
fn find_keyboard_devices() -> Result<Vec<Device>, HotkeyError> {
    let mut keyboards = Vec::new();

    let devices = evdev::enumerate().collect::<Vec<_>>();

    if devices.is_empty() {
        // Likely a permissions issue
        return Err(HotkeyError::NoDevices);
    }

    for (_path, device) in devices {
        // Check if this device has key event support and looks like a keyboard
        if let Some(supported_keys) = device.supported_keys() {
            // A keyboard should support common letter keys
            if supported_keys.contains(KeyCode::KEY_A)
                && supported_keys.contains(KeyCode::KEY_Z)
                && supported_keys.contains(KeyCode::KEY_SPACE)
            {
                keyboards.push(device);
            }
        }
    }

    Ok(keyboards)
}

/// Monitor a single keyboard device for hotkey events.
fn monitor_device(
    mut device: Device,
    bindings: &[HotkeyBinding],
    hotkey_tx: mpsc::Sender<HotkeyEvent>,
    keystroke_tx: Option<mpsc::Sender<KeystrokeEvent>>,
) {
    let name = device.name().unwrap_or("unknown").to_string();
    let mut modifiers = Modifiers::default();

    loop {
        match device.fetch_events() {
            Ok(events) => {
                for event in events {
                    if event.event_type() != EventType::KEY {
                        continue;
                    }

                    // Destructure the event to get the key code
                    let EventSummary::Key(_key_event, key, value) = event.destructure() else {
                        continue;
                    };

                    // value: 0 = release, 1 = press, 2 = repeat
                    let pressed = value == 1;
                    let released = value == 0;

                    // Update modifier state
                    match key {
                        KeyCode::KEY_LEFTCTRL | KeyCode::KEY_RIGHTCTRL => {
                            modifiers.ctrl = pressed || (!released && modifiers.ctrl);
                        }
                        KeyCode::KEY_LEFTSHIFT | KeyCode::KEY_RIGHTSHIFT => {
                            modifiers.shift = pressed || (!released && modifiers.shift);
                        }
                        KeyCode::KEY_LEFTALT | KeyCode::KEY_RIGHTALT => {
                            modifiers.alt = pressed || (!released && modifiers.alt);
                        }
                        KeyCode::KEY_LEFTMETA | KeyCode::KEY_RIGHTMETA => {
                            modifiers.super_key = pressed || (!released && modifiers.super_key);
                        }
                        _ => {}
                    }

                    // Check hotkey bindings on key press (not repeat or release)
                    if pressed {
                        log::trace!(
                            "[{}] key={:?} modifiers=[{}{}{}{}]",
                            name,
                            key,
                            if modifiers.ctrl { "C" } else { "" },
                            if modifiers.alt { "A" } else { "" },
                            if modifiers.shift { "S" } else { "" },
                            if modifiers.super_key { "M" } else { "" },
                        );
                        for binding in bindings {
                            if binding.combo.key == key
                                && binding.combo.modifiers.matches(&modifiers)
                            {
                                debug!(
                                    "Hotkey triggered: {} -> {}",
                                    binding.combo, binding.action
                                );
                                if hotkey_tx
                                    .send(HotkeyEvent {
                                        action: binding.action,
                                    })
                                    .is_err()
                                {
                                    info!(
                                        "Hotkey channel closed, stopping device monitor for {}",
                                        name
                                    );
                                    return;
                                }
                            }
                        }
                    }

                    // Check for Quick Paste (Super + number key 1-9)
                    if pressed && modifiers.super_key {
                        let quick_paste_n = match key {
                            KeyCode::KEY_1 => Some(1u8),
                            KeyCode::KEY_2 => Some(2),
                            KeyCode::KEY_3 => Some(3),
                            KeyCode::KEY_4 => Some(4),
                            KeyCode::KEY_5 => Some(5),
                            KeyCode::KEY_6 => Some(6),
                            KeyCode::KEY_7 => Some(7),
                            KeyCode::KEY_8 => Some(8),
                            KeyCode::KEY_9 => Some(9),
                            _ => None,
                        };
                        if let Some(n) = quick_paste_n {
                            debug!("Quick Paste triggered: Super+{}", n);
                            if hotkey_tx
                                .send(HotkeyEvent {
                                    action: HotkeyAction::QuickPaste(n),
                                })
                                .is_err()
                            {
                                info!(
                                    "Hotkey channel closed, stopping device monitor for {}",
                                    name
                                );
                                return;
                            }
                        }
                    }

                    // Forward keystroke to text expander channel
                    if let Some(ref ktx) = keystroke_tx {
                        if (pressed || released)
                            && ktx.send(KeystrokeEvent { key, pressed }).is_err()
                        {
                            // Keystroke channel closed -- text expander might not be running.
                            // This is not fatal, just stop forwarding.
                            debug!("Keystroke channel closed");
                        }
                    }
                }
            }
            Err(e) => {
                // Device disconnected or error
                warn!(
                    "Device '{}' error: {}. Stopping monitor for this device.",
                    name, e
                );
                return;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hotkey_daemon_new_valid() {
        let daemon = HotkeyDaemon::new(
            "Super+V",
            "Super+Shift+V",
            "Super+Shift+C",
            "Ctrl+Alt+Space",
        );
        assert!(daemon.is_ok());
        let daemon = daemon.unwrap();
        assert_eq!(daemon.bindings.len(), 4);
    }

    #[test]
    fn test_hotkey_daemon_new_invalid() {
        let result = HotkeyDaemon::new(
            "Super+V",
            "InvalidKey!!!",
            "Super+Shift+C",
            "Ctrl+Alt+Space",
        );
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("InvalidKey"));
    }

    #[test]
    fn test_hotkey_action_display() {
        assert_eq!(HotkeyAction::ToggleOverlay.to_string(), "ToggleOverlay");
        assert_eq!(HotkeyAction::PasteStackMode.to_string(), "PasteStackMode");
        assert_eq!(
            HotkeyAction::QuickCopyToPinboard.to_string(),
            "QuickCopyToPinboard"
        );
        assert_eq!(HotkeyAction::ToggleExpander.to_string(), "ToggleExpander");
        assert_eq!(HotkeyAction::QuickPaste(3).to_string(), "QuickPaste(3)");
    }

    #[test]
    fn test_bindings_match_correct_actions() {
        let daemon = HotkeyDaemon::new(
            "Super+V",
            "Super+Shift+V",
            "Super+Shift+C",
            "Ctrl+Alt+Space",
        )
        .unwrap();

        // Verify each binding maps to the right action
        assert_eq!(daemon.bindings[0].action, HotkeyAction::ToggleOverlay);
        assert_eq!(daemon.bindings[0].combo.key, KeyCode::KEY_V);
        assert!(daemon.bindings[0].combo.modifiers.super_key);

        assert_eq!(daemon.bindings[1].action, HotkeyAction::PasteStackMode);
        assert_eq!(daemon.bindings[1].combo.key, KeyCode::KEY_V);
        assert!(daemon.bindings[1].combo.modifiers.super_key);
        assert!(daemon.bindings[1].combo.modifiers.shift);

        assert_eq!(
            daemon.bindings[2].action,
            HotkeyAction::QuickCopyToPinboard
        );
        assert_eq!(daemon.bindings[2].combo.key, KeyCode::KEY_C);

        assert_eq!(daemon.bindings[3].action, HotkeyAction::ToggleExpander);
        assert_eq!(daemon.bindings[3].combo.key, KeyCode::KEY_SPACE);
        assert!(daemon.bindings[3].combo.modifiers.ctrl);
        assert!(daemon.bindings[3].combo.modifiers.alt);
    }

    #[test]
    fn test_hotkey_event_clone() {
        let event = HotkeyEvent {
            action: HotkeyAction::ToggleOverlay,
        };
        let cloned = event.clone();
        assert_eq!(cloned.action, HotkeyAction::ToggleOverlay);
    }

    #[test]
    fn test_keystroke_event_clone() {
        let event = KeystrokeEvent {
            key: KeyCode::KEY_A,
            pressed: true,
        };
        let cloned = event.clone();
        assert_eq!(cloned.key, KeyCode::KEY_A);
        assert!(cloned.pressed);
    }
}
