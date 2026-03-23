# Paste User Guide

A comprehensive walkthrough of every feature in Paste.

## Table of Contents

1. [Getting Started](#1-getting-started)
2. [Clipboard History](#2-clipboard-history)
3. [The Filmstrip](#3-the-filmstrip)
4. [Quick Paste](#4-quick-paste)
5. [Search & Power Search](#5-search--power-search)
6. [Pinboards](#6-pinboards)
7. [Favorites](#7-favorites)
8. [Quick Look Preview](#8-quick-look-preview)
9. [Paste Stack](#9-paste-stack)
10. [Multi-Item Paste](#10-multi-item-paste)
11. [Inline Editing](#11-inline-editing)
12. [Text Expander](#12-text-expander)
13. [Macros](#13-macros)
14. [Fill-in Fields](#14-fill-in-fields)
15. [Snippet Management](#15-snippet-management)
16. [Drag and Drop](#16-drag-and-drop)
17. [Rich Paste](#17-rich-paste)
18. [Clipboard Persistence](#18-clipboard-persistence)
19. [Settings](#19-settings)
20. [Light/Dark Mode](#20-lightdark-mode)
21. [System Tray](#21-system-tray)
22. [Autostart](#22-autostart)
23. [Data Storage](#23-data-storage)

---

## 1. Getting Started

### Installation

See the [README](../README.md#installation) for installation instructions via .deb, AppImage, or building from source.

### First Run

When Paste launches for the first time it:

1. Creates a configuration file at `~/.config/paste/config.toml` with sensible defaults
2. Creates the SQLite database at `~/.local/share/paste/paste.db`
3. Creates the image storage directory at `~/.local/share/paste/images/`
4. Starts monitoring the system clipboard
5. Shows the system tray icon

### Input Group Setup

Paste uses Linux's evdev interface for global hotkeys and text expander keystroke monitoring. This requires membership in the `input` group:

```bash
sudo usermod -aG input $USER
```

You must **log out and back in** (or reboot) for this to take effect. Verify with:

```bash
groups | grep input
```

Without this, global hotkeys (Super+V) and the text expander will not work. The filmstrip can still be opened from the system tray menu.

### Wayland Text Injection

On Wayland, Paste needs a tool to inject text at the cursor position. Install at least one:

- **ydotool** — universal, works on all Wayland compositors (GNOME, KDE, Hyprland, Sway). Requires the `ydotoold` daemon.
- **wtype** — lightweight, works only on wlroots-based compositors (Sway, Hyprland). No daemon needed.

On X11, `xdotool` is used automatically.

---

## 2. Clipboard History

### How Capture Works

Paste monitors two clipboard selections:

- **CLIPBOARD** — filled by Ctrl+C, right-click copy, etc. This is the primary selection most users interact with.
- **PRIMARY** — filled by mouse text selection. Can be disabled in settings if unwanted.

On Wayland, `wl-paste --watch` is used for event-driven monitoring. On X11, the XFixes extension provides selection change notifications. Neither approach uses polling.

### Content Types

Paste detects and categorizes clipboard content:

| Type | How Detected | Preview |
|------|-------------|---------|
| **Text** | Plain text without code patterns | First few lines, proportional font |
| **Code** | Text with syntax patterns (braces, `fn`, `import`, indentation) | Syntax-highlighted, monospace |
| **Link** | Text matching URL pattern or `text/uri-list` MIME | URL with domain, favicon if available |
| **Image** | `image/png`, `image/jpeg`, etc. | Thumbnail preview |
| **File** | `file://` URI from file managers | File icon + filename + size |

### Deduplication

Paste prevents clutter from repeated copies:

- **Hash-based dedup** — each clip is hashed (SHA-256). If the hash matches the most recent entry, the duplicate is skipped.
- **Growing text detection** — when `merge_growing` is enabled (default), if you select progressively more text (e.g., selecting a word, then a line, then a paragraph), Paste replaces the earlier partial clips instead of creating separate entries.
- **Debounce** — rapid copies within the debounce window (default 500ms) are collapsed into a single entry.

### Application Exclusion

Password managers and other sensitive applications can be excluded from clipboard capture. By default, `1password`, `keepassxc`, `bitwarden`, and `lastpass` are excluded. Edit the list in Settings or in `config.toml`:

```toml
[clipboard]
excluded_apps = ["1password", "keepassxc", "bitwarden", "lastpass"]
```

---

## 3. The Filmstrip

The filmstrip is the main UI. Press **Super+V** to toggle it.

### Layout

The filmstrip appears as a horizontal strip anchored to the bottom of your screen. It contains:

- **Top bar** — Search input, view tabs (History, Pinboards, Snippets), and a settings gear icon
- **Card strip** — Horizontally scrollable content cards, newest on the left
- **Card footer** — Source app icon, relative timestamp ("2m ago"), content type indicator

### Navigation

| Key | Action |
|-----|--------|
| Left / Right | Move between cards |
| Enter | Paste the selected clip (rich content preserved) |
| Shift+Enter | Paste as plain text (strip formatting) |
| Space | Quick Look preview |
| Tab | Cycle between History, Pinboards, and Snippets views |
| Esc | Close preview, clear search, or dismiss the filmstrip |

### Card Types

Each card renders differently based on content type:

- **Text cards** — show the first few lines in a proportional font
- **Code cards** — show syntax-highlighted text in a monospace font with a language badge
- **Link cards** — show the URL domain and page title
- **Image cards** — show a thumbnail preview with dimensions in the footer
- **File cards** — show a file icon, filename, and file size

### Overlay Positioning

On **Wayland**, Paste applies compositor-specific window rules:
- Hyprland: uses `hyprctl` to set float, pin, noborder, noshadow rules
- Sway: uses `swaymsg` for floating, sticky, borderless rules
- GNOME/KDE: standard window positioning with always-on-top

On **X11**, EWMH properties (`_NET_WM_WINDOW_TYPE_DOCK`, `_NET_WM_STATE_ABOVE`) are set for proper overlay behavior.

---

## 4. Quick Paste

Quick Paste lets you paste recent clips without opening the filmstrip.

**Super+1** pastes the most recent clip, **Super+2** pastes the second most recent, and so on through **Super+9**.

This is the fastest way to re-paste something you just copied. The clip is injected via clipboard (set clipboard content, then simulate Ctrl+V) to preserve rich formatting.

---

## 5. Search & Power Search

### Basic Search

Press **/** or **Ctrl+F** while the filmstrip is open to focus the search bar. Type to filter clips in real time. Search is powered by SQLite FTS5, which supports full-text matching across all text content.

### Power Search

Press **Ctrl+F** again (when the search bar is already focused) to toggle Power Search filters:

- **Content type** — filter by text, code, link, image, or file
- **Source application** — filter by the app the content was copied from
- **Date range** — filter by when the content was copied
- **Favorites only** — show only starred clips

Filters combine with the text query. For example, you can search for "deploy" filtered to "code" type from "VS Code" in the last week.

Search results are debounced (100ms) so the UI stays responsive while typing.

---

## 6. Pinboards

Pinboards are named, color-coded collections for organizing frequently-used clips.

### Creating a Pinboard

1. Switch to the **Pinboards** tab (press Tab in the filmstrip)
2. Click the **+** button or use the create dialog
3. Enter a name and choose a color

### Saving Clips to a Pinboard

- Select a clip and press **Ctrl+P** — a pinboard picker appears
- Choose the target pinboard from the list

### Browsing Pinboards

Switch to the Pinboards tab to see all your pinboards. Click a pinboard to see its clips.

### Retention Exemption

Clips saved to a pinboard are **exempt from retention policies**. They persist indefinitely regardless of the max history age or count settings. This makes pinboards ideal for content you want to keep permanently: email templates, code snippets, frequently-used URLs, canned replies.

### Managing Pinboards

Pinboards can be renamed, recolored, or deleted through the UI. Deleting a pinboard does not delete the clips — they return to the general history.

---

## 7. Favorites

Star any clip by selecting it and pressing **F**. Favorited clips display a star indicator on their card.

Like pinboard clips, **favorites are exempt from retention policies**. They will not be deleted by age or count limits.

Use the favorites filter in Power Search to quickly find starred clips across all of your history.

---

## 8. Quick Look Preview

Select any clip and press **Space** to open a full-content preview.

- **Text/Code** — shows the complete text content, scrollable
- **Images** — shows the full-resolution image
- **Links** — shows the complete URL

Press **Space** or **Escape** to close the preview.

---

## 9. Paste Stack

Paste Stack is a sequential copy-then-paste workflow for batch content movement.

### Activating

Press **Super+Shift+V** to toggle Paste Stack mode. The filmstrip indicates when Paste Stack is active.

### Workflow

1. **Activate Paste Stack** (Super+Shift+V)
2. **Copy multiple items** — each copy adds to the stack queue
3. **Switch to the target** — go to the document or app where you want to paste
4. **Paste sequentially** — each Ctrl+V pastes the next item in the stack, in the order you copied them
5. **Stack auto-deactivates** when empty

### Queue Management

While the stack is active, you can:

- View all queued items in the filmstrip
- Remove specific items from the queue
- Reorder items by dragging
- Clear the entire stack

This is ideal for moving content between documents, filling out forms, or assembling content from multiple sources.

---

## 10. Multi-Item Paste

Select multiple clips and paste them all at once.

### Selecting Multiple Clips

- **Ctrl+click** — toggle individual clips in the selection
- **Shift+click** — select a range of clips

### Pasting

When multiple clips are selected, pressing Enter or using the paste action concatenates their text content with newlines and pastes the combined result.

---

## 11. Inline Editing

Select a text or code clip and press **Ctrl+E** to edit its content directly in the filmstrip.

- An editor opens with the clip's current content
- Make your changes
- Press **Ctrl+Enter** to save, or **Escape** to cancel

Inline editing is available for text and code clips only (not images or files). The edited content is saved back to the database, and the content hash is updated.

---

## 12. Text Expander

The text expander lets you type short abbreviations that automatically expand into full snippets.

### How It Works

1. Create a snippet with an abbreviation (e.g., `;sig`) and content (e.g., your email signature)
2. Type the abbreviation anywhere on your system
3. When the trigger fires (word boundary or immediately, depending on settings), the abbreviation is deleted and replaced with the expanded content

### Trigger Modes

- **Word boundary** (default) — the snippet expands when you type the abbreviation followed by a space, tab, or punctuation. This is the safest mode and prevents accidental triggers.
- **Immediate** — the snippet expands as soon as the last character of the abbreviation is typed. Faster but more prone to false positives with short abbreviations.

Configure in Settings or `config.toml`:

```toml
[expander]
trigger = "word_boundary"  # or "immediate"
```

### Toggling

Press **Ctrl+Alt+Space** to toggle the text expander on or off. The system tray menu also shows the current state and allows toggling.

---

## 13. Macros

Snippet content can contain macros that are evaluated at expansion time.

### Date/Time Macros

| Macro | Output | Example |
|-------|--------|---------|
| `%Y` | 4-digit year | 2026 |
| `%m` | Month (01-12) | 03 |
| `%d` | Day (01-31) | 22 |
| `%H` | Hour 24h (00-23) | 14 |
| `%M` | Minute (00-59) | 30 |
| `%S` | Second (00-59) | 45 |
| `%A` | Weekday name | Sunday |
| `%B` | Month name | March |
| `%p` | AM/PM | PM |
| `%%` | Literal `%` | % |

**Example:** `;today` with content `Meeting notes for %Y-%m-%d` expands to `Meeting notes for 2026-03-22`

### Date Math

Use `%date(+/-Nunit)` for relative dates. Output format is `YYYY-MM-DD`.

| Expression | Meaning |
|-----------|---------|
| `%date(+5d)` | 5 days from now |
| `%date(-1w)` | 1 week ago |
| `%date(+3M)` | 3 months from now |
| `%date(+1y)` | 1 year from now |
| `%date(-2h)` | 2 hours ago |
| `%date(+30m)` | 30 minutes from now |

Units: `m` (minutes), `h` (hours), `d` (days), `w` (weeks), `M` (months), `y` (years).

**Example:** `;deadline` with content `Due by %date(+14d)` expands to `Due by 2026-04-05`

### Clipboard Macro

`%clipboard` inserts the current clipboard text content at expansion time.

**Example:** `;quote` with content `> %clipboard` wraps whatever you last copied in a blockquote.

### Cursor Positioning

`%|` marks where the cursor should be placed after expansion.

**Example:** `;reply` with content `Hi %|,\n\nThanks for your message.` places the cursor right after "Hi " so you can type the recipient's name.

### Shell Commands

`%shell(command)` executes a shell command and inserts its stdout output.

| Example | Output |
|---------|--------|
| `%shell(date +%s)` | Unix timestamp |
| `%shell(hostname)` | Machine hostname |
| `%shell(git branch --show-current)` | Current git branch |
| `%shell(curl -s wttr.in?format="%t")` | Current temperature |

Commands time out after 5 seconds. Errors produce `[shell error: ...]` text.

### Nested Snippets

`%snippet(abbreviation)` expands another snippet inline, enabling composable templates.

**Example:** If `;phone` expands to `555-1234`, then `;contact` with content `John Smith\nPhone: %snippet(;phone)` expands to:

```
John Smith
Phone: 555-1234
```

Maximum nesting depth is 10. Circular references are detected and produce an error. The evaluation context (clipboard, date) is shared across the chain.

See [Text Expander Syntax Reference](text-expander.md) for the full specification.

---

## 14. Fill-in Fields

Fill-in fields add interactive prompts that appear before a snippet is expanded.

### Syntax

| Syntax | Field Type |
|--------|-----------|
| `%fill(name)` | Single-line text input |
| `%fill(name:default=value)` | Text with default value |
| `%fillarea(notes)` | Multi-line text area |
| `%fillpopup(tone:Professional:Casual:Formal)` | Dropdown selector |

### How It Works

When a snippet containing fill-in fields is triggered, a dialog window appears with input fields for each placeholder. Fill in the values and click "Expand" (or press Ctrl+Enter) to insert the completed text. Press Escape or Cancel to abort.

### Example

A snippet `;letter` with content:

```
Dear %fill(recipient),

%fillarea(body)

%fillpopup(closing:Best regards:Sincerely:Cheers),
%fill(name:default=John Smith)
```

When triggered, a dialog appears with four fields: recipient (text), body (textarea), closing (dropdown with three options), and name (text with "John Smith" pre-filled).

---

## 15. Snippet Management

### Creating Snippets

1. Open the filmstrip (Super+V) and switch to the **Snippets** tab
2. Click the **+** button to create a new snippet
3. Enter:
   - **Abbreviation** — the trigger text (e.g., `;sig`)
   - **Name** — a human-readable label
   - **Content** — the expansion template (supports all macros)
   - **Content type** — plain text, script (shell output), or fill-in
   - **Group** — optional organizational group
   - **Description** — optional notes about the snippet

### Editing and Deleting

Click a snippet to edit it, or use the delete button to remove it. Changes take effect immediately.

### Groups

Organize snippets into named groups for easier management. Create groups from the Snippets tab. Snippets can be assigned to groups or left ungrouped. Deleting a group does not delete its snippets — they become ungrouped.

### Importing from espanso

Click "Import espanso" in the Snippets tab to import snippets from `~/.config/espanso/match/*.yml`. Variable mapping:

| espanso | Paste |
|---------|-------|
| `{{clipboard}}` | `%clipboard` |
| `{{date}}` | `%Y-%m-%d` |
| `{{time}}` | `%H:%M:%S` |
| `{{newline}}` | `\n` |

Duplicate abbreviations are automatically skipped during import.

### JSON Export/Import

Export all your snippets and groups to a JSON file for backup or transfer between machines. Import from a previously exported JSON file. Duplicate abbreviations are skipped. Script snippets are flagged during import with a warning since they execute arbitrary commands.

---

## 16. Drag and Drop

### Dragging Out

Drag a clip card from the filmstrip and drop it into another application. The text content is transferred via the drag data.

### Dropping In

Drop text content from another application into the filmstrip to create a new clip entry. The dropped text is stored with "Paste (drop)" as the source application.

---

## 17. Rich Paste

When you paste a clip using Enter (not Shift+Enter), Paste preserves rich content:

- **HTML clips** — formatting is preserved (bold, italic, links, etc.) via the `text/html` clipboard MIME type
- **Image clips** — pasted as actual images, not as file paths
- **Text clips** — pasted as plain text

Use **Shift+Enter** to force plain text paste, stripping all formatting.

---

## 18. Clipboard Persistence

On Wayland, the clipboard is owned by the source application. When that application closes, the clipboard content is lost — this is a well-known Wayland limitation.

Paste solves this by capturing clipboard content as soon as it changes. Even if the source app closes, the content is preserved in Paste's history and can be re-pasted. With `wl-clipboard` installed, Paste can also re-assert the clipboard content when the source app exits.

---

## 19. Settings

Access Settings via the gear icon in the filmstrip or from the system tray menu.

### Appearance

- **Theme** — system (auto-detect), light, or dark
- **Filmstrip height** — height in pixels (default: 300)
- **Cards visible** — number of cards shown at once (default: 6)
- **Animation speed** — multiplier for all animations (0 = instant, 1 = normal)
- **Blur background** — semi-transparent blur behind the filmstrip (compositor-dependent)

### Hotkeys

- **Toggle overlay** — default: Super+V
- **Paste Stack mode** — default: Super+Shift+V
- **Quick copy to pinboard** — default: Super+Shift+C
- **Toggle expander** — default: Ctrl+Alt+Space

### Clipboard

- **Monitor PRIMARY** — capture mouse-selected text (default: on)
- **Monitor CLIPBOARD** — capture Ctrl+C copies (default: on)
- **Max content size** — skip items larger than this (default: 10 MB)
- **Merge growing text** — replace partial selections with complete ones (default: on)
- **Debounce** — ignore rapid consecutive copies within this window (default: 500ms)

### Exclusions

Manage the list of applications whose clipboard content should never be captured. Add or remove apps by name. Default exclusions: 1password, keepassxc, bitwarden, lastpass.

### Storage & Retention

- **Max history age** — delete clips older than N days (default: 90, 0 = unlimited)
- **Max history count** — maximum number of clips (default: 10000, 0 = unlimited)
- **Max image size** — skip images larger than N MB (default: 10)
- **Max total storage** — total storage cap in MB (default: 500)
- **Run Cleanup Now** — manually trigger retention enforcement
- **Clear All History** — delete all non-pinboard, non-favorite clips

Storage statistics (total clips, total size, database size) are displayed in this section.

### Text Expander

- **Enabled** — toggle the expander on/off (default: on)
- **Trigger mode** — word_boundary or immediate (default: word_boundary)
- **Typing speed** — delay between injected characters in ms (default: 0 = instant)

### Injection

- **Method** — auto, xdotool, ydotool, wtype, or clipboard (default: auto)

Auto mode detects the display server and available tools, choosing the best option.

### Autostart

Enable or disable the systemd user service for launching Paste at login. See [Autostart](#22-autostart) for details.

---

## 20. Light/Dark Mode

Paste supports three theme modes:

- **System** (default) — automatically follows your desktop environment's light/dark preference
- **Light** — always use the light theme
- **Dark** — always use the dark theme

Configure in Settings or `config.toml`:

```toml
[ui]
theme = "system"  # system | light | dark
```

Paste also respects the `prefers-reduced-motion` media query. When reduced motion is enabled in your system settings, animations are minimized or disabled.

---

## 21. System Tray

Paste runs as a system tray application. The tray icon provides a context menu with:

- **Show Clipboard (Super+V)** — open the filmstrip
- **Paste Stack: OFF/ON** — toggle Paste Stack mode
- **Text Expander: ON/OFF** — toggle the text expander
- **Settings** — open the Settings panel
- **About Paste** — version information
- **Quit** — exit the application

On GNOME, you need the AppIndicator extension for tray icons:

```bash
sudo apt install gnome-shell-extension-appindicator
```

On Hyprland, ensure your bar (e.g., waybar) has a tray module configured.

---

## 22. Autostart

Paste can start automatically when you log in via a systemd user service.

### Enabling

Enable autostart from Settings, or install manually:

The service file is created at `~/.config/systemd/user/paste.service`. A desktop entry is also created at `~/.local/share/applications/paste.desktop`.

### Service Details

```ini
[Unit]
Description=Paste - Clipboard Manager
After=graphical-session.target

[Service]
Type=simple
ExecStart=/path/to/paste
Restart=on-failure
RestartSec=5

[Install]
WantedBy=default.target
```

### Manual Control

```bash
# Check status
systemctl --user status paste.service

# Start/stop
systemctl --user start paste.service
systemctl --user stop paste.service

# Enable/disable autostart
systemctl --user enable paste.service
systemctl --user disable paste.service
```

---

## 23. Data Storage

### File Locations

| Path | Contents |
|------|----------|
| `~/.config/paste/config.toml` | Configuration file |
| `~/.local/share/paste/paste.db` | SQLite database (clips, pinboards, snippets) |
| `~/.local/share/paste/images/` | Stored image files and thumbnails |
| `~/.local/share/paste/paste.log` | Application log file |
| `~/.config/systemd/user/paste.service` | Autostart service (if enabled) |

### Database

The SQLite database stores:

- **clips** — clipboard history with content, metadata, timestamps, and access counts
- **clips_fts** — FTS5 full-text search index synchronized via triggers
- **pinboards** — named collections with colors and positions
- **snippets** — text expander abbreviations and templates
- **snippet_groups** — organizational groups for snippets
- **paste_stack** — temporary queue for Paste Stack mode
- **schema_version** — migration tracking

### Migrations

The database schema is versioned. On startup, Paste checks the current schema version and runs any pending migrations. Before migrating, a backup of the database file is created (e.g., `paste.v1.bak`). Migrations run in a transaction and are rolled back on failure.

### Logs

Application logs are written to `~/.local/share/paste/paste.log`. The log file is rotated at 5 MB (old content is moved to `paste.log.old`). Set the `RUST_LOG` environment variable to control log verbosity:

```bash
RUST_LOG=debug paste    # verbose logging
RUST_LOG=warn paste     # warnings and errors only
```

### Backup

To back up your Paste data:

```bash
# Back up everything
cp ~/.local/share/paste/paste.db ~/paste-backup.db
cp -r ~/.local/share/paste/images/ ~/paste-images-backup/
cp ~/.config/paste/config.toml ~/paste-config-backup.toml
```

To restore, copy the files back to their original locations. Paste will detect and use the existing database on the next launch.
