# Paste

macOS Paste-quality clipboard management with integrated text expansion, built natively for Linux.

## Features

- **Visual filmstrip** — Horizontal strip of rich content cards (text, code, links, images, files) with type-specific previews
- **Pinboards** — Named, color-coded collections for organizing clips
- **Full-text search** — FTS5-powered search with Power Search filters (type, app, date range)
- **Paste Stack** — Sequential copy-then-paste workflow for batch content movement
- **Text expander** — Type abbreviations that expand to full snippets with macros (date/time, clipboard, shell commands, fill-in fields, nested snippets)
- **Quick Paste** — Super+1-9 to instantly paste recent clips without the filmstrip
- **Quick Look** — Space to preview full content of any clip
- **System tray** — Persistent tray icon with context menu
- **Light/dark mode** — Automatic system theme detection with manual override
- **Works everywhere** — X11 and Wayland, GNOME/KDE/Hyprland/Sway

## Installation

### Prerequisites

```bash
# System dependencies (Ubuntu/Debian)
sudo apt install \
  libwebkit2gtk-4.1-0 \
  libgtk-3-0 \
  libayatana-appindicator3-1 \
  wl-clipboard \
  xdotool

# For text injection on Wayland (at least one required)
sudo apt install ydotool  # Universal Wayland
# or
sudo apt install wtype    # wlroots only (Sway, Hyprland)

# Add your user to the input group (for global hotkeys)
sudo usermod -aG input $USER
# Log out and back in for this to take effect
```

### From .deb package

Download the latest `.deb` from [Releases](https://github.com/nbramia/paste/releases):

```bash
sudo dpkg -i paste_0.1.0_amd64.deb
```

### From AppImage

Download the `.AppImage` from [Releases](https://github.com/nbramia/paste/releases):

```bash
chmod +x Paste_0.1.0_amd64.AppImage
./Paste_0.1.0_amd64.AppImage
```

### From source

```bash
# Build dependencies
sudo apt install \
  libwebkit2gtk-4.1-dev \
  libgtk-3-dev \
  libayatana-appindicator3-dev \
  librsvg2-dev \
  libsoup-3.0-dev

# Clone and build
git clone https://github.com/nbramia/paste.git
cd paste
npm install
cargo tauri build
```

## Quick Start

1. **Launch Paste** — run the app or enable autostart in Settings
2. **Copy anything** — clipboard content is automatically captured
3. **Press Super+V** — the filmstrip overlay appears at the bottom
4. **Navigate** — use arrow keys to browse, Enter to paste
5. **Search** — press `/` to search your clipboard history
6. **Pin favorites** — press Ctrl+P to save clips to pinboards

## Keyboard Shortcuts

| Key | Action |
|-----|--------|
| Super+V | Toggle filmstrip |
| Super+1-9 | Quick paste Nth recent clip |
| ← → | Navigate between cards |
| Enter | Paste selected clip |
| Shift+Enter | Paste as plain text |
| Space | Quick Look preview |
| / or Ctrl+F | Focus search |
| F | Toggle favorite |
| Ctrl+P | Save to pinboard |
| Ctrl+E | Edit clip |
| Del | Remove clip |
| Tab | Cycle views |
| Esc | Close preview/search |

See [docs/keyboard-shortcuts.md](docs/keyboard-shortcuts.md) for the full reference.

## Configuration

Config file at `~/.config/paste/config.toml`. Created automatically on first run.

See [docs/configuration.md](docs/configuration.md) for the full reference.

## Text Expander

Type abbreviations anywhere to expand them into full snippets:

```
;sig  →  Best regards,
         John Smith

;date →  2026-03-22

;email → john@example.com
```

Supports macros: `%Y-%m-%d`, `%clipboard`, `%shell(command)`, `%fill(name)`, `%snippet(abbr)`.

See [docs/text-expander.md](docs/text-expander.md) for the full syntax reference.

## Tech Stack

| Component | Technology |
|-----------|-----------|
| Backend | Rust (Tauri v2) |
| Frontend | React 19 + TypeScript + TailwindCSS v4 |
| Storage | SQLite with FTS5 full-text search |
| Clipboard | wl-paste (Wayland) / XFixes (X11) |
| Input | evdev (global hotkeys + keystroke monitoring) |
| Animations | Framer Motion |
| Packaging | .deb, AppImage |

## Development

```bash
npm install
cargo tauri dev
```

### Running tests

```bash
# Frontend tests
npm test

# Rust tests (requires system dependencies)
cd src-tauri && cargo test
```

## License

MIT
