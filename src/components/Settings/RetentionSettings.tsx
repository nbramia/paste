import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";

interface StorageStats {
  total_clips: number;
  pinboard_clips: number;
  favorite_clips: number;
  total_size_bytes: number;
  oldest_clip_date: string | null;
  newest_clip_date: string | null;
}

function formatBytes(bytes: number): string {
  if (bytes === 0) return "0 B";
  const units = ["B", "KB", "MB", "GB"];
  const i = Math.floor(Math.log(bytes) / Math.log(1024));
  const val = bytes / Math.pow(1024, i);
  return `${val.toFixed(i > 0 ? 1 : 0)} ${units[i]}`;
}

function formatDate(iso: string | null): string {
  if (!iso) return "\u2014";
  const d = new Date(iso);
  return d.toLocaleDateString();
}

export function RetentionSettings() {
  const [stats, setStats] = useState<StorageStats | null>(null);
  const [loading, setLoading] = useState(true);
  const [message, setMessage] = useState<string | null>(null);

  const loadStats = useCallback(async () => {
    try {
      setLoading(true);
      const result = await invoke<StorageStats>("get_storage_stats");
      setStats(result);
    } catch (err) {
      console.error("Failed to load storage stats:", err);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    loadStats();
  }, [loadStats]);

  const handleRunRetention = async () => {
    try {
      const deleted = await invoke<number>("run_retention");
      setMessage(
        deleted > 0 ? `Cleaned up ${deleted} clips` : "Nothing to clean up",
      );
      await loadStats();
      setTimeout(() => setMessage(null), 3000);
    } catch (err) {
      setMessage(`Error: ${err}`);
      setTimeout(() => setMessage(null), 5000);
    }
  };

  const handleClearHistory = async () => {
    try {
      const deleted = await invoke<number>("clear_all_history");
      setMessage(`Cleared ${deleted} clips`);
      await loadStats();
      setTimeout(() => setMessage(null), 3000);
    } catch (err) {
      setMessage(`Error: ${err}`);
      setTimeout(() => setMessage(null), 5000);
    }
  };

  if (loading)
    return <div className="text-xs text-text-muted">Loading...</div>;

  return (
    <div>
      <h3 className="mb-2 text-xs font-medium text-text-secondary">
        Storage & Retention
      </h3>

      {/* Stats */}
      {stats && (
        <div className="mb-3 grid grid-cols-2 gap-2">
          <div className="rounded bg-surface-secondary p-2">
            <p className="text-lg font-bold text-text-primary">
              {stats.total_clips}
            </p>
            <p className="text-[10px] text-text-muted">Total clips</p>
          </div>
          <div className="rounded bg-surface-secondary p-2">
            <p className="text-lg font-bold text-text-primary">
              {formatBytes(stats.total_size_bytes)}
            </p>
            <p className="text-[10px] text-text-muted">Total size</p>
          </div>
          <div className="rounded bg-surface-secondary p-2">
            <p className="text-lg font-bold text-text-primary">
              {stats.pinboard_clips}
            </p>
            <p className="text-[10px] text-text-muted">Pinboard clips</p>
          </div>
          <div className="rounded bg-surface-secondary p-2">
            <p className="text-lg font-bold text-text-primary">
              {stats.favorite_clips}
            </p>
            <p className="text-[10px] text-text-muted">Favorites</p>
          </div>
        </div>
      )}

      {stats && stats.oldest_clip_date && (
        <p className="mb-3 text-[10px] text-text-faint">
          History: {formatDate(stats.oldest_clip_date)} &mdash;{" "}
          {formatDate(stats.newest_clip_date)}
        </p>
      )}

      {/* Retention info */}
      <div className="mb-3 rounded border border-border-subtle bg-surface-secondary p-2 text-xs text-text-muted">
        <p>
          Auto-cleanup: clips older than 90 days or exceeding 10,000 items are
          removed.
        </p>
        <p className="mt-1">
          Pinboard clips and favorites are never deleted.
        </p>
        <p className="mt-1">
          Retention runs automatically on startup and every hour.
        </p>
      </div>

      {/* Message */}
      {message && (
        <div className="mb-2 rounded bg-accent-soft px-2 py-1 text-xs text-accent-muted">
          {message}
        </div>
      )}

      {/* Actions */}
      <div className="flex gap-2">
        <button
          onClick={handleRunRetention}
          className="rounded bg-surface-secondary px-3 py-1.5 text-xs text-text-secondary hover:text-text-primary"
        >
          Run Cleanup Now
        </button>
        <button
          onClick={handleClearHistory}
          className="rounded bg-red-950/30 px-3 py-1.5 text-xs text-red-400 hover:bg-red-950/40"
        >
          Clear All History
        </button>
      </div>
    </div>
  );
}
