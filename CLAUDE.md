# Paste — Agent Development Guide

@vision.md
@architecture.md

## Project

Paste is a clipboard manager + text expander for Linux, built with Tauri v2 (Rust backend) + React 19 / TypeScript (frontend). It runs on X11 and Wayland.

## Stack & Conventions

### Rust (src-tauri/)
- Edition 2021, stable toolchain
- `thiserror` for error types, `log` for structured logging
- `serde` + `serde_json` for serialization (all Tauri IPC types must derive Serialize/Deserialize)
- Storage: `rusqlite` with `bundled-full` — all queries use parameterized statements, never string interpolation
- No `unwrap()` in production paths — use `map_err(|e| e.to_string())` for Tauri commands, proper error propagation internally
- Clipboard monitoring polls via `xclip` (XWayland) to avoid `wl-paste` desktop side-effects
- Text injection: ydotool for typing, xdotool for backspaces (ydotool key codes are broken on some versions)

### TypeScript (src/)
- React 19 with functional components and hooks only
- TailwindCSS v4 with `@tailwindcss/vite` plugin (required — without it, styles don't load)
- Framer Motion for animations — wrap in `AnimatePresence` for mount/unmount
- All Tauri IPC via `invoke()` from `@tauri-apps/api/core`
- Event listening via `listen()` from `@tauri-apps/api/event`
- Components use semantic theme tokens (`bg-surface-card`, `text-text-primary`, `bg-accent`) — never raw color values

### Fonts & Design
- IBM Plex Sans for headings (font-heading class)
- Public Sans for body text (set on body)
- Warm gray + amber/gold accent palette
- All accent colors use `accent`, `accent-hover`, `accent-soft`, `accent-muted` tokens

## Architecture Boundaries

- **Rust ↔ React**: All communication via Tauri commands (Rust `#[tauri::command]` → TypeScript `invoke()`). No direct file access from frontend.
- **Storage**: All DB access goes through `Storage` struct methods. Never raw SQL in Tauri commands.
- **Clipboard → Storage**: Clipboard monitor sends `ClipItem` via `mpsc::channel` → receiver thread calls `storage.insert_clip()` → emits `clip-added` event → frontend reloads.
- **Injector**: All text injection goes through the `Injector` trait. Never spawn xdotool/ydotool directly from Tauri commands.

## Development Workflow

### Running
```bash
npm install              # first time only
npx tauri dev            # dev mode (NOT cargo tauri dev)
```

### Testing
```bash
npm test                 # frontend (Vitest + React Testing Library)
cd src-tauri && cargo test  # Rust (requires GTK system libs)
```

### Adding a feature
1. `/draft-issue` — creates a well-scoped GitHub issue with acceptance criteria
2. `/implement #N` — orchestrates: plan → code → review → address → merge
3. The implement skill delegates to sub-skills and does NOT write code itself

### Skills overview
| Skill | Purpose |
|-------|---------|
| `/draft-issue` | Create GitHub issues optimized for agent implementation |
| `/implement` | Full lifecycle orchestrator: plan → code → review → merge |
| `/implement-plan` | Explore codebase, produce implementation plan |
| `/implement-code` | Write code, tests, commit, open PR |
| `/implement-address` | Fix review findings |
| `/merge-pr` | Verify checks, merge, clean up |
| `/pr-check` | Validate PR against project standards |
| `/catchup` | Summarize recent project activity |
| `/standup` | Personal daily summary |
| `/stale` | Find stale PRs, orphan branches, dead issues |
| `/mine-for-ideas` | Analyze a topic and surface actionable ideas |

## Rules

- Never commit to main directly for feature work — always branch + PR
- PR titles use conventional commits: `feat:`, `fix:`, `refactor:`, `docs:`, `perf:`, `a11y:`
- Branch naming: `feat/N-short-description`, `fix/N-description`
- Every Tauri command must be registered in `invoke_handler` in lib.rs
- Every new CSS color must be a theme token in `@theme` block — no hardcoded hex in components
- Schema changes require a new migration in `storage/migrations.rs`
- New snippets should be loaded into the expander matcher on change
