# Troubleshooting

## "No keyboard devices found" / Permission denied

**Symptom:** Hotkeys don't work, error about `/dev/input/event*` access.

**Fix:** Add your user to the `input` group:

```bash
sudo usermod -aG input $USER
```

Then **log out and back in** (or reboot). Verify:

```bash
groups | grep input
```

## ydotoold not running (Wayland text injection)

**Symptom:** Text expansion or paste doesn't work on Wayland (GNOME, KDE).

**Fix:** Start the ydotool daemon:

```bash
# Start once
sudo ydotoold &

# Or enable as a service
sudo systemctl enable --now ydotoold
```

Your user must be in the `input` group for ydotool.

## wtype not working on GNOME/KDE

**Symptom:** `wtype` fails on non-wlroots compositors.

**Cause:** `wtype` only works on wlroots-based compositors (Sway, Hyprland). GNOME (Mutter) and KDE (KWin) don't support `wlr-virtual-keyboard-v1`.

**Fix:** Use `ydotool` instead. Set in config:

```toml
[injection]
method = "ydotool"
```

Or set to `"auto"` (default) — Paste will detect the best available method.

## Clipboard content lost when app closes (Wayland)

**Symptom:** Copied text disappears when the source application is closed.

**Cause:** On Wayland, the clipboard is owned by the source application. When it exits, the clipboard is cleared.

**Fix:** Paste automatically re-asserts clipboard content when this happens. If it's not working, ensure `wl-clipboard` is installed:

```bash
sudo apt install wl-clipboard
```

## System tray icon not visible

**Symptom:** No tray icon appears.

**On GNOME:** Install the AppIndicator extension:

```bash
# Ubuntu
sudo apt install gnome-shell-extension-appindicator
```

Then enable it in GNOME Extensions.

**On Hyprland:** Ensure `waybar` is configured with a tray module.

## "xdotool not found" / "wl-paste not found"

**Fix:** Install the missing package:

```bash
# For X11
sudo apt install xdotool xclip

# For Wayland
sudo apt install wl-clipboard ydotool
```

## High CPU usage

**Possible causes:**
- Clipboard polling interval too aggressive (should be 500ms)
- Retention not running (very large history)

**Fix:** Run cleanup in Settings → Storage & Retention → "Run Cleanup Now".

## SQLite database corruption

**Symptom:** App crashes on startup, storage errors in logs.

**Fix:**

1. Stop Paste
2. Back up the database: `cp ~/.local/share/paste/paste.db ~/.local/share/paste/paste.db.bak`
3. Delete and let it recreate: `rm ~/.local/share/paste/paste.db`
4. Restart Paste

Note: This will clear your clipboard history. Pinboard items and snippets will be lost.

## Build from source fails

**Common missing packages:**

```bash
# Ubuntu/Debian
sudo apt install \
  libwebkit2gtk-4.1-dev \
  libgtk-3-dev \
  libayatana-appindicator3-dev \
  librsvg2-dev \
  libsoup-3.0-dev \
  libjavascriptcoregtk-4.1-dev

# Also need
sudo apt install build-essential curl wget
```

Ensure Rust is installed via [rustup](https://rustup.rs/) and Node.js 22+ is installed.
