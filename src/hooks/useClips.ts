import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { ClipData } from "../App";

export function useClips() {
  const [clips, setClips] = useState<ClipData[]>([]);
  const [loading, setLoading] = useState(true);

  const loadClips = useCallback(async () => {
    try {
      setLoading(true);
      const result = await invoke<ClipData[]>("get_clips", {
        offset: 0,
        limit: 50,
      });
      setClips(result);
    } catch (err) {
      console.error("Failed to load clips:", err);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    loadClips();
  }, [loadClips]);

  return { clips, loading, reload: loadClips };
}
