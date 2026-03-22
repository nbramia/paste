---
name: pr-check
description: Validate a PR against project standards before requesting review
argument-hint: [pr-number]
---

# Skill: PR Check

Review a pull request against paste's PR standards.

## Trigger

The user asks to check, review, or validate a PR (e.g., "check PR #15", "review this PR").

## Instructions

### 1. Load the PR

```bash
gh pr view <number> --json title,body,headRefName,commits,additions,deletions,files
gh pr diff <number>
```

### 2. Run All 7 Checks

#### Check 1: Branch Naming

Branch must follow `<type>/<issue>-<description>` format:
- `feat/42-pinboard-crud`
- `fix/87-clipboard-race-condition`
- `refactor/103-storage-abstraction`
- `test/55-expander-coverage`
- `chore/60-update-deps`

Valid types: `feat`, `fix`, `refactor`, `test`, `docs`, `chore`, `perf`

**Pass/Fail:** Fail if branch name doesn't match pattern.

#### Check 2: Title Format

PR title must:
- Start with a type prefix: `feat:`, `fix:`, `refactor:`, `test:`, `docs:`, `chore:`, `perf:`
- Be under 72 characters
- Use imperative mood ("add", "fix", "remove" — not "added", "fixes", "removing")
- Be specific (not "fix stuff" or "updates")

**Pass/Fail:** Fail if title doesn't match format or is vague.

#### Check 3: Summary Quality

PR body must include:
- **Summary section** — what the PR does and why (not just "closes #N")
- **Changes section** — bullet list of key changes
- **Issue reference** — `Closes #N` or `Refs #N`

**Pass/Fail:** Fail if summary is missing, empty, or is just an issue reference with no context.

#### Check 4: Test Evidence

PR must include evidence that tests pass:
- Test output in the PR body, OR
- All CI checks passing, OR
- Explicit statement of what was tested and how

New code must have corresponding tests. Check that:
- New Rust functions/modules in `src-tauri/src/` have tests (unit or integration)
- New React components in `src/components/` have test files
- Modified behavior has updated tests
- Tests are meaningful (not just `assert!(true)` or `expect(true).toBe(true)`)

**Pass/Fail:** Fail if no test evidence or new code has no tests.

#### Check 5: Size

PRs should be focused:
- **Small (ideal):** < 200 lines changed, 1-3 files
- **Medium (acceptable):** 200-500 lines, 3-8 files
- **Large (needs justification):** 500+ lines or 8+ files

Large PRs are acceptable for:
- New components that span the IPC boundary (Rust backend + React frontend)
- Refactors that must be atomic
- Generated code (Tauri config, schema definitions)
- Initial scaffolding of new modules

**Pass/Fail:** Warn (not fail) if large without justification.

#### Check 6: Commit Quality

Commits should be:
- Logical units (not "WIP", "fix typo", "try again")
- Using conventional prefixes (`feat:`, `fix:`, etc.)
- Descriptive first lines under 72 characters

A single squash commit is fine for small PRs. Multiple commits should tell a coherent story for larger PRs.

**Pass/Fail:** Warn if commits are messy but the PR is otherwise good.

#### Check 7: Issue References

- PR should reference a GitHub issue
- The referenced issue should exist and be open
- The PR changes should align with what the issue describes

**Pass/Fail:** Warn if no issue reference. Fail if PR claims to close an issue but the changes don't match.

### 3. Report

```markdown
## PR Check: #<number>

| # | Check | Status | Notes |
|---|-------|--------|-------|
| 1 | Branch naming | Pass/Fail | [details] |
| 2 | Title format | Pass/Fail | [details] |
| 3 | Summary quality | Pass/Fail | [details] |
| 4 | Test evidence | Pass/Fail | [details] |
| 5 | Size | Pass/Warn/Fail | [details] |
| 6 | Commit quality | Pass/Warn | [details] |
| 7 | Issue reference | Pass/Warn/Fail | [details] |

**Overall:** Pass / Fail (N issues)

### Action Items (if any)
1. [What needs to be fixed before merge]
2. [Another item]
```

### 4. Actionable Feedback

If the PR fails any check, provide specific, actionable fixes:
- Bad title? Suggest a corrected title.
- Missing tests? Identify which functions need tests.
- Too large? Suggest how to split the PR.
- Missing summary? Draft a summary based on the diff.
