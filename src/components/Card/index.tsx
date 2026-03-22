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
  onSelect: () => void;
  onPaste: () => void;
}

const typeColors: Record<string, string> = {
  text: "bg-blue-500",
  code: "bg-green-500",
  link: "bg-purple-500",
  image: "bg-orange-500",
  file: "bg-yellow-500",
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

export function Card({
  clip,
  index,
  isSelected,
  onSelect,
  onPaste,
}: CardProps) {
  const typeColor = typeColors[clip.content_type] || "bg-neutral-500";

  return (
    <div
      data-index={index}
      onClick={onSelect}
      onDoubleClick={onPaste}
      className={`flex w-48 shrink-0 cursor-pointer flex-col rounded-lg border transition-all shadow-sm dark:shadow-none ${
        isSelected
          ? "border-blue-500 ring-2 ring-blue-500/30 bg-surface-hover"
          : "border-border-default bg-surface-card hover:bg-surface-hover"
      }`}
    >
      {/* Content type indicator */}
      <div className={`h-1 rounded-t-lg ${typeColor}`} />

      {/* Content preview — dispatched by type */}
      <CardContent clip={clip} />

      {/* Footer */}
      <div className="flex items-center gap-2 border-t border-border-subtle px-3 py-1.5">
        <span className="truncate text-xs text-text-muted">
          {clip.source_app || "Unknown"}
        </span>
        <span className="ml-auto text-xs text-text-faint">
          {formatRelativeTime(clip.created_at)}
        </span>
      </div>
    </div>
  );
}
