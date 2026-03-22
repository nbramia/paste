import { RefObject } from "react";
import { Card } from "../Card";
import type { ClipData } from "../../App";

interface FilmstripProps {
  clips: ClipData[];
  selectedIndex: number;
  onSelect: (index: number) => void;
  onPaste: () => void;
  loading: boolean;
  containerRef: RefObject<HTMLDivElement | null>;
}

export function Filmstrip({
  clips,
  selectedIndex,
  onSelect,
  onPaste,
  loading,
  containerRef,
}: FilmstripProps) {
  if (loading) {
    return (
      <div className="flex flex-1 items-center justify-center text-neutral-500">
        Loading...
      </div>
    );
  }

  if (clips.length === 0) {
    return (
      <div className="flex flex-1 items-center justify-center text-neutral-500">
        <div className="text-center">
          <p className="text-lg">No clipboard history yet</p>
          <p className="mt-1 text-sm">Copy something to get started</p>
        </div>
      </div>
    );
  }

  return (
    <div
      ref={containerRef}
      className="flex flex-1 items-stretch gap-3 overflow-x-auto px-4 py-3"
      style={{ scrollBehavior: "smooth" }}
    >
      {clips.map((clip, index) => (
        <Card
          key={clip.id}
          clip={clip}
          index={index}
          isSelected={index === selectedIndex}
          onSelect={() => onSelect(index)}
          onPaste={onPaste}
        />
      ))}
    </div>
  );
}
