import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";

export function AutostartSettings() {
  const [installed, setInstalled] = useState(false);
  const [loading, setLoading] = useState(true);
  const [message, setMessage] = useState<string | null>(null);

  useEffect(() => {
    invoke<boolean>("get_autostart_status")
      .then(setInstalled)
      .catch(() => setInstalled(false))
      .finally(() => setLoading(false));
  }, []);

  const handleToggle = async () => {
    try {
      if (installed) {
        const msg = await invoke<string>("uninstall_autostart");
        setInstalled(false);
        setMessage(msg);
      } else {
        const msg = await invoke<string>("install_autostart");
        setInstalled(true);
        setMessage(msg);
      }
      setTimeout(() => setMessage(null), 3000);
    } catch (err) {
      setMessage(`Error: ${err}`);
      setTimeout(() => setMessage(null), 5000);
    }
  };

  if (loading) return <div className="text-text-muted text-xs">Loading...</div>;

  return (
    <div>
      <h3 className="mb-2 text-xs font-medium text-text-secondary">Autostart</h3>
      <div className="flex items-center justify-between">
        <div>
          <p className="text-xs text-text-muted">Start Paste on login</p>
          <p className="text-[10px] text-text-faint">
            Uses a systemd user service for automatic startup
          </p>
        </div>
        <button
          onClick={handleToggle}
          className={`rounded px-3 py-1.5 text-xs font-medium transition-colors ${
            installed
              ? "bg-accent-soft text-accent-muted hover:bg-accent-soft"
              : "bg-surface-secondary text-text-muted hover:text-text-primary"
          }`}
        >
          {installed ? "Enabled" : "Disabled"}
        </button>
      </div>
      {message && (
        <p className="mt-1 text-[10px] text-accent-muted">{message}</p>
      )}
    </div>
  );
}
