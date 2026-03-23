import { useState, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { ClipData } from "../../App";
import type { PinboardData } from "../../hooks/usePinboards";
import { Card } from "../Card";
import { CreatePinboardDialog } from "./CreatePinboardDialog";

interface PinboardViewProps {
  pinboards: PinboardData[];
  onReload: () => void;
  onCreatePinboard: (name: string, color: string) => void;
  onUpdatePinboard: (id: string, name: string, color: string) => void;
  onDeletePinboard: (id: string) => void;
}

export function PinboardView({
  pinboards,
  onReload: _onReload,
  onCreatePinboard,
  onUpdatePinboard,
  onDeletePinboard,
}: PinboardViewProps) {
  const [selectedPinboard, setSelectedPinboard] = useState<string | null>(null);
  const [clips, setClips] = useState<ClipData[]>([]);
  const [loading, setLoading] = useState(false);
  const [showCreate, setShowCreate] = useState(false);
  const [editingPinboard, setEditingPinboard] = useState<PinboardData | null>(null);
  const [deletingPinboard, setDeletingPinboard] = useState<PinboardData | null>(null);

  const loadPinboardClips = useCallback(async (pinboardId: string) => {
    setLoading(true);
    try {
      const result = await invoke<ClipData[]>("get_clips", {
        offset: 0,
        limit: 100,
        pinboardId: pinboardId,
      });
      setClips(result);
    } catch (err) {
      console.error("Failed to load pinboard clips:", err);
    } finally {
      setLoading(false);
    }
  }, []);

  const handleSelectPinboard = (id: string) => {
    setSelectedPinboard(id);
    loadPinboardClips(id);
  };

  const handleCreateSave = (name: string, color: string) => {
    onCreatePinboard(name, color);
    setShowCreate(false);
  };

  const handleEditSave = (name: string, color: string) => {
    if (editingPinboard) {
      onUpdatePinboard(editingPinboard.id, name, color);
      setEditingPinboard(null);
    }
  };

  const handleDelete = (id: string) => {
    onDeletePinboard(id);
    if (selectedPinboard === id) {
      setSelectedPinboard(null);
      setClips([]);
    }
  };

  // If no pinboard selected, show the pinboard list
  if (!selectedPinboard) {
    return (
      <div className="flex flex-1 flex-col overflow-hidden p-4">
        <div className="mb-3 flex items-center justify-between">
          <h2 className="font-heading text-sm font-semibold text-text-secondary tracking-wide">Pinboards</h2>
          <button
            onClick={() => setShowCreate(true)}
            className="rounded bg-accent px-2 py-1 text-xs font-medium text-white hover:bg-accent-hover"
          >
            + New
          </button>
        </div>

        {pinboards.length === 0 ? (
          <div className="flex flex-1 items-center justify-center text-text-muted">
            <div className="text-center">
              <p className="text-sm">No pinboards yet</p>
              <p className="mt-1 text-xs">Create one to save your favorite clips</p>
            </div>
          </div>
        ) : (
          <div className="flex flex-wrap gap-3">
            {pinboards.map((pb) => (
              <div
                key={pb.id}
                onClick={() => handleSelectPinboard(pb.id)}
                className="group flex h-56 w-56 shrink-0 cursor-pointer flex-col rounded-lg border border-border-default bg-surface-card hover:bg-surface-hover transition-colors"
              >
                {/* Color bar */}
                <div className="h-2 rounded-t-lg" style={{ backgroundColor: pb.color }} />

                {/* Content area */}
                <div className="flex flex-1 flex-col items-center justify-center gap-2 p-4">
                  <span
                    className="h-10 w-10 rounded-full"
                    style={{ backgroundColor: pb.color }}
                  />
                  <span className="font-heading text-base font-semibold text-text-primary">
                    {pb.name}
                  </span>
                </div>

                {/* Actions */}
                <div className="flex items-center justify-end gap-1 border-t border-border-subtle px-3 py-2 opacity-0 group-hover:opacity-100">
                  <button
                    onClick={(e) => { e.stopPropagation(); setEditingPinboard(pb); }}
                    className="text-text-faint hover:text-text-secondary"
                    title="Edit"
                  >
                    <svg className="h-4 w-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                      <path d="M11 4H4a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2v-7" />
                      <path d="M18.5 2.5a2.121 2.121 0 0 1 3 3L12 15l-4 1 1-4 9.5-9.5z" />
                    </svg>
                  </button>
                  <button
                    onClick={(e) => { e.stopPropagation(); setDeletingPinboard(pb); }}
                    className="text-text-faint hover:text-red-400"
                    title="Delete"
                  >
                    <svg className="h-4 w-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                      <path d="M18 6L6 18M6 6l12 12" />
                    </svg>
                  </button>
                </div>
              </div>
            ))}
          </div>
        )}

        {showCreate && (
          <CreatePinboardDialog
            title="Create Pinboard"
            onSave={handleCreateSave}
            onClose={() => setShowCreate(false)}
          />
        )}

        {editingPinboard && (
          <CreatePinboardDialog
            title="Edit Pinboard"
            initialName={editingPinboard.name}
            initialColor={editingPinboard.color}
            onSave={handleEditSave}
            onClose={() => setEditingPinboard(null)}
          />
        )}

        {/* Delete confirmation */}
        {deletingPinboard && (
          <div
            className="fixed inset-0 z-50 flex items-center justify-center bg-black/50"
            onClick={() => setDeletingPinboard(null)}
          >
            <div
              className="w-72 rounded-lg border border-border-default bg-surface-primary p-4 shadow-xl"
              onClick={(e) => e.stopPropagation()}
            >
              <h3 className="font-heading text-sm font-semibold text-text-primary">
                Delete pinboard?
              </h3>
              <p className="mt-2 text-sm text-text-secondary">
                Are you sure you want to delete <strong>{deletingPinboard.name}</strong>? Clips in this pinboard will be unlinked but not deleted.
              </p>
              <div className="mt-4 flex justify-end gap-2">
                <button
                  onClick={() => setDeletingPinboard(null)}
                  className="rounded px-3 py-1.5 text-xs text-text-muted hover:text-text-primary"
                >
                  Cancel
                </button>
                <button
                  onClick={() => {
                    handleDelete(deletingPinboard.id);
                    setDeletingPinboard(null);
                  }}
                  className="rounded bg-red-600 px-3 py-1.5 text-xs font-medium text-white hover:bg-red-500"
                >
                  Delete
                </button>
              </div>
            </div>
          </div>
        )}
      </div>
    );
  }

  // Show clips for selected pinboard
  const currentPb = pinboards.find((pb) => pb.id === selectedPinboard);

  return (
    <div className="flex flex-1 flex-col overflow-hidden">
      {/* Back button + pinboard header */}
      <div className="flex items-center gap-2 px-4 py-2 border-b border-border-subtle">
        <button
          onClick={() => { setSelectedPinboard(null); setClips([]); }}
          className="text-text-muted hover:text-text-primary"
        >
          <svg className="h-4 w-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
            <path d="M19 12H5M12 19l-7-7 7-7" />
          </svg>
        </button>
        {currentPb && (
          <>
            <span className="h-3 w-3 rounded-full" style={{ backgroundColor: currentPb.color }} />
            <span className="font-heading text-sm font-semibold text-text-secondary tracking-wide">{currentPb.name}</span>
          </>
        )}
        <span className="ml-auto text-xs text-text-muted">
          {clips.length} clip{clips.length !== 1 ? "s" : ""}
        </span>
      </div>

      {/* Clips in horizontal strip */}
      {loading ? (
        <div className="flex flex-1 items-center justify-center text-text-muted">Loading...</div>
      ) : clips.length === 0 ? (
        <div className="flex flex-1 items-center justify-center text-text-muted">
          <p className="text-sm">No clips in this pinboard</p>
        </div>
      ) : (
        <div className="flex flex-1 items-stretch gap-3 overflow-x-auto px-4 py-3">
          {clips.map((clip, index) => (
            <Card
              key={clip.id}
              clip={clip}
              index={index}
              isSelected={false}
              isMultiSelected={false}
              onSelect={() => {}}
              onPaste={() => {}}
            />
          ))}
        </div>
      )}
    </div>
  );
}

// Re-export sub-components
export { PinboardPicker } from "./PinboardPicker";
export { CreatePinboardDialog } from "./CreatePinboardDialog";
