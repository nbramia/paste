---
name: draft-issue
description: Create well-structured GitHub issues optimized for /implement
argument-hint: <brief description of what needs to be done>
---

# Skill: Draft Issue

Draft a well-structured GitHub issue for the paste project.

## Trigger

The user asks to create, draft, or write a GitHub issue.

## Instructions

### 1. Gather Context

Before drafting, read:
- `README.md` — project overview, features, architecture, configuration
- `architecture.md` — component design, data flow, responsiveness targets, storage schema
- `vision.md` — design principles, roadmap, non-goals, success metrics

Understand the core architecture: clipboard monitoring (X11/Wayland) -> SQLite storage with FTS5 -> filmstrip overlay UI -> text expansion engine -> text injection (xdotool/ydotool/wtype). Understand the Tauri v2 IPC boundary between Rust backend and React frontend.

### 2. Choose Template Size

**Standard Issue** — for focused, single-concern changes (most issues):
- Touches 1-3 files
- Single component (e.g., clipboard monitor, storage, filmstrip UI, expander)
- Can be completed in one PR

**Large Issue** — for cross-cutting changes that span multiple components or the IPC boundary:
- Touches 4+ files or crosses the Rust/TypeScript boundary
- Needs sub-tasks or a phased approach
- May require multiple PRs

### 3. Draft the Issue

#### Standard Issue Template

```markdown
## Summary

[1-2 sentences. What is the problem or feature? Why does it matter?]

## Context

[What is the current behavior? What triggered this issue? Link to relevant
code in `src-tauri/src/` or `src/` if applicable.]

## Proposed Solution

[Concrete description of what to implement or change. Reference specific
files, modules, or components.]

### Changes Required

- [ ] `src-tauri/src/<module>/<file>.rs` — [description of change]
- [ ] `src/components/<Component>/index.tsx` — [description of change]
- [ ] Tests — [test coverage for the change]

## Acceptance Criteria

- [ ] [Observable, testable criterion]
- [ ] [Another criterion]
- [ ] Rust tests pass: `cargo test`
- [ ] Clippy clean: `cargo clippy -- -D warnings`
- [ ] Frontend tests pass: `npm run test`
- [ ] Lint clean: `npm run lint`

## Notes

[Optional: responsiveness impact, storage implications, alternative
approaches considered, links to relevant upstream docs.]
```

#### Large Issue Template

```markdown
## Summary

[1-2 sentences. What is the cross-cutting change?]

## Context

[Background on why this is needed. Reference architecture.md or vision.md
sections if applicable.]

## Design

[How should this be implemented? What components are affected?
What are the key design decisions? Does this cross the IPC boundary?]

### Component Impact

| Component | Module | Change | Responsiveness Impact |
|-----------|--------|--------|----------------------|
| [component] | [module] | [what changes] | [+/- Xms or none] |

## Sub-Tasks

- [ ] **Task 1:** [description] — `src-tauri/src/<module>/<file>.rs`
- [ ] **Task 2:** [description] — `src/components/<Component>/index.tsx`
- [ ] **Task 3:** [description] — tests

## Acceptance Criteria

- [ ] [Observable, testable criterion]
- [ ] [Responsiveness criterion — e.g., "overlay appears within 100ms of hotkey"]
- [ ] Rust tests pass: `cargo test`
- [ ] Clippy clean: `cargo clippy -- -D warnings`
- [ ] Frontend tests pass: `npm run test`
- [ ] Lint clean: `npm run lint`

## Notes

[Optional: phasing strategy, rollback plan, storage migration needs.]
```

### 4. Acceptance Criteria Guidance

Good acceptance criteria are:
- **Observable** — "overlay renders within 100ms of hotkey press" not "overlay is fast"
- **Testable** — can be verified with a cargo test, vitest assertion, or manual check
- **Scoped** — tied to this issue, not aspirational improvements

Always include:
- `cargo test` passes
- `cargo clippy -- -D warnings` is clean
- `npm run test` passes
- `npm run lint` is clean
- Any responsiveness impacts are documented

For performance-sensitive changes (clipboard capture, search, text expansion, overlay rendering), include specific responsiveness criteria referencing the targets in `architecture.md`:
- Overlay appearance: < 100ms
- Search results: < 50ms
- Text expansion: < 30ms
- Clipboard capture: < 50ms

### 5. Validation Checklist

Before presenting the issue:

- [ ] Title is concise and starts with a verb (Add, Fix, Update, Remove, Refactor)
- [ ] Summary explains *why*, not just *what*
- [ ] Proposed solution references specific files in `src-tauri/src/` or `src/`
- [ ] Acceptance criteria are testable
- [ ] Labels are suggested (bug, enhancement, refactor, performance, etc.)
- [ ] Issue doesn't duplicate an existing open issue (check with `gh issue list`)
- [ ] Scope is appropriate — not too large for a single PR (standard) or has sub-tasks (large)

### 6. Create the Issue

Use `gh issue create` to create the issue on GitHub:

```bash
gh issue create --title "Title here" --body "$(cat <<'EOF'
[issue body]
EOF
)"
```

Add appropriate labels with `--label` if the repository has them configured.
