interface TextCardProps {
  text: string;
}

export function TextCard({ text }: TextCardProps) {
  const truncated = text.length > 300 ? text.slice(0, 300) + "\u2026" : text;
  const lines = truncated.split("\n").slice(0, 8);
  const display = lines.join("\n") + (text.split("\n").length > 8 ? "\u2026" : "");

  return (
    <div className="flex-1 overflow-hidden px-3 py-2">
      <p className="whitespace-pre-wrap break-words text-sm leading-relaxed text-text-primary">
        {display}
      </p>
    </div>
  );
}
