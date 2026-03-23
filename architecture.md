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
│  │  Text         │←───────┤          │  ├─ Search Bar          │   │
│  │  Expander     │        │          │  └─ Settings Panel      │   │
│  │  Engine       │        │          └────────────┬────────────┘   │
│  └──────┬───────┘        │                       │                 │
│         │                │                       │                 │
│  ┌──────▼───────┐  ┌─────▼────────┐  ┌──────────▼──────────┐      │
│  │  Text         │  │  Hotkey      │  │  System Tray        │      │
│  │  Injector     │  │  Daemon      │  │  (Tauri tray-icon)  │      │
│  │  (xdo/ydo)   │  │  (evdev)     │  │                     │      │
│  └──────────────┘  └──────────────┘  └─────────────────────┘      │
│                                                                     │
│  ┌──────────────┐  ┌──────────────┐  ┌─────────────────────┐      │
│  │  Overlay      │  │  Logging     │  │  Service            │      │
│  │  Positioning  │  │  (file+      │  │  (systemd           │      │
│  │  Module       │  │   stderr)    │  │   autostart)        │      │
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
| Storage | SQLite via rusqlite 0.39 + FTS5 | Proven, lightweight, full-text search built-in, single-file database |
| Clipboard (Wayland) | wl-clipboard (`wl-paste --watch`) | Event-driven, handles all MIME types, standard Wayland clipboard access |
| Clipboard (X11) | x11rb crate + XFixes | Event-driven via `XFixesSelectSelectionInput`, no polling needed |
| Global shortcuts | evdev crate | Kernel-level input, works on X11 + Wayland, no root needed (input group) |
| Text injection | ydotool / xdotool / wtype | Covers all display servers and compositors |
| System tray | Tauri built-in tray-icon | Uses Tauri v2's native tray icon support with menu builder API |
| Overlay positioning | Tauri window API + compositor IPC | Positions window at bottom edge; applies Hyprland/Sway/X11 rules via CLI tools |
| Config | TOML | Rust-native, human-readable, well-typed |
| Packaging | Tauri bundler | Generates .deb, .AppImage |
| Build system | Cargo (Rust) + npm (frontend) | Standard tooling for both ecosystems |

### Why Tauri v2?

- **Lightweight:** ~5-10MB binary vs Electron's 100MB+. WebKitGTK is already installed on most Linux systems.
- **Rust backend:** Direct access to Linux system APIs (evdev, wayland-client, x11rb) without FFI overhead.
- **Rich UI:** Web technologies enable the visually rich filmstrip UI that makes Paste special — animations, gradients, rich content previews, responsive layouts.
- **IPC:** Tauri's command/event system provides type-safe communication between Rust backend and React frontend.
- **Multi-window:** Supports creating additional windows for fill-in field dialogs, settings, etc.
- **System tray:** Built-in tray icon support via the `tray-icon` feature, eliminating the need for a separate crate.

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
wl-paste --watch --type text cat     -> captures text clipboard changes
wl-paste --watch --type image/png cat -> captures image clipboard changes
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

The deduplication module (`clipboard/dedup.rs`) implements two strategies:

1. **Hash-based dedup** — SHA-256 hash of the content. If the hash matches the most recent entry, the duplicate is skipped.
2. **Growing text detection** — When `merge_growing` is enabled (default), if new content is a superset of the most recent clip (e.g., the user selected a word, then extended the selection to a paragraph), the older partial clip is replaced instead of creating a new entry.
3. **Debounce** — Rapid consecutive copies within the debounce window (default 500ms) are collapsed.

#### Application Exclusion

Certain applications should never have their clipboard content captured (password managers, etc.).

On X11: the source application's window class is available via `XGetClassHint` on the selection owner window.

On Wayland: the focused application is queried via compositor-specific methods.

Configuration:
```toml
[clipboard]
excluded_apps = ["1password", "keepassxc", "bitwarden", "lastpass"]
```

Exclusion list is managed at runtime via Tauri commands (`get_excluded_apps`, `add_excluded_app`, `remove_excluded_app`).

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

-- FTS5 sync triggers (auto-update index on insert/delete/update)

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

