import { motion } from "framer-motion";
import type { ClipData } from "../../App";

interface CardPreviewProps {
  clip: ClipData;
  onClose: () => void;
  onPaste: () => void;
}

function formatDate(iso: string): string {
  return new Date(iso).toLocaleString();
}

function PreviewContent({ clip }: { clip: ClipData }) {
  switch (clip.content_type) {
    case "code":
      return (
        <pre className="flex-1 overflow-auto whitespace-pre-wrap break-words rounded bg-surface-secondary p-4 font-mono text-sm leading-relaxed text-emerald-700 dark:text-emerald-300/90">
          {clip.text_content || ""}
        </pre>
      );
    case "link": {
      const url = clip.text_content?.trim() || "";
      let title: string | null = null;
      if (clip.metadata) {
        try {
          const parsed = JSON.parse(clip.metadata);
          title = parsed.title || null;
        } catch {
          /* ignore */
        }
      }
      return (
        <div className="flex flex-1 flex-col gap-3 overflow-auto p-4">
          <div className="flex items-center gap-2">
            <svg
              className="h-5 w-5 shrink-0 text-purple-400"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              strokeWidth="2"
            >
              <circle cx="12" cy="12" r="10" />
              <path d="M2 12h20M12 2a15.3 15.3 0 0 1 4 10 15.3 15.3 0 0 1-4 10 15.3 15.3 0 0 1-4-10 15.3 15.3 0 0 1 4-10z" />
            </svg>
            <span className="text-sm font-medium text-purple-300">{url}</span>
          </div>
          {title && <p className="text-sm text-text-primary">{title}</p>}
          {clip.html_content && (
            <div className="rounded border border-border-subtle bg-surface-secondary p-3 text-xs text-text-secondary">
              <p className="mb-1 text-[10px] uppercase tracking-wider text-text-faint">
                HTML preview
              </p>
              <pre className="whitespace-pre-wrap break-words">
                {clip.html_content.slice(0, 500)}
              </pre>
            </div>
          )}
        </div>
      );
    }
    case "image":
      return (
        <div className="flex flex-1 flex-col items-center justify-center gap-2 p-4">
          <svg
            className="h-16 w-16 text-orange-400/50"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            strokeWidth="1.5"
          >
            <rect x="3" y="3" width="18" height="18" rx="2" />
            <circle cx="8.5" cy="8.5" r="1.5" />
            <path d="M21 15l-5-5L5 21" />
          </svg>
          <p className="text-sm text-text-muted">Image clip</p>
          {clip.image_path && (
            <p className="text-xs text-text-faint">{clip.image_path}</p>
          )}
          {clip.metadata &&
            (() => {
              try {
                const m = JSON.parse(clip.metadata!);
                if (m.size_bytes) {
                  const kb = Math.round(m.size_bytes / 1024);
                  return (
                    <p className="text-xs text-text-faint">
                      {kb > 1024
                        ? `${(kb / 1024).toFixed(1)} MB`
                        : `${kb} KB`}
                    </p>
                  );
                }
              } catch {
                /* ignore */
              }
              return null;
            })()}
        </div>
      );
    case "file":
      return (
        <div className="flex flex-1 flex-col items-center justify-center gap-2 p-4">
          <svg
            className="h-16 w-16 text-yellow-400/50"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            strokeWidth="1.5"
          >
            <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z" />
            <polyline points="14 2 14 8 20 8" />
          </svg>
          <p className="text-sm font-medium text-text-primary break-all text-center">
            {clip.text_content || ""}
          </p>
        </div>
      );
    case "text":
    default:
      return (
        <div className="flex-1 overflow-auto p-4">
          <p className="whitespace-pre-wrap break-words text-sm leading-relaxed text-text-primary">
            {clip.text_content || ""}
          </p>
        </div>
      );
  }
}

export function CardPreview({ clip, onClose, onPaste }: CardPreviewProps) {
  const handleCopy = async () => {
    if (clip.text_content) {
      try {
        await navigator.clipboard.writeText(clip.text_content);
      } catch {
        // Fallback — navigator.clipboard might not work in Tauri WebView
        console.warn("Clipboard API unavailable");
      }
    }
  };

  const contentTypeLabels: Record<string, string> = {
    text: "Text",
    code: "Code",
    link: "Link",
    image: "Image",
    file: "File",
  };

  return (
    <motion.div
      className="fixed inset-0 z-40 flex items-center justify-center bg-black/60"
      initial={{ opacity: 0 }}
      animate={{ opacity: 1 }}
      exit={{ opacity: 0 }}
      transition={{ duration: 0.15 }}
      onClick={onClose}
    >
      <motion.div
        className="flex max-h-[70vh] w-[600px] max-w-[90vw] flex-col overflow-hidden rounded-xl border border-border-default bg-surface-primary shadow-2xl"
        initial={{ scale: 0.9, opacity: 0 }}
        animate={{ scale: 1, opacity: 1 }}
        exit={{ scale: 0.9, opacity: 0 }}
        transition={{ type: "spring", stiffness: 400, damping: 30 }}
        onClick={(e) => e.stopPropagation()}
      >
        {/* Header */}
        <div className="flex items-center justify-between border-b border-border-default px-4 py-2">
          <div className="flex items-center gap-2">
            <span className="rounded bg-blue-900/30 px-1.5 py-0.5 text-[10px] font-medium text-blue-400">
              {contentTypeLabels[clip.content_type] || clip.content_type}
            </span>
            {clip.source_app && (
              <span className="text-xs text-text-muted">
                {clip.source_app}
              </span>
            )}
          </div>
          <div className="flex items-center gap-1">
            <span className="text-[10px] text-text-faint">
              {formatDate(clip.created_at)}
            </span>
            <button
              onClick={onClose}
              className="ml-2 rounded p-0.5 text-text-faint hover:text-text-primary"
              title="Close (Space or Esc)"
            >
              <svg
                className="h-4 w-4"
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                strokeWidth="2"
              >
                <path d="M18 6L6 18M6 6l12 12" />
              </svg>
            </button>
          </div>
        </div>

        {/* Content */}
        <PreviewContent clip={clip} />

        {/* Footer actions */}
        <div className="flex items-center justify-between border-t border-border-default px-4 py-2">
          <div className="text-[10px] text-text-faint">
            {clip.content_size > 0 && (
              <span>
                {clip.content_size > 1024
                  ? `${(clip.content_size / 1024).toFixed(1)} KB`
                  : `${clip.content_size} B`}
              </span>
            )}
            {clip.access_count > 0 && (
              <span className="ml-2">Pasted {clip.access_count}x</span>
            )}
          </div>
          <div className="flex gap-2">
            <button
              onClick={handleCopy}
              className="rounded bg-surface-secondary px-3 py-1 text-xs text-text-secondary hover:text-text-primary"
            >
              Copy
            </button>
            <button
              onClick={() => {
                onPaste();
                onClose();
              }}
              className="rounded bg-blue-600 px-3 py-1 text-xs font-medium text-white hover:bg-blue-500"
            >
              Paste
            </button>
          </div>
        </div>
      </motion.div>
    </motion.div>
  );
}
