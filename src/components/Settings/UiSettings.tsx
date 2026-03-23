interface UiSettingsProps {
  ui: {
    theme: string;
    filmstrip_height: number;
    cards_visible: number;
    animation_speed: number;
    blur_background: boolean;
  };
  onChange: (ui: UiSettingsProps["ui"]) => void;
  onThemeChange: (theme: "system" | "light" | "dark") => void;
}

export function UiSettings({ ui, onChange, onThemeChange }: UiSettingsProps) {
  const handleThemeChange = (theme: string) => {
    onChange({ ...ui, theme });
    onThemeChange(theme as "system" | "light" | "dark");
  };

  return (
    <div>
      <h3 className="mb-2 text-xs font-medium text-text-secondary">Appearance</h3>
      <div className="space-y-2">
        <div className="flex items-center justify-between">
          <label className="text-xs text-text-muted">Theme</label>
          <select
            value={ui.theme}
            onChange={(e) => handleThemeChange(e.target.value)}
            className="w-32 rounded bg-surface-secondary px-2 py-1 text-xs text-text-primary outline-none"
          >
            <option value="system">System</option>
            <option value="light">Light</option>
            <option value="dark">Dark</option>
          </select>
        </div>
        <div className="flex items-center justify-between">
          <label className="text-xs text-text-muted">Filmstrip height (px)</label>
          <input
            type="number"
            value={ui.filmstrip_height}
            onChange={(e) => onChange({ ...ui, filmstrip_height: parseInt(e.target.value) || 200 })}
            onKeyDown={(e) => e.stopPropagation()}
            min={100}
            max={800}
            className="w-20 rounded bg-surface-secondary px-2 py-1 text-right text-xs text-text-primary outline-none focus:ring-1 focus:ring-accent"
          />
        </div>
        <div className="flex items-center justify-between">
          <label className="text-xs text-text-muted">Cards visible</label>
          <input
            type="number"
            value={ui.cards_visible}
            onChange={(e) => onChange({ ...ui, cards_visible: parseInt(e.target.value) || 4 })}
            onKeyDown={(e) => e.stopPropagation()}
            min={2}
            max={20}
            className="w-20 rounded bg-surface-secondary px-2 py-1 text-right text-xs text-text-primary outline-none focus:ring-1 focus:ring-accent"
          />
        </div>
        <div className="flex items-center justify-between">
          <label className="text-xs text-text-muted">Animation speed</label>
          <input
            type="number"
            value={ui.animation_speed}
            onChange={(e) => onChange({ ...ui, animation_speed: parseFloat(e.target.value) || 0 })}
            onKeyDown={(e) => e.stopPropagation()}
            min={0}
            max={3}
            step={0.1}
            className="w-20 rounded bg-surface-secondary px-2 py-1 text-right text-xs text-text-primary outline-none focus:ring-1 focus:ring-accent"
          />
        </div>
        <label className="flex items-center justify-between">
          <span className="text-xs text-text-muted">Blur background</span>
          <input
            type="checkbox"
            checked={ui.blur_background}
            onChange={(e) => onChange({ ...ui, blur_background: e.target.checked })}
            className="h-4 w-4 rounded accent-accent"
          />
        </label>
      </div>
    </div>
  );
}
