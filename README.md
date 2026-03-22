# Paste

macOS Paste-quality clipboard management with integrated text expansion, built natively for Linux.

## Features (planned)

- Visual filmstrip clipboard history with rich content previews
- Pinboards for organizing frequently-used clips
- Full-text search with filters
- Paste Stack for sequential copy-paste workflows
- Integrated text expander with abbreviation triggers
- Works on X11 and Wayland

## Development

### Prerequisites

```bash
# Tauri v2 system dependencies
sudo apt install libwebkit2gtk-4.1-dev libgtk-3-dev libayatana-appindicator3-dev librsvg2-dev

# Runtime dependencies
sudo apt install wl-clipboard xdotool
```

### Setup

```bash
npm install
cargo tauri dev
```

### Build

```bash
cargo tauri build
```

## Tech Stack

- **Backend:** Rust (Tauri v2)
- **Frontend:** React 19 + TypeScript + TailwindCSS v4
- **Storage:** SQLite with FTS5
- **Clipboard:** wl-paste (Wayland) / XFixes (X11)
- **Input:** evdev (global hotkeys + keystroke monitoring)

## License

MIT
