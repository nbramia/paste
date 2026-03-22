import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { SnippetData, SnippetGroupData } from "../../hooks/useSnippets";
import { SnippetEditor } from "./SnippetEditor";

interface SnippetViewProps {
  snippets: SnippetData[];
  groups: SnippetGroupData[];
  onCreateSnippet: (data: {
    abbreviation: string;
    name: string;
    content: string;
    contentType: string;
    groupId: string | null;
    description: string | null;
  }) => Promise<void>;
  onUpdateSnippet: (
    id: string,
    data: {
      abbreviation: string;
      name: string;
      content: string;
      contentType: string;
      groupId: string | null;
      description: string | null;
    },
  ) => Promise<void>;
  onDeleteSnippet: (id: string) => Promise<void>;
  onCreateGroup: (name: string) => Promise<void>;
  onDeleteGroup: (id: string) => Promise<void>;
  onReload: () => Promise<void>;
  loading: boolean;
}

export function SnippetView({
  snippets,
  groups,
  onCreateSnippet,
  onUpdateSnippet,
  onDeleteSnippet,
  onCreateGroup,
  onDeleteGroup,
  onReload,
  loading,
}: SnippetViewProps) {
  const [showEditor, setShowEditor] = useState(false);
  const [editingSnippet, setEditingSnippet] = useState<SnippetData | undefined>();
  const [showNewGroup, setShowNewGroup] = useState(false);
  const [newGroupName, setNewGroupName] = useState("");
  const [importMessage, setImportMessage] = useState<string | null>(null);

  const handleImportEspanso = async () => {
    try {
      const result = await invoke<{ imported: number; skipped: number; errors: string[] }>(
        "import_espanso",
        {}
      );
      const parts = [];
      if (result.imported > 0) parts.push(`${result.imported} imported`);
      if (result.skipped > 0) parts.push(`${result.skipped} skipped (duplicates)`);
      if (result.errors.length > 0) parts.push(`${result.errors.length} errors`);
      setImportMessage(parts.join(", ") || "No snippets found");
      await onReload();
      setTimeout(() => setImportMessage(null), 5000);
    } catch (err) {
      setImportMessage(`Import failed: ${err}`);
      setTimeout(() => setImportMessage(null), 5000);
    }
  };

  if (loading) {
    return (
      <div className="flex flex-1 items-center justify-center text-text-muted">
        Loading...
      </div>
    );
  }

  const ungrouped = snippets.filter((s) => !s.group_id);
  const grouped = groups.map((g) => ({
    group: g,
    snippets: snippets.filter((s) => s.group_id === g.id),
  }));

  const handleCreate = () => {
    setEditingSnippet(undefined);
    setShowEditor(true);
  };

  const handleEdit = (snippet: SnippetData) => {
    setEditingSnippet(snippet);
    setShowEditor(true);
  };

  const handleSave = async (data: {
    abbreviation: string;
    name: string;
    content: string;
    contentType: string;
    groupId: string | null;
    description: string | null;
  }) => {
    try {
      if (editingSnippet) {
        await onUpdateSnippet(editingSnippet.id, data);
      } else {
        await onCreateSnippet(data);
      }
      setShowEditor(false);
      setEditingSnippet(undefined);
    } catch (err) {
      console.error("Failed to save snippet:", err);
    }
  };

  const handleCreateGroup = async () => {
    if (newGroupName.trim()) {
      await onCreateGroup(newGroupName.trim());
      setNewGroupName("");
      setShowNewGroup(false);
    }
  };

  return (
    <div className="flex flex-1 flex-col overflow-y-auto p-4">
      {/* Header */}
      <div className="mb-3 flex items-center justify-between">
        <h2 className="text-sm font-medium text-text-secondary">
          Snippets ({snippets.length})
        </h2>
        <div className="flex gap-2">
          <button
            onClick={handleImportEspanso}
            className="rounded px-2 py-1 text-xs text-text-muted hover:text-text-primary"
            title="Import snippets from espanso config"
          >
            Import espanso
          </button>
          <button
            onClick={() => setShowNewGroup(true)}
            className="rounded px-2 py-1 text-xs text-text-muted hover:text-text-primary"
          >
            + Group
          </button>
          <button
            onClick={handleCreate}
            className="rounded bg-blue-600 px-2 py-1 text-xs font-medium text-white hover:bg-blue-500"
          >
            + Snippet
          </button>
        </div>
      </div>

      {/* Import feedback */}
      {importMessage && (
        <div className="mb-3 rounded bg-surface-secondary px-3 py-2 text-xs text-text-secondary">
          {importMessage}
        </div>
      )}

      {/* New group input */}
      {showNewGroup && (
        <div className="mb-3 flex gap-2">
          <input
            type="text"
            value={newGroupName}
            onChange={(e) => setNewGroupName(e.target.value)}
            placeholder="Group name"
            autoFocus
            onKeyDown={(e) => {
              if (e.key === "Enter") handleCreateGroup();
              if (e.key === "Escape") setShowNewGroup(false);
              e.stopPropagation();
            }}
            className="flex-1 rounded bg-surface-secondary px-2 py-1 text-sm text-text-primary placeholder-text-faint outline-none"
          />
          <button
            onClick={handleCreateGroup}
            className="rounded bg-blue-600 px-2 py-1 text-xs text-white"
          >
            Add
          </button>
          <button
            onClick={() => setShowNewGroup(false)}
            className="text-xs text-text-muted"
          >
            Cancel
          </button>
        </div>
      )}

      {/* Empty state */}
      {snippets.length === 0 && (
        <div className="flex flex-1 items-center justify-center text-text-muted">
          <div className="text-center">
            <p className="text-sm">No snippets yet</p>
            <p className="mt-1 text-xs">
              Create a snippet to start expanding text
            </p>
          </div>
        </div>
      )}

      {/* Grouped snippets */}
      {grouped.map(({ group, snippets: groupSnippets }) => (
        <div key={group.id} className="mb-4">
          <div className="group mb-1.5 flex items-center gap-2">
            <span className="text-xs font-medium uppercase tracking-wider text-text-muted">
              {group.name}
            </span>
            <button
              onClick={() => onDeleteGroup(group.id)}
              className="text-text-faint opacity-0 hover:text-red-400 group-hover:opacity-100"
              title="Delete group"
            >
              <svg className="h-3 w-3" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                <path d="M18 6L6 18M6 6l12 12" />
              </svg>
            </button>
          </div>
          <div className="space-y-1">
            {groupSnippets.map((s) => (
              <SnippetCard key={s.id} snippet={s} onEdit={handleEdit} onDelete={onDeleteSnippet} />
            ))}
            {groupSnippets.length === 0 && (
              <p className="text-xs text-text-faint pl-2">No snippets in this group</p>
            )}
          </div>
        </div>
      ))}

      {/* Ungrouped snippets */}
      {ungrouped.length > 0 && (
        <div className="mb-4">
          {groups.length > 0 && (
            <div className="mb-1.5">
              <span className="text-xs font-medium uppercase tracking-wider text-text-muted">
                Ungrouped
              </span>
            </div>
          )}
          <div className="space-y-1">
            {ungrouped.map((s) => (
              <SnippetCard key={s.id} snippet={s} onEdit={handleEdit} onDelete={onDeleteSnippet} />
            ))}
          </div>
        </div>
      )}

      {/* Editor modal */}
      {showEditor && (
        <SnippetEditor
          snippet={editingSnippet}
          groups={groups}
          onSave={handleSave}
          onClose={() => {
            setShowEditor(false);
            setEditingSnippet(undefined);
          }}
        />
      )}
    </div>
  );
}

