import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { ClipData } from "../App";

const PAGE_SIZE = 50;

export function useClips() {
  const [clips, setClips] = useState<ClipData[]>([]);
  const [loading, setLoading] = useState(true);
  const [loadingMore, setLoadingMore] = useState(false);
  const [hasMore, setHasMore] = useState(true);

  const loadClips = useCallback(async () => {
    try {
      setLoading(true);
      const result = await invoke<ClipData[]>("get_clips", {
        offset: 0,
        limit: PAGE_SIZE,
      });
      setClips(result);
      setHasMore(result.length === PAGE_SIZE);
    } catch (err) {
      console.error("Failed to load clips:", err);
    } finally {
      setLoading(false);
    }
  }, []);

  const loadMore = useCallback(async () => {
    if (loadingMore || !hasMore) return;
    try {
      setLoadingMore(true);
      const result = await invoke<ClipData[]>("get_clips", {
        offset: clips.length,
        limit: PAGE_SIZE,
      });
      if (result.length > 0) {
        setClips((prev) => [...prev, ...result]);
      }
      setHasMore(result.length === PAGE_SIZE);
    } catch (err) {
      console.error("Failed to load more clips:", err);
    } finally {
      setLoadingMore(false);
    }
  }, [clips.length, hasMore, loadingMore]);

  useEffect(() => {
    loadClips();
  }, [loadClips]);

  return { clips, loading, loadingMore, hasMore, reload: loadClips, loadMore };
}
