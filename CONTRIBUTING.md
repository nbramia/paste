# Contributing to Paste

## How This Project Is Developed

Paste is developed using an **agent-driven workflow** — AI coding agents (Claude Code) handle implementation, while humans direct strategy, review architecture decisions, and validate the user experience.

This isn't "AI-generated code." It's a deliberate development methodology where:

- **Humans** define what to build (vision, architecture, issue scoping)
- **Agents** execute implementation within defined constraints (skills, conventions, guardrails)
- **Humans** test the running application and course-correct

The result is a codebase that's consistent, well-documented, and built to be maintained by both humans and agents.

## The Framework

### CLAUDE.md — The Operating Manual

[CLAUDE.md](CLAUDE.md) is the primary document that guides agent behavior. It defines:

- Stack conventions (what patterns to use, what to avoid)
- Architecture boundaries (where the IPC barrier is, how storage works)
- Development workflow (how to run, test, add features)
- Rules (no unwrap in production, theme tokens not raw colors, migrations for schema changes)

Every agent session loads this file automatically. It's the difference between "here's a repo, figure it out" and "here's exactly how this codebase works."

### Skills — Composable Workflows

The `.claude/skills/` directory contains 11 skill definitions that decompose development into structured, repeatable workflows:

| Skill | Role |
|-------|------|
| `/draft-issue` | Scope work into well-defined GitHub issues with acceptance criteria |
| `/implement` | **Orchestrator** — coordinates plan → code → review → merge without writing code itself |
| `/implement-plan` | Explore the codebase and produce a concrete implementation plan |
| `/implement-code` | Write code and tests on a feature branch, open a PR |
| `/implement-address` | Fix review findings from the orchestrator's review passes |
| `/merge-pr` | Verify checks pass, merge, clean up branches |
| `/pr-check` | Validate a PR against project standards (7 checks) |
| `/catchup` | Summarize recent project activity |
| `/standup` | Generate a daily status update |
| `/stale` | Find stale PRs, orphan branches, dead issues |
| `/mine-for-ideas` | Analyze a topic and produce structured recommendations |

The key design: `/implement` is an **orchestrator** that delegates to sub-skills. It doesn't write code — it coordinates planning, coding, reviewing, and merging as separate phases. This separation means each phase can be retried independently if something goes wrong.

### Hooks — Automated Guardrails

The `.claude/settings.json` defines hooks that enforce rules automatically:

- **PreToolUse**: Blocks destructive git operations (`--force`, `reset --hard`, `branch -D`) and manual publishing
- **UserPromptSubmit**: Reminds about the release workflow when "deploy" or "release" is mentioned
- **Permissions**: Whitelists safe commands (build, test, lint, git operations) for smoother workflow

These hooks run before/after every tool call — they're not suggestions, they're enforcement.

### Conventions Over Configuration

This project uses documented conventions rather than opaque configuration files. The rules live in CLAUDE.md and skill files where they're readable, auditable, and versionable. The hooks in `settings.json` enforce the most critical constraints; everything else is convention.

Why this approach:
- **Readable**: Anyone (human or agent) can read CLAUDE.md and understand the rules
- **Auditable**: Conventions are in version control with full git history
- **Flexible**: Adding a new rule is adding a line to CLAUDE.md, not debugging a config schema
- **Portable**: The same conventions work across different AI tools and workflows

## Development Workflow

### Adding a Feature

```
1. /draft-issue "Add widget support"     → Creates a scoped GitHub issue
2. /implement #42                         → Orchestrates the full lifecycle:
   ├── Phase 1: Plan (explore codebase, design approach)
   ├── Phase 2: Code (implement on feature branch, open PR)
   ├── Phase 3: Review (3 passes: correctness, testing, architecture)
   ├── Phase 4: Address (fix any findings)
   └── Phase 5: Merge (verify checks, squash merge, clean up)
```

### Running Locally

```bash
npm install
npx tauri dev          # Dev mode with hot reload
npm test               # Frontend tests (Vitest)
cd src-tauri && cargo test   # Rust tests
```

### PR Standards

PRs are validated by the `/pr-check` skill against 7 checks:

1. **Branch naming**: `feat/42-short-description`
2. **Title format**: `feat: concise description` (conventional commits)
3. **Summary quality**: Explains what and why, not just "closes #42"
4. **Test evidence**: New code has tests, test output included
5. **Size**: Focused (< 200 lines ideal, > 500 needs justification)
6. **Commit quality**: Logical units, not "WIP" or "fix typo"
7. **Issue reference**: Links to the GitHub issue

### Adding a New Skill

Create `.claude/skills/<name>/SKILL.md` with YAML frontmatter:

```markdown
---
name: my-skill
description: One-line description of what the skill does
argument-hint: <what arguments it takes>
---

# Skill: My Skill

## Trigger
When the user asks to...

## Instructions
Step-by-step instructions for the agent...
```

Then reference it in the skills table in CLAUDE.md.

## Architecture

See [architecture.md](architecture.md) for the full technical reference. The key boundary:

- **Rust backend** (`src-tauri/`): Clipboard monitoring, storage (SQLite), text expander engine, hotkey daemon, text injection
- **React frontend** (`src/`): Filmstrip UI, search, pinboards, snippets, settings
- **IPC boundary**: All Rust ↔ React communication via Tauri commands (`#[tauri::command]` → `invoke()`)

Agents must respect this boundary — no direct file access from the frontend, no UI logic in Rust.

## Code Style

- **Rust**: `cargo clippy -- -D warnings` clean, `cargo fmt` formatted, `thiserror` for errors, `log` for diagnostics
- **TypeScript**: `tsc --noEmit` clean, functional React components, TailwindCSS utility classes with semantic theme tokens
- **Commits**: Conventional format (`feat:`, `fix:`, `docs:`, `perf:`, `a11y:`)
- **Colors**: Theme tokens (`bg-accent`, `text-text-primary`) — never hardcoded hex values
- **Schema**: New tables/columns require a migration in `storage/migrations.rs`
