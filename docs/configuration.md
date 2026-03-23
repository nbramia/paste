# Configuration Reference

Paste uses a TOML configuration file at `~/.config/paste/config.toml`. It is created automatically with defaults on first run. All fields are optional — missing fields use defaults.

All settings are also editable through the Settings UI (gear icon in the filmstrip or system tray menu).

## Hotkeys

```toml
[hotkeys]
toggle_overlay = "Super+V"           # Open/close the filmstrip
paste_stack_mode = "Super+Shift+V"   # Toggle Paste Stack mode
quick_copy_to_pinboard = "Super+Shift+C"  # Quick save to pinboard
toggle_expander = "Ctrl+Alt+Space"   # Enable/disable text expander
```

Format: `Modifier+Key`. Modifiers: `Ctrl`, `Alt`, `Shift`, `Super` (also `Meta`, `Win`). Keys: `A-Z`, `0-9`, `F1-F12`, `Space`, `Tab`, `Escape`, etc.

## Clipboard

```toml
[clipboard]
monitor_primary = true              # Monitor PRIMARY selection (mouse select)
monitor_clipboard = true            # Monitor CLIPBOARD (Ctrl+C)
excluded_apps = ["1password", "keepassxc", "bitwarden", "lastpass"]
max_content_size_mb = 10            # Skip items larger than this
merge_growing = true                # Replace partial selections with complete ones
debounce_ms = 500                   # Ignore rapid copies within this window (ms)
```

**Merge growing:** When enabled (default), if you progressively select more text (word, then line, then paragraph), Paste replaces the earlier partial clips with the latest, more complete selection instead of storing each as a separate entry.

**Debounce:** When applications rapidly fire multiple clipboard events (e.g., some editors update the clipboard on every keystroke), the debounce window collapses these into a single capture. Default: 500ms.

## Storage

```toml
[storage]
max_history_days = 90               # Delete clips older than this (0 = unlimited)
max_history_count = 10000           # Maximum number of clips (0 = unlimited)
max_image_size_mb = 10              # Skip images larger than this
max_total_storage_mb = 500          # Total storage cap
db_path = "~/.local/share/paste/paste.db"
image_dir = "~/.local/share/paste/images"
```

Pinboard clips and favorites are exempt from retention policies.

Retention is enforced automatically on startup and every hour. You can also trigger it manually from Settings.

## UI

```toml
[ui]
theme = "system"                    # system | light | dark
filmstrip_height = 300              # Filmstrip height in pixels
cards_visible = 6                   # Number of cards visible at once
animation_speed = 1.0               # Animation speed multiplier (0 = instant)
blur_background = true              # Blur behind filmstrip (compositor-dependent)
```

**Theme values:**
- `system` — automatically follow system light/dark preference
- `light` — always use light theme
- `dark` — always use dark theme

**Animation speed:** Multiplier applied to all UI animations. Set to `0` for instant transitions. Values above `1.0` slow animations down.

**Blur background:** Semi-transparent blur effect behind the filmstrip. Requires compositor support. Falls back to a solid background if unsupported.

## Text Expander

```toml
[expander]
enabled = true                      # Enable/disable text expander
trigger = "word_boundary"           # word_boundary | immediate
typing_speed = 0                    # Delay between characters in ms (0 = instant)
```

**Trigger modes:**
- `word_boundary` — Expand only when a space or punctuation follows the abbreviation (recommended)
- `immediate` — Expand as soon as the abbreviation is fully typed

**Typing speed:** Introduces a delay between each character when injecting expanded text. Useful for applications that can't handle rapid input. Default `0` inserts all characters at once.

## Injection

```toml
[injection]
method = "auto"                     # auto | xdotool | ydotool | wtype | clipboard
```

**Methods:**
- `auto` — Detect display server and available tools automatically (recommended)
- `xdotool` — Force X11 injection
- `ydotool` — Force Wayland injection (requires ydotoold daemon)
- `wtype` — Force wlroots injection (Sway, Hyprland only)
- `clipboard` — Clipboard injection fallback (set clipboard + Ctrl+V)

If the configured method fails at startup, Paste falls back to the clipboard injector automatically.

## Full Default Configuration

For reference, this is the complete default configuration file:

```toml
[hotkeys]
toggle_overlay = "Super+V"
paste_stack_mode = "Super+Shift+V"
quick_copy_to_pinboard = "Super+Shift+C"
toggle_expander = "Ctrl+Alt+Space"

[clipboard]
monitor_primary = true
monitor_clipboard = true
excluded_apps = ["1password", "keepassxc", "bitwarden", "lastpass"]
max_content_size_mb = 10
merge_growing = true
debounce_ms = 500

[storage]
max_history_days = 90
max_history_count = 10000
max_image_size_mb = 10
max_total_storage_mb = 500
db_path = "~/.local/share/paste/paste.db"
image_dir = "~/.local/share/paste/images"

[ui]
theme = "system"
filmstrip_height = 300
cards_visible = 6
animation_speed = 1.0
blur_background = true

[expander]
enabled = true
trigger = "word_boundary"
typing_speed = 0

[injection]
method = "auto"
```
