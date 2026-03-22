interface TextCardProps {
  text: string;
}

export function TextCard({ text }: TextCardProps) {
  const truncated = text.length > 200 ? text.slice(0, 200) + "\u2026" : text;
  const lines = truncated.split("\n").slice(0, 6);
  const display = lines.join("\n") + (text.split("\n").length > 6 ? "\u2026" : "");

  return (
    <div className="flex-1 overflow-hidden px-3 py-2">
      <p className="whitespace-pre-wrap break-words text-xs leading-relaxed text-neutral-300">
        {display}
      </p>
    </div>
  );
}
