interface InjectionSettingsProps {
  injection: {
    method: string;
  };
  onChange: (injection: InjectionSettingsProps["injection"]) => void;
}

export function InjectionSettings({ injection, onChange }: InjectionSettingsProps) {
  return (
    <div>
      <h3 className="mb-2 text-xs font-medium text-text-secondary">Text Injection</h3>
      <div className="space-y-2">
        <div className="flex items-center justify-between">
          <label className="text-xs text-text-muted">Method</label>
          <select
            value={injection.method}
            onChange={(e) => onChange({ ...injection, method: e.target.value })}
            className="w-40 rounded bg-surface-secondary px-2 py-1 text-xs text-text-primary outline-none"
          >
            <option value="auto">Auto-detect</option>
            <option value="xdotool">xdotool (X11)</option>
            <option value="ydotool">ydotool (Wayland)</option>
            <option value="wtype">wtype (wlroots)</option>
            <option value="clipboard">Clipboard fallback</option>
          </select>
        </div>
      </div>
    </div>
  );
}
