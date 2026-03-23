# Paste — Vision

## One-liner

macOS Paste-quality clipboard management with integrated text expansion, built natively for Linux.

## The Problem

Clipboard management on Linux is stuck in the early 2010s. The options are:

- **CopyQ/GPaste/Clipman:** Functional but visually basic — text-list dropdowns, no rich previews, no visual organization. CopyQ has scripting power but its UI is utilitarian.
- **cliphist/clipcat:** Wayland-native but terminal-oriented — pipe output to rofi/dmenu, no visual previews, no card-based UI.
- **Desktop-specific:** Klipper (KDE-only), GPaste (GNOME-only) — fragmented, can't follow you across desktop environments.

Meanwhile, macOS users have **Paste** — a beautiful horizontal filmstrip overlay with rich content previews, pinboards for organization, powerful search with filters, and the ingenious Paste Stack for sequential copy-paste workflows. It transforms clipboard management from a utilitarian necessity into something genuinely delightful.

Text expansion is similarly fragmented. **espanso** exists on Linux and is capable, but it's a separate tool with its own config, its own daemon, and no integration with clipboard history. macOS users can combine Paste with TextExpander for a seamless workflow. Linux users cobble together multiple tools.

There was no Linux equivalent that combined both. We built it.

## What We Built

A system tray application that:

1. **Monitors the system clipboard** on X11 and Wayland, capturing text, images, URLs, files, and rich content
2. **Stores unlimited clipboard history** in a local SQLite database with full-text search
3. **Presents a Paste-style filmstrip overlay** — a horizontal strip of rich content cards that slides up from the bottom of the screen on a global hotkey
4. **Supports pinboards** — named, color-coded collections for organizing frequently-used snippets, templates, and references
5. **Provides powerful search** — full-text, filterable by content type, source application, date range, and favorites
6. **Implements Paste Stack** — sequential copy-then-paste workflow for batch content movement
7. **Integrates text expansion** — type an abbreviation anywhere, and it expands to a full snippet with support for fill-in fields, date/time macros, clipboard content, shell scripts, and nested snippets
8. All data stays **local**, with **zero cloud dependency**

## Design Principles

### 1. Visual Delight

The UI is the product. Paste's filmstrip is beautiful not by accident — it's the reason people love it. We match that visual quality: rich card previews, smooth animations, thoughtful typography, proper light/dark mode. No utilitarian text lists.

### 2. Works Everywhere

X11 and Wayland. GNOME, KDE, Hyprland, Sway, i3. The app works without configuration changes across display servers and desktop environments. We use kernel-level input (evdev) and protocol-level clipboard access to avoid DE-specific dependencies.

### 3. Instant Response

The overlay appears in under 100ms from hotkey press. Search results filter as you type. Pasting an item happens instantly. Text expansion triggers without perceptible delay. Speed is a feature.

### 4. Local-First, Always

No cloud sync. No accounts. No telemetry. Your clipboard history and snippets never leave your machine. This is a privacy guarantee, not a limitation.

### 5. One Tool, Not Two

Clipboard management and text expansion are two sides of the same coin — managing and inserting text efficiently. They share infrastructure (storage, text injection, global shortcuts) and UX patterns (search, organization, quick access). Integrating them eliminates the friction of running separate daemons with separate configs.

### 6. Keyboard-First, Mouse-Friendly

Power users navigate entirely by keyboard. But the filmstrip is also beautiful to browse with a mouse or trackpad. Both input methods are first-class.

## User Experience

### Core Flow: Clipboard History

```
[User copies anything] -> Item captured -> Stored with metadata
[User presses Super+V] -> Filmstrip overlay slides up from bottom
[User browses/searches/selects] -> Item pasted at cursor
[Overlay dismisses automatically]
```

### The Filmstrip

A horizontal strip of content cards, newest on the left, scrolling infinitely to the right. Each card shows:

- **Rich preview**: Rendered text (with syntax highlighting for code), image thumbnails, URL cards with favicons and titles, file icons
- **Source badge**: Icon of the application it was copied from
- **Timestamp**: Relative time ("2m ago", "yesterday")
- **Type indicator**: Color-coded edge (text, image, link, file, code)

Cards are uniformly sized for visual rhythm. The strip is resizable by dragging the top edge. Keyboard navigation: arrow keys to select, Enter to paste, Space to preview full content.

### Pinboards

User-created collections with custom names and colors. Items can be dragged from history to a pinboard, or saved directly. Pinboards persist indefinitely (history may have retention limits). Use cases: email templates, code snippets, design tokens, frequently-used URLs, canned replies.

Access pinboards from the filmstrip via tabs at the top, or a dedicated keyboard shortcut.

### Search

Always-visible search bar at the top of the filmstrip. Typing filters results in real-time. Power Search (press the search shortcut again) adds structured filters:

- Content type (text, image, link, file, code)
- Source application
- Date range
- Favorites
- Pinboard

### Paste Stack

Activate Paste Stack mode (Super+Shift+V). Everything you copy enters an ordered queue. When done collecting, each Ctrl+V pastes the next item in sequence. Items can be reordered or removed before pasting. Perfect for moving content between documents, filling forms, or assembling content from multiple sources.

