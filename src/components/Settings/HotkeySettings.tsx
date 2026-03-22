interface HotkeySettingsProps {
  hotkeys: {
    toggle_overlay: string;
    paste_stack_mode: string;
    quick_copy_to_pinboard: string;
    toggle_expander: string;
  };
  onChange: (hotkeys: HotkeySettingsProps["hotkeys"]) => void;
}

export function HotkeySettings({ hotkeys, onChange }: HotkeySettingsProps) {
  const entries: { key: keyof typeof hotkeys; label: string }[] = [
    { key: "toggle_overlay", label: "Toggle filmstrip" },
    { key: "paste_stack_mode", label: "Paste Stack mode" },
    { key: "quick_copy_to_pinboard", label: "Save to pinboard" },
    { key: "toggle_expander", label: "Toggle text expander" },
  ];

  return (
    <div>
      <h3 className="mb-2 text-xs font-medium text-text-secondary">Hotkeys</h3>
      <div className="space-y-2">
        {entries.map(({ key, label }) => (
          <div key={key} className="flex items-center justify-between">
            <label className="text-xs text-text-muted">{label}</label>
            <input
              type="text"
              value={hotkeys[key]}
              onChange={(e) => onChange({ ...hotkeys, [key]: e.target.value })}
              onKeyDown={(e) => e.stopPropagation()}
              className="w-40 rounded bg-surface-secondary px-2 py-1 text-right text-xs text-text-primary outline-none focus:ring-1 focus:ring-blue-500"
            />
          </div>
        ))}
      </div>
      <p className="mt-1 text-[10px] text-text-faint">
        Format: Super+V, Ctrl+Shift+C, etc. Changes apply on restart.
      </p>
    </div>
  );
}
