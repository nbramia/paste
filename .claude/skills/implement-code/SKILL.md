---
name: implement-code
description: Implement code, tests, and create PR for implement workflow
context: fork
agent: general-purpose
argument-hint: <issue-number-or-0> <task description, plan, and instructions>
---

# Skill: Implement Code

Implement the plan from a GitHub issue on a feature branch and open a PR.

## Trigger

Delegated from the **implement** orchestrator after the plan is approved.

## Instructions

### 1. Set Up Branch

```bash
git checkout main
git pull origin main
git checkout -b <branch-name>
```

Branch naming: `<type>/<issue>-<short-description>`
- `feat/42-pinboard-crud`
- `fix/87-clipboard-race-condition`
- `refactor/103-storage-abstraction`

### 2. Implement the Plan

Follow the plan step-by-step. For each change:

1. Read the existing file first to understand current patterns
2. Make the change following project conventions
3. Run tests after each significant change

#### Rust Conventions (Backend — `src-tauri/`)

- **Strong typing** with custom types:
  ```rust
  #[derive(Debug, Clone, Serialize, Deserialize)]
  pub struct ClipEntry {
      pub id: i64,
      pub content: ClipContent,
      pub timestamp: chrono::DateTime<chrono::Utc>,
      pub pinned: bool,
      pub source_app: Option<String>,
  }

  #[derive(Debug, Clone, Serialize, Deserialize)]
  pub enum ClipContent {
      Text(String),
      Image { path: PathBuf, thumbnail: PathBuf },
      File(Vec<PathBuf>),
  }
  ```
- **thiserror for errors**:
  ```rust
  #[derive(Debug, thiserror::Error)]
  pub enum StorageError {
      #[error("Database error: {0}")]
      Database(#[from] rusqlite::Error),
      #[error("Clip not found: {0}")]
      NotFound(i64),
      #[error("FTS index error: {0}")]
      FtsError(String),
  }
  ```
- **Tauri commands** for IPC:
  ```rust
  #[tauri::command]
  pub async fn search_clips(
      query: String,
      limit: Option<usize>,
      state: tauri::State<'_, AppState>,
  ) -> Result<Vec<ClipEntry>, String> {
      state.storage.search(&query, limit.unwrap_or(50))
          .map_err(|e| e.to_string())
  }
  ```
- **Logging** via the `log` crate:
  ```rust
  use log::{info, warn, error, debug};
  info!("Clipboard entry stored: id={}, type={:?}", entry.id, entry.content_type());
  ```
- **Error handling** — use `Result<T, E>` and `?` operator, not `.unwrap()`:
  ```rust
  pub fn get_clip(&self, id: i64) -> Result<ClipEntry, StorageError> {
      let conn = self.pool.get()?;
      let entry = conn.query_row(
          "SELECT * FROM clips WHERE id = ?1",
          params![id],
          |row| ClipEntry::try_from(row),
      ).map_err(|_| StorageError::NotFound(id))?;
      Ok(entry)
  }
  ```
- **tokio for async**:
  ```rust
  pub async fn monitor_clipboard(&self) -> Result<(), ClipboardError> {
      let mut rx = self.watcher.subscribe();
      while let Ok(event) = rx.recv().await {
          self.handle_clip_event(event).await?;
      }
      Ok(())
  }
  ```

#### TypeScript/React Conventions (Frontend — `src/`)

- **Functional components with hooks**:
  ```tsx
  interface CardProps {
    clip: ClipEntry;
    onSelect: (id: number) => void;
    isActive: boolean;
  }

  export function Card({ clip, onSelect, isActive }: CardProps) {
    const previewText = useClipPreview(clip);
    return (
      <motion.div
        className={cn("rounded-lg p-3 cursor-pointer", isActive && "ring-2 ring-blue-500")}
        onClick={() => onSelect(clip.id)}
        whileHover={{ scale: 1.02 }}
      >
        <p className="text-sm text-gray-200 truncate">{previewText}</p>
      </motion.div>
    );
  }
  ```
- **Custom hooks for Tauri IPC**:
  ```tsx
  import { invoke } from "@tauri-apps/api/core";

  export function useClipboard() {
    const [clips, setClips] = useState<ClipEntry[]>([]);

    const search = useCallback(async (query: string) => {
      const results = await invoke<ClipEntry[]>("search_clips", { query });
      setClips(results);
    }, []);

    return { clips, search };
  }
  ```
- **TailwindCSS** for all styling (no inline styles or CSS modules)
- **Framer Motion** for animations (overlay enter/exit, card transitions)
- **TypeScript strict** — no `any`, explicit interfaces for all props and IPC types

