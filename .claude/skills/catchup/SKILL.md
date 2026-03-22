---
name: catchup
description: Synthesize recent PR activity into a cross-PR net-effects briefing
argument-hint: "[hours] [me|others]"
---

# Skill: Catchup

Summarize recent project activity to get up to speed quickly.

## Trigger

The user asks to catch up on recent activity (e.g., "catch me up", "what happened recently", "what's new").

## Instructions

### 1. Determine Time Range

Default to the last 24 hours. If the user specifies a time range, use that instead.

Calculate the cutoff timestamp:
```bash
# Last H hours (Linux)
SINCE=$(date -u -d "${H} hours ago" --iso-8601=seconds)
```

### 2. Gather Activity

Run these in parallel:

```bash
# Recent commits on main
git log --oneline --since="$SINCE" main

# Recent commits on all branches
git log --oneline --since="$SINCE" --all

# Open PRs with recent activity
gh pr list --state open --json number,title,updatedAt,headRefName,author

# Recently merged PRs
gh pr list --state merged --json number,title,mergedAt,headRefName --limit 10

# Recently closed/opened issues
gh issue list --state all --json number,title,state,updatedAt,createdAt --limit 20
```

### 3. Read Context

For any significant changes (merged PRs, large commits), read relevant context:
- PR descriptions and review comments
- Commit messages
- Changed files (to understand scope)
- `README.md` for any project-level changes

### 4. Synthesize

Organize the summary by importance:

```markdown
## Catchup: [time range]

### Merged (shipped)
[PRs merged to main, most significant first]

- **#N: title** — [1-sentence summary of what changed and why]
- **#N: title** — [1-sentence summary]

### In Progress
[Open PRs with recent activity]

- **#N: title** (branch: `name`) — [status: ready for review / WIP / has conflicts]

### Issues
[Recently opened or updated issues]

- **#N: title** — [new / updated / closed]

### Key Changes
[If any commits changed important files, highlight them]

- `src-tauri/src/<module>/<file>.rs` — [what changed]
- `src/components/<Component>/index.tsx` — [what changed]
- `architecture.md` — [what changed]

### Nothing Happened
[If the time range has no activity, say so clearly]

No commits, PRs, or issue updates in the last [time range].
```

### 5. Highlight Important Items

Flag anything that needs attention:
- PRs that need review
- Failing CI checks
- Issues that are blocked
- Breaking changes to the IPC interface, storage schema, or config format
- Changes to `architecture.md` or `vision.md` (project direction changes)

### Guidelines

- Keep it concise. This is a summary, not a changelog.
- Lead with what matters most (merged changes > open PRs > issues).
- Include PR/issue numbers so the user can dig deeper.
- If there's no activity, say so immediately rather than generating filler.
- Focus on *what changed and why*, not file-level diffs.
