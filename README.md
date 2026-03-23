# Paste

macOS Paste-quality clipboard management with integrated text expansion, built natively for Linux.

[Features](#features) · [Installation](#installation) · [Quick Start](#quick-start) · [User Guide](docs/user-guide.md) · [Configuration](docs/configuration.md)

## Why Paste?

Linux clipboard management is stuck in the early 2010s. CopyQ, GPaste, and Clipman are functional but visually basic. Meanwhile, macOS users enjoy **Paste** — a beautiful filmstrip of clipboard history — and **TextExpander** for text shortcuts. Linux has nothing that combines both.

Paste brings that experience to Linux: a visual filmstrip overlay, pinboards for organizing clips, powerful search, a full text expander, and it works on both X11 and Wayland.

## Features

### Clipboard Manager
- **Visual filmstrip** — Horizontal strip of rich content cards with type-specific previews (text, code, links, images, files)
- **Full-text search** — FTS5-powered instant search with Power Search filters (type, app, date, favorites)
- **Pinboards** — Named, color-coded collections for organizing frequently-used clips
- **Quick Paste** — Super+1-9 to paste recent clips without opening the filmstrip
- **Quick Look** — Space to preview full content of any clip
- **Paste Stack** — Sequential copy-then-paste for batch workflows
- **Multi-select** — Ctrl+click to select multiple clips, paste them concatenated
- **Favorites** — Star clips for quick access (exempt from retention)
- **Inline editing** — Ctrl+E to edit text clips before pasting
- **Drag & drop** — Drag clips out to other apps, drop content in
- **Rich paste** — Images paste as images, HTML preserves formatting
- **Clipboard persistence** — Content survives source app closing on Wayland
- **Smart dedup** — Growing text detection and rapid copy debounce

### Text Expander
- **Abbreviation triggers** — Type `;sig` + space to insert your full signature
- **Date/time macros** — `%Y-%m-%d`, `%H:%M`, `%date(+5d)` for relative dates
- **Dynamic content** — `%clipboard`, `%shell(command)`, `%snippet(abbr)` for nesting
- **Fill-in fields** — `%fill(name)` triggers a dialog for dynamic content
- **Snippet management** — Create, edit, organize in groups from the UI
- **Import/export** — Import from espanso, export/import JSON for backup

### Desktop Integration
- **System tray** — Persistent icon with context menu
- **Light/dark mode** — Follows system theme with manual override
- **Autostart** — systemd user service for login startup
- **Accessibility** — ARIA attributes, keyboard navigation, reduced motion support
- **Works everywhere** — X11 and Wayland (Hyprland, Sway, GNOME, KDE)

## Installation

### Prerequisites

```bash
# Runtime dependencies (Ubuntu/Debian)
sudo apt install libwebkit2gtk-4.1-0 libgtk-3-0 libayatana-appindicator3-1
sudo apt install xclip xdotool

# For Wayland text injection (at least one)
sudo apt install ydotool  # universal
sudo apt install wtype    # Sway/Hyprland only

# Global hotkeys require input group membership
sudo usermod -aG input $USER
# Log out and back in
```

### From .deb

```bash
sudo dpkg -i paste_0.1.0_amd64.deb
```

### From AppImage

```bash
chmod +x Paste_0.1.0_amd64.AppImage
./Paste_0.1.0_amd64.AppImage
```

### From Source

```bash
# Build dependencies
sudo apt install libwebkit2gtk-4.1-dev libgtk-3-dev \
  libayatana-appindicator3-dev librsvg2-dev libsoup-3.0-dev

git clone https://github.com/nbramia/paste.git
cd paste
npm install
npm install -D @tailwindcss/vite    # Required for Tailwind v4
npx tauri build
```

## Quick Start

1. **Launch Paste** — run the binary or enable autostart in Settings
2. **Copy anything** — text, code, URLs are captured automatically
3. **Super+Alt+V** — open the filmstrip (Cmd+Option+V on Mac keyboard with Toshy)
4. **Arrow keys + Enter** — navigate and paste
5. **Double-click** a card to copy its content to clipboard
6. **Right-click** a card for context menu: copy, save to pinboard, toggle favorite, delete
7. **Mouse wheel** scrolls the filmstrip horizontally
8. **/** — search your history
9. **;sig + space** — expand text snippets (create them in the Snippets tab)

## Keyboard Shortcuts

| Key | Action |
|-----|--------|
| Super+Alt+V | Toggle filmstrip |
| Super+1-9 | Quick paste Nth clip |
| Left / Right | Navigate cards |
| Enter | Paste |
| Shift+Enter | Paste plain text |
| Double-click | Copy to clipboard |
| Right-click | Context menu (copy, pin, favorite, delete) |
| Mouse wheel | Scroll filmstrip horizontally |
| Space | Quick Look preview |
| / or Ctrl+F | Search |
| F | Toggle favorite |
| Ctrl+P | Save to pinboard |
| Ctrl+E | Edit clip |
| Del | Remove clip |
| Tab | Cycle views |
| Esc | Close |

Full reference: [docs/keyboard-shortcuts.md](docs/keyboard-shortcuts.md)

## Configuration

Config file: `~/.config/paste/config.toml` (auto-created on first run). All options are editable via the Settings UI.

Full reference: [docs/configuration.md](docs/configuration.md)

## Documentation

- **[User Guide](docs/user-guide.md)** — Complete feature walkthrough
- **[Configuration](docs/configuration.md)** — All config options
- **[Text Expander](docs/text-expander.md)** — Snippet syntax reference
- **[Keyboard Shortcuts](docs/keyboard-shortcuts.md)** — Full cheat sheet
- **[Troubleshooting](docs/troubleshooting.md)** — Common issues & fixes

## Tech Stack

| Component | Technology |
|-----------|-----------|
| Backend | Rust (Tauri v2) |
| Frontend | React 19 + TypeScript + TailwindCSS v4 |
| Storage | SQLite with FTS5 full-text search |
| Clipboard | xclip polling via XWayland (Wayland) / XFixes (X11) |
| Input | evdev (global hotkeys + text expander) |
| Injection | xdotool / ydotool / wtype |
| Animations | Framer Motion |
| Packaging | .deb, AppImage |
| CI/CD | GitHub Actions |

## Development

```bash
npm install
npm install -D @tailwindcss/vite    # Required — Tailwind v4 won't load without this
npx tauri dev                       # dev mode with hot reload
npm test                            # frontend tests (Vitest)
cd src-tauri && cargo test          # Rust tests
```

**Note:** The window starts as a normal decorated window during development (overlay positioning is disabled). The UI uses a warm gray + amber/gold palette with IBM Plex Sans headers and Public Sans body text.

## License

MIT
