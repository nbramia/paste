# Troubleshooting

## "No keyboard devices found" / Permission denied

**Symptom:** Global hotkeys (Super+V) and text expander don't work. Error about `/dev/input/event*` access in logs.

**Fix:** Add your user to the `input` group:

```bash
sudo usermod -aG input $USER
```

Then **log out and back in** (or reboot). Verify:

```bash
groups | grep input
```

Note: The filmstrip can still be opened from the system tray menu without `input` group membership.

## ydotoold not running (Wayland text injection)

**Symptom:** Text expansion or paste doesn't work on Wayland (GNOME, KDE). Paste falls back to clipboard injection.

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

**Fix:** Paste automatically captures clipboard content as it changes, so the content is preserved in history even if the source app closes. If clipboard persistence is not working, ensure `wl-clipboard` is installed:

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

**On Sway:** Ensure your bar supports the StatusNotifierItem protocol.

## "xdotool not found" / "wl-paste not found"

**Fix:** Install the missing package:

```bash
# For X11
sudo apt install xdotool xclip

# For Wayland
sudo apt install wl-clipboard ydotool
```

## Injector falls back to clipboard method

**Symptom:** Log message says "Falling back to clipboard injector."

**Cause:** The configured injection method was not available at startup.

**Fix:** Install the appropriate tool for your display server:

```bash
# X11
sudo apt install xdotool

# Wayland (universal)
sudo apt install ydotool
sudo systemctl enable --now ydotoold

# Wayland (wlroots only: Sway, Hyprland)
sudo apt install wtype
```

Then set `[injection] method = "auto"` in config.toml and restart Paste.

## High CPU usage

**Possible causes:**
- Very large clipboard history with no retention limits set
- Frequent clipboard changes from polling applications

**Fix:** Run cleanup in Settings, or manually via the command:

```bash
# Check history size
sqlite3 ~/.local/share/paste/paste.db "SELECT COUNT(*) FROM clips;"
```

Set retention limits in config.toml:

```toml
[storage]
max_history_days = 90
max_history_count = 10000
```

## SQLite database corruption

**Symptom:** App crashes on startup, storage errors in logs.

**Fix:**

1. Stop Paste
2. Back up the database: `cp ~/.local/share/paste/paste.db ~/.local/share/paste/paste.db.bak`
3. Delete and let it recreate: `rm ~/.local/share/paste/paste.db`
4. Restart Paste

Note: This will clear your clipboard history. Pinboard items and snippets will be lost.

If Paste cannot open the database, it automatically falls back to in-memory storage and logs the error. Check `~/.local/share/paste/paste.log` for details.

## Checking Logs

Paste logs to both stderr and a file:

```bash
# View the log file
cat ~/.local/share/paste/paste.log

# Watch live logs
tail -f ~/.local/share/paste/paste.log

# Run with debug logging
RUST_LOG=debug paste
```

The log file is rotated at 5 MB.

## Text expander not triggering

**Possible causes:**
1. Expander is disabled — check the system tray menu or Settings
2. Not in `input` group — global keystroke monitoring requires it
3. Trigger mode mismatch — if set to `word_boundary`, you need to type a space/punctuation after the abbreviation
4. Abbreviation conflict — check if the abbreviation is too short or conflicts with common words

**Fix:** Toggle the expander via Ctrl+Alt+Space. Verify your abbreviation is registered in the Snippets tab.

## Overlay appears in wrong position

**Symptom:** The filmstrip doesn't anchor to the bottom of the screen.

**Cause:** Monitor detection may have picked the wrong monitor, or compositor-specific rules failed to apply.

**Fix:** Check the log for overlay positioning messages. On Hyprland/Sway, ensure `hyprctl` or `swaymsg` is available in PATH.

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
