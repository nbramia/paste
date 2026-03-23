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

There was no Linux equivalent that combined both. Paste is that equivalent.

## Why One Tool, Not Two

Clipboard management and text expansion are two sides of the same coin — managing and inserting text efficiently. They share the same infrastructure: storage for content, global keyboard shortcuts, text injection into any application, search and organization patterns. Running them as separate daemons with separate configs and separate UIs is unnecessary friction.

Paste unifies them. Your clipboard history, your pinned snippets, and your text expander abbreviations all live in one place, searchable from one interface, backed by one database. Copy a URL from your browser, pin it to a "Work Links" pinboard, create a snippet that expands to it — all without leaving the filmstrip.

## What Paste Is

A system tray application that captures everything you copy, presents it in a beautiful filmstrip overlay, and lets you organize, search, and re-paste anything from your clipboard history. On top of that, it's a full text expander — type a short abbreviation anywhere and it expands to a complete snippet with dynamic macros, fill-in fields, shell command output, and nested references.

It works on X11 and Wayland, across GNOME, KDE, Hyprland, Sway, and i3, without requiring different configurations. All data stays local — no cloud, no accounts, no telemetry.

## Design Principles

### Visual Delight

The UI is the product. Paste's filmstrip is beautiful not by accident — it's the reason people love the macOS app. Rich card previews with type-specific rendering (syntax-highlighted code, link cards with domains, image thumbnails, file icons with extension badges), smooth spring-physics animations, thoughtful light and dark themes. No utilitarian text lists.

### Instant Response

The overlay appears in under 100ms. Search results filter as you type. Text expansion triggers without perceptible delay. Pasting happens instantly. Every architectural decision — SQLite with FTS5, evdev for kernel-level input, Rust for the backend — serves this principle. Speed isn't a feature; it's the foundation.

### Works Everywhere

One binary, one config, every Linux desktop. X11 and Wayland. GNOME, KDE, Hyprland, Sway, i3. The app uses kernel-level input (evdev) and protocol-level clipboard access to avoid desktop-environment-specific dependencies. Compositor-specific optimizations (Hyprland window rules, Sway IPC, X11 EWMH properties) are applied automatically when detected — but the app works without them.

### Local-First, Always

No cloud sync. No accounts. No telemetry. Audio, clipboard content, and snippets never leave the machine. This is a privacy guarantee, not a fallback mode. The SQLite database, the config file, the log file — everything lives under `~/.local/share/paste/` and `~/.config/paste/`, owned by the user, readable by the user, deletable by the user.

### Keyboard-First, Mouse-Friendly

Every action is reachable by keyboard: arrow keys to navigate, Enter to paste, Space to preview, `/` to search, Ctrl+P to pin, F to favorite, Ctrl+E to edit, Tab to switch views. But the filmstrip is also designed to be browsed with a mouse or trackpad — click to select, double-click to paste, drag to reorder, drop to capture. Both input methods are first-class.

## The Core Experience

### Copy → Browse → Paste

Everything you copy is automatically captured with metadata: content type, source application, timestamp. Press Super+V and the filmstrip slides up from the bottom of the screen — a horizontal strip of cards, newest first, each showing a rich preview of what was copied. Arrow keys to navigate, Enter to paste at the cursor. The overlay dismisses. Total time from thought to action: under two seconds.

### Organize What Matters

Not everything in your clipboard history is equally important. Pinboards let you pull clips out of the ephemeral history stream and into named, color-coded collections that persist indefinitely. Email templates in a "Work" pinboard. Code snippets in a "Dev" pinboard. Frequently-used URLs in a "Links" pinboard. Favorites (press F) provide an even lighter-weight bookmark — starred clips survive retention cleanup without needing to belong to a pinboard.

### Find Anything

The search bar is always visible. Type to filter instantly — powered by SQLite FTS5, results rank by relevance. Power Search adds structured filters: content type, source application, date range, favorites-only. Ten thousand clips? Still fast.

### Type Less, Write More

The integrated text expander watches every keystroke (via the same evdev connection as the hotkey daemon — no second input listener). Type `;sig` and it becomes your full email signature. Type `//date` and it becomes today's date. Type `;meeting` and a dialog asks you for the attendee name, the time, and the topic, then assembles the full meeting note. Shell commands, nested snippets, clipboard content insertion, cursor positioning — the macro system is complete.

## Who This Is For

Paste is for Linux users who:

- **Copy and paste frequently** — developers switching between docs and code, support agents assembling responses, writers collecting research
- **Want visual organization** — not a plain text dropdown, but a filmstrip they can scan, search, and curate
- **Use text shortcuts** — email signatures, code boilerplate, canned replies, date stamps, form-filling templates
- **Work across desktops** — Wayland today, X11 tomorrow, Hyprland at home, GNOME at work
- **Care about privacy** — no cloud clipboard sync, no data leaving the machine

## What Paste Is Not

- **Not a cloud clipboard** — there is no sync, no account, no server. Sync may come later as an opt-in encrypted feature, but local-first is the default forever.
- **Not a note-taking app** — Paste captures clipboard content and lets you organize it. It doesn't provide a rich text editor, a notebook, or a knowledge base.
- **Not cross-platform** — it's Linux-only by design. macOS already has Paste. Windows has its own ecosystem. Linux deserves a native tool that embraces its display server diversity rather than abstracting it away.
- **Not a background service you forget about** — the filmstrip is designed to be opened, browsed, and enjoyed. The text expander runs silently, but the clipboard manager is an active part of your workflow.

## Technical Identity

Paste is a **Tauri v2 application** — a Rust backend with a React/TypeScript frontend rendered in a WebView. This gives it the visual richness of a web app (CSS animations, flexible layouts, rich content rendering) with the system-level access of a native app (evdev input, Wayland protocols, SQLite storage, subprocess management). The binary is ~5-10MB. RAM usage is ~100-250MB depending on history size. Idle CPU is effectively zero.

The choice of Rust is deliberate: clipboard monitoring, keystroke processing, and text injection are latency-sensitive operations that benefit from zero-overhead abstractions and direct system call access. The choice of React is equally deliberate: the filmstrip UI — with its card previews, animations, drag-and-drop, and responsive theming — is dramatically easier to build and iterate on with web technologies than with GTK4 or Qt6.

## The Future

The current implementation covers the complete v1.0 vision and much of what was originally planned for v2.0. What remains on the horizon:

- **OCR search** — search text inside screenshots and images
- **Cloud sync** — optional, end-to-end encrypted sync between machines
- **Context-aware suggestions** — suggest clips based on the focused application
- **Multi-language snippet triggers** — abbreviations that work across keyboard layouts
- **Plugin system** — extend Paste with community-built integrations
