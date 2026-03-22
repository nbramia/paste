# Paste — Architecture

## Hardware Profile

The primary development target is an AMD-based Linux workstation:

| Component | Spec |
|-----------|------|
| CPU | AMD Ryzen AI MAX+ 395 — 16 cores / 32 threads |
| ISA Extensions | AVX-512, AVX2, SSE4.2 |
| RAM | 64GB unified (shared with iGPU) |
| Display Server | Wayland (primary) + X11 (supported) |
| Desktop Environment | Any (GNOME, KDE, Hyprland, Sway, i3, etc.) |

**Key constraint:** The application must work identically on both X11 and Wayland, across all major desktop environments, without requiring different configurations.

## System Overview

```
┌─────────────────────────────────────────────────────────────────────┐
│                              Paste                                  │
│                                                                     │
│  ┌──────────────┐  ┌──────────────┐  ┌─────────────────────────┐   │
│  │  Clipboard    │→│  Storage     │→│  Filmstrip Overlay       │   │
│  │  Monitor      │  │  Engine      │  │  (Tauri + React)        │   │
│  │  (X11/Wl)    │  │  (SQLite)    │  │                         │   │
│  └──────────────┘  └──────┬───────┘  │  ├─ History View        │   │
│                           │          │  ├─ Pinboard View       │   │
│  ┌──────────────┐         │          │  ├─ Snippet View        │   │
│  │  Text         │←───────┤          │  └─ Search Bar          │   │
│  │  Expander     │        │          └────────────┬────────────┘   │
│  │  Engine       │        │                       │                 │
│  └──────┬───────┘        │                       │                 │
│         │                │                       │                 │
│  ┌──────▼───────┐  ┌─────▼────────┐  ┌──────────▼──────────┐      │
│  │  Text         │  │  Hotkey      │  │  System Tray        │      │
│  │  Injector     │  │  Daemon      │  │  (StatusNotifier)   │      │
│  │  (xdo/ydo)   │  │  (evdev)     │  │                     │      │
│  └──────────────┘  └──────────────┘  └─────────────────────┘      │
└─────────────────────────────────────────────────────────────────────┘
```

## Technology Stack

