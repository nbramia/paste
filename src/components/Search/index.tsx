import { useState, useEffect, useRef, useCallback } from "react";
import { motion, AnimatePresence } from "framer-motion";
import { invoke } from "@tauri-apps/api/core";

interface SearchProps {
  onSearch: (query: string, filters: SearchFilters) => void;
  onClear: () => void;
  searchRef: React.RefObject<HTMLInputElement | null>;
}

export interface SearchFilters {
  contentType: string | null;
  sourceApp: string | null;
  dateRange: string | null; // "today" | "7d" | "30d" | null
  isFavorite: boolean;
}

export function Search({ onSearch, onClear, searchRef }: SearchProps) {
  const [query, setQuery] = useState("");
  const [showPowerSearch, setShowPowerSearch] = useState(false);
  const [filters, setFilters] = useState<SearchFilters>({
    contentType: null,
    sourceApp: null,
    dateRange: null,
    isFavorite: false,
  });
  const [sourceApps, setSourceApps] = useState<string[]>([]);
  const debounceRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  // Load source apps for dropdown
  useEffect(() => {
    if (showPowerSearch) {
      invoke<string[]>("get_source_apps")
        .then(setSourceApps)
        .catch(() => setSourceApps([]));
    }
  }, [showPowerSearch]);

  // Debounced search
  const triggerSearch = useCallback(
    (q: string, f: SearchFilters) => {
      if (debounceRef.current) clearTimeout(debounceRef.current);
      debounceRef.current = setTimeout(() => {
        if (q.trim() || f.contentType || f.sourceApp || f.dateRange || f.isFavorite) {
          onSearch(q, f);
        } else {
          onClear();
        }
      }, 100);
    },
    [onSearch, onClear],
  );

  const handleQueryChange = (value: string) => {
    setQuery(value);
    triggerSearch(value, filters);
  };

  const handleFilterChange = (newFilters: SearchFilters) => {
    setFilters(newFilters);
    triggerSearch(query, newFilters);
  };

  const handleClear = () => {
    setQuery("");
    setFilters({ contentType: null, sourceApp: null, dateRange: null, isFavorite: false });
    setShowPowerSearch(false);
    onClear();
  };

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === "Escape") {
      e.preventDefault();
      e.stopPropagation();
      if (query || filters.contentType || filters.sourceApp || filters.dateRange || filters.isFavorite) {
        handleClear();
      } else {
        (e.target as HTMLInputElement).blur();
      }
    }
    // Prevent arrow keys and Enter from propagating to App's handler while typing
    if (["ArrowLeft", "ArrowRight", "Enter", "Delete", "Backspace", "Tab"].includes(e.key)) {
      e.stopPropagation();
    }
  };

  return (
    <div className="border-b border-border-default px-4 py-2" role="search">
      <div className="flex items-center gap-2">
        {/* Search icon */}
        <svg
          className="h-4 w-4 shrink-0 text-text-muted"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          strokeWidth="2"
        >
          <circle cx="11" cy="11" r="8" />
          <path d="M21 21l-4.35-4.35" />
        </svg>

        <input
          ref={searchRef}
          type="text"
          role="searchbox"
          aria-label="Search clipboard history"
          value={query}
          onChange={(e) => handleQueryChange(e.target.value)}
          onKeyDown={handleKeyDown}
          placeholder="Search clips... (/ or Ctrl+F)"
          className="flex-1 bg-transparent text-sm text-text-primary placeholder-text-muted outline-none"
        />

        {/* Power Search toggle */}
        <button
          onClick={() => setShowPowerSearch(!showPowerSearch)}
          aria-expanded={showPowerSearch}
          aria-label={showPowerSearch ? "Hide search filters" : "Show search filters"}
          className={`rounded px-2 py-0.5 text-xs transition-colors ${
            showPowerSearch
              ? "bg-accent text-white"
              : "text-text-muted hover:text-text-secondary"
          }`}
          title="Toggle Power Search (Ctrl+F)"
        >
          Filters
        </button>

        {/* Clear button */}
        {(query || filters.contentType || filters.sourceApp || filters.dateRange || filters.isFavorite) && (
          <button
            onClick={handleClear}
            className="text-text-muted hover:text-text-primary"
            title="Clear search"
          >
            <svg className="h-4 w-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
              <path d="M18 6L6 18M6 6l12 12" />
            </svg>
          </button>
        )}
      </div>

      {/* Power Search filters */}
      <AnimatePresence>
        {showPowerSearch && (
          <motion.div
            initial={{ height: 0, opacity: 0 }}
            animate={{ height: "auto", opacity: 1 }}
            exit={{ height: 0, opacity: 0 }}
            transition={{ duration: 0.15, ease: "easeOut" }}
            className="overflow-hidden"
          >
            <div className="mt-2 flex flex-wrap items-center gap-2">
              {/* Content type */}
              <select
                value={filters.contentType || ""}
                onChange={(e) =>
                  handleFilterChange({ ...filters, contentType: e.target.value || null })
                }
                className="rounded bg-surface-secondary px-2 py-1 text-xs text-text-secondary outline-none"
              >
                <option value="">All types</option>
                <option value="text">Text</option>
                <option value="code">Code</option>
                <option value="link">Link</option>
                <option value="image">Image</option>
                <option value="file">File</option>
              </select>

              {/* Source app */}
              <select
                value={filters.sourceApp || ""}
                onChange={(e) =>
                  handleFilterChange({ ...filters, sourceApp: e.target.value || null })
                }
                className="rounded bg-surface-secondary px-2 py-1 text-xs text-text-secondary outline-none"
              >
                <option value="">All apps</option>
                {sourceApps.map((app) => (
                  <option key={app} value={app}>
                    {app}
                  </option>
                ))}
              </select>

              {/* Date range */}
              <div className="flex gap-1">
                {[
                  { label: "Today", value: "today" },
                  { label: "7d", value: "7d" },
                  { label: "30d", value: "30d" },
                ].map(({ label, value }) => (
                  <button
                    key={value}
                    onClick={() =>
                      handleFilterChange({
                        ...filters,
                        dateRange: filters.dateRange === value ? null : value,
                      })
                    }
                    className={`rounded px-2 py-0.5 text-xs transition-colors ${
                      filters.dateRange === value
                        ? "bg-accent text-white"
                        : "bg-surface-secondary text-text-muted hover:text-text-secondary"
                    }`}
                  >
                    {label}
                  </button>
                ))}
              </div>

              {/* Favorites filter */}
              <button
                onClick={() =>
                  handleFilterChange({ ...filters, isFavorite: !filters.isFavorite })
                }
                className={`rounded px-2 py-0.5 text-xs transition-colors ${
                  filters.isFavorite
                    ? "bg-amber-600 text-white"
                    : "bg-surface-secondary text-text-muted hover:text-text-primary"
                }`}
              >
                ★ Favorites
              </button>
            </div>
          </motion.div>
        )}
      </AnimatePresence>
    </div>
  );
}
