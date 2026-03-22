---
name: mine-for-ideas
description: Analyze a topic or open-source repo and surface ideas paste could learn from
argument-hint: <github-url | topic> [context/guidance]
---

# Skill: Mine for Ideas

Deeply analyze a topic in the context of paste and produce a structured analysis with concrete recommendations.

## Trigger

The user asks to explore, analyze, research, or think through a technical topic related to paste (e.g., "explore how other clipboard managers handle image storage", "analyze FTS5 vs tantivy for search", "think about how to support rich text snippets").

## Instructions

### 1. Gather Context

Before analyzing, build a thorough understanding of paste:

**Core documents:**
- `README.md` — features, architecture overview, configuration, project structure
- `architecture.md` — component design, data flow, responsiveness targets, storage schema, display server compatibility
- `vision.md` — design principles, roadmap, non-goals, success metrics

**Source code** (as relevant to the topic):
- `src-tauri/src/main.rs` — Tauri entry point
- `src-tauri/src/lib.rs` — shared library, Tauri command definitions
- `src-tauri/src/clipboard/` — clipboard monitoring (wl-paste for Wayland, XFixes for X11)
- `src-tauri/src/storage/` — SQLite with FTS5, clip CRUD, image storage
- `src-tauri/src/expander/` — text expander engine (snippet matching, variable substitution)
- `src-tauri/src/hotkey/` — evdev global shortcuts
- `src-tauri/src/injector/` — text injection (xdotool for X11, ydotool/wtype for Wayland)
- `src-tauri/src/tray/` — system tray via ksni
- `src-tauri/src/config.rs` — TOML configuration
- `src/components/Filmstrip/` — main filmstrip overlay UI
- `src/components/Card/` — individual clip cards
- `src/components/Search/` — search bar and filters
- `src/components/Pinboard/` — pinboard management
- `src/components/Snippet/` — snippet/text expander management
- `src/components/Settings/` — settings panel
- `src/hooks/` — custom React hooks
- `src/stores/` — state management

### 2. Frame the Analysis

Define:
- **Question:** What specific question are we trying to answer?
- **Scope:** What parts of the system are affected?
- **Constraints:** What are the hard constraints? (responsiveness targets, local-only, Linux-only, X11 + Wayland support, etc.)
- **Design principles to honor:** Which of paste's design principles (from vision.md) are most relevant?

### 3. Analyze

Structure the analysis around these dimensions (include all that are relevant):

#### Technical Feasibility
- Can this be done within paste's Tauri v2 + Rust + React architecture?
- What components are affected (clipboard, storage, filmstrip, expander, injector, tray)?
- Does it require new Rust crates or npm packages?
- Does it work on both X11 and Wayland?
- Does it cross the Tauri IPC boundary?

#### Responsiveness Impact
Reference the specific targets from architecture.md:
- Overlay appearance (hotkey to visible): < 100ms
- Search results (keystroke to results): < 50ms
- Text expansion (trigger to injected text): < 30ms
- Clipboard capture (copy to stored): < 50ms
- Paste action (select to injected): < 50ms

Will this change blow any target? Can it be made to fit?

#### Storage Impact
Reference the storage design from architecture.md:
- SQLite database size and growth rate
- FTS5 index overhead
- Image/file storage strategy (paths vs blobs)
- Retention policy and cleanup

Will this change affect storage performance, size, or schema?

#### UI/UX Impact
- Does this change the core flow (hotkey -> filmstrip -> select -> paste)?
- Does it affect the filmstrip overlay layout, animations, or responsiveness?
- Does it require new UI components or settings?
- Is it discoverable or does it add hidden complexity?
- Does it follow the existing TailwindCSS + Framer Motion patterns?

#### Clipboard Architecture
- How does this interact with the clipboard monitoring pipeline?
- Does it affect content type detection (text, image, file)?
- Are there race conditions with rapid clipboard changes?
- How does it behave across X11 and Wayland clipboard protocols?
- Does it interact with clipboard ownership/selection mechanisms?

#### Text Injection Strategy
- How does this affect text injection (xdotool/ydotool/wtype)?
- Are there special character handling concerns?
- Does it work with all target applications?
- Are there timing or focus-switching issues?
- Does it handle multiline content and formatting?

#### Display Server Compatibility
- Does this work identically on X11 and Wayland?
- Are there Wayland-specific limitations (no global window positioning, restricted clipboard access)?
- Does it require different code paths per display server?
- How does it interact with compositors (GNOME, KDE, Sway, Hyprland)?

### 4. Explore Alternatives

For each viable approach, provide:

```markdown
### Option A: [Name]

**Description:** [1-2 sentences]

**Pros:**
- [pro]
- [pro]

**Cons:**
- [con]
- [con]

**Responsiveness impact:** [+/- Xms on which target]
**Storage impact:** [+/- XMB or schema change]
**Complexity:** low / medium / high
**Fits design principles:** [which ones it honors, which it tensions]
```

Always include at least one "do nothing" option that explains the cost of inaction.

### 5. Recommendation

```markdown
## Recommendation

**Preferred:** Option [X]

**Rationale:** [2-3 sentences explaining why, referencing specific
constraints, targets, or design principles]

**Implementation effort:** [rough estimate — small/medium/large]

**Suggested next steps:**
1. [concrete action]
2. [concrete action]
3. [concrete action]

**Open questions:**
- [question that needs answering before committing]
- [another question]
```

### 6. Output Format

Present the full analysis as a structured document:

```markdown
# Analysis: [Topic]

## Question
[What are we trying to decide?]

## Context
[Relevant paste architecture and constraints]

## Options
[Option A, B, C analysis]

## Tradeoff Matrix

| Dimension | Option A | Option B | Option C |
|-----------|----------|----------|----------|
| Responsiveness impact | [value] | [value] | [value] |
| Storage impact | [value] | [value] | [value] |
| Complexity | [value] | [value] | [value] |
| UX impact | [value] | [value] | [value] |
| X11/Wayland compat | [value] | [value] | [value] |
| Fits principles | [value] | [value] | [value] |

## Recommendation
[Preferred option with rationale]

## Next Steps
[Concrete actions]
```

### Guidelines

- **Be specific.** "This adds ~20ms to clipboard capture" is useful. "This might be slower" is not.
- **Reference real numbers.** Use responsiveness measurements, storage sizes, benchmark data from the architecture docs.
- **Honor the constraints.** Don't recommend cloud-based solutions for a local-only project. Don't recommend Electron patterns for a Tauri project. Don't recommend X11-only solutions when Wayland support is required.
- **Be honest about unknowns.** If you're estimating responsiveness impact, say so. If a library's compatibility with Wayland is unknown, say so.
- **Consider the roadmap.** Does this align with the phasing in vision.md?
- **Think cross-platform within Linux.** X11 and Wayland are both first-class targets. Solutions must work on both or have clear fallback strategies.
