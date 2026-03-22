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
        className="w-64 rounded-lg border border-neutral-700 bg-neutral-800 p-3 shadow-xl"
        onClick={(e) => e.stopPropagation()}
      >
        <h3 className="mb-2 text-sm font-medium text-neutral-300">Save to Pinboard</h3>

        {pinboards.length === 0 ? (
          <p className="text-xs text-neutral-500 mb-2">No pinboards yet</p>
        ) : (
          <div className="mb-2 max-h-48 space-y-1 overflow-y-auto">
            {pinboards.map((pb) => (
              <button
                key={pb.id}
                onClick={() => onSelect(pb.id)}
                className="flex w-full items-center gap-2 rounded px-2 py-1.5 text-left text-sm text-neutral-300 hover:bg-neutral-700"
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
