interface ExpanderSettingsProps {
  expander: {
    enabled: boolean;
    trigger: string;
    typing_speed: number;
  };
  onChange: (expander: ExpanderSettingsProps["expander"]) => void;
}

export function ExpanderSettings({ expander, onChange }: ExpanderSettingsProps) {
  return (
    <div>
      <h3 className="mb-2 text-xs font-medium text-text-secondary">Text Expander</h3>
      <div className="space-y-2">
        <label className="flex items-center justify-between">
          <span className="text-xs text-text-muted">Enabled</span>
          <input
            type="checkbox"
            checked={expander.enabled}
            onChange={(e) => onChange({ ...expander, enabled: e.target.checked })}
            className="h-4 w-4 rounded accent-accent"
          />
        </label>
        <div className="flex items-center justify-between">
          <label className="text-xs text-text-muted">Trigger mode</label>
          <select
            value={expander.trigger}
            onChange={(e) => onChange({ ...expander, trigger: e.target.value })}
            className="w-40 rounded bg-surface-secondary px-2 py-1 text-xs text-text-primary outline-none"
          >
            <option value="word_boundary">Word boundary</option>
            <option value="immediate">Immediate</option>
          </select>
        </div>
        <div className="flex items-center justify-between">
          <label className="text-xs text-text-muted">Typing delay (ms)</label>
          <input
            type="number"
            value={expander.typing_speed}
            onChange={(e) => onChange({ ...expander, typing_speed: parseInt(e.target.value) || 0 })}
            onKeyDown={(e) => e.stopPropagation()}
            min={0}
            max={100}
            className="w-20 rounded bg-surface-secondary px-2 py-1 text-right text-xs text-text-primary outline-none focus:ring-1 focus:ring-accent"
          />
        </div>
      </div>
    </div>
  );
}
