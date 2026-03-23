interface CodeCardProps {
  text: string;
  metadata: string | null;
}

function detectLanguage(text: string, metadata: string | null): string | null {
  // Check metadata first
  if (metadata) {
    try {
      const parsed = JSON.parse(metadata);
      if (parsed.language) return parsed.language;
    } catch {
      /* ignore */
    }
  }

  const trimmed = text.trim();

  // Heuristic language detection from content
  if (trimmed.includes("fn ") && trimmed.includes("->")) return "Rust";
  if (trimmed.includes("def ") && trimmed.includes(":")) return "Python";
  if (
    trimmed.includes("function ") ||
    (trimmed.includes("const ") && trimmed.includes("=>"))
  )
    return "JavaScript";
  if (trimmed.includes("import ") && trimmed.includes("from "))
    return "Python";
  if (trimmed.includes("#include")) return "C/C++";
  if (trimmed.includes("package ") && trimmed.includes("func ")) return "Go";
  if (trimmed.includes("class ") && trimmed.includes("{")) return "Java";
  if (
    trimmed.includes("SELECT ") ||
    trimmed.includes("INSERT ") ||
    trimmed.includes("CREATE ")
  )
    return "SQL";
  if (trimmed.startsWith("{") && trimmed.endsWith("}")) return "JSON";
  if (trimmed.startsWith("---") || trimmed.includes(": ")) return "YAML";
  if (trimmed.startsWith("<!DOCTYPE") || trimmed.startsWith("<html"))
    return "HTML";
  if (trimmed.startsWith("#!/bin/")) return "Shell";

  return null;
}

export function CodeCard({ text, metadata }: CodeCardProps) {
  const language = detectLanguage(text, metadata);
  const lines = text.split("\n").slice(0, 8);
  const display =
    lines.join("\n") + (text.split("\n").length > 8 ? "\n\u2026" : "");

  return (
    <div className="flex flex-1 flex-col overflow-hidden">
      {language && (
        <div className="flex justify-end px-2 pt-1.5">
          <span className="rounded bg-green-100 text-green-700 dark:bg-accent-soft dark:text-accent-muted px-1.5 py-0.5 text-xs font-medium">
            {language}
          </span>
        </div>
      )}
      <div className="flex-1 overflow-hidden px-3 py-2">
        <pre className="whitespace-pre-wrap break-words font-mono text-sm leading-relaxed text-emerald-800 dark:text-emerald-400">
          {display}
        </pre>
      </div>
    </div>
  );
}
