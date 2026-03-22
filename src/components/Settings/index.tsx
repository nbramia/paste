import { ExclusionList } from "./ExclusionList";
import { RetentionSettings } from "./RetentionSettings";

export function Settings() {
  return (
    <div className="flex flex-1 flex-col overflow-y-auto p-4">
      <h2 className="mb-4 text-sm font-medium text-text-secondary">Settings</h2>

      <div className="max-w-md space-y-6">
        <RetentionSettings />

        <div className="border-t border-border-default pt-4">
          <ExclusionList />
        </div>

        <div className="border-t border-border-default pt-4">
          <p className="text-xs text-text-faint">
            More settings coming soon (hotkeys, UI theme, text expander).
          </p>
        </div>
      </div>
    </div>
  );
}
