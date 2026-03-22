import { useState } from "react";
import type { SnippetData, SnippetGroupData } from "../../hooks/useSnippets";

interface SnippetEditorProps {
  snippet?: SnippetData;
  groups: SnippetGroupData[];
  onSave: (data: {
    abbreviation: string;
    name: string;
    content: string;
    contentType: string;
    groupId: string | null;
    description: string | null;
  }) => void;
  onClose: () => void;
}

export function SnippetEditor({ snippet, groups, onSave, onClose }: SnippetEditorProps) {
  const [abbreviation, setAbbreviation] = useState(snippet?.abbreviation || "");
  const [name, setName] = useState(snippet?.name || "");
  const [content, setContent] = useState(snippet?.content || "");
  const [contentType, setContentType] = useState(snippet?.content_type || "plain");
  const [groupId, setGroupId] = useState<string | null>(snippet?.group_id || null);
  const [description, setDescription] = useState(snippet?.description || "");
  const [error, setError] = useState<string | null>(null);

  const isEditing = !!snippet;

  const handleSave = () => {
    if (!abbreviation.trim()) {
      setError("Abbreviation is required");
      return;
    }
    if (!name.trim()) {
      setError("Name is required");
      return;
    }
    if (!content.trim()) {
      setError("Content is required");
      return;
    }
    setError(null);
    onSave({
      abbreviation: abbreviation.trim(),
      name: name.trim(),
      content,
      contentType,
      groupId,
      description: description.trim() || null,
    });
  };

  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center bg-black/50"
      onClick={onClose}
    >
      <div
        className="w-96 max-h-[80vh] overflow-y-auto rounded-lg border border-border-default bg-surface-primary p-4 shadow-xl"
        onClick={(e) => e.stopPropagation()}
        onKeyDown={(e) => {
          if (e.key === "Escape") onClose();
          e.stopPropagation();
        }}
      >
        <h3 className="mb-3 text-sm font-medium text-text-primary">
          {isEditing ? "Edit Snippet" : "Create Snippet"}
        </h3>

        {error && (
          <div className="mb-3 rounded bg-red-900/30 px-3 py-1.5 text-xs text-red-400">
            {error}
          </div>
        )}

        {/* Abbreviation */}
        <label className="mb-1 block text-xs text-text-muted">Abbreviation</label>
        <input
          type="text"
          value={abbreviation}
          onChange={(e) => setAbbreviation(e.target.value)}
          placeholder="e.g., ;sig"
          autoFocus
          className="mb-3 w-full rounded bg-surface-secondary px-3 py-1.5 text-sm text-text-primary placeholder-text-faint outline-none focus:ring-1 focus:ring-blue-500"
        />

        {/* Name */}
        <label className="mb-1 block text-xs text-text-muted">Name</label>
        <input
          type="text"
          value={name}
          onChange={(e) => setName(e.target.value)}
          placeholder="e.g., Email Signature"
          className="mb-3 w-full rounded bg-surface-secondary px-3 py-1.5 text-sm text-text-primary placeholder-text-faint outline-none focus:ring-1 focus:ring-blue-500"
        />

        {/* Content */}
        <label className="mb-1 block text-xs text-text-muted">Content</label>
        <textarea
          value={content}
          onChange={(e) => setContent(e.target.value)}
          placeholder="Expansion text..."
          rows={5}
          className="mb-2 w-full rounded bg-surface-secondary px-3 py-1.5 font-mono text-sm text-text-primary placeholder-text-faint outline-none focus:ring-1 focus:ring-blue-500"
        />
        <p className="mb-3 text-[10px] text-text-faint">
          Macros: %Y (year) %m (month) %d (day) %H (hour) %M (min) %clipboard %date(+5d) %| (cursor) %fill(name) %fillarea(name) %fillpopup(name:opt1:opt2) %shell(command) %snippet(abbr)
        </p>

        {/* Content type */}
        <label className="mb-1 block text-xs text-text-muted">Type</label>
        <select
          value={contentType}
          onChange={(e) => setContentType(e.target.value)}
          className="mb-3 w-full rounded bg-surface-secondary px-3 py-1.5 text-sm text-text-primary outline-none"
        >
          <option value="plain">Plain text</option>
          <option value="script">Shell script</option>
          <option value="fill-in">Fill-in fields</option>
        </select>

        {/* Group */}
        <label className="mb-1 block text-xs text-text-muted">Group</label>
        <select
          value={groupId || ""}
          onChange={(e) => setGroupId(e.target.value || null)}
          className="mb-3 w-full rounded bg-surface-secondary px-3 py-1.5 text-sm text-text-primary outline-none"
        >
          <option value="">No group</option>
          {groups.map((g) => (
            <option key={g.id} value={g.id}>
              {g.name}
            </option>
          ))}
        </select>

        {/* Description */}
        <label className="mb-1 block text-xs text-text-muted">Description (optional)</label>
        <input
          type="text"
          value={description}
          onChange={(e) => setDescription(e.target.value)}
          placeholder="What this snippet does"
          className="mb-4 w-full rounded bg-surface-secondary px-3 py-1.5 text-sm text-text-primary placeholder-text-faint outline-none focus:ring-1 focus:ring-blue-500"
        />

        {/* Actions */}
        <div className="flex justify-end gap-2">
          <button
            onClick={onClose}
            className="rounded px-3 py-1.5 text-xs text-text-muted hover:text-text-primary"
          >
            Cancel
          </button>
          <button
            onClick={handleSave}
            className="rounded bg-blue-600 px-3 py-1.5 text-xs font-medium text-white hover:bg-blue-500"
          >
            {isEditing ? "Save Changes" : "Create Snippet"}
          </button>
        </div>
      </div>
    </div>
  );
}
