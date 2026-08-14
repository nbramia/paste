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

### Verification gate

All four pass before a PR goes up, and all four are standing acceptance criteria
on every issue:

```bash
cargo test                      # in src-tauri/
cargo clippy -- -D warnings     # in src-tauri/
npm run test                    # vitest
npm run lint                    # tsc --noEmit
```

`cargo check` on top of those confirms the Rust and TypeScript sides of the IPC
boundary still line up after a command signature changes.

### Test patterns

- Storage tests run against in-memory SQLite; file-based tests use a tempdir.
- Mock everything external — evdev devices, the display server, clipboard
  access, and Tauri `invoke` in frontend tests.
- Test behaviour, not implementation. A test that needs real hardware belongs
  behind a mock instead.

### Testing input, focus and paste without a human

`src-tauri/examples/e2e_probe.rs` creates a uinput keyboard and emits real key
events, so the hotkey -> overlay -> paste flow can be driven end to end from a
script. The hotkey daemon sees it exactly as it sees a physical keyboard.

```bash
cargo build --release --example e2e_probe
./target/release/examples/e2e_probe ctrl-alt-v right enter
```

Two things it must respect:

- The device name carries the `XWayKeyz (virtual)` prefix so xwaykeyz-based
  keymappers (Toshy) leave it alone; without that the chord is remapped before
  Paste sees it. It avoids the `paste-injection` token, which is how Paste's own
  daemon excludes its injector device.
- Wait out `HOTPLUG_SCAN_INTERVAL` (2s) before emitting — the daemon has to
  adopt the new device first, or the keys go nowhere. `PROBE_WARMUP_MS`
  controls this.

Pair it with a GTK target window that writes its buffer to a file to verify a
paste actually landed. Prefer this over asking the user to test by hand: it
turned an unfalsifiable "focus feels broken" report into a measurement (1/5
before the fix, 13/13 after).

### Escalation

Stop and ask a human before implementing any of these:

| Trigger |
|---------|
| evdev input handling or global hotkey registration |
| the xclip poll loop in `clipboard/wayland.rs` or its dedup rules |
| SQLite schema changes (they need a migration in `storage/migrations.rs`) |
| injection-backend selection behind the `Injector` trait |
| system tray integration |
| a new dependency in `src-tauri/Cargo.toml` or `package.json` |
| the Tauri IPC command interface |
| the config schema in `config.rs` |
| an acceptance criterion that is ambiguous or untestable |
| a test that would need a real display server, clipboard, or evdev device |

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

## Framework Philosophy

This project uses **conventions over configuration**. Agent behavior is defined in this file and the skill files — not in opaque config schemas. The rules are readable, auditable, and versionable. Hooks in `.claude/settings.json` enforce the most critical constraints (destructive operations, release workflow); everything else is convention.

The `/implement` orchestrator writes review findings to `/tmp/paste-implement-findings-<issue>-<pass>.md`. The `/implement-address` skill reads and resolves them. This is the inter-skill communication pattern.

## Rules

- Never commit to main directly for feature work — always branch + PR
- PR titles use conventional commits: `feat:`, `fix:`, `refactor:`, `docs:`, `perf:`, `a11y:`
- Branch naming: `feat/N-short-description`, `fix/N-description`
- Every Tauri command must be registered in `invoke_handler` in lib.rs
- Every new CSS color must be a theme token in `@theme` block — no hardcoded hex in components
- Schema changes require a new migration in `storage/migrations.rs`
- Maintain documentation as you go — a change that alters behavior, adds a component boundary, or invalidates something in `architecture.md` / `vision.md` / this file updates those docs in the same PR, not later
- New snippets should be loaded into the expander matcher on change
- Force push, hard reset, and recursive deletes of a root or home path are blocked by `scripts/guard-destructive.py` — use the PR workflow
- When wrapping up a body of work, cut a **GitHub release**. This is a public open-source repo: be deliberate. Bump the version in `src-tauri/Cargo.toml`, `src-tauri/tauri.conf.json` and `package.json` together, write release notes aimed at users (not a commit log), and update `README.md` when behavior or setup changed. Releases are built by `.github/workflows/release.yml` — never publish artifacts by hand.
- Merging is not shipping — after a merge to main, rebuild and restart the running app so the merged code is actually live:
  ```bash
  npm run build && cd src-tauri && cargo build --release
  systemctl --user restart paste.service
  ```
  The systemd unit runs `src-tauri/target/release/paste`, so a merge alone changes nothing on the machine. Verify with `systemctl --user status paste.service` and confirm the binary timestamp is newer than the merge.
