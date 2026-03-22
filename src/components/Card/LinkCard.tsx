interface LinkCardProps {
  text: string;
  metadata: string | null;
}

function extractDomain(url: string): string {
  try {
    const u = new URL(url.trim());
    return u.hostname.replace(/^www\./, "");
  } catch {
    return url.trim().slice(0, 30);
  }
}

function extractPath(url: string): string {
  try {
    const u = new URL(url.trim());
    const path = u.pathname + u.search;
    return path.length > 1 ? path : "";
  } catch {
    return "";
  }
}

export function LinkCard({ text, metadata }: LinkCardProps) {
  const url = text.trim();
  const domain = extractDomain(url);
  const path = extractPath(url);

  let title: string | null = null;
  if (metadata) {
    try {
      const parsed = JSON.parse(metadata);
      title = parsed.title || null;
    } catch {
      /* ignore */
    }
  }

  return (
    <div className="flex flex-1 flex-col justify-center overflow-hidden px-3 py-2">
      {/* Globe icon + domain */}
      <div className="flex items-center gap-1.5">
        <svg
          className="h-3.5 w-3.5 shrink-0 text-purple-500 dark:text-purple-400"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          strokeWidth="2"
        >
          <circle cx="12" cy="12" r="10" />
          <path d="M2 12h20M12 2a15.3 15.3 0 0 1 4 10 15.3 15.3 0 0 1-4 10 15.3 15.3 0 0 1-4-10 15.3 15.3 0 0 1 4-10z" />
        </svg>
        <span className="truncate text-xs font-medium text-purple-700 dark:text-purple-300">
          {domain}
        </span>
      </div>

      {/* Title if available */}
      {title && (
        <p className="mt-1.5 line-clamp-2 text-xs leading-snug text-text-primary">
          {title}
        </p>
      )}

      {/* Path */}
      {path && (
        <p className="mt-1 truncate text-[10px] text-text-muted">{path}</p>
      )}

      {/* Full URL at bottom */}
      <p className="mt-auto truncate pt-2 text-[10px] text-text-faint">
        {url}
      </p>
    </div>
  );
}
