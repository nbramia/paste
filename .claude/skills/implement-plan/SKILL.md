---
name: implement-plan
description: Explore codebase and plan implementation for implement workflow
context: fork
agent: general-purpose
argument-hint: <task description and optional instructions>
---

# Skill: Implement Plan

Create a concrete implementation plan for a GitHub issue.

## Trigger

Delegated from the **implement** orchestrator, or when the user asks to plan an implementation.

## Instructions

### 1. Understand the Issue

Read the issue thoroughly. Identify:
- What needs to change (features, bugs, refactors)
- Acceptance criteria (what "done" looks like)
- Constraints (responsiveness targets, storage limits, display server compatibility)

### 2. Gather Project Context

Read these files for conventions and architecture:
- `README.md` — project overview, feature list, architecture diagram, project structure
- `architecture.md` — component design, data flow, responsiveness targets, storage schema
- `vision.md` — design principles, roadmap phases, non-goals
- `src-tauri/Cargo.toml` — Rust dependencies, features, edition
- `package.json` — frontend dependencies, scripts, configuration

Understand the project structure:
```
src-tauri/
    src/
        main.rs              # Tauri entry point
        lib.rs               # Shared library, Tauri command definitions
        clipboard/           # Clipboard monitoring (wl-paste for Wayland, XFixes for X11)
        storage/             # SQLite with FTS5, image storage, clip CRUD
        expander/            # Text expander engine (snippet matching, variable substitution)
        hotkey/              # evdev global shortcuts (display-server-independent)
        injector/            # Text injection (xdotool for X11, ydotool/wtype for Wayland)
        tray/                # System tray via ksni
        config.rs            # TOML configuration loading and validation
    Cargo.toml
src/                         # React frontend (TypeScript)
    components/
        Filmstrip/           # Main filmstrip overlay (horizontal clip strip)
        Card/                # Individual clip cards (text, image, file previews)
        Search/              # Search bar and filter controls
        Pinboard/            # Pinboard management UI
        Snippet/             # Snippet/text expander management
        Settings/            # Settings panel
    hooks/                   # Custom React hooks (useClipboard, useSearch, etc.)
    stores/                  # State management
    App.tsx                  # Root component
    main.tsx                 # React entry point
package.json
```

### 3. Project Conventions

Follow these Rust + TypeScript/React conventions:

**Rust (Backend — `src-tauri/`):**
- **Edition 2021** — use modern Rust syntax
- **Strong typing** — leverage the type system; prefer newtypes and enums over primitives
- **thiserror for errors** — `#[derive(thiserror::Error)]` for custom error types, not string errors
- **serde for serialization** — `#[derive(Serialize, Deserialize)]` for IPC types and config
- **tokio for async** — async runtime for I/O-bound operations (clipboard monitoring, file I/O)
- **Clippy clean** — no warnings with `cargo clippy -- -D warnings`
- **Tauri commands** — `#[tauri::command]` for IPC, return `Result<T, String>` or custom error types
- **Logging** — use `log` crate with `tracing` or `env_logger`, not println!
- **No unsafe** — avoid unsafe code unless absolutely necessary and well-documented

**TypeScript/React (Frontend — `src/`):**
- **React 19** with strict mode enabled
- **Functional components** with hooks (no class components)
- **TailwindCSS** for styling (utility-first, no CSS modules)
- **Framer Motion** for animations (overlay transitions, card animations)
- **TypeScript strict mode** — no `any` types, explicit return types on exported functions
- **Custom hooks** for shared logic (prefixed with `use`)
- **Tauri API** — use `@tauri-apps/api` for IPC (`invoke`, `listen`)

**Testing:**
- **Rust:** `cargo test` — unit tests in `#[cfg(test)]` modules, integration tests in `tests/`
- **Frontend:** `vitest` — component tests with Testing Library, unit tests for hooks/utils
- **Mocking:** Mock evdev, display servers, clipboard access in tests

### 4. Write the Plan

Structure the plan as:

```markdown
## Plan: [Issue Title]

### Summary
[1-2 sentences on the approach]

### Files to Modify
| File | Change | Risk |
|------|--------|------|
| `src-tauri/src/<module>/<file>.rs` | [what changes] | low/medium/high |
| `src/components/<Component>/index.tsx` | [what changes] | low/medium/high |
| Tests | [what tests to add] | low |

### New Files (if any)
| File | Purpose |
|------|---------|
| `src-tauri/src/<module>/<file>.rs` | [purpose] |
| `src/components/<Component>/index.tsx` | [purpose] |

### Approach

#### Step 1: [description]
[Details of what to do and why]

#### Step 2: [description]
[Details]

### Test Strategy
- [What Rust unit tests to add]
- [What frontend component/unit tests to add]
- [What mocks/fixtures are needed]
- [What integration tests if applicable]

### Risk Assessment
| Risk | Likelihood | Impact | Mitigation |
|------|-----------|--------|------------|
| [risk] | low/medium/high | [impact] | [mitigation] |

### Responsiveness Impact
[Will this affect responsiveness targets from architecture.md?
- Overlay appearance: < 100ms
- Search results: < 50ms
- Text expansion: < 30ms
- Clipboard capture: < 50ms
State "no impact" explicitly if none.]

### Out of Scope
[Anything related but explicitly NOT part of this implementation]
```

### 5. Validate the Plan

Before presenting:
- [ ] Every acceptance criterion from the issue is addressed
- [ ] All modified files exist in the project (or are explicitly new)
- [ ] Test strategy covers the acceptance criteria
- [ ] Risk assessment is honest (not everything is "low risk")
- [ ] No unnecessary changes beyond what the issue requires
- [ ] The plan respects responsiveness targets (< 100ms overlay, < 50ms search, < 30ms expansion, < 50ms clipboard capture)
- [ ] IPC boundary is considered (Rust commands properly defined, serde types match)
- [ ] Dependencies on other issues are identified

### 6. Present and Confirm

Present the plan to the orchestrator (or user). Wait for approval before proceeding to implementation.
