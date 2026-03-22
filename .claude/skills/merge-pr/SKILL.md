---
name: merge-pr
description: Merge a PR and update upstream GitHub issues with progress
argument-hint: <pr-number>
---

# Skill: Merge PR

Merge an approved pull request.

## Trigger

Delegated from the **implement** orchestrator, or when the user asks to merge a PR.

## Instructions

### 1. Pre-Merge Checks

Before merging, verify:

```bash
# PR is approved or has no required reviewers
gh pr view <number> --json reviewDecision,state,mergeable

# Check CI status (if configured)
gh pr checks <number>

# Tests pass on the branch
git checkout <branch>

# Rust verification
cargo test
cargo clippy -- -D warnings

# Frontend verification
npm run test
npm run lint
```

All must pass. If any fail, stop and report back.

### 2. Check for Conflicts

```bash
gh pr view <number> --json mergeable
```

If there are merge conflicts:
1. Rebase the branch on main:
   ```bash
   git checkout <branch>
   git fetch origin main
   git rebase origin/main
   ```
2. Resolve conflicts
3. Run tests again after conflict resolution
4. Push the rebased branch:
   ```bash
   git push --force-with-lease
   ```

### 3. Merge

Use squash merge for clean history:

```bash
gh pr merge <number> --squash --delete-branch
```

If the PR has multiple meaningful commits that tell a useful story, use a regular merge:

```bash
gh pr merge <number> --merge --delete-branch
```

### 4. Post-Merge

```bash
# Switch back to main
git checkout main
git pull origin main

# Verify the merge
git log --oneline -5
```

### 5. Report

```markdown
## PR Merged

**PR:** #<number>
**Branch:** <branch-name> (deleted)
**Merge method:** squash / merge
**Commit:** <short-sha>
**Status:** Merged to main
```

### Error Handling

| Situation | Action |
|-----------|--------|
| PR has merge conflicts | Rebase and resolve, then retry |
| Tests fail on branch | Report back, do not merge |
| PR is in draft state | Report back, do not merge |
| PR has failing CI checks | Report back, do not merge |
| Branch protection rules block merge | Report the specific blocker to the user |
