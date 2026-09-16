import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

interface ImageCardProps {
  clipId: string;
  imagePath: string | null;
  metadata: string | null;
}

interface ImageMeta {
  sizeInfo: string | null;
  dimensions: string | null;
}

function parseMeta(metadata: string | null): ImageMeta {
  if (!metadata) return { sizeInfo: null, dimensions: null };
  try {
    const parsed = JSON.parse(metadata);
    const kb = parsed.size_bytes ? Math.round(parsed.size_bytes / 1024) : null;
    return {
      sizeInfo:
        kb === null ? null : kb > 1024 ? `${(kb / 1024).toFixed(1)} MB` : `${kb} KB`,
      dimensions:
        parsed.width && parsed.height ? `${parsed.width}×${parsed.height}` : null,
    };
  } catch {
    return { sizeInfo: null, dimensions: null };
  }
}

export function ImageCard({ clipId, imagePath, metadata }: ImageCardProps) {
  const { sizeInfo, dimensions } = parseMeta(metadata);
  const [thumbnail, setThumbnail] = useState<string | null>(null);

  // The thumbnail arrives as a data URI over IPC rather than by pointing an
  // <img> at the file: the frontend has no direct file access, and widening
  // the asset protocol for one card is not worth the CSP surface.
  useEffect(() => {
    if (!imagePath) return;
    let cancelled = false;

    invoke<string | null>("get_clip_thumbnail", { id: clipId })
      .then((uri) => {
        if (!cancelled) setThumbnail(uri ?? null);
      })
      .catch(() => {
        // Falls back to the placeholder icon below.
        if (!cancelled) setThumbnail(null);
      });

    return () => {
      cancelled = true;
    };
  }, [clipId, imagePath]);

  return (
    <div className="flex flex-1 flex-col items-center justify-center overflow-hidden p-2">
      {thumbnail ? (
        <img
          src={thumbnail}
          alt="Clipboard image preview"
          className="max-h-full min-h-0 max-w-full flex-1 rounded object-contain"
        />
      ) : (
        <>
          {/* Image placeholder icon */}
          <svg
            className="h-10 w-10 text-orange-500 dark:text-orange-400/70"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            strokeWidth="1.5"
          >
            <rect x="3" y="3" width="18" height="18" rx="2" />
            <circle cx="8.5" cy="8.5" r="1.5" />
            <path d="M21 15l-5-5L5 21" />
          </svg>
          <span className="mt-1.5 text-[10px] font-medium text-orange-500 dark:text-orange-400/70">
            Image
          </span>
        </>
      )}
      {(dimensions || sizeInfo) && (
        <span className="mt-0.5 shrink-0 text-[10px] text-text-muted">
          {[dimensions, sizeInfo].filter(Boolean).join(" · ")}
        </span>
      )}
    </div>
  );
}
