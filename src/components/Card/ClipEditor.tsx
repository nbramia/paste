import { useState, useEffect, useRef } from "react";
import { motion } from "framer-motion";
import { invoke } from "@tauri-apps/api/core";
import type { ClipData } from "../../App";

interface ClipEditorProps {
  clip: ClipData;
  onSave: () => void;
  onClose: () => void;
}

export function ClipEditor({ clip, onSave, onClose }: ClipEditorProps) {
  const [content, setContent] = useState(clip.text_content || "");
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const textareaRef = useRef<HTMLTextAreaElement>(null);

  useEffect(() => {
    // Focus and select all on mount
    textareaRef.current?.focus();
  }, []);

  const handleSave = async () => {
    if (content === clip.text_content) {
      onClose(); // No changes
      return;
    }
    try {
      setSaving(true);
      await invoke("update_clip_content", { id: clip.id, content });
      onSave();
      onClose();
    } catch (err) {
      setError(`Failed to save: ${err}`);
      setSaving(false);
    }
  };

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === "Enter" && (e.ctrlKey || e.metaKey)) {
      e.preventDefault();
      handleSave();
    }
    if (e.key === "Escape") {
      e.preventDefault();
      onClose();
    }
    // Stop all key events from propagating to App's handler
    e.stopPropagation();
  };

  const isCode = clip.content_type === "code";

  return (
    <motion.div
      className="fixed inset-0 z-50 flex items-center justify-center bg-black/60"
      role="dialog"
      aria-modal="true"
      aria-label="Edit clip"
      initial={{ opacity: 0 }}
      animate={{ opacity: 1 }}
      exit={{ opacity: 0 }}
      transition={{ duration: 0.15 }}
      onClick={onClose}
    >
      <motion.div
        className="flex w-[600px] max-w-[90vw] flex-col overflow-hidden rounded-xl border border-border-default bg-surface-primary shadow-2xl"
        initial={{ scale: 0.9, opacity: 0 }}
        animate={{ scale: 1, opacity: 1 }}
        exit={{ scale: 0.9, opacity: 0 }}
        transition={{ type: "spring", stiffness: 400, damping: 30 }}
        onClick={(e) => e.stopPropagation()}
      >
        {/* Header */}
        <div className="flex items-center justify-between border-b border-border-default px-4 py-2">
          <div className="flex items-center gap-2">
            <span className="font-heading text-sm font-semibold text-text-primary tracking-wide">Edit Clip</span>
            <span className="rounded bg-accent-soft px-1.5 py-0.5 text-[10px] font-medium text-accent-muted">
              {clip.content_type}
            </span>
          </div>
          <button
            onClick={onClose}
            className="rounded p-0.5 text-text-faint hover:text-text-primary"
          >
            <svg className="h-4 w-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
              <path d="M18 6L6 18M6 6l12 12" />
            </svg>
          </button>
        </div>

        {/* Editor */}
        <div className="p-4">
          {error && (
            <div className="mb-2 rounded bg-red-950/30 px-3 py-1.5 text-xs text-red-400">
              {error}
            </div>
          )}
          <textarea
            ref={textareaRef}
            value={content}
            onChange={(e) => setContent(e.target.value)}
            onKeyDown={handleKeyDown}
            rows={12}
            className={`w-full rounded bg-surface-secondary p-3 text-sm leading-relaxed text-text-primary outline-none focus:ring-1 focus:ring-accent ${
              isCode ? "font-mono" : ""
            }`}
          />
        </div>

        {/* Footer */}
        <div className="flex items-center justify-between border-t border-border-default px-4 py-2">
          <span className="text-[10px] text-text-faint">
            Ctrl+Enter to save, Escape to cancel
          </span>
          <div className="flex gap-2">
            <button
              onClick={onClose}
              className="rounded px-3 py-1.5 text-xs text-text-muted hover:text-text-primary"
            >
              Cancel
            </button>
            <button
              onClick={handleSave}
              disabled={saving}
              className="rounded bg-accent px-3 py-1.5 text-xs font-medium text-white hover:bg-accent-hover disabled:opacity-50"
            >
              {saving ? "Saving..." : "Save"}
            </button>
          </div>
        </div>
      </motion.div>
    </motion.div>
  );
}
