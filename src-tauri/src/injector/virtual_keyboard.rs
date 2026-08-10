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
