---
name: implement
description: Implementation, review, and merge — full lifecycle or any subset
argument-hint: <#issue | PR-number | freeform task> [instructions]
---

# Skill: Implement (Orchestrator)

Orchestrate end-to-end implementation of a GitHub issue: plan, code, review, address, merge.

## Trigger

The user asks to implement a GitHub issue (e.g., "implement #42", "work on this issue").

## Instructions

You are the **orchestrator**. You coordinate four phases and delegate to sub-skills. You do NOT write code yourself.

### Phase 0: Load Context

1. Read the issue (use `gh issue view <number>`)
2. Read project context:
   - `README.md` — project overview, architecture, features
   - `architecture.md` — component design, data flow, responsiveness targets
   - `Cargo.toml` — Rust dependencies, features
   - `package.json` — frontend dependencies, scripts
3. Verify the issue has clear acceptance criteria. If not, ask the user to clarify before proceeding.

### Phase 1: Plan

Delegate to the **implement-plan** skill.

Input: The issue number and context gathered in Phase 0.

Expected output: A concrete implementation plan with:
- Files to create or modify
- Approach for each change
- Test strategy
- Risk assessment

Review the plan. If it looks reasonable, proceed. If it has gaps, ask the planner to revise.

### Phase 2: Code

Delegate to the **implement-code** skill.

Input: The issue number and the plan from Phase 1.

Expected output:
- Implementation committed to a feature branch
- Rust tests pass: `cargo test`
- Clippy clean: `cargo clippy -- -D warnings`
- Frontend tests pass: `npm run test`
- Lint clean: `npm run lint`
- PR created with a clear description

### Phase 3: Review and Address

Run three review passes on the PR. For each, analyze the diff and check for issues:

**Review: Correctness**
- Does the code do what the issue asks?
- Are there edge cases missed?
- Are error paths handled (both Rust Result/Option and TypeScript try/catch)?
- Do the tests actually verify the acceptance criteria?

**Review: Testing**
- Is test coverage adequate?
- Are tests testing behavior, not implementation?
- Are there missing test cases (error paths, boundary conditions)?
- Do Rust tests use appropriate patterns (#[test], #[tokio::test] for async)?
- Do frontend tests use vitest patterns properly (describe, it, expect)?

**Review: Architecture**
- Does this follow paste's patterns? (Tauri commands for IPC, thiserror for Rust errors, React hooks for state, TailwindCSS for styling)
- Is Rust code in the right module? (clipboard/, storage/, expander/, hotkey/, injector/, tray/)
- Is React code in the right component? (Filmstrip/, Card/, Search/, Pinboard/, Snippet/, Settings/)
- Any unnecessary coupling between backend and frontend?
- Will this impact responsiveness targets?
- Is the Tauri IPC boundary clean (proper command definitions, serde serialization)?

Write findings to `/tmp/paste-implement-findings-<issue>-<pass>.md`.

If findings exist, delegate to the **implement-address** skill to fix them. Then re-review. Loop until clean (max 3 iterations).

### Phase 4: Merge

Once reviews are clean:
1. Verify all tests pass one final time:
   - `cargo test`
   - `cargo clippy -- -D warnings`
   - `npm run test`
   - `npm run lint`
2. Delegate to the **merge-pr** skill

### Escalation

Stop and ask the user before proceeding if any of these are true:
- Changes touch evdev input handling or global hotkey registration
- Changes modify clipboard monitoring logic (wl-paste/XFixes integration)
- Changes alter the SQLite schema or FTS5 configuration
- Changes affect text injection strategy (xdotool/ydotool/wtype selection)
- Changes modify the system tray (ksni) integration
- The plan requires adding a new dependency to `Cargo.toml` or `package.json`
- The plan changes the Tauri IPC command interface
- The plan changes the configuration schema (config.rs / TOML format)
- Any acceptance criterion is ambiguous or untestable
- Tests require real display servers, clipboard access, or evdev devices (should be mocked)

### Output Format

At each phase transition, report:
```
## Phase N Complete: [Phase Name]
**Status:** [Pass/Issues Found]
**Summary:** [1-2 sentences]
**Next:** [What happens next]
```

At completion:
```
## Implementation Complete
**Issue:** #<number>
**PR:** #<pr-number>
**Changes:** [Brief summary of what was implemented]
**Tests:** [Number of new/modified tests]
**Status:** Merged to main
```
