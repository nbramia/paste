import { useState, useEffect, useRef } from "react";

interface FillInField {
  type: "text" | "textarea" | "popup";
  name: string;
  default_value?: string | null;
  options?: string[];
}

interface FillInDialogProps {
  title: string;
  fields: FillInField[];
  onSubmit: (values: Record<string, string>) => void;
  onCancel: () => void;
}

export function FillInDialog({
  title,
  fields,
  onSubmit,
  onCancel,
}: FillInDialogProps) {
  const [values, setValues] = useState<Record<string, string>>(() => {
    const initial: Record<string, string> = {};
    for (const field of fields) {
      if (field.type === "popup" && field.options && field.options.length > 0) {
        initial[field.name] = field.options[0];
      } else {
        initial[field.name] = field.default_value || "";
      }
    }
    return initial;
  });

  const firstInputRef = useRef<
    HTMLInputElement | HTMLTextAreaElement | HTMLSelectElement
  >(null);

  useEffect(() => {
    firstInputRef.current?.focus();
  }, []);

  const handleChange = (name: string, value: string) => {
    setValues((prev) => ({ ...prev, [name]: value }));
  };

  const handleSubmit = () => {
    onSubmit(values);
  };

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === "Enter" && !e.shiftKey) {
      // Don't submit from textarea (allow newlines with plain Enter there)
      const target = e.target as HTMLElement;
      if (target.tagName !== "TEXTAREA") {
        e.preventDefault();
        handleSubmit();
      }
    }
    if (e.key === "Escape") {
      e.preventDefault();
      onCancel();
    }
    e.stopPropagation();
  };

  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center bg-black/50"
      role="dialog"
      aria-modal="true"
      aria-label={title}
      onClick={onCancel}
    >
      <div
        className="w-96 max-h-[80vh] overflow-y-auto rounded-lg border border-border-default bg-surface-primary p-4 shadow-xl"
        onClick={(e) => e.stopPropagation()}
        onKeyDown={handleKeyDown}
      >
        <h3 className="mb-3 text-sm font-medium text-text-primary">{title}</h3>

        <div className="space-y-3">
          {fields.map((field, index) => (
            <div key={field.name}>
              <label className="mb-1 block text-xs font-medium text-text-muted capitalize">
                {field.name.replace(/_/g, " ")}
              </label>

              {field.type === "text" && (
                <input
                  ref={
                    index === 0
                      ? (firstInputRef as React.RefObject<HTMLInputElement>)
                      : undefined
                  }
                  type="text"
                  value={values[field.name] || ""}
                  onChange={(e) => handleChange(field.name, e.target.value)}
                  className="w-full rounded bg-surface-secondary px-3 py-1.5 text-sm text-text-primary outline-none focus:ring-1 focus:ring-blue-500"
                />
              )}

              {field.type === "textarea" && (
                <textarea
                  ref={
                    index === 0
                      ? (firstInputRef as React.RefObject<HTMLTextAreaElement>)
                      : undefined
                  }
                  value={values[field.name] || ""}
                  onChange={(e) => handleChange(field.name, e.target.value)}
                  rows={3}
                  className="w-full rounded bg-surface-secondary px-3 py-1.5 text-sm text-text-primary outline-none focus:ring-1 focus:ring-blue-500"
                />
              )}

              {field.type === "popup" && (
                <select
                  ref={
                    index === 0
                      ? (firstInputRef as React.RefObject<HTMLSelectElement>)
                      : undefined
                  }
                  value={values[field.name] || ""}
                  onChange={(e) => handleChange(field.name, e.target.value)}
                  className="w-full rounded bg-surface-secondary px-3 py-1.5 text-sm text-text-primary outline-none focus:ring-1 focus:ring-blue-500"
                >
                  {field.options?.map((opt) => (
                    <option key={opt} value={opt}>
                      {opt}
                    </option>
                  ))}
                </select>
              )}
            </div>
          ))}
        </div>

        <div className="mt-4 flex justify-end gap-2">
          <button
            onClick={onCancel}
            className="rounded px-3 py-1.5 text-xs text-text-muted hover:text-text-primary"
          >
            Cancel
          </button>
          <button
            onClick={handleSubmit}
            className="rounded bg-blue-600 px-3 py-1.5 text-xs font-medium text-white hover:bg-blue-500"
          >
            Expand
          </button>
        </div>
      </div>
    </div>
  );
}
