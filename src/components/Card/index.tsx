import { memo, useRef, useEffect } from "react";
import { motion } from "framer-motion";
import type { ClipData } from "../../App";
import { TextCard } from "./TextCard";
import { CodeCard } from "./CodeCard";
import { LinkCard } from "./LinkCard";
import { ImageCard } from "./ImageCard";
import { FileCard } from "./FileCard";

interface CardProps {
  clip: ClipData;
  index: number;
  isSelected: boolean;
  isMultiSelected: boolean;
  onSelect: (event: React.MouseEvent) => void;
  onPaste: () => void;
  onContextMenu?: (event: React.MouseEvent) => void;
}

const typeColors: Record<string, string> = {
  text: "bg-type-text",
  code: "bg-type-code",
  link: "bg-type-link",
  image: "bg-type-image",
  file: "bg-type-file",
};

function formatRelativeTime(isoDate: string): string {
  const now = Date.now();
  const then = new Date(isoDate).getTime();
  const diffMs = now - then;
  const diffSec = Math.floor(diffMs / 1000);
  const diffMin = Math.floor(diffSec / 60);
  const diffHr = Math.floor(diffMin / 60);
  const diffDay = Math.floor(diffHr / 24);

  if (diffSec < 60) return "just now";
  if (diffMin < 60) return `${diffMin}m ago`;
  if (diffHr < 24) return `${diffHr}h ago`;
  if (diffDay < 7) return `${diffDay}d ago`;
  return new Date(isoDate).toLocaleDateString();
}

function CardContent({ clip }: { clip: ClipData }) {
  switch (clip.content_type) {
    case "code":
      return (
        <CodeCard text={clip.text_content || ""} metadata={clip.metadata} />
      );
    case "link":
      return (
        <LinkCard text={clip.text_content || ""} metadata={clip.metadata} />
      );
    case "image":
      return (
        <ImageCard
          imagePath={clip.image_path}
          metadata={clip.metadata}
        />
      );
    case "file":
      return <FileCard text={clip.text_content || ""} />;
    case "text":
    default:
      return <TextCard text={clip.text_content || ""} />;
  }
}

function getCardLabel(clip: ClipData): string {
  const type = clip.content_type;
  const source = clip.source_app || "unknown app";
  const preview = clip.text_content
    ? clip.text_content.slice(0, 50).replace(/\n/g, " ")
    : type === "image"
      ? "Image"
      : "Clip";
  return `${type} from ${source}: ${preview}`;
}

function CardBase({
  clip,
  index,
  isSelected,
  isMultiSelected,
  onSelect,
  onPaste,
  onContextMenu,
}: CardProps) {
  const typeColor = typeColors[clip.content_type] || "bg-neutral-500";
  const cardRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const el = cardRef.current;
    if (!el) return;

    const handleDragStart = (e: DragEvent) => {
      if (!e.dataTransfer) return;

      // Set drag data based on content type
      if (clip.text_content) {
        e.dataTransfer.setData("text/plain", clip.text_content);
      }

      if (clip.content_type === "link" && clip.text_content) {
        e.dataTransfer.setData("text/uri-list", clip.text_content);
      }

      if (clip.html_content) {
        e.dataTransfer.setData("text/html", clip.html_content);
      }

      // Set drag effect
      e.dataTransfer.effectAllowed = "copy";
    };

    el.addEventListener("dragstart", handleDragStart);
    return () => el.removeEventListener("dragstart", handleDragStart);
  }, [clip]);

  return (
    <motion.div
      ref={cardRef}
      data-index={index}
      role="button"
      tabIndex={isSelected ? 0 : -1}
      aria-label={getCardLabel(clip)}
      aria-selected={isSelected || isMultiSelected}
      draggable
      onClick={onSelect}
      onDoubleClick={onPaste}
      onContextMenu={onContextMenu}
      animate={{
        scale: isSelected ? 1.03 : 1,
      }}
      transition={{
        type: "spring",
        stiffness: 400,
        damping: 25,
        mass: 0.8,
      }}
      className={`relative flex h-44 w-48 shrink-0 cursor-pointer flex-col rounded-lg border transition-colors shadow-sm dark:shadow-none ${
        isMultiSelected
          ? "border-emerald-500 ring-2 ring-emerald-500/30 bg-surface-hover"
          : isSelected
            ? "border-accent ring-2 ring-accent/25 bg-surface-hover"
            : "border-border-default bg-surface-card hover:bg-surface-hover"
      }`}
    >
      {/* Multi-select checkmark */}
      {isMultiSelected && (
        <div className="absolute top-2 right-2 z-10 flex h-5 w-5 items-center justify-center rounded-full bg-emerald-500 text-white">
          <svg className="h-3 w-3" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="3">
            <path d="M20 6L9 17l-5-5" />
          </svg>
        </div>
      )}

      {/* Content type indicator */}
      <div className={`h-1.5 rounded-t-lg ${typeColor}`} />

      {/* Content preview — dispatched by type */}
      <div className="flex-1 overflow-hidden">
        <CardContent clip={clip} />
      </div>

      {/* Footer */}
      <div className="flex items-center gap-2 border-t border-border-subtle px-3 py-1.5">
        {clip.is_favorite && (
          <svg className="h-3 w-3 shrink-0 text-yellow-400" viewBox="0 0 24 24" fill="currentColor">
            <path d="M12 2l3.09 6.26L22 9.27l-5 4.87 1.18 6.88L12 17.77l-6.18 3.25L7 14.14 2 9.27l6.91-1.01L12 2z" />
          </svg>
        )}
        <span className="truncate text-xs text-text-muted">
          {clip.source_app || "Unknown"}
        </span>
        <span className="ml-auto text-xs text-text-faint">
          {formatRelativeTime(clip.created_at)}
        </span>
      </div>
    </motion.div>
  );
}

export const Card = memo(CardBase);