### 3. Write Tests

For every change, write or update tests:

#### Rust Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clip_entry_serialization() {
        let entry = ClipEntry {
            id: 1,
            content: ClipContent::Text("hello".into()),
            timestamp: chrono::Utc::now(),
            pinned: false,
            source_app: Some("firefox".into()),
        };
        let json = serde_json::to_string(&entry).unwrap();
        let deserialized: ClipEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.id, entry.id);
    }

    #[test]
    fn test_fts_search_ranking() {
        let storage = TestStorage::new_in_memory();
        storage.insert_text("hello world").unwrap();
        storage.insert_text("hello there").unwrap();
        let results = storage.search("hello", 10).unwrap();
        assert_eq!(results.len(), 2);
    }

    #[tokio::test]
    async fn test_clipboard_monitor_receives_events() {
        let (tx, rx) = tokio::sync::broadcast::channel(16);
        let monitor = ClipboardMonitor::new_with_channel(tx);
        // ... mock clipboard event
    }
}
```

#### Frontend Tests

```tsx
// src/components/Card/Card.test.tsx
import { describe, it, expect, vi } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import { Card } from "./Card";

describe("Card", () => {
  const mockClip: ClipEntry = {
    id: 1,
    content: { type: "text", value: "Hello world" },
    timestamp: new Date().toISOString(),
    pinned: false,
  };

  it("renders clip preview text", () => {
    render(<Card clip={mockClip} onSelect={vi.fn()} isActive={false} />);
    expect(screen.getByText("Hello world")).toBeInTheDocument();
  });

  it("calls onSelect when clicked", () => {
    const onSelect = vi.fn();
    render(<Card clip={mockClip} onSelect={onSelect} isActive={false} />);
    fireEvent.click(screen.getByText("Hello world"));
    expect(onSelect).toHaveBeenCalledWith(1);
  });
});
```

Testing guidelines:
- Test behavior, not implementation details
- Mock external dependencies (evdev, display servers, clipboard access, SQLite for frontend)
- Use in-memory SQLite for Rust storage tests
- Use `@testing-library/react` for component tests
- Mock Tauri `invoke` in frontend tests
- Use `tmp_path` or tempdir for file-based tests

### 4. Verify

Run the full verification suite:

```bash
# Rust tests pass
cargo test

# Clippy clean
cargo clippy -- -D warnings

# Frontend tests pass
npm run test

# Frontend lint clean
npm run lint

# Tauri build check (verify IPC types match)
cargo check
```

All must pass before proceeding.

### 5. Commit

Make focused commits. Each commit should be a logical unit:

```bash
git add src-tauri/src/storage/mod.rs src-tauri/src/storage/fts.rs
git commit -m "feat: add FTS5 full-text search for clips

Implement SQLite FTS5 virtual table for clip content search.
Supports prefix matching, phrase queries, and BM25 ranking.
Search results return within 50ms for 10K+ entries.

Closes #42"
```

Commit message conventions:
- Prefix: `feat:`, `fix:`, `refactor:`, `test:`, `docs:`, `chore:`, `perf:`
- First line: imperative mood, under 72 characters
- Body: explain *why*, not just *what*
- Reference issue with `Closes #N` or `Refs #N`

### 6. Open PR

```bash
git push -u origin <branch-name>
```

Create the PR:

```bash
gh pr create --title "<type>: <concise description>" --body "$(cat <<'EOF'
## Summary

[1-3 sentences on what this PR does and why]

Closes #<issue-number>

## Changes

- [Bullet list of key changes]
- [Another change]

## Test Evidence

```
$ cargo test
[paste relevant output or summary]

$ cargo clippy -- -D warnings
[no warnings]

$ npm run test
[paste relevant output or summary]

$ npm run lint
All checks passed!
```

## Responsiveness Impact

[State impact or "No impact on responsiveness targets."]

## Checklist

- [ ] Rust tests pass: `cargo test`
- [ ] Clippy clean: `cargo clippy -- -D warnings`
- [ ] Frontend tests pass: `npm run test`
- [ ] Lint clean: `npm run lint`
- [ ] Tauri IPC types match between Rust and TypeScript
- [ ] Logging (not println!) for diagnostics
- [ ] Acceptance criteria from issue are met
EOF
)"
```

### 7. Report

Report back to the orchestrator:
- PR number and URL
- Summary of changes
- Test results (pass count, new tests added)
- Any deviations from the plan and why
