import { useState, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { ClipData } from "../App";
import type { SearchFilters } from "../components/Search";

function dateRangeToISO(range: string | null): { from: string | null; to: string | null } {
  if (!range) return { from: null, to: null };

  const now = new Date();
  const to = now.toISOString();

  switch (range) {
    case "today": {
      const start = new Date(now);
      start.setHours(0, 0, 0, 0);
      return { from: start.toISOString(), to };
    }
    case "7d": {
      const start = new Date(now);
      start.setDate(start.getDate() - 7);
      return { from: start.toISOString(), to };
    }
    case "30d": {
      const start = new Date(now);
      start.setDate(start.getDate() - 30);
      return { from: start.toISOString(), to };
    }
    default:
      return { from: null, to: null };
  }
}

export function useSearch() {
  const [results, setResults] = useState<ClipData[]>([]);
  const [isSearching, setIsSearching] = useState(false);
  const [loading, setLoading] = useState(false);

  const search = useCallback(async (query: string, filters: SearchFilters) => {
    setIsSearching(true);
    setLoading(true);

    const { from, to } = dateRangeToISO(filters.dateRange);

    try {
      if (query.trim()) {
        // Use FTS5 search
        const result = await invoke<ClipData[]>("search_clips", {
          query,
          contentType: filters.contentType,
          sourceApp: filters.sourceApp,
          dateFrom: from,
          dateTo: to,
          pinboardId: null,
          isFavorite: filters.isFavorite || null,
        });
        setResults(result);
      } else {
        // No query text but filters active — use get_clips with filters
        const result = await invoke<ClipData[]>("get_clips", {
          offset: 0,
          limit: 50,
          contentType: filters.contentType,
          sourceApp: filters.sourceApp,
          isFavorite: filters.isFavorite || null,
        });
        setResults(result);
      }
    } catch (err) {
      console.error("Search failed:", err);
      setResults([]);
    } finally {
      setLoading(false);
    }
  }, []);

  const clearSearch = useCallback(() => {
    setResults([]);
    setIsSearching(false);
  }, []);

  return { results, isSearching, loading, search, clearSearch };
}
