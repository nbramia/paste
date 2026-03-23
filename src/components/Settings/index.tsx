import { useState, useEffect, useCallback, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { ExclusionList } from "./ExclusionList";
import { RetentionSettings } from "./RetentionSettings";
import { HotkeySettings } from "./HotkeySettings";
import { ClipboardSettings } from "./ClipboardSettings";
import { UiSettings } from "./UiSettings";
import { ExpanderSettings } from "./ExpanderSettings";
import { InjectionSettings } from "./InjectionSettings";
import { AutostartSettings } from "./AutostartSettings";
import { useTheme } from "../../hooks/useTheme";

interface AppConfig {
  hotkeys: {
    toggle_overlay: string;
    paste_stack_mode: string;
    quick_copy_to_pinboard: string;
    toggle_expander: string;
  };
  clipboard: {
    monitor_primary: boolean;
    monitor_clipboard: boolean;
    excluded_apps: string[];
    max_content_size_mb: number;
  };
  storage: {
    max_history_days: number;
    max_history_count: number;
    max_image_size_mb: number;
    max_total_storage_mb: number;
    db_path: string;
    image_dir: string;
  };
  ui: {
    theme: string;
    filmstrip_height: number;
    cards_visible: number;
    animation_speed: number;
    blur_background: boolean;
  };
  expander: {
    enabled: boolean;
    trigger: string;
    typing_speed: number;
  };
  injection: {
    method: string;
  };
}

export function Settings() {
  const [config, setConfig] = useState<AppConfig | null>(null);
  const [loading, setLoading] = useState(true);
  const [saveMessage, setSaveMessage] = useState<string | null>(null);
  const { setTheme } = useTheme();
  const saveTimeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  const loadConfig = useCallback(async () => {
    try {
      setLoading(true);
      const result = await invoke<AppConfig>("get_config");
      setConfig(result);
    } catch (err) {
      console.error("Failed to load config:", err);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    loadConfig();
  }, [loadConfig]);

  const saveConfig = useCallback(async (newConfig: AppConfig) => {
    // Debounce saves
    if (saveTimeoutRef.current) clearTimeout(saveTimeoutRef.current);
    saveTimeoutRef.current = setTimeout(async () => {
      try {
        await invoke("save_config", { config: newConfig });
        setSaveMessage("Saved");
        setTimeout(() => setSaveMessage(null), 1500);
      } catch (err) {
        setSaveMessage(`Error: ${err}`);
        setTimeout(() => setSaveMessage(null), 3000);
      }
    }, 300);
  }, []);

  const updateConfig = useCallback(
    (partial: Partial<AppConfig>) => {
      if (!config) return;
      const newConfig = { ...config, ...partial };
      setConfig(newConfig);
      saveConfig(newConfig);
    },
    [config, saveConfig],
  );

  const handleReset = async () => {
    try {
      const defaultConfig = await invoke<AppConfig>("reset_config");
      setConfig(defaultConfig);
      setTheme("system");
      setSaveMessage("Reset to defaults");
      setTimeout(() => setSaveMessage(null), 2000);
    } catch (err) {
      setSaveMessage(`Error: ${err}`);
      setTimeout(() => setSaveMessage(null), 3000);
    }
  };

  if (loading || !config) {
    return (
      <div className="flex flex-1 items-center justify-center text-text-muted">
        Loading settings...
      </div>
    );
  }

  return (
    <div className="flex flex-1 flex-col overflow-y-auto p-4">
      <div className="mb-4 flex items-center justify-between">
        <h2 className="font-heading text-sm font-semibold text-text-secondary tracking-wide">Settings</h2>
        <div className="flex items-center gap-2">
          {saveMessage && (
            <span className="text-[10px] text-accent-muted">{saveMessage}</span>
          )}
          <button
            onClick={handleReset}
            className="rounded px-2 py-1 text-[10px] text-text-faint hover:text-text-primary"
          >
            Reset to defaults
          </button>
        </div>
      </div>

      <div className="max-w-md space-y-6">
        <UiSettings
          ui={config.ui}
          onChange={(ui) => updateConfig({ ui })}
          onThemeChange={setTheme}
        />

        <div className="border-t border-border-default pt-4">
          <AutostartSettings />
        </div>

        <div className="border-t border-border-default pt-4">
          <HotkeySettings
            hotkeys={config.hotkeys}
            onChange={(hotkeys) => updateConfig({ hotkeys })}
          />
        </div>

        <div className="border-t border-border-default pt-4">
          <ClipboardSettings
            clipboard={config.clipboard}
            onChange={(clipboard) => updateConfig({ clipboard })}
          />
        </div>

        <div className="border-t border-border-default pt-4">
          <ExclusionList />
        </div>

        <div className="border-t border-border-default pt-4">
          <RetentionSettings />
        </div>

        <div className="border-t border-border-default pt-4">
          <ExpanderSettings
            expander={config.expander}
            onChange={(expander) => updateConfig({ expander })}
          />
        </div>

        <div className="border-t border-border-default pt-4">
          <InjectionSettings
            injection={config.injection}
            onChange={(injection) => updateConfig({ injection })}
          />
        </div>

        <div className="border-t border-border-default pt-4 pb-8">
          <h3 className="mb-1 text-xs font-medium text-text-secondary">About</h3>
          <p className="text-xs text-text-muted">Paste v0.1.0</p>
          <p className="text-[10px] text-text-faint">
            macOS Paste-quality clipboard manager for Linux
          </p>
        </div>
      </div>
    </div>
  );
}
