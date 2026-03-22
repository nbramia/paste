interface ImageCardProps {
  imagePath: string | null;
  metadata: string | null;
}

export function ImageCard({ metadata }: ImageCardProps) {
  let sizeInfo: string | null = null;
  if (metadata) {
    try {
      const parsed = JSON.parse(metadata);
      if (parsed.size_bytes) {
        const kb = Math.round(parsed.size_bytes / 1024);
        sizeInfo = kb > 1024 ? `${(kb / 1024).toFixed(1)} MB` : `${kb} KB`;
      }
    } catch {
      /* ignore */
    }
  }

  return (
    <div className="flex flex-1 flex-col items-center justify-center overflow-hidden p-2">
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
      {sizeInfo && (
        <span className="mt-0.5 text-[10px] text-text-muted">{sizeInfo}</span>
      )}
    </div>
  );
}
