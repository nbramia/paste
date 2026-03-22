import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";

export interface PinboardData {
  id: string;
  name: string;
  color: string;
  icon: string | null;
  position: number;
  created_at: string;
}

export function usePinboards() {
  const [pinboards, setPinboards] = useState<PinboardData[]>([]);
  const [loading, setLoading] = useState(true);

  const loadPinboards = useCallback(async () => {
    try {
      setLoading(true);
      const result = await invoke<PinboardData[]>("list_pinboards");
      setPinboards(result);
    } catch (err) {
      console.error("Failed to load pinboards:", err);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    loadPinboards();
  }, [loadPinboards]);

  const createPinboard = useCallback(async (name: string, color: string) => {
    try {
      await invoke("create_pinboard", { name, color });
      await loadPinboards();
    } catch (err) {
      console.error("Failed to create pinboard:", err);
    }
  }, [loadPinboards]);

  const updatePinboard = useCallback(async (id: string, name: string, color: string) => {
    try {
      await invoke("update_pinboard", { id, name, color });
      await loadPinboards();
    } catch (err) {
      console.error("Failed to update pinboard:", err);
    }
  }, [loadPinboards]);

  const deletePinboard = useCallback(async (id: string) => {
    try {
      await invoke("delete_pinboard", { id });
      await loadPinboards();
    } catch (err) {
      console.error("Failed to delete pinboard:", err);
    }
  }, [loadPinboards]);

  const addClipToPinboard = useCallback(async (clipId: string, pinboardId: string) => {
    try {
      await invoke("add_clip_to_pinboard", { clipId, pinboardId });
    } catch (err) {
      console.error("Failed to add clip to pinboard:", err);
    }
  }, []);

  const removeClipFromPinboard = useCallback(async (clipId: string) => {
    try {
      await invoke("remove_clip_from_pinboard", { clipId });
    } catch (err) {
      console.error("Failed to remove clip from pinboard:", err);
    }
  }, []);

  return {
    pinboards,
    loading,
    reload: loadPinboards,
    createPinboard,
    updatePinboard,
    deletePinboard,
    addClipToPinboard,
    removeClipFromPinboard,
  };
}
