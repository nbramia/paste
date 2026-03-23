interface ClipboardSettingsProps {
  clipboard: {
    monitor_primary: boolean;
    monitor_clipboard: boolean;
    excluded_apps: string[];
    max_content_size_mb: number;
  };
  onChange: (clipboard: ClipboardSettingsProps["clipboard"]) => void;
}

export function ClipboardSettings({ clipboard, onChange }: ClipboardSettingsProps) {
  return (
    <div>
      <h3 className="mb-2 text-xs font-medium text-text-secondary">Clipboard</h3>
      <div className="space-y-2">
        <label className="flex items-center justify-between">
          <span className="text-xs text-text-muted">Monitor CLIPBOARD (Ctrl+C)</span>
          <input
            type="checkbox"
            checked={clipboard.monitor_clipboard}
            onChange={(e) => onChange({ ...clipboard, monitor_clipboard: e.target.checked })}
            className="h-4 w-4 rounded accent-accent"
          />
        </label>
        <label className="flex items-center justify-between">
          <span className="text-xs text-text-muted">Monitor PRIMARY (mouse select)</span>
          <input
            type="checkbox"
            checked={clipboard.monitor_primary}
            onChange={(e) => onChange({ ...clipboard, monitor_primary: e.target.checked })}
            className="h-4 w-4 rounded accent-accent"
          />
        </label>
        <div className="flex items-center justify-between">
          <label className="text-xs text-text-muted">Max content size (MB)</label>
          <input
            type="number"
            value={clipboard.max_content_size_mb}
            onChange={(e) => onChange({ ...clipboard, max_content_size_mb: parseInt(e.target.value) || 1 })}
            onKeyDown={(e) => e.stopPropagation()}
            min={1}
            max={100}
            className="w-20 rounded bg-surface-secondary px-2 py-1 text-right text-xs text-text-primary outline-none focus:ring-1 focus:ring-accent"
          />
        </div>
      </div>
    </div>
  );
}