### Text Expander

Type a short abbreviation (e.g., `;sig`, `//date`, `,,addr`) anywhere in the system. The abbreviation is detected, deleted, and replaced with the expanded snippet.

Snippet capabilities:

- **Plain text**: Simple replacement
- **Date/time macros**: `%Y-%m-%d`, `%H:%M`, date math (`%date(+5d)`)
- **Clipboard content**: `%clipboard` inserts current clipboard
- **Cursor positioning**: `%|` places cursor after expansion
- **Fill-in fields**: Expansion triggers a form dialog for dynamic content
- **Shell scripts**: Snippet content is the output of a shell command
- **Nested snippets**: Reference other snippets for composable templates

Snippets are organized in groups and manageable from the filmstrip UI (a dedicated Snippets tab alongside History and Pinboards).

### Quick Paste

Hold the activation hotkey + press a number key (1-9) to instantly paste the Nth most recent item without opening the filmstrip.

## Target Hardware

### Primary Target

- **AMD Ryzen AI MAX+ 395** (or similar modern Linux workstation)
- 64GB RAM
- X11 or Wayland display server

### Minimum Requirements

- Any x86_64 or ARM64 Linux system
- 4GB RAM
- X11 or Wayland
- No GPU required

### Resource Budget

| Component | RAM | Notes |
|-----------|-----|-------|
| Tauri/WebView runtime | ~60-80MB | Lightweight vs Electron's ~200MB |
| SQLite + FTS5 index | ~10-50MB | Scales with history size |
| Image thumbnail cache | ~20-100MB | Configurable limit |
| Rust clipboard/expander daemon | ~5-10MB | |
| **Total** | **~100-250MB** | Varies with history size |

This is a lightweight application — no ML models, no inference engines. The resource footprint is dominated by the WebView and cached thumbnails.

## Non-Goals (For Now)

- **Cloud sync** — local-only; sync is a future consideration
- **Mobile companion** — desktop Linux only
- **Rich text editing** — we display rich text, we don't provide a rich text editor
- **OCR search** — searching text inside images is a future feature
- **Browser extension** — the global clipboard monitor captures browser copies already
- **Windows/macOS** — Linux-only; Paste already exists on macOS

## Success Metrics

- Overlay appears in < 100ms from hotkey press
- Clipboard capture latency < 50ms (copy to stored)
- Search results filter in < 50ms
- Text expansion trigger-to-insertion < 30ms
- Idle CPU usage < 0.1%
- Idle RAM < 150MB
- Works on X11 and Wayland without configuration changes
- Supports all content types: text, rich text, images, URLs, files
- History survives reboots (persistent storage)

## Roadmap

### v0.1 — Core Clipboard Manager ✅

- ✅ Clipboard monitoring (X11 + Wayland)
- ✅ SQLite storage with metadata
- ✅ Basic filmstrip overlay (Tauri window)
- ✅ Keyboard navigation and paste
- ✅ System tray with basic menu
- ✅ Global hotkey via evdev

### v0.2 — Visual Polish & Organization ✅

- ✅ Rich content previews (images, links, code)
- ✅ Pinboards (create, rename, color, drag-to-save)
- ✅ Search with real-time filtering
- ✅ Power Search filters (type, app, date, favorites)
- ✅ Light/dark mode
- ✅ Smooth animations and transitions
- ✅ Quick Paste (hotkey + number)

### v0.3 — Text Expander ✅

- ✅ Abbreviation monitoring via evdev
- ✅ Plain text expansion
- ✅ Date/time macros
- ✅ Clipboard content macro
- ✅ Cursor positioning
- ✅ Snippet management UI in filmstrip
- ✅ Import from espanso configs

### v0.4 — Advanced Expander & UX ✅

- ✅ Fill-in field dialogs
- ✅ Shell script snippets
- ✅ Nested snippets
- ✅ Snippet groups/organization
- ✅ Paste Stack mode
- ✅ Configurable retention policies

### v1.0 — Release ✅

- ✅ Comprehensive keyboard shortcuts
- ✅ Settings UI
- ✅ Proper packaging (.deb, AppImage)
- ✅ Documentation and onboarding
- ✅ Performance optimization pass
- ✅ Accessibility review

### v2.0 — Future (Partially Complete)

- ✅ Snippet sharing/import/export (implemented: JSON export/import)
- ✅ Smart deduplication (implemented: growing text detection, debounce)
- ✅ Multi-item paste (implemented: Ctrl+click multi-select, concatenated paste)
- ✅ Inline editing (implemented: Ctrl+E to edit text clips)
- ✅ Drag and drop (implemented: drag clips out, drop content in)
- ✅ Rich paste (implemented: HTML and image paste preservation)
- ✅ Clipboard persistence (implemented: survives source app close on Wayland)
- ✅ Autostart service (implemented: systemd user service)
- ✅ Structured logging (implemented: file + stderr with rotation)
- ✅ Database migrations (implemented: versioned schema with backup)
- OCR search in images — not yet implemented
- Cloud sync (optional, encrypted) — not yet implemented
- Context-aware paste suggestions — not yet implemented
- Multi-language snippet triggers — not yet implemented
