import { useState } from "react";

interface CreatePinboardDialogProps {
  initialName?: string;
  initialColor?: string;
  title: string;
  onSave: (name: string, color: string) => void;
  onClose: () => void;
}

const PRESET_COLORS = [
  "#ef4444", "#f97316", "#eab308", "#22c55e",
  "#06b6d4", "#3b82f6", "#8b5cf6", "#ec4899",
];

export function CreatePinboardDialog({
  initialName = "",
  initialColor = "#3b82f6",
  title,
  onSave,
  onClose,
}: CreatePinboardDialogProps) {
  const [name, setName] = useState(initialName);
  const [color, setColor] = useState(initialColor);

  const handleSave = () => {
    if (name.trim()) {
      onSave(name.trim(), color);
    }
  };

  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center bg-black/50"
      role="dialog"
      aria-modal="true"
      aria-label={title}
      onClick={onClose}
    >
      <div
        className="w-72 rounded-lg border border-border-default bg-surface-card p-4 shadow-xl"
        onClick={(e) => e.stopPropagation()}
        onKeyDown={(e) => {
          if (e.key === "Enter") handleSave();
          if (e.key === "Escape") onClose();
          e.stopPropagation();
        }}
      >
        <h3 className="mb-3 text-sm font-medium text-text-secondary">{title}</h3>

        <input
          type="text"
          value={name}
          onChange={(e) => setName(e.target.value)}
          placeholder="Pinboard name"
          autoFocus
          className="mb-3 w-full rounded bg-surface-secondary px-3 py-1.5 text-sm text-text-primary placeholder-text-muted outline-none focus:ring-1 focus:ring-blue-500"
        />

        <div className="mb-3">
          <p className="mb-1.5 text-xs text-text-muted">Color</p>
          <div className="flex flex-wrap gap-2">
            {PRESET_COLORS.map((c) => (
              <button
                key={c}
                onClick={() => setColor(c)}
                className={`h-6 w-6 rounded-full transition-all ${
                  color === c ? "ring-2 ring-blue-500 ring-offset-2 ring-offset-surface-card" : ""
                }`}
                style={{ backgroundColor: c }}
              />
            ))}
          </div>
        </div>

        <div className="flex justify-end gap-2">
          <button
            onClick={onClose}
            className="rounded px-3 py-1 text-xs text-text-muted hover:text-text-primary"
          >
            Cancel
          </button>
          <button
            onClick={handleSave}
            disabled={!name.trim()}
            className="rounded bg-blue-600 px-3 py-1 text-xs font-medium text-white hover:bg-blue-500 disabled:opacity-50"
          >
            Save
          </button>
        </div>
      </div>
    </div>
  );
}