-- Schema version tracking
CREATE TABLE schema_version (
    version INTEGER NOT NULL,
    applied_at TEXT NOT NULL
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

#### Migration System

The storage module (`storage/migrations.rs`) implements a versioned migration system:

- Migrations are sequential SQL scripts defined in code
- The `schema_version` table tracks which migrations have been applied
- On startup, `run_migrations()` checks the current version and runs any pending migrations
- Each migration runs in a transaction and is rolled back on failure
- Before migrating, the database file is backed up (e.g., `paste.v1.bak`)
- The system is idempotent — running migrations on an up-to-date database is a no-op

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

Pinboard items and favorites are exempt from retention policy (they persist indefinitely).

Retention is enforced on startup (after a 5-second delay), then periodically every hour via a background scheduler thread.

#### Storage Statistics

The `get_storage_stats` command provides runtime statistics: total clip count, total storage size, and database file size. Displayed in the Settings UI.

---

### 3. Filmstrip Overlay (Tauri + React)

The primary UI — a horizontal filmstrip anchored to the bottom of the screen.

#### Window Behavior

The overlay module (`overlay.rs`) positions the Tauri window using the Tauri window API:

1. Detect the current monitor (or primary monitor as fallback)
2. Calculate position: full monitor width, anchored to bottom edge
3. Set window size and position via `PhysicalSize` and `PhysicalPosition`
4. Apply compositor-specific rules for overlay behavior

**Wayland compositors:**
- **Hyprland**: Uses `hyprctl keyword windowrulev2` to set float, pin, noborder, noshadow, noanim rules by window title
- **Sway**: Uses `swaymsg for_window` to set floating, sticky, borderless rules by window title
- **GNOME/KDE**: Standard window positioning with always-on-top hints

**X11:** Uses `xprop` to set EWMH properties:
- `_NET_WM_WINDOW_TYPE_DOCK` for panel behavior
- `_NET_WM_STATE_ABOVE, _NET_WM_STATE_STICKY` for always-on-top across workspaces

This approach replaced the originally planned `gtk4-layer-shell` approach, using compositor IPC instead for broader compatibility.

#### Filmstrip Layout

```
┌─────────────────────────────────────────────────────────────────┐
│  [Search] [History] [Pinboards] [Snippets]              [gear] │
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
  AND (?5 IS NULL OR c.is_favorite = TRUE)
ORDER BY rank;
```

Search is debounced (100ms) on the frontend to avoid excessive queries during typing.

---

### 4. Text Expander Engine

Background service that monitors keystrokes and expands abbreviations.

#### Architecture

The text expander is split across several modules:

- `expander/buffer.rs` — Rolling character buffer for keystroke accumulation
- `expander/keymap.rs` — evdev keycode to character mapping
- `expander/matcher.rs` — Abbreviation matching algorithm
- `expander/engine.rs` — Orchestration: buffer + matcher + injection
- `expander/template.rs` — Template parsing and macro evaluation
- `expander/import.rs` — espanso YAML import
- `expander/export.rs` — JSON export/import

#### Keystroke Monitoring

Uses the evdev crate to read from `/dev/input/event*` devices. User must be in the `input` group.

The hotkey daemon and text expander share the same evdev connection — a single thread per keyboard device reads all events and dispatches them to both the hotkey matcher and the abbreviation buffer.

#### Abbreviation Matching

Maintains a rolling character buffer (max 100 chars). On each keystroke:

1. Append character to buffer
2. Check if buffer suffix matches any abbreviation
3. If match found:
   a. Emit backspace keystrokes to delete the abbreviation (N backspaces for N-char abbreviation)
   b. Evaluate the snippet template (resolve macros, scripts, etc.)
   c. Inject the expanded text via text injector
4. Reset buffer on word boundary or after a configurable timeout

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
    FillIn(FillInSpec),          // %fill(name), %fillarea(name), %fillpopup(name:opt1:opt2)
}
```

Fill-in fields are extracted before expansion. When present, a Tauri dialog window appears for the user to provide values before the text is inserted.

The `ExpansionContext` carries shared state across nested evaluations: clipboard content, fill-in values, and recursion depth tracking (max 10).

---

### 5. Global Hotkey Daemon

Uses the `evdev` crate to capture global keyboard shortcuts regardless of focused application or display server.

#### Registered Hotkeys

| Hotkey | Action | Configurable |
|--------|--------|-------------|
| Super+V | Toggle filmstrip overlay | Yes |
| Super+Shift+V | Toggle Paste Stack mode | Yes |
| Super+Shift+C | Quick copy to pinboard | Yes |
| Ctrl+Alt+Space | Toggle text expander on/off | Yes |
| Super+1-9 | Quick paste Nth item | No |

#### Implementation

Reads from `/dev/input/event*` devices via evdev. Detects key combinations by tracking modifier state. Emits events to the main application via channels.

The daemon (`hotkey/daemon.rs`) spawns one thread per keyboard device. The `hotkey/keys.rs` module handles keycode-to-key mapping and modifier state tracking.

---

### 6. Text Injector

Injects expanded text or clipboard content at the current cursor position.

#### Strategy Selection

```rust
fn select_injector(method: &str) -> Result<Arc<dyn Injector>> {
    match method {
        "auto" => { /* detect display server and available tools */ }
        "xdotool" => { /* X11 */ }
        "ydotool" => { /* Wayland universal */ }
        "wtype" => { /* wlroots only */ }
        "clipboard" => { /* fallback: set clipboard + Ctrl+V */ }
    }
}
```

The `Injector` trait provides two methods:

- `inject_text(&self, text: &str)` — typing simulation for text expansion
- `inject_via_clipboard(&self, text: &str)` — clipboard injection for paste operations
- `inject_rich(&self, content: &RichContent)` — rich paste preserving HTML/images

The injector falls back to clipboard injection if the configured method fails at initialization.

#### Injection Methods

| Method | Tool | Use Case |
|--------|------|----------|
| xdotool | `xdotool type --clearmodifiers` | X11 typing simulation |
| ydotool | `ydotool type` | Wayland universal (requires ydotoold) |
| wtype | `wtype` | wlroots compositors (Sway, Hyprland) |
| clipboard | `wl-copy/xclip + key sim` | Fallback: set clipboard + Ctrl+V |

For paste operations from the filmstrip, clipboard injection is always used to preserve rich content (HTML, images).

---

### 7. System Tray

Uses **Tauri v2's built-in tray icon** support (`tauri::tray::TrayIconBuilder`).

The original architecture planned to use the `ksni` crate for StatusNotifierItem. In practice, Tauri's built-in tray support was sufficient and simpler to integrate — it uses the same underlying AppIndicator/StatusNotifier protocols on Linux.

#### Tray Menu

```
Show Clipboard (Super+V)
─────────────
Paste Stack: OFF
Text Expander: ON
─────────────
Settings
About Paste
─────────────
Quit
```

Menu events are handled via Tauri's event system: each menu item emits a Tauri event that the frontend or backend can listen to.

---

### 8. Logging Module

The logging module (`logging.rs`) provides structured logging with dual output:

- **stderr** — for development and terminal debugging
- **File** — `~/.local/share/paste/paste.log` for production debugging

Features:
- Reads `RUST_LOG` environment variable for level control (default: `info`)
- Timestamps in local time with millisecond precision
- Log file rotation at 5 MB (old log moved to `paste.log.old`)
- Uses `env_logger` with a custom formatter that writes to both outputs

---

### 9. Service Module

The service module (`service.rs`) manages systemd user service integration:

- **Install**: Creates `~/.config/systemd/user/paste.service` and `~/.local/share/applications/paste.desktop`
- **Enable**: Runs `systemctl --user enable paste.service`
- **Uninstall**: Stops, disables, and removes the service file and desktop entry
- **Status check**: Verifies if the service file exists

The service is configured with `Restart=on-failure` and `RestartSec=5` for reliability.

---

## Process Architecture

```
┌──────────────────────────────────────────────────────┐
│              Main Process (Rust/Tauri)                │
│                                                      │
│  ├─ Main thread (Tauri event loop)                   │
│  │   └─ WebView management, IPC dispatch             │
│  │                                                   │
│  ├─ Input daemon thread (evdev)                      │  <- reads keyboard events
│  │   ├─ Hotkey matcher                               │
│  │   └─ Text expander keystroke buffer               │
│  │                                                   │
│  ├─ Clipboard monitor thread                         │  <- Wayland: wl-paste subprocess
│  │   └─ Content processing + dedup + storage         │     X11: XFixes event loop
│  │                                                   │
│  ├─ Retention scheduler thread                       │  <- hourly cleanup
│  │                                                   │
│  └─ Database (rusqlite + Mutex)                      │  <- shared across threads
│                                                      │
│  WebView (React App)                                 │
│  ├─ Filmstrip component                              │
│  ├─ Search component                                 │
│  ├─ Pinboard manager                                 │
│  ├─ Snippet manager                                  │
│  └─ Settings panel                                   │
└──────────────────────────────────────────────────────┘
```

### Thread Allocation

| Thread | Purpose | Notes |
|--------|---------|-------|
| Main (Tauri) | Event loop, IPC, window management | Async runtime (tokio) |
| Input daemon | evdev keyboard monitoring | Blocks on device read; 1 thread per device |
| Clipboard monitor | Clipboard change detection | Blocks on wl-paste or XFixes |
| Retention scheduler | Periodic cleanup | Runs every hour after 5-second startup delay |
| **Total** | **~4-6 threads** | Lightweight |

### IPC (Tauri Commands)

Frontend communicates with Rust backend via Tauri's command system. Key commands:

**Clipboard:**
- `get_clips` — paginated history with filters (type, app, pinboard, favorites)
- `search_clips` — FTS5 search with Power Search filters
- `paste_clip` — rich paste (HTML + images preserved)
- `paste_clip_plain` — plain text paste (strip formatting)
- `paste_clips_multi` — concatenated multi-clip paste
- `delete_clip` — remove clip from history
- `update_clip_content` — inline editing
- `toggle_favorite` — star/unstar clips
- `create_clip_from_text` — create clip from dropped content

**Pinboards:**
- `list_pinboards`, `create_pinboard`, `update_pinboard`, `delete_pinboard`
- `add_clip_to_pinboard`, `remove_clip_from_pinboard`

**Paste Stack:**
- `toggle_paste_stack`, `get_paste_stack`, `get_paste_stack_status`
- `add_to_paste_stack`, `pop_paste_stack`, `remove_from_paste_stack`
- `reorder_paste_stack`, `clear_paste_stack`

**Snippets:**
- `list_snippets`, `create_snippet`, `update_snippet`, `delete_snippet`
- `list_snippet_groups`, `create_snippet_group`, `delete_snippet_group`
- `get_fill_in_fields`, `expand_with_fill_ins`
- `preview_espanso_import`, `import_espanso`
- `export_snippets`, `import_snippets_json`

**Settings & Maintenance:**
- `get_config`, `save_config`, `reset_config`
- `get_excluded_apps`, `add_excluded_app`, `remove_excluded_app`
- `get_storage_stats`, `run_retention`, `clear_all_history`
- `get_autostart_status`, `install_autostart`, `uninstall_autostart`
- `quick_paste` — Super+N quick paste

Events from Rust to frontend:
```rust
app.emit("clip-added", &new_clip)?;
app.emit("tray-show-overlay", ())?;
app.emit("tray-toggle-expander", ())?;
app.emit("tray-toggle-paste-stack", ())?;
app.emit("tray-open-settings", ())?;
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
excluded_apps = ["1password", "keepassxc", "bitwarden", "lastpass"]
max_content_size_mb = 10     # Skip items larger than this
merge_growing = true         # Replace partial selections with complete ones
debounce_ms = 500            # Ignore rapid copies within this window

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

