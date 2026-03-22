---
name: stale
description: Housekeeping report — find stale PRs, orphan branches, and stale issues
argument-hint: [days]
---

# Skill: Stale

Find stale PRs, orphan branches, and stale issues that need attention or cleanup.

## Trigger

The user asks about stale items, cleanup, or housekeeping (e.g., "find stale PRs", "what needs cleanup", "any dead branches").

## Instructions

### 1. Scan for Stale PRs

```bash
# All open PRs with their last update time
gh pr list --state open --json number,title,updatedAt,headRefName,author,reviewDecision
```

Flag PRs as stale if:
- No activity in 7+ days
- Has merge conflicts
- Has requested changes that haven't been addressed
- CI is failing

### 2. Scan for Orphan Branches

```bash
# Remote branches
git fetch --prune
git branch -r --list 'origin/*' | grep -v 'origin/main' | grep -v 'origin/HEAD'

# For each branch, check if it has an associated open PR
# Orphan = branch exists but no open PR references it
```

Flag branches as orphan if:
- No open PR references the branch
- Last commit is 14+ days old
- Branch has been merged to main (should have been deleted)

### 3. Scan for Stale Issues

```bash
# Open issues sorted by update time
gh issue list --state open --json number,title,updatedAt,labels,assignees --limit 50
```

Flag issues as stale if:
- No activity in 30+ days
- Assigned but no associated PR or branch
- Labeled as blocked with no recent update
- Superseded by other issues or changes

### 4. Find Quick Wins

Look for items that can be cleaned up immediately:
- Merged branches that weren't deleted
- PRs that are clearly abandoned (no activity in 30+ days)
- Issues that have been completed but not closed
- Draft PRs with no activity

### 5. Report

```markdown
## Stale Items Report

### Stale PRs (no activity in 7+ days)
| PR | Title | Last Updated | Issue |
|----|-------|-------------|-------|
| #N | [title] | [date] | [what's blocking it] |

### Orphan Branches (no open PR)
| Branch | Last Commit | Age | Action |
|--------|-------------|-----|--------|
| `branch-name` | [date] | [N days] | Delete / Investigate |

### Stale Issues (no activity in 30+ days)
| Issue | Title | Last Updated | Suggestion |
|-------|-------|-------------|------------|
| #N | [title] | [date] | Close / Update / Reprioritize |

### Quick Wins (can be cleaned up now)
- [ ] Delete merged branch `branch-name`
- [ ] Close completed issue #N
- [ ] Close abandoned PR #N

### Summary
- **Stale PRs:** N
- **Orphan branches:** N
- **Stale issues:** N
- **Quick wins:** N items ready for immediate cleanup
```

### 6. Offer to Clean Up

After presenting the report, offer to:
- Delete orphan branches (with confirmation)
- Close abandoned PRs (with a comment explaining why)
- Close stale issues (with a comment)
- Update stale items with a ping comment

Always ask before taking destructive action (deleting branches, closing issues/PRs).

### Guidelines

- Be conservative with "stale" labels. A PR with complex changes that's 8 days old isn't necessarily stale.
- Context matters: a PR waiting on a dependency isn't stale, it's blocked.
- For orphan branches, check if they might be someone's experimental work before suggesting deletion.
- Quick wins should be genuinely quick and non-controversial.
