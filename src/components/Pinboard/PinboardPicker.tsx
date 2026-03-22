import type { PinboardData } from "../../hooks/usePinboards";

interface PinboardPickerProps {
  pinboards: PinboardData[];
  onSelect: (pinboardId: string) => void;
  onCreateNew: () => void;
  onClose: () => void;
}

export function PinboardPicker({ pinboards, onSelect, onCreateNew, onClose }: PinboardPickerProps) {
  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center bg-black/50"
      onClick={onClose}
    >
      <div
        className="w-64 rounded-lg border border-border-default bg-surface-card p-3 shadow-xl"
        onClick={(e) => e.stopPropagation()}
      >
        <h3 className="mb-2 text-sm font-medium text-text-secondary">Save to Pinboard</h3>

        {pinboards.length === 0 ? (
          <p className="text-xs text-text-muted mb-2">No pinboards yet</p>
        ) : (
          <div className="mb-2 max-h-48 space-y-1 overflow-y-auto">
            {pinboards.map((pb) => (
              <button
                key={pb.id}
                onClick={() => onSelect(pb.id)}
                className="flex w-full items-center gap-2 rounded px-2 py-1.5 text-left text-sm text-text-secondary hover:bg-surface-hover"
              >
                <span
                  className="h-3 w-3 shrink-0 rounded-full"
                  style={{ backgroundColor: pb.color }}
                />
                <span className="truncate">{pb.name}</span>
              </button>
            ))}
          </div>
        )}

        <button
          onClick={onCreateNew}
          className="w-full rounded bg-blue-600 px-2 py-1.5 text-xs font-medium text-white hover:bg-blue-500"
        >
          + New Pinboard
        </button>
      </div>
    </div>
  );
}
