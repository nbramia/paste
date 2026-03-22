---
name: standup
description: Personal daily summary — what you shipped, what's in progress, what needs attention
argument-hint: [hours]
---

# Skill: Standup

Generate a standup update summarizing recent work and next steps.

## Trigger

The user asks for a standup update (e.g., "standup", "what did I do yesterday", "generate standup").

## Instructions

### 1. Determine Time Range

Default to the last 24 hours. Adjust if the user specifies (e.g., "since Friday" for a Monday standup).

```bash
# Last H hours (Linux)
SINCE=$(date -u -d "${H} hours ago" --iso-8601=seconds)
```

### 2. Gather Activity

```bash
# My recent commits
git log --oneline --since="$SINCE" --author="$(git config user.name)" --all

# PRs I created or updated
gh pr list --state all --author "@me" --json number,title,state,updatedAt,mergedAt

# Issues I'm assigned to
gh issue list --assignee "@me" --state open --json number,title,state

# Issues I commented on recently
gh issue list --state all --json number,title,state,updatedAt --limit 20
```

### 3. Format Standup

```markdown
## Standup — [date]

### Done (since last standup)
- [Completed item — PR merged, issue closed, feature implemented]
- [Another completed item]

### In Progress
- [What I'm currently working on — with PR or issue reference]
- [Another in-progress item]

### Blocked (if any)
- [What's blocked and why]

### Next
- [What I plan to work on next]
- [Another planned item]
```

### 4. Guidelines

- Keep each bullet to 1 line
- Reference PR and issue numbers
- Lead with the most impactful work
- "Done" = merged or shipped, not just committed
- "In Progress" = has a branch or open PR
- Only include "Blocked" if something is actually blocked
- "Next" should be concrete (specific issue numbers), not vague
- If there's genuinely no activity, say "Light day — no commits or PR activity" rather than inventing filler
