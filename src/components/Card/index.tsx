import type { ClipData } from "../../App";

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

function truncateText(
  text: string,
  maxLines: number,
  maxChars: number,
): string {
  const lines = text.split("\n").slice(0, maxLines);
  const truncated = lines.join("\n");
  if (truncated.length > maxChars) {
    return truncated.slice(0, maxChars) + "\u2026";
  }
  if (text.split("\n").length > maxLines) {
    return truncated + "\u2026";
  }
  return truncated;
}

export function Card({
  clip,
  index,
  isSelected,
  onSelect,
  onPaste,
}: CardProps) {
  const preview = clip.text_content
    ? truncateText(clip.text_content, 6, 200)
    : clip.image_path
      ? "[Image]"
      : "[Empty]";

  const typeColor = typeColors[clip.content_type] || "bg-neutral-500";

  return (
    <div
      data-index={index}
      onClick={onSelect}
      onDoubleClick={onPaste}
      className={`flex w-48 shrink-0 cursor-pointer flex-col rounded-lg border transition-all ${
        isSelected
          ? "border-blue-500 bg-neutral-750 ring-2 ring-blue-500/30"
          : "border-neutral-700 bg-neutral-800 hover:border-neutral-600"
      }`}
    >
      {/* Content type indicator */}
      <div className={`h-1 rounded-t-lg ${typeColor}`} />

      {/* Preview */}
      <div className="flex-1 overflow-hidden px-3 py-2">
        <pre className="whitespace-pre-wrap break-words font-mono text-xs leading-relaxed text-neutral-300">
          {preview}
        </pre>
      </div>

      {/* Footer */}
      <div className="flex items-center gap-2 border-t border-neutral-700/50 px-3 py-1.5">
        <span className="truncate text-xs text-neutral-500">
          {clip.source_app || "Unknown"}
        </span>
        <span className="ml-auto text-xs text-neutral-600">
          {formatRelativeTime(clip.created_at)}
        </span>
      </div>
    </div>
  );
}
