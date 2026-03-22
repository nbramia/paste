import { RefObject, useCallback, useEffect, useState } from "react";
import { motion, AnimatePresence } from "framer-motion";
import { invoke } from "@tauri-apps/api/core";
import { Card } from "../Card";
import type { ClipData } from "../../App";
import { useAnimation } from "../../hooks/useAnimation";

interface FilmstripProps {
  clips: ClipData[];
  selectedIndex: number;
  multiSelectedIds: Set<string>;
  onSelect: (index: number, event: React.MouseEvent) => void;
  onPaste: () => void;
  loading: boolean;
  containerRef: RefObject<HTMLDivElement | null>;
  onLoadMore?: () => void;
  hasMore?: boolean;
  loadingMore?: boolean;
  onClipCreated?: () => void;
}

export function Filmstrip({
  clips,
  selectedIndex,
  multiSelectedIds,
  onSelect,
  onPaste,
  loading,
  containerRef,
  onLoadMore,
  hasMore,
  loadingMore,
  onClipCreated,
}: FilmstripProps) {
  const anim = useAnimation();
  const [isDragOver, setIsDragOver] = useState(false);

  const handleDragOver = useCallback((e: React.DragEvent) => {
    e.preventDefault();
    e.dataTransfer.dropEffect = "copy";
    setIsDragOver(true);
  }, []);

  const handleDragLeave = useCallback(() => {
    setIsDragOver(false);
  }, []);

  const handleDrop = useCallback(async (e: React.DragEvent) => {
    e.preventDefault();
    setIsDragOver(false);

    // Check for dropped text
    const text = e.dataTransfer.getData("text/plain");
    if (text) {
      try {
        await invoke("create_clip_from_text", { text });
        onClipCreated?.();
      } catch (err) {
        console.error("Failed to create clip from drop:", err);
      }
      return;
    }

    // Check for dropped files (read file names as text clips)
    if (e.dataTransfer.files.length > 0) {
      try {
        for (const file of Array.from(e.dataTransfer.files)) {
          // Read text files
          if (file.type.startsWith("text/") || file.name.endsWith(".txt") || file.name.endsWith(".md")) {
            const fileText = await file.text();
            await invoke("create_clip_from_text", { text: fileText, contentType: "text" });
          } else {
            // For non-text files, store the file path/name as a file clip
            await invoke("create_clip_from_text", {
              text: file.name,
              contentType: "file",
            });
          }
        }
        onClipCreated?.();
      } catch (err) {
        console.error("Failed to create clip from dropped file:", err);
      }
    }
  }, [onClipCreated]);

  const handleScroll = useCallback(() => {
    if (!onLoadMore || !hasMore || loadingMore) return;
    const container = containerRef.current;
    if (!container) return;

    const { scrollLeft, scrollWidth, clientWidth } = container;
    if (scrollWidth - scrollLeft - clientWidth < 300) {
      onLoadMore();
    }
  }, [onLoadMore, hasMore, loadingMore, containerRef]);

  useEffect(() => {
    const container = containerRef.current;
    if (!container) return;

    container.addEventListener("scroll", handleScroll);
    return () => container.removeEventListener("scroll", handleScroll);
  }, [containerRef, handleScroll]);

  if (loading) {
    return (
      <div className="flex flex-1 items-center justify-center text-text-muted">
        Loading...
      </div>
    );
  }

  if (clips.length === 0) {
    return (
      <div className="flex flex-1 items-center justify-center text-text-muted">
        <div className="text-center">
          <p className="text-lg">No clipboard history yet</p>
          <p className="mt-1 text-sm">Copy something to get started</p>
        </div>
      </div>
    );
  }

  return (
    <div
      ref={containerRef}
      role="listbox"
      aria-label="Clipboard history"
      aria-orientation="horizontal"
      onDragOver={handleDragOver}
      onDragLeave={handleDragLeave}
      onDrop={handleDrop}
      className={`flex flex-1 items-stretch gap-3 overflow-x-auto px-4 py-3 transition-colors ${
        isDragOver ? "bg-blue-500/10 ring-2 ring-inset ring-blue-500/30" : ""
      }`}
    >
      <AnimatePresence mode="popLayout">
        {clips.map((clip, index) => (
          <motion.div
            key={clip.id}
            role="option"
            aria-selected={index === selectedIndex}
            aria-label={`${clip.content_type} clip from ${clip.source_app || "unknown"}`}
            layout={anim.isEnabled}
            initial={anim.isEnabled ? { opacity: 0, scale: 0.9, x: -20 } : false}
            animate={{ opacity: 1, scale: 1, x: 0 }}
            exit={anim.isEnabled ? { opacity: 0, scale: 0.9 } : undefined}
            transition={anim.isEnabled ? {
              delay: index < 50 ? index * anim.staggerDelay : 0,
              duration: anim.duration(0.2),
              ease: "easeOut",
            } : { duration: 0 }}
            className="shrink-0"
          >
            <Card
              clip={clip}
              index={index}
              isSelected={index === selectedIndex}
              isMultiSelected={multiSelectedIds.has(clip.id)}
              onSelect={(e: React.MouseEvent) => onSelect(index, e)}
              onPaste={onPaste}
            />
          </motion.div>
        ))}
      </AnimatePresence>
      {loadingMore && (
        <div className="flex shrink-0 items-center justify-center px-4">
          <span className="text-xs text-text-muted">Loading more...</span>
        </div>
      )}
      {!hasMore && clips.length > 0 && (
        <div className="flex shrink-0 items-center justify-center px-2">
          <span className="text-[10px] text-text-faint">End of history</span>
        </div>
      )}
    </div>
  );
}
