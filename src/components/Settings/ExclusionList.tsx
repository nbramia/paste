import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";

export function ExclusionList() {
  const [apps, setApps] = useState<string[]>([]);
  const [newApp, setNewApp] = useState("");
  const [loading, setLoading] = useState(true);

  const loadApps = async () => {
    try {
      const result = await invoke<string[]>("get_excluded_apps");
      setApps(result);
    } catch (err) {
      console.error("Failed to load excluded apps:", err);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    loadApps();
  }, []);

  const handleAdd = async () => {
    if (!newApp.trim()) return;
    try {
      const updated = await invoke<string[]>("add_excluded_app", { appName: newApp.trim() });
      setApps(updated);
      setNewApp("");
    } catch (err) {
      console.error("Failed to add excluded app:", err);
    }
  };

  const handleRemove = async (app: string) => {
    try {
      const updated = await invoke<string[]>("remove_excluded_app", { appName: app });
      setApps(updated);
    } catch (err) {
      console.error("Failed to remove excluded app:", err);
    }
  };

  if (loading) return <div className="text-text-muted text-xs">Loading...</div>;

  return (
    <div>
      <h3 className="mb-2 text-xs font-medium text-text-secondary">
        Excluded Applications
      </h3>
      <p className="mb-2 text-[10px] text-text-faint">
        Clipboard content from these apps will not be captured (case-insensitive substring match).
      </p>

      <div className="mb-2 space-y-1">
        {apps.map((app) => (
          <div
            key={app}
            className="flex items-center justify-between rounded bg-surface-secondary px-2 py-1"
          >
            <span className="text-xs text-text-primary">{app}</span>
            <button
              onClick={() => handleRemove(app)}
              className="text-text-faint hover:text-red-400"
              title="Remove"
            >
              <svg className="h-3 w-3" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                <path d="M18 6L6 18M6 6l12 12" />
              </svg>
            </button>
          </div>
        ))}
        {apps.length === 0 && (
          <p className="text-[10px] text-text-faint">No excluded apps</p>
        )}
      </div>

      <div className="flex gap-2">
        <input
          type="text"
          value={newApp}
          onChange={(e) => setNewApp(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === "Enter") handleAdd();
            e.stopPropagation();
          }}
          placeholder="App name (e.g., keepassxc)"
          className="flex-1 rounded bg-surface-secondary px-2 py-1 text-xs text-text-primary placeholder-text-faint outline-none focus:ring-1 focus:ring-accent"
        />
        <button
          onClick={handleAdd}
          disabled={!newApp.trim()}
          className="rounded bg-accent px-2 py-1 text-xs text-white hover:bg-accent-hover disabled:opacity-50"
        >
          Add
        </button>
      </div>
    </div>
  );
}
