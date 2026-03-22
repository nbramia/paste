interface FileCardProps {
  text: string;
}

function extractFilename(path: string): string {
  const trimmed = path.trim();
  // Handle file:// URIs
  const cleaned = trimmed.startsWith("file://") ? trimmed.slice(7) : trimmed;
  const parts = cleaned.split("/");
  return parts[parts.length - 1] || cleaned;
}

function getFileExtension(filename: string): string {
  const dot = filename.lastIndexOf(".");
  return dot > 0 ? filename.slice(dot + 1).toLowerCase() : "";
}

export function FileCard({ text }: FileCardProps) {
  const filename = extractFilename(text);
  const ext = getFileExtension(filename);

  return (
    <div className="flex flex-1 flex-col items-center justify-center overflow-hidden px-3 py-2">
      {/* File icon */}
      <svg
        className="h-8 w-8 text-yellow-400"
        viewBox="0 0 24 24"
        fill="none"
        stroke="currentColor"
        strokeWidth="1.5"
      >
        <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z" />
        <polyline points="14 2 14 8 20 8" />
      </svg>
      {ext && (
        <span className="mt-1 rounded bg-yellow-900/40 px-1.5 py-0.5 text-[10px] font-medium uppercase text-yellow-400">
          {ext}
        </span>
      )}
      <p className="mt-1.5 max-w-full truncate text-center text-xs text-neutral-300">
        {filename}
      </p>
    </div>
  );
}
