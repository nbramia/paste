# Keyboard Shortcuts

## Global (via evdev)

These work regardless of which application is focused. They require membership in the `input` group.

| Shortcut | Action |
|---|---|
| Super+Alt+V | Toggle filmstrip overlay (Cmd+Option+V on Mac keyboard with Toshy) |
| Super+Shift+V | Toggle Paste Stack mode |
| Super+Shift+C | Quick copy to pinboard |
| Ctrl+Alt+Space | Toggle text expander on/off |
| Super+1-9 | Quick paste Nth most recent clip |

All global hotkeys are configurable in `~/.config/paste/config.toml` under `[hotkeys]` (except Super+1-9).

## Filmstrip Navigation

Active when the filmstrip is visible.

| Shortcut | Action |
|---|---|
| Left / Right | Navigate between cards |
| Enter | Paste selected clip (rich content preserved) |
| Shift+Enter | Paste as plain text (strip formatting) |
| Double-click | Copy clip content to clipboard (does not paste at cursor) |
| Right-click | Context menu: copy, save to pinboard, toggle favorite, delete |
| Mouse wheel | Scroll filmstrip horizontally (vertical wheel maps to horizontal scroll) |
| Space | Toggle Quick Look preview |
| Escape | Close preview, clear search, or dismiss filmstrip |
| Del / Backspace | Remove selected clip from history |
| Tab | Cycle views: History, Pinboards, Snippets |

## Search

| Shortcut | Action |
|---|---|
| / or Ctrl+F | Focus search bar |
| Ctrl+F (again) | Toggle Power Search filters |
| Escape | Clear search / unfocus |

## Actions

| Shortcut | Action |
|---|---|
| Ctrl+P | Save selected clip to pinboard (opens picker) |
| Ctrl+E | Edit selected clip inline (text/code only) |
| F | Toggle favorite on selected clip |
| Ctrl+click | Add/remove clip from multi-selection |
| Shift+click | Select range of clips |

## Quick Look Preview

| Shortcut | Action |
|---|---|
| Space | Open/close preview |
| Escape | Close preview |

## In Dialogs (editors, fill-in fields, pickers)

| Shortcut | Action |
|---|---|
| Ctrl+Enter | Save/submit |
| Escape | Cancel/close |
| Tab | Next field |

## Escape Priority

Escape processes in this order:
1. Close Quick Look preview (if open)
2. Clear search (if active)
3. Dismiss filmstrip overlay