Configuration is managed by `config.rs` with full `#[serde(default)]` support — any missing fields fall back to sensible defaults. Validation ensures theme, trigger mode, and injection method are valid values.

---

## Security Considerations

- **Keyboard monitoring:** The evdev-based input daemon has access to all keyboard events (same as a keylogger). This is mitigated by: (a) the user explicitly adding themselves to the `input` group, (b) all data staying local, (c) no network access. Documented clearly in the user guide.
- **Clipboard content:** May contain sensitive data (passwords, tokens). Excluded apps list prevents capturing from known password managers. Retention policies provide automatic cleanup.
- **Shell script snippets:** Execute arbitrary commands. Import warnings are shown when importing snippets containing shell commands.
- **Image storage:** Screenshots may contain sensitive information. Respect retention policies and provide easy deletion.
- **No telemetry.** No analytics. No network calls. Ever.

---

## Testing Strategy

### Unit Tests (Rust — `cargo test`)

- Clipboard content type detection
- SQLite schema migrations (fresh, idempotent, sequential)
- FTS5 search query building
- Template parsing and evaluation (date macros, nested snippets, fill-ins)
- Abbreviation matching algorithm
- Deduplication logic (hash, growing text, debounce)
- Retention policy enforcement
- Configuration parsing, validation, and round-trip serialization
- Service file generation
- Overlay positioning (no-panic tests)

### Frontend Tests (React — Vitest + React Testing Library)

- Filmstrip rendering with mock data
- Search filtering behavior
- Keyboard navigation
- Pinboard management UI
- Snippet editor UI
- Card component rendering for each content type

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
tauri = { version = "2", features = ["tray-icon"] }
tauri-plugin-shell = "2"
rusqlite = { version = "0.39", features = ["bundled-full"] }
evdev = "0.13"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
serde_yaml = "0.9"             # espanso import
tokio = { version = "1", features = ["full"] }
uuid = { version = "1", features = ["v7"] }
chrono = { version = "0.4", features = ["serde"] }
image = "0.25"                  # Thumbnail generation
sha2 = "0.10"                   # Content hashing
toml = "0.8"                    # Configuration
dirs = "5"                      # XDG directory paths
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
sudo apt install libgtk-3-dev libayatana-appindicator3-dev librsvg2-dev libsoup-3.0-dev
```