/** Individual snippet card */
function SnippetCard({
  snippet,
  onEdit,
  onDelete,
}: {
  snippet: SnippetData;
  onEdit: (s: SnippetData) => void;
  onDelete: (id: string) => void;
}) {
  const preview =
    snippet.content.length > 60
      ? snippet.content.slice(0, 60) + "\u2026"
      : snippet.content;

  return (
    <div className="group flex items-center gap-3 rounded border border-border-default bg-surface-card px-3 py-2">
      {/* Abbreviation */}
      <code className="shrink-0 rounded bg-blue-900/30 px-1.5 py-0.5 text-xs font-bold text-blue-400">
        {snippet.abbreviation}
      </code>

      {/* Name + preview */}
      <div className="min-w-0 flex-1">
        <p className="truncate text-xs font-medium text-text-primary">
          {snippet.name}
        </p>
        <p className="truncate text-[10px] text-text-faint">{preview}</p>
      </div>

      {/* Use count */}
      {snippet.use_count > 0 && (
        <span className="shrink-0 text-[10px] text-text-faint">
          {snippet.use_count}x
        </span>
      )}

      {/* Actions */}
      <div className="flex shrink-0 gap-1 opacity-0 group-hover:opacity-100">
        <button
          onClick={() => onEdit(snippet)}
          className="text-text-faint hover:text-text-primary"
          title="Edit"
        >
          <svg className="h-3.5 w-3.5" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
            <path d="M11 4H4a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2v-7" />
            <path d="M18.5 2.5a2.121 2.121 0 0 1 3 3L12 15l-4 1 1-4 9.5-9.5z" />
          </svg>
        </button>
        <button
          onClick={() => onDelete(snippet.id)}
          className="text-text-faint hover:text-red-400"
          title="Delete"
        >
          <svg className="h-3.5 w-3.5" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
            <path d="M18 6L6 18M6 6l12 12" />
          </svg>
        </button>
      </div>
    </div>
  );
}

export { SnippetEditor } from "./SnippetEditor";
