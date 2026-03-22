interface CardProps {
  id: string;
  contentType: string;
  preview: string;
}

export function Card({ id: _id, contentType: _contentType, preview }: CardProps) {
  return (
    <div className="flex h-full w-48 shrink-0 flex-col rounded-lg border border-neutral-700 bg-neutral-800 p-3">
      <p className="line-clamp-4 text-sm text-neutral-300">{preview}</p>
    </div>
  );
}
