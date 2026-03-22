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
      onClick={onClose}
    >
      <div
        className="w-72 rounded-lg border border-neutral-700 bg-neutral-800 p-4 shadow-xl"
        onClick={(e) => e.stopPropagation()}
        onKeyDown={(e) => {
          if (e.key === "Enter") handleSave();
          if (e.key === "Escape") onClose();
          e.stopPropagation();
        }}
      >
        <h3 className="mb-3 text-sm font-medium text-neutral-300">{title}</h3>

        <input
          type="text"
          value={name}
          onChange={(e) => setName(e.target.value)}
          placeholder="Pinboard name"
          autoFocus
          className="mb-3 w-full rounded bg-neutral-900 px-3 py-1.5 text-sm text-white placeholder-neutral-500 outline-none focus:ring-1 focus:ring-blue-500"
        />

        <div className="mb-3">
          <p className="mb-1.5 text-xs text-neutral-500">Color</p>
          <div className="flex flex-wrap gap-2">
            {PRESET_COLORS.map((c) => (
              <button
                key={c}
                onClick={() => setColor(c)}
                className={`h-6 w-6 rounded-full transition-all ${
                  color === c ? "ring-2 ring-white ring-offset-2 ring-offset-neutral-800" : ""
                }`}
                style={{ backgroundColor: c }}
              />
            ))}
          </div>
        </div>

        <div className="flex justify-end gap-2">
          <button
            onClick={onClose}
            className="rounded px-3 py-1 text-xs text-neutral-400 hover:text-white"
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
