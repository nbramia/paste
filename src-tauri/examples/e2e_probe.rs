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
//!   chords: ctrl-alt-v | ctrl-v | right | left | enter | escape
//! Example: cargo run --example e2e_probe -- ctrl-alt-v right enter

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
