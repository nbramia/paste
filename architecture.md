# Paste — Architecture

## Key Design Decisions

These are the architectural choices that shaped the project. Each involved trade-offs — this section explains what was decided and why.

1. **Tauri v2 over Electron or native GTK** — Tauri gives us a Rust backend with direct system access (evdev, Wayland protocols, SQLite) and a web frontend for the visually rich filmstrip UI. ~5MB binary vs Electron's 100MB+. The trade-off: WebKitGTK rendering isn't as consistent as Chromium, but the resource savings are dramatic.

2. **xclip polling over wl-paste --watch** — The original design used `wl-paste --watch` for event-driven clipboard monitoring. In practice, the compositor didn't support the `wlr-data-control` protocol, and `wl-paste` polling caused desktop side-effects (trash icon bouncing from rapid subprocess spawning). `xclip` via XWayland polls cleanly with no visible side-effects. Trade-off: 1-second polling latency instead of instant event-driven capture.

   `wl-paste` is still kept on hand as a second opinion. Reading the clipboard through XWayland means trusting a bridge that can wedge, and when it does `xclip` keeps exiting 0 with stale bytes — a failure with no error to detect (#121, see Staleness detection below). Comparing the two readers is the only way to tell an idle clipboard apart from a blind one. `wl-paste` is polled at 1Hz only while failed over, which bounds the side-effects that motivated this decision in the first place.

3. **Tauri built-in tray over ksni crate** — The architecture originally planned to use the `ksni` crate for StatusNotifierItem. Tauri v2's built-in `tray-icon` feature turned out to be sufficient and eliminated an external dependency. Same underlying protocol (AppIndicator/StatusNotifier), simpler integration.

4. **Convention-driven agent framework over settings-file configuration** — Agent behavior is defined in CLAUDE.md and skill files rather than opaque configuration. Rules are readable, auditable, and versionable. A single hook in `.claude/settings.json` blocks the few operations that destroy work irreversibly; everything else is convention, enforced by review rather than by a gate.

5. **xdotool for backspaces, ydotool for typing** — ydotool's key code syntax (`14:1 14:0`) was unreliable across versions, producing garbage characters instead of backspaces. xdotool's `key BackSpace` works reliably under XWayland. The injector uses ydotool for `type` (text insertion) and xdotool for `key` (backspaces). Pragmatic, not elegant.

6. **SQLite with LIKE search over FTS5 or an external engine** — Clipboard history, snippets, and pinboards all live in one SQLite database, and search is a parameterized `LIKE '%query%'` scan. An FTS5 index was built first but never queried; measured at 10k clips the LIKE scan takes ~3ms, comfortably inside the 50ms target, so the index was dropped in migration 2 rather than wired up (#102). Substring matching also suits short clipboard content better than FTS5 tokenization — `ell` finds `hello`. The trade-off accepted: no relevance ranking, and search cost grows linearly with history size. Versioned migrations handle schema evolution. Single-writer concurrency (Mutex<Connection>) is fine for a desktop app.

7. **Rollback journal over WAL** — SQLite runs in `journal_mode=DELETE`, not WAL. WAL exists so readers and writers can work concurrently; this app has a single `Mutex<Connection>` and no concurrent access, so it bought nothing while introducing a silent data-loss mode. If the `-wal` and `-shm` sidecars are removed while the app holds them open — which happened in practice — every commit goes to an unlinked inode. SQLite reports success, the UI shows the data, and it is all discarded at exit; two days of clipboard history were nearly lost that way (#113). With a rollback journal there are no long-lived sidecars and every commit lands in `paste.db` itself, so an external reader always sees current data. The cost is an extra fsync per transaction, which is irrelevant at a few clips per minute.

---

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
| Language (backend) | Rust | Performance, memory safety, excellent Linux ecosystem (evdev, rusqlite, subprocess control) |
| Language (frontend) | TypeScript | Type safety, rich ecosystem for UI components |
| App framework | Tauri v2 | Lightweight (~5MB vs Electron's 100MB+), Rust backend, WebView frontend, built-in IPC |
| Frontend framework | React 19 + TailwindCSS v4 | Largest ecosystem, well-documented Tauri integration, Framer Motion for animations |
| Animations | Framer Motion | Production-grade animation library, spring physics, layout animations |
| Storage | SQLite via rusqlite 0.39 | Proven, lightweight, single-file database; search is a parameterized LIKE scan (see decision 6) |
| Clipboard (Wayland) | xclip via XWayland (polling) | Replaced `wl-paste --watch` which caused desktop side-effects; `wl-paste` retained as a staleness cross-check |
| Clipboard (X11) | same xclip poller | xclip reads the CLIPBOARD selection identically under X11 and XWayland, so one backend covers both |
| Global shortcuts | evdev crate | Kernel-level input, works on X11 + Wayland, no root needed (input group) |
| Text injection | ydotool / xdotool / wtype | Covers all display servers and compositors |
| System tray | Tauri built-in tray-icon | Uses Tauri v2's native tray icon support with menu builder API |
| Overlay positioning | Tauri window API + compositor IPC | Disabled during development; window starts as normal decorated window |
| Config | TOML | Rust-native, human-readable, well-typed |
| Packaging | Tauri bundler | Generates .deb, .AppImage |
| Build system | Cargo (Rust) + npm (frontend) | Standard tooling for both ecosystems |

### Why Tauri v2?

- **Lightweight:** ~5-10MB binary vs Electron's 100MB+. WebKitGTK is already installed on most Linux systems.
- **Rust backend:** Direct access to Linux system APIs (evdev, uinput, SQLite) without FFI overhead.
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

#### Wayland Implementation (xclip via XWayland)

```
xclip -selection clipboard -o        -> reads current clipboard text
```

- Polling-based: a background thread periodically reads the clipboard via `xclip -selection clipboard -o` through XWayland
- Each poll's content goes through `ClipDedup`, which drops exact repeats, folds a grown selection into the clip it extends, and debounces rapid re-copies (see Deduplication below)
- **Images ride the same poll as text** — `xclip -o` serves whatever the selection owner offers regardless of the target requested, so a copied image arrives on the ordinary text read as non-UTF8 bytes. Those bytes are sniffed for image magic numbers and captured from there (#122). There is no second polling loop: the original design ran one against `wl-paste --type image/png` once a second, and that rapid spawning is what made the GNOME trash icon bounce, so capture was disabled outright and every copied screenshot was discarded. Folding it into the existing read costs no extra subprocess
- The original design used `wl-paste --watch` for event-driven monitoring, but this was replaced because `wl-paste` caused desktop side-effects on some compositors
- Re-copying clips to the clipboard uses a `copy_to_clipboard` Tauri command that invokes `xclip -selection clipboard`

#### X11

There is no separate X11 backend. `xclip -selection clipboard -o` reads the CLIPBOARD selection identically whether it talks to a real X server or to XWayland, so the poller above serves both display servers and nothing branches on which one is running.

An event-driven implementation using the XFixes extension (`XFixesSelectSelectionInput`) lived in `clipboard/x11.rs` but was never constructed. It was removed in #103 along with the `x11rb` dependency and the unused `detect_display_server` helper. Its one advantage over polling was sub-second capture latency on native X11; `git log` has the code if that becomes worth having.

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

The deduplication module (`clipboard/dedup.rs`) implements three strategies:

1. **Hash-based dedup** — SHA-256 hash of the content. If the hash matches the most recent entry, the duplicate is skipped.
2. **Growing text detection** — When `merge_growing` is enabled (default), if new content is a superset of the most recent clip (e.g., the user selected a word, then extended the selection to a paragraph), the older partial clip is replaced instead of creating a new entry.
3. **Debounce** — Rapid consecutive copies within the debounce window (default 500ms) are collapsed.

All three run inside the poll loop, which holds one `ClipDedup` for its lifetime. `Accept` inserts a new clip; `Replace` calls `Storage::replace_latest_clip`, which inserts the new clip and deletes the superseded one — unless that clip is favorited or filed in a pinboard, in which case both are kept and nothing the user deliberately saved is discarded.

> **Note on `debounce_ms`:** the poller wakes once a second, so two *different* clipboard contents are almost always more than the default 500ms apart and the debounce branch rarely fires. It becomes meaningful only if the poll interval drops below the debounce window, or if capture becomes event-driven (#103).

Two `[clipboard]` keys remain unwired: `monitor_primary` and `monitor_clipboard`. The polling watcher reads only the CLIPBOARD selection; PRIMARY monitoring is unimplemented rather than merely disconnected.

#### Staleness detection

Reading the clipboard through XWayland means depending on the bridge that
mirrors the Wayland selection into X11. That bridge can wedge. When it does,
`xclip` keeps exiting 0 and keeps returning the same bytes while the real
clipboard moves on — there is no error, no empty read, and nothing to catch.

Every poll then lands on `DedupResult::Duplicate`, which logs nothing. Capture
goes completely dead while the tray icon, the overlay, retention and the
hotkeys all keep working. Two windows were measured on the development machine
before the fix — four days and a day and a half — each ending at a restart
rather than at a fix, with not one log line in between (#121).

`StaleGuard` closes that hole by keeping a second reader on hand:

| State | Behaviour |
|-------|-----------|
| Clipboard changing | Nothing extra happens; `xclip` at 1Hz as before |
| Unchanged for 60 polls | Read `wl-paste` once and compare |
| Readers disagree | Log at error level, fail over to `wl-paste` |
| Failed over, readers agree again | Log recovery, fail back to `xclip` |

Comparison ignores trailing newlines, since `wl-paste --no-newline` strips one
and `xclip -o` does not.

Failing over is load-bearing, not a convenience. Borrowing the other reader's
value for a single poll while leaving the stale reader primary makes the stale
bytes look new again on the very next poll, and the loop alternates between the
two values once a second.

The cross-check costs at most one extra subprocess per minute, and none while
the user is actively copying. 1Hz `wl-paste` polling — the thing that caused
the desktop side-effects behind decision 2 — happens only while failed over.

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
- Thumbnail: `{id}_thumb.webp` (longest edge scaled to 256px, aspect ratio preserved; lossless WebP)

Thumbnails are derived from the original's path rather than stored in the database, so every path that deletes an image clip must go through `images::remove_image_and_thumbnail` — deleting only `image_path` leaks the sidecar. Retention and `delete_clip` both do.

The frontend receives a thumbnail as a `data:` URI from the `get_clip_thumbnail` command rather than loading the file directly. That keeps the "no direct file access from the frontend" boundary and avoids widening the app's CSP with the asset protocol for the sake of one card.

Thumbnails are generated on capture using the `image` crate. Only thumbnails are loaded into the filmstrip; originals are loaded on-demand for full preview.

#### Database Health

SQLite writes happily to a file that has been unlinked or replaced underneath
it, reporting success the whole time. Nothing in the normal code path notices,
so the condition is completely silent.

`Storage` records the device and inode of the database file when it opens, and
`health_check()` compares that against whatever is at the path now:

| State | Meaning |
|-------|---------|
| `Ok` | The open file is still the file at the path |
| `Missing` | Nothing at the path; writes are going to an unlinked inode |
| `Replaced` | A different file occupies the path; our writes are invisible to it |
| `InMemory` | Test database, nothing to verify |

It runs at startup and on each hourly retention tick, logging at error level
with recovery instructions. `Missing` and `Replaced` both mean writes are being
lost, which `is_losing_writes()` reports.

Recovery while the process is still alive: copy its open descriptors out of
`/proc/<pid>/fd/` — the data exists in the orphaned inode until exit.

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

Both limits are read from config. `0` means unlimited, and is mapped to `None` before it reaches `enforce_retention` — `Some(0)` would read as "delete everything older than zero days" and take the whole history with it. These two values were hardcoded to `(90, 10000)` at the call site until #120, which made `[storage]` inert: history was capped at 90 days no matter what the config said, and there was no way to keep clips indefinitely. The effective policy is logged at startup.

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

#### Keyboard Ownership

Multiple views render clip cards, and each keeps its own selection state. To avoid two views acting on the same keypress, keyboard handling is **owned by whichever view is currently showing cards**:

| Scope | Owner | Keys |
|-------|-------|------|
| Global | `App` | `Escape` (dismiss), `Tab` / `Alt+Arrow` (switch tabs), `/` and `Ctrl+F` (focus search) |
| History filmstrip | `App` | Arrows, `Enter`, `Space`, `Delete`/`Backspace`, `f`, `Ctrl+E`, `Ctrl+P` |
| Pinboard clip strip | `PinboardView` | Arrows, `Enter` |

`App`'s window-level handler computes `ownsClipKeys = activeTab === "history" && !showPasteStack` and stands down for everything except the global keys when that is false. `PinboardView` registers its own window listener only while a pinboard's clips are open. Without this split, `Enter` inside a pinboard operated on the history list's `selectedIndex` and copied an unrelated clip (#99).

Modal dialogs (`CreatePinboardDialog`, `SnippetEditor`, `FillInDialog`) call `stopPropagation()` on keydown, so window-level handlers never see keys typed into a form field.

#### Taking keyboard focus

The overlay is summoned by an evdev hotkey, which the compositor never sees, so
mutter has no user interaction to attribute the activation to. Under GNOME
Wayland it therefore grants focus only on the window's **first** map; every
later show is a re-map and focus-stealing prevention denies it. Measured on the
development machine, 1 show in 5 got focus — always the first after startup —
leaving the user to click before arrow keys worked.

Neither `set_focus()` nor `gtk_window_present_with_time()` fixes this: without
an xdg-activation token GTK falls back to a plain present, which is what was
already being refused.

`activate_overlay_via_shell()` instead asks GNOME Shell to do it, over the
`org.gnome.Shell.Extensions.Windows` interface (provided by extensions such as
Window Calls). That code runs inside the compositor and is not subject to the
restriction. The overlay window is matched by **pid** rather than window class,
since the class differs between dev and packaged builds.

This is opportunistic, in the same spirit as the compositor-specific window
rules: when the interface is absent it is a no-op and behaviour is unchanged.

#### Confirming a clip

`Enter` and double-click both **paste** the selected clip into whatever had focus before the overlay opened:

1. `paste_clip` records the access, spawns a worker thread, and returns.
2. The worker hides the overlay and waits for the compositor to report focus has moved off Paste.
3. It injects `Ctrl+V` via the persistent uinput virtual keyboard (see the Text Injector section).

Step 1 matters as much as the others. A synchronous Tauri command runs on the
main thread, which is also the GTK event loop, so waiting there for the focus
hand-back is a deadlock: the focus-out event cannot be processed until the
command returns, and the overlay keeps focus for exactly as long as it is
waited on. The symptom was a `Ctrl+V` injected into the hidden overlay and
silently lost — with the compositor reporting `Paste` as focused at the moment
of injection. Doing the work off-thread lets focus return in ~65ms.

Hiding first is essential — a visible, focused overlay swallows the synthetic paste.

If injection fails, the frontend falls back to `copy_to_clipboard` and dismisses, so the clip still reaches the clipboard and the user can paste manually. The action is never silently lost.

> **History:** confirm was originally a copy-and-dismiss that deliberately left the user to press `Ctrl+V` (#99), on the grounds that it worked regardless of injection backend. Once injection became reliable (#97, #98) that caution cost a keystroke on every paste, so #114 switched confirm to a real paste.

`Escape` and the backdrop click dismiss without pasting.

#### Search Architecture

Frontend sends search queries to Rust backend via Tauri command.

`Storage::search_clips` executes a parameterized `LIKE '%query%'` scan over `clips.text_content` and `clips.source_app`, with optional filters appended as bound parameters:

```sql
SELECT c.* FROM clips c
WHERE (c.text_content LIKE ? OR c.source_app LIKE ?)
  AND (c.content_type = ?)      -- appended only when filtered
  AND (c.source_app = ?)
  AND (c.created_at >= ?)
  AND (c.created_at <= ?)
  AND (c.is_favorite = ?)
ORDER BY created_at DESC LIMIT ? OFFSET ?;
```

Substring matching means `ell` finds `hello`, which FTS5 tokenization would not. The cost is no relevance ranking and a scan that grows linearly with history — measured at ~3ms over 10k clips, well inside the 50ms target, which is why the FTS5 index was dropped rather than wired up (#102, migration 2). A regression guard lives in `storage/search.rs` as `bench_search_at_ten_thousand_clips`.

For reference, the originally designed FTS5 query, which never ran:

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
| Ctrl+Alt+V | Toggle filmstrip overlay (Cmd+Option+V with Toshy) | Yes |
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
| clipboard | `xclip + key sim` | Fallback: set clipboard + Ctrl+V |

For paste operations from the filmstrip, clipboard injection is always used to preserve rich content (HTML, images).

**ydotool backspace workaround:** ydotool's key codes for backspace were broken/inconsistent, so the text expander uses `xdotool` as a fallback for emitting backspace keystrokes when deleting abbreviations before expansion.

---

### 7. System Tray

Uses **Tauri v2's built-in tray icon** support (`tauri::tray::TrayIconBuilder`).

The original architecture planned to use the `ksni` crate for StatusNotifierItem. In practice, Tauri's built-in tray support was sufficient and simpler to integrate — it uses the same underlying AppIndicator/StatusNotifier protocols on Linux.

#### Tray Menu

```
Show Clipboard (Ctrl+Alt+V)
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
│  ├─ Clipboard monitor thread                         │  <- xclip polling via XWayland
│  │   └─ Content processing + dedup + storage         │     (text and images, one loop)
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
| Clipboard monitor | Clipboard change detection | Polls xclip via XWayland |
| Retention scheduler | Periodic cleanup | Runs every hour after 5-second startup delay |
| **Total** | **~4-6 threads** | Lightweight |

### IPC (Tauri Commands)

Frontend communicates with Rust backend via Tauri's command system. There are ~46 commands organized into 6 domains:

| Domain | Commands | Purpose |
|--------|----------|---------|
| Clipboard | 11 | CRUD, search, paste (rich/plain/multi), copy to clipboard, favorites, image thumbnails |
| Pinboards | 6 | CRUD, assign/remove clips |
| Paste Stack | 8 | Lifecycle management (toggle, push, pop, reorder, clear) |
| Snippets | 12 | CRUD, groups, fill-in fields, espanso import, JSON export/import |
| Settings | 6 | Config read/write/reset, exclusion list management |
| Maintenance | 4 | Storage stats, retention, autostart |

The backend also emits events to the frontend: `clip-added` (triggers history reload), and tray menu events (`tray-show-overlay`, `tray-toggle-expander`, etc.).

All commands are registered in `src-tauri/src/lib.rs` in the `invoke_handler` macro. See that file for the full inventory.

---

## Configuration

TOML config file at `~/.config/paste/config.toml`:

```toml
[hotkeys]
toggle_overlay = "Ctrl+Alt+V"
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
    "@tailwindcss/vite": "^4",
    "vite": "^6",
    "vitest": "^2",
    "@testing-library/react": "^16"
  }
}
```

### System Packages

```bash
# Clipboard monitoring + text injection
sudo apt install xclip xdotool ydotool wtype

# WebView (usually pre-installed on modern Linux)
sudo apt install libwebkit2gtk-4.1-0

# Tauri build dependencies
sudo apt install libgtk-3-dev libayatana-appindicator3-dev librsvg2-dev libsoup-3.0-dev
```
