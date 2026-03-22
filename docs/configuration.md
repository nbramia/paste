# Configuration Reference

Paste uses a TOML configuration file at `~/.config/paste/config.toml`. It is created automatically with defaults on first run. All fields are optional — missing fields use defaults.

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
```

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

## UI

```toml
[ui]
theme = "system"                    # system | light | dark
filmstrip_height = 300              # Filmstrip height in pixels
cards_visible = 6                   # Number of cards visible at once
animation_speed = 1.0               # Animation speed multiplier (0 = instant)
blur_background = true              # Blur behind filmstrip (compositor-dependent)
```

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
