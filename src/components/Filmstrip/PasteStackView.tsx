import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { ClipData } from "../../App";

interface PasteStackViewProps {
  onStatusChange?: (active: boolean, count: number) => void;
}

export function PasteStackView({ onStatusChange }: PasteStackViewProps) {
  const [items, setItems] = useState<ClipData[]>([]);
  const [active, setActive] = useState(false);
  const [loading, setLoading] = useState(true);

  const loadStack = useCallback(async () => {
    try {
      setLoading(true);
      const [isActive, count] = await invoke<[boolean, number]>("get_paste_stack_status");
      setActive(isActive);
      if (isActive) {
        const stackItems = await invoke<ClipData[]>("get_paste_stack");
        setItems(stackItems);
      } else {
        setItems([]);
      }
      onStatusChange?.(isActive, isActive ? count : 0);
    } catch (err) {
      console.error("Failed to load paste stack:", err);
    } finally {
      setLoading(false);
    }
  }, [onStatusChange]);

  useEffect(() => {
    loadStack();
    // Poll every 500ms to keep UI in sync when clips are added externally
    const interval = setInterval(loadStack, 500);
    return () => clearInterval(interval);
  }, [loadStack]);

  const handleToggle = async () => {
    try {
      const newActive = await invoke<boolean>("toggle_paste_stack");
      setActive(newActive);
      if (!newActive) {
        setItems([]);
      }
      await loadStack();
    } catch (err) {
      console.error("Failed to toggle paste stack:", err);
    }
  };

  const handleRemove = async (clipId: string) => {
    try {
      await invoke("remove_from_paste_stack", { clipId });
      await loadStack();
    } catch (err) {
      console.error("Failed to remove from stack:", err);
    }
  };

  const handleClear = async () => {
    try {
      await invoke("clear_paste_stack");
      setActive(false);
      setItems([]);
      onStatusChange?.(false, 0);
    } catch (err) {
      console.error("Failed to clear paste stack:", err);
    }
  };

  const handlePasteNext = async () => {
    try {
      await invoke("pop_paste_stack");
      await loadStack();
    } catch (err) {
      console.error("Failed to paste from stack:", err);
    }
  };

  if (loading) {
    return (
      <div className="flex flex-1 items-center justify-center text-neutral-500">
        Loading...
      </div>
    );
  }

  return (
    <div className="flex flex-1 flex-col overflow-hidden p-4">
      {/* Header */}
      <div className="mb-3 flex items-center justify-between">
        <div className="flex items-center gap-2">
          <h2 className="text-sm font-medium text-neutral-300">Paste Stack</h2>
          <span
            className={`rounded-full px-2 py-0.5 text-[10px] font-medium ${
              active
                ? "bg-green-900/50 text-green-400"
                : "bg-neutral-700 text-neutral-400"
            }`}
          >
            {active ? `ON · ${items.length} items` : "OFF"}
          </span>
        </div>

        <div className="flex gap-2">
          {active && items.length > 0 && (
            <>
              <button
                onClick={handlePasteNext}
                className="rounded bg-blue-600 px-2 py-1 text-xs font-medium text-white hover:bg-blue-500"
              >
                Paste Next
              </button>
              <button
                onClick={handleClear}
                className="rounded px-2 py-1 text-xs text-neutral-400 hover:text-white"
              >
                Clear
              </button>
            </>
          )}
          <button
            onClick={handleToggle}
            className={`rounded px-2 py-1 text-xs font-medium ${
              active
                ? "bg-red-900/50 text-red-400 hover:bg-red-900"
                : "bg-green-900/50 text-green-400 hover:bg-green-900"
            }`}
          >
            {active ? "Deactivate" : "Activate"}
          </button>
        </div>
      </div>

      {/* Instructions when inactive */}
      {!active && (
        <div className="flex flex-1 items-center justify-center text-neutral-500">
          <div className="text-center">
            <p className="text-sm">Paste Stack is inactive</p>
            <p className="mt-1 text-xs">
              Activate to start collecting clips. Each Ctrl+V will paste the
              next item in sequence.
            </p>
            <p className="mt-2 text-xs text-neutral-600">
              Shortcut: Super+Shift+V
            </p>
          </div>
        </div>
      )}

      {/* Stack items when active */}
      {active && items.length === 0 && (
        <div className="flex flex-1 items-center justify-center text-neutral-500">
          <div className="text-center">
            <p className="text-sm">Stack is empty — copy items to add them</p>
            <p className="mt-1 text-xs">
              Items are pasted in order (first in, first out)
            </p>
          </div>
        </div>
      )}

      {active && items.length > 0 && (
        <div className="flex-1 space-y-1 overflow-y-auto">
          {items.map((item, index) => (
            <div
              key={item.id}
              className="flex items-center gap-2 rounded border border-neutral-700 bg-neutral-800 px-3 py-2"
            >
              <span className="w-5 shrink-0 text-center text-xs font-mono text-neutral-600">
                {index + 1}
              </span>
              <p className="flex-1 truncate text-xs text-neutral-300">
                {item.text_content || "[Non-text content]"}
              </p>
              <button
                onClick={() => handleRemove(item.id)}
                className="shrink-0 text-neutral-600 hover:text-red-400"
                title="Remove from stack"
              >
                <svg
                  className="h-3.5 w-3.5"
                  viewBox="0 0 24 24"
                  fill="none"
                  stroke="currentColor"
                  strokeWidth="2"
                >
                  <path d="M18 6L6 18M6 6l12 12" />
                </svg>
              </button>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