| Component | Technology | Rationale |
|-----------|-----------|-----------|
| Language (backend) | Rust | Performance, memory safety, excellent Linux ecosystem (evdev, wayland-client, x11rb) |
| Language (frontend) | TypeScript | Type safety, rich ecosystem for UI components |
| App framework | Tauri v2 | Lightweight (~5MB vs Electron's 100MB+), Rust backend, WebView frontend, built-in IPC |
| Frontend framework | React 19 + TailwindCSS v4 | Largest ecosystem, well-documented Tauri integration, Framer Motion for animations |
| Animations | Framer Motion | Production-grade animation library, spring physics, layout animations |
| Storage | SQLite via rusqlite + FTS5 | Proven, lightweight, full-text search built-in, single-file database |
| Clipboard (Wayland) | wl-clipboard (`wl-paste --watch`) | Event-driven, handles all MIME types, standard Wayland clipboard access |
| Clipboard (X11) | x11rb crate + XFixes | Event-driven via `XFixesSelectSelectionInput`, no polling needed |
| Global shortcuts | evdev crate | Kernel-level input, works on X11 + Wayland, no root needed (input group) |
| Text injection | ydotool / xdotool / wtype | Covers all display servers and compositors |
| System tray | ksni crate | StatusNotifierItem protocol, works on KDE/GNOME/Hyprland/Sway |
| Overlay positioning | gtk4-layer-shell (Wayland) / EWMH (X11) | Proper overlay/panel behavior on both display servers |
| Config | TOML | Rust-native, human-readable, well-typed |
| Packaging | Tauri bundler | Generates .deb, .AppImage, .rpm |
| Build system | Cargo (Rust) + npm/pnpm (frontend) | Standard tooling for both ecosystems |

### Why Tauri v2?

- **Lightweight:** ~5-10MB binary vs Electron's 100MB+. WebKitGTK is already installed on most Linux systems.
- **Rust backend:** Direct access to Linux system APIs (evdev, wayland-client, x11rb) without FFI overhead.
- **Rich UI:** Web technologies enable the visually rich filmstrip UI that makes Paste special — animations, gradients, rich content previews, responsive layouts.
- **IPC:** Tauri's command/event system provides type-safe communication between Rust backend and React frontend.
- **Multi-window:** Supports creating additional windows for fill-in field dialogs, settings, etc.
- **System tray:** Built-in system tray support (though we'll use ksni for better Linux integration).

### Why Not GTK4/Qt6 Native?

A native toolkit would give us lighter resource usage and more native feel. However:
- The filmstrip UI with rich card previews, smooth animations, and responsive layout is significantly easier to build and iterate on with web technologies.
- GTK4's animation system is capable but less ergonomic than Framer Motion for complex layout animations.
- The visual quality bar we're targeting (matching macOS Paste) is more achievable with CSS + a modern animation library.
- WebView overhead (~60-80MB RAM) is acceptable for a desktop application on modern hardware.

### Why Not Electron?

- 5-10x larger binary and RAM footprint than Tauri.
- No Rust backend — would need Node.js addons for system-level operations.
- Bundles Chromium, which is redundant on Linux where WebKitGTK is available.

---

## Component Architecture

### 1. Clipboard Monitor

Captures clipboard changes on both X11 and Wayland with a unified interface.

#### Wayland Implementation

```
wl-paste --watch --type text cat     → captures text clipboard changes
wl-paste --watch --type image/png cat → captures image clipboard changes
```

- Two `wl-paste --watch` subprocesses: one for text MIME types, one for image MIME types
- stdout is piped to the Rust process for parsing and storage
- Event-driven: `wl-paste` blocks until clipboard changes, then outputs the new content
- Additional MIME types (text/html, text/uri-list) are read via separate `wl-paste --type <mime>` calls when a change is detected, to capture all representations of the copied content

#### X11 Implementation

```rust
// Pseudo-code for X11 clipboard monitoring
let conn = x11rb::connect()?;
conn.xfixes_select_selection_input(root, CLIPBOARD, SELECTION_EVENT_MASK)?;

loop {
    let event = conn.wait_for_event()?;
    match event {
        XFixesSelectionNotify { .. } => {
            // Request clipboard content via TARGETS, then each desired type
            let content = read_selection(&conn, CLIPBOARD)?;
            store(content);
        }
    }
}
```

- Uses XFixes extension for event-driven notification (no polling)
- Monitors both CLIPBOARD (Ctrl+C) and PRIMARY (mouse selection) selections
- Reads all available TARGETS to capture multiple representations (text/plain, text/html, image/png, etc.)

#### Display Server Detection

```rust
fn detect_display_server() -> DisplayServer {
    if std::env::var("WAYLAND_DISPLAY").is_ok() {
        DisplayServer::Wayland
    } else if std::env::var("DISPLAY").is_ok() {
        DisplayServer::X11
    } else {
        panic!("No display server detected");
    }
}
```

Detected once at startup. All clipboard operations dispatch through a `ClipboardBackend` trait:

```rust
trait ClipboardBackend: Send + Sync {
    fn start_monitoring(&self, tx: Sender<ClipItem>) -> Result<()>;
    fn set_clipboard(&self, content: &ClipContent) -> Result<()>;
    fn get_clipboard(&self) -> Result<ClipContent>;
}
```

#### Content Type Detection

When clipboard content is captured, we determine the content type:

| Priority | MIME Type | Content Type | Notes |
|----------|-----------|-------------|-------|
| 1 | image/png, image/jpeg, image/tiff | Image | Store as file, generate thumbnail |
| 2 | text/uri-list | Link / File | Parse URIs; file:// = File, http(s):// = Link |
| 3 | text/html | Rich Text | Store HTML + extract plain text |
| 4 | text/plain | Text / Code | Heuristic: detect code via syntax patterns |
| 5 | application/* | File | Application-specific data |

Code detection heuristic for text/plain: look for patterns like `{`, `=>`, `def `, `fn `, `function `, `import `, `#include`, `class `, semicolons at end of lines, indentation patterns. If confidence > threshold, mark as Code and detect language for syntax highlighting.

#### Deduplication

Before storing, compute SHA-256 hash of the content. If the hash matches the most recent entry, skip storage (consecutive duplicate). This prevents "copy same thing twice" clutter without removing intentional re-copies after other content.

#### Application Exclusion

Certain applications should never have their clipboard content captured (password managers, etc.).

On X11: the source application's window class is available via `XGetClassHint` on the selection owner window.

On Wayland: `wl-paste` doesn't expose the source application. We can use `xdg-desktop-portal` or compositor-specific DBus APIs to query the focused application. Alternatively, maintain a list of excluded applications and check the focused window via compositor-specific protocols.

Configuration:
```toml
[clipboard]
excluded_apps = ["1password", "keepassxc", "bitwarden"]
```

---

### 2. Storage Engine

SQLite database at `~/.local/share/paste/paste.db` with the following schema:

#### Schema

```sql
-- Clipboard history
CREATE TABLE clips (
    id TEXT PRIMARY KEY,              -- UUID v7 (time-ordered)
    content_type TEXT NOT NULL,        -- 'text', 'image', 'link', 'file', 'code'
    text_content TEXT,                 -- Plain text content (searchable)
    html_content TEXT,                 -- HTML representation (if available)
    image_path TEXT,                   -- Path to stored image file (relative)
    source_app TEXT,                   -- Application name/identifier
    source_app_icon TEXT,             -- Path to app icon
    content_hash TEXT NOT NULL,        -- SHA-256 for deduplication
    content_size INTEGER NOT NULL,     -- Size in bytes
    metadata TEXT,                     -- JSON: { url, title, favicon, language, dimensions, ... }
    pinboard_id TEXT REFERENCES pinboards(id) ON DELETE SET NULL,
    is_favorite BOOLEAN DEFAULT FALSE,
    created_at TEXT NOT NULL,          -- ISO 8601 timestamp
    accessed_at TEXT,                  -- Last pasted timestamp
    access_count INTEGER DEFAULT 0
);

-- Full-text search index
CREATE VIRTUAL TABLE clips_fts USING fts5(
    text_content,
    content='clips',
    content_rowid='rowid'
);

-- Pinboards
CREATE TABLE pinboards (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    color TEXT NOT NULL,               -- Hex color code
    icon TEXT,                         -- Optional emoji or icon identifier
    position INTEGER NOT NULL,         -- Sort order
    created_at TEXT NOT NULL
);

-- Text expander snippets
CREATE TABLE snippets (
    id TEXT PRIMARY KEY,
    abbreviation TEXT NOT NULL UNIQUE,  -- Trigger string
    name TEXT NOT NULL,                 -- Human-readable name
    content TEXT NOT NULL,              -- Expansion template
    content_type TEXT NOT NULL,         -- 'plain', 'script', 'fill-in'
    group_id TEXT REFERENCES snippet_groups(id) ON DELETE SET NULL,
    description TEXT,
    use_count INTEGER DEFAULT 0,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

-- Snippet groups
CREATE TABLE snippet_groups (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    position INTEGER NOT NULL,
    created_at TEXT NOT NULL
);

-- Paste Stack (temporary, active session only)
CREATE TABLE paste_stack (
    id TEXT PRIMARY KEY,
    clip_id TEXT NOT NULL REFERENCES clips(id) ON DELETE CASCADE,
    position INTEGER NOT NULL,
    created_at TEXT NOT NULL
);
```

#### Indexes

```sql
CREATE INDEX idx_clips_created_at ON clips(created_at DESC);
CREATE INDEX idx_clips_content_type ON clips(content_type);
CREATE INDEX idx_clips_source_app ON clips(source_app);
CREATE INDEX idx_clips_pinboard_id ON clips(pinboard_id);
CREATE INDEX idx_clips_content_hash ON clips(content_hash);
CREATE INDEX idx_snippets_abbreviation ON snippets(abbreviation);
```

#### Image Storage

Images are stored as files in `~/.local/share/paste/images/`:
- Original: `{id}.{ext}` (png, jpg, etc.)
- Thumbnail: `{id}_thumb.webp` (256x256, for filmstrip preview)

Thumbnails are generated on capture using the `image` crate. Only thumbnails are loaded into the filmstrip; originals are loaded on-demand for full preview.

#### Retention Policy

```toml
[storage]
max_history_days = 90          # Delete clips older than this (0 = unlimited)
max_history_count = 10000      # Maximum number of clips (0 = unlimited)
max_image_size_mb = 10         # Skip images larger than this
max_total_storage_mb = 500     # Total storage cap including images
```

Pinboard items are exempt from retention policy (they persist indefinitely).

---

### 3. Filmstrip Overlay (Tauri + React)

The primary UI — a horizontal filmstrip that slides up from the bottom of the screen.

#### Window Behavior

**Wayland:** Use `gtk4-layer-shell` via Tauri plugin to create a layer surface anchored to the bottom edge of the screen. This provides:
- Overlay-level rendering (above normal windows, below notifications)
- No focus stealing from the active application
- Proper multi-monitor awareness (appears on the monitor with the focused window)
- Keyboard grab for navigation while open

**X11:** Use EWMH window properties:
- `_NET_WM_WINDOW_TYPE_DOCK` or `_NET_WM_WINDOW_TYPE_DIALOG`
- `_NET_WM_STATE_ABOVE` for always-on-top
- Position at bottom of active monitor
- Use `XGrabKeyboard` for keyboard capture while open

#### Window Properties

- **Width:** Full screen width (or configurable percentage)
- **Height:** Resizable by dragging top edge (default: ~300px, showing 6-8 cards)
- **Background:** Semi-transparent with blur (compositor-dependent; fallback to solid dark/light)
- **Animation:** Slide up from bottom edge (200ms ease-out) on show, slide down on dismiss

#### Filmstrip Layout

```
┌─────────────────────────────────────────────────────────────────┐
│  [Search 🔍] [History] [Pinboards ▾] [Snippets]    [⚙️]       │
├─────────────────────────────────────────────────────────────────┤
│ ┌─────────┐ ┌─────────┐ ┌─────────┐ ┌─────────┐ ┌─────────┐  │
│ │         │ │         │ │         │ │         │ │         │  │
│ │  Card 1 │ │  Card 2 │ │  Card 3 │ │  Card 4 │ │  Card 5 │  │
│ │ (newest)│ │         │ │         │ │         │ │         │  │
│ │         │ │         │ │         │ │         │ │         │  │
│ ├─────────┤ ├─────────┤ ├─────────┤ ├─────────┤ ├─────────┤  │
│ │ Chrome  │ │ VS Code │ │ Slack   │ │ Firefox │ │ Term    │  │
│ │ 2m ago  │ │ 5m ago  │ │ 12m ago │ │ 1h ago  │ │ 2h ago  │  │
│ └─────────┘ └─────────┘ └─────────┘ └─────────┘ └─────────┘  │
│                                                         ◀──── │
└─────────────────────────────────────────────────────────────────┘
```

#### Card Component

Each card renders based on content type:

| Content Type | Preview Rendering |
|-------------|-------------------|
| Text | First ~4 lines of text, monospace if code, proportional if prose |
| Code | Syntax-highlighted with detected language badge |
| Image | Thumbnail with aspect-ratio-aware scaling, dimensions in footer |
| Link | Favicon + page title + domain, styled as a link card |
| File | File icon + filename + size |
| Rich Text | Rendered HTML preview (sanitized) |

Card footer shows: source app icon + name, relative timestamp, content type color indicator.

#### Keyboard Navigation

| Key | Action |
|-----|--------|
| ← → | Navigate between cards |
| Enter | Paste selected item and dismiss |
| Shift+Enter | Paste as plain text |
| Space | Toggle full preview of selected card |
| / or Ctrl+F | Focus search bar |
| Ctrl+F (again) | Toggle Power Search filters |
| Tab | Cycle between History / Pinboards / Snippets |
| Ctrl+P | Save selected item to pinboard (shows pinboard picker) |
| Delete / Backspace | Remove selected item from history |
| Escape | Dismiss overlay |
| 1-9 (with modifier) | Quick paste Nth item |

#### Search Architecture

Frontend sends search queries to Rust backend via Tauri command. Backend executes SQLite FTS5 query:

```sql
-- Basic search
SELECT c.* FROM clips c
JOIN clips_fts f ON c.rowid = f.rowid
WHERE clips_fts MATCH ?
ORDER BY rank;

-- Power Search with filters
SELECT c.* FROM clips c
JOIN clips_fts f ON c.rowid = f.rowid
WHERE clips_fts MATCH ?
  AND (?1 IS NULL OR c.content_type = ?1)
  AND (?2 IS NULL OR c.source_app = ?2)
  AND (?3 IS NULL OR c.created_at >= ?3)
  AND (?4 IS NULL OR c.created_at <= ?4)
ORDER BY rank;
```

Search is debounced (100ms) on the frontend to avoid excessive queries during typing.

---

### 4. Text Expander Engine

Background service that monitors keystrokes and expands abbreviations.

#### Keystroke Monitoring

```rust
// Uses evdev crate — same as linux-whisper's hotkey daemon
// Reads from /dev/input/event* devices
// User must be in the 'input' group

fn monitor_keystrokes(tx: Sender<KeyEvent>) -> Result<()> {
    let devices = evdev::enumerate()
        .filter(|(_, d)| d.supported_keys().map_or(false, |k| k.contains(Key::KEY_A)));

    for (_, device) in devices {
        // Spawn a thread per keyboard device
        thread::spawn(move || {
            for event in device.fetch_events()? {
                if event.event_type() == EventType::KEY {
                    tx.send(KeyEvent::from(event))?;
                }
            }
        });
    }
}
```

#### Abbreviation Matching

Maintains a rolling character buffer (max 100 chars). On each keystroke:

1. Append character to buffer
2. Check if buffer suffix matches any abbreviation
3. If match found:
   a. Emit backspace keystrokes to delete the abbreviation (N backspaces for N-char abbreviation)
   b. Evaluate the snippet template (resolve macros, scripts, etc.)
   c. Inject the expanded text via text injector
4. Reset buffer on word boundary (space, enter, tab) or after a configurable timeout

#### Word Boundary Detection

Abbreviations only trigger at word boundaries to prevent false positives. A character is a word boundary if it's a space, tab, newline, or punctuation character. The abbreviation must be preceded by a word boundary (or be at the start of input) and followed by a trigger character (typically space, tab, enter, or punctuation).

Configurable trigger behavior:
```toml
[expander]
trigger = "word_boundary"  # word_boundary | immediate
# word_boundary: triggers when abbreviation is followed by space/punctuation
# immediate: triggers as soon as abbreviation is typed (careful with false positives)
```

#### Template Evaluation

```rust
enum TemplateToken {
    Literal(String),
    DateFormat(String),          // %Y, %m, %d, %H, %M, %S
    DateMath(DateMathExpr),      // %date(+5d), %date(-1w)
    Clipboard,                   // %clipboard
    CursorPosition,              // %|
    ShellCommand(String),        // %shell(command)
    NestedSnippet(String),       // %snippet(abbreviation)
    FillIn(FillInSpec),          // %fill(name:type:default)
}

fn evaluate_template(template: &str, ctx: &ExpansionContext) -> Result<ExpansionResult> {
    let tokens = parse_template(template)?;
    let mut output = String::new();
    let mut cursor_pos: Option<usize> = None;
    let mut fill_ins: Vec<FillInSpec> = vec![];

    for token in tokens {
        match token {
            TemplateToken::Literal(s) => output.push_str(&s),
            TemplateToken::DateFormat(fmt) => output.push_str(&chrono::Local::now().format(&fmt).to_string()),
            TemplateToken::Clipboard => output.push_str(&ctx.clipboard_content),
            TemplateToken::CursorPosition => cursor_pos = Some(output.len()),
            TemplateToken::ShellCommand(cmd) => {
                let result = Command::new("sh").arg("-c").arg(&cmd).output()?;
                output.push_str(&String::from_utf8_lossy(&result.stdout).trim());
            }
            TemplateToken::NestedSnippet(abbr) => {
                let nested = ctx.db.get_snippet_by_abbreviation(&abbr)?;
                let result = evaluate_template(&nested.content, ctx)?;
                output.push_str(&result.text);
            }
            TemplateToken::FillIn(spec) => fill_ins.push(spec),
            _ => {}
        }
    }

    Ok(ExpansionResult { text: output, cursor_pos, fill_ins })
}
```

#### Fill-in Fields

When a snippet contains fill-in fields, a Tauri dialog window appears:

```
┌─────────────────────────────────────────┐
│  Expand: Email Reply Template           │
├─────────────────────────────────────────┤
│                                         │
│  Recipient Name: [_________________]    │
│                                         │
│  Tone: [Professional ▾]                │
│         Professional                    │
│         Casual                          │
│         Formal                          │
│                                         │
│  Additional context:                    │
│  [________________________________]     │
│  [________________________________]     │
│                                         │
│            [Cancel]  [Expand]           │
└─────────────────────────────────────────┘
```

The dialog is a separate Tauri window, created on-demand and destroyed after use.

---

### 5. Global Hotkey Daemon

Uses the `evdev` crate to capture global keyboard shortcuts regardless of focused application or display server.

#### Registered Hotkeys

| Hotkey | Action | Configurable |
|--------|--------|-------------|
| Super+V | Toggle filmstrip overlay | Yes |
| Super+Shift+V | Toggle filmstrip in Paste Stack mode | Yes |
| Super+Shift+C | Quick copy to pinboard | Yes |
| Ctrl+Alt+Space | Toggle text expander on/off | Yes |

#### Implementation

Same approach as linux-whisper: read from `/dev/input/event*` devices via evdev. Detect key combinations by tracking modifier state. Emit events to the main application via channels.

The hotkey daemon and text expander keystroke monitor share the same evdev connection — a single thread reads all keyboard events and dispatches them to both the hotkey matcher and the abbreviation buffer.

```rust
fn input_daemon(hotkey_tx: Sender<HotkeyEvent>, keystroke_tx: Sender<KeyEvent>) {
    let devices = evdev::enumerate()
        .filter(|(_, d)| is_keyboard(d));

    for (_, device) in devices {
        thread::spawn(move || {
            let mut modifier_state = ModifierState::default();
            for event in device.fetch_events()? {
                if event.event_type() == EventType::KEY {
                    modifier_state.update(&event);

                    // Check hotkey combinations
                    if let Some(hotkey) = check_hotkey(&event, &modifier_state) {
                        hotkey_tx.send(hotkey)?;
                    }

                    // Forward character keystrokes to text expander
                    if let Some(ch) = event_to_char(&event, &modifier_state) {
                        keystroke_tx.send(KeyEvent { char: ch, .. })?;
                    }
                }
            }
        });
    }
}
```

---

### 6. Text Injector

Injects expanded text or clipboard content at the current cursor position.

#### Strategy Selection

```rust
fn select_injector() -> Box<dyn TextInjector> {
    let display = detect_display_server();
    match display {
        DisplayServer::Wayland => {
            if compositor_is_wlroots() {
                Box::new(WtypeInjector)       // wlroots: Sway, Hyprland
            } else {
                Box::new(YdotoolInjector)     // GNOME, KDE, others
            }
        }
        DisplayServer::X11 => Box::new(XdotoolInjector),
    }
}
```

#### Injection Methods

**xdotool (X11):**
```bash
xdotool type --clearmodifiers -- "$text"
```

**ydotool (Wayland — universal):**
```bash
ydotool type -- "$text"
```
Requires `ydotoold` daemon running with uinput access.

**wtype (Wayland — wlroots only):**
```bash
wtype -- "$text"
```
No daemon needed, but only works on wlroots compositors (Sway, Hyprland, etc.).

**Clipboard injection fallback:**
```bash
# Save current clipboard
old_clip=$(wl-paste)
# Set clipboard to new content
echo -n "$text" | wl-copy
# Simulate Ctrl+V
ydotool key ctrl+v
# Restore clipboard after delay
sleep 0.1 && echo -n "$old_clip" | wl-copy
```

#### For Paste Operations (from filmstrip)

When pasting from the filmstrip, we use clipboard injection rather than typing simulation:
1. Set the clipboard to the selected item's content (including rich formats if available)
2. Simulate Ctrl+V
3. This preserves rich text, images, and other non-text content

For text expansion, we use typing simulation (xdotool/ydotool/wtype) for better reliability with plain text.

---

### 7. System Tray

**Crate:** `ksni` for StatusNotifierItem protocol (DBus-based).

#### Tray Menu

```
📋 Paste
├── History (5,234 items)
├── ─────────────
├── Paste Stack: OFF
├── Text Expander: ON
├── ─────────────
├── Search... (Super+V)
├── Quick Paste → [submenu with last 5 items]
├── ─────────────
├── Pinboards → [submenu listing pinboards]
├── ─────────────
├── Settings
├── About
└── Quit
```

#### Tray Icon States

| State | Icon | When |
|-------|------|------|
| Normal | 📋 clipboard icon | Default idle state |
| Paste Stack Active | 📋 with stack indicator | Paste Stack mode is on |
| Expander Disabled | 📋 with X overlay | Text expander is paused |

---

### 8. Overlay Positioning

The filmstrip overlay must appear at the bottom of the screen, above other windows but below notifications.

#### Wayland: Layer Shell

The `wlr-layer-shell` protocol (and its standardized successor `ext-layer-shell-v1`) allows creating surfaces that are anchored to screen edges, similar to panels and docks.

```rust
// Via gtk4-layer-shell bindings
layer_surface.set_layer(Layer::Overlay);
layer_surface.set_anchor(Edge::Bottom | Edge::Left | Edge::Right);
layer_surface.set_size(0, filmstrip_height); // full width, configured height
layer_surface.set_keyboard_mode(KeyboardMode::OnDemand);
```

Supported compositors: Sway, Hyprland, and all wlroots-based compositors natively. Mutter (GNOME) has partial support via `ext-layer-shell-v1` as of GNOME 46+. KWin (KDE) supports `wlr-layer-shell` as of Plasma 6.

**Fallback for unsupported compositors:** Use a regular window positioned at the bottom of the screen with `_NET_WM_STATE_ABOVE` equivalent hints. Less precise but functional.

#### X11: EWMH

```rust
// Window properties for overlay behavior
set_property(window, "_NET_WM_WINDOW_TYPE", &[TYPE_DOCK]);
set_property(window, "_NET_WM_STATE", &[STATE_ABOVE, STATE_STICKY]);
set_property(window, "_NET_WM_STRUT_PARTIAL", &strut); // reserve screen space
```

Position the window at the bottom of the active monitor. Use `_NET_ACTIVE_WINDOW` to determine which monitor has the focused window.

---

## Process Architecture

```
┌──────────────────────────────────────────────────┐
│              Main Process (Rust/Tauri)            │
│                                                    │
│  ├─ Main thread (Tauri event loop)               │
│  │   └─ WebView management, IPC dispatch          │
│  │                                                │
│  ├─ Input daemon thread (evdev)                   │  ← reads keyboard events
│  │   ├─ Hotkey matcher                            │
│  │   └─ Text expander keystroke buffer            │
│  │                                                │
│  ├─ Clipboard monitor thread                      │  ← Wayland: wl-paste subprocess
│  │   └─ Content processing + storage              │     X11: XFixes event loop
│  │                                                │
│  ├─ System tray thread (ksni)                     │  ← DBus StatusNotifierItem
│  │                                                │
│  └─ Database connection pool (rusqlite)            │  ← shared across threads
│                                                    │
│  WebView (React App)                               │
│  ├─ Filmstrip component                           │
│  ├─ Search component                              │
│  ├─ Pinboard manager                              │
│  ├─ Snippet manager                               │
│  └─ Settings panel                                │
└──────────────────────────────────────────────────┘
```

### Thread Allocation

| Thread | Purpose | Notes |
|--------|---------|-------|
| Main (Tauri) | Event loop, IPC, window management | Async runtime (tokio) |
| Input daemon | evdev keyboard monitoring | Blocks on device read |
| Clipboard monitor | Clipboard change detection | Blocks on wl-paste or XFixes |
| System tray | StatusNotifierItem DBus | ksni event loop |
| DB pool | SQLite operations | Via r2d2 connection pool or direct mutex |
| **Total** | **~4-5 threads** | Lightweight |

### IPC (Tauri Commands)

Frontend communicates with Rust backend via Tauri's command system:

```rust
#[tauri::command]
async fn get_clips(
    offset: usize,
    limit: usize,
    search: Option<String>,
    content_type: Option<String>,
    source_app: Option<String>,
    pinboard_id: Option<String>,
) -> Result<Vec<Clip>, String> { ... }

#[tauri::command]
async fn paste_clip(id: String) -> Result<(), String> { ... }

#[tauri::command]
async fn create_pinboard(name: String, color: String) -> Result<Pinboard, String> { ... }

#[tauri::command]
async fn save_snippet(snippet: SnippetInput) -> Result<Snippet, String> { ... }

#[tauri::command]
async fn toggle_paste_stack() -> Result<bool, String> { ... }
```

Events from Rust to frontend (e.g., new clipboard item captured):
```rust
app.emit("clip-added", &new_clip)?;
app.emit("paste-stack-updated", &stack)?;
```

---

## Configuration

TOML config file at `~/.config/paste/config.toml`:

```toml
[hotkeys]
toggle_overlay = "Super+V"
paste_stack_mode = "Super+Shift+V"
quick_copy_to_pinboard = "Super+Shift+C"
toggle_expander = "Ctrl+Alt+Space"

[clipboard]
monitor_primary = true       # Monitor PRIMARY selection (mouse select)
monitor_clipboard = true     # Monitor CLIPBOARD (Ctrl+C)
excluded_apps = ["1password", "keepassxc", "bitwarden"]
max_content_size_mb = 10     # Skip items larger than this

[storage]
max_history_days = 90
max_history_count = 10000
max_image_size_mb = 10
max_total_storage_mb = 500
db_path = "~/.local/share/paste/paste.db"
image_dir = "~/.local/share/paste/images"

[ui]
theme = "system"             # system | light | dark
filmstrip_height = 300       # pixels
cards_visible = 6            # number of cards visible at once
animation_speed = 1.0        # multiplier (0 = instant, 2 = slow)
blur_background = true       # compositor-dependent

[expander]
enabled = true
trigger = "word_boundary"    # word_boundary | immediate
typing_speed = 0             # ms between characters (0 = instant)

[injection]
method = "auto"              # auto | xdotool | ydotool | wtype | clipboard
```

---

## Security Considerations

- **Keyboard monitoring:** The evdev-based input daemon has access to all keyboard events (same as a keylogger). This is mitigated by: (a) the user explicitly adding themselves to the `input` group, (b) all data staying local, (c) no network access. Document this tradeoff clearly.
- **Clipboard content:** May contain sensitive data (passwords, tokens). Excluded apps list prevents capturing from known password managers. An auto-clear option deletes clipboard items after configurable time.
- **Shell script snippets:** Execute arbitrary commands. Warn users when creating shell snippets. Never execute snippets from untrusted sources.
- **Image storage:** Screenshots may contain sensitive information. Respect retention policies and provide easy deletion.
- **No telemetry.** No analytics. No network calls. Ever.

---

## Testing Strategy

### Unit Tests (Rust — `cargo test`)

- Clipboard content type detection
- SQLite schema migrations
- FTS5 search query building
- Template parsing and evaluation (date macros, nested snippets, etc.)
- Abbreviation matching algorithm
- Deduplication logic
- Retention policy enforcement
- Configuration parsing and validation

### Integration Tests (Rust)

- Clipboard monitor → storage → retrieval pipeline
- Text expander abbreviation detection → template evaluation → injection
- Paste Stack lifecycle (activate, collect, paste sequence, deactivate)
- Database migrations across versions

### Frontend Tests (React — Vitest + React Testing Library)

- Filmstrip rendering with mock data
- Search filtering behavior
- Keyboard navigation
- Pinboard management UI
- Snippet editor UI
- Card component rendering for each content type

### E2E Tests (Playwright or similar)

- Full flow: copy content → open filmstrip → search → paste
- Text expansion in a test application
- Pinboard drag-and-drop
- Fill-in field dialog

### Manual Testing Matrix

- Display servers: X11, Wayland (GNOME, KDE, Hyprland, Sway)
- Applications: Firefox, Chrome, VS Code, terminal (kitty, alacritty), Slack, LibreOffice
- Content types: plain text, rich text, images, URLs, files, code
- Edge cases: very large content, rapid consecutive copies, clipboard from closing app

---

## Dependencies

### Rust (Cargo.toml)

```toml
[dependencies]
tauri = { version = "2", features = ["tray-icon", "shell"] }
rusqlite = { version = "0.32", features = ["bundled", "fts5"] }
evdev = "0.13"
ksni = "0.2"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tokio = { version = "1", features = ["full"] }
uuid = { version = "1", features = ["v7"] }
chrono = "0.4"
image = "0.25"              # Thumbnail generation
sha2 = "0.10"               # Content hashing
toml = "0.8"                # Configuration
dirs = "5"                  # XDG directory paths
log = "0.4"
env_logger = "0.11"
thiserror = "2"

[target.'cfg(target_os = "linux")'.dependencies]
x11rb = { version = "0.13", features = ["xfixes"] }  # X11 clipboard
```

### Frontend (package.json)

```json
{
  "dependencies": {
    "@tauri-apps/api": "^2",
    "react": "^19",
    "react-dom": "^19",
    "framer-motion": "^11",
    "tailwindcss": "^4"
  },
  "devDependencies": {
    "typescript": "^5.5",
    "@tauri-apps/cli": "^2",
    "vite": "^6",
    "vitest": "^2",
    "@testing-library/react": "^16"
  }
}
```

### System Packages

```bash
# Text injection (at least one required)
sudo apt install xdotool ydotool wtype wl-clipboard

# WebView (usually pre-installed on modern Linux)
sudo apt install libwebkit2gtk-4.1-0

# Tauri build dependencies
sudo apt install libgtk-3-dev libayatana-appindicator3-dev librsvg2-dev
```
