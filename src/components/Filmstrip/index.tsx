import { RefObject } from "react";
import { motion, AnimatePresence } from "framer-motion";
import { Card } from "../Card";
import type { ClipData } from "../../App";
import { useAnimation } from "../../hooks/useAnimation";

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
  const anim = useAnimation();

  if (loading) {
    return (
      <div className="flex flex-1 items-center justify-center text-text-muted">
        Loading...
      </div>
    );
  }

  if (clips.length === 0) {
    return (
      <div className="flex flex-1 items-center justify-center text-text-muted">
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
    >
      <AnimatePresence mode="popLayout">
        {clips.map((clip, index) => (
          <motion.div
            key={clip.id}
            layout={anim.isEnabled}
            initial={anim.isEnabled ? { opacity: 0, scale: 0.9, x: -20 } : false}
            animate={{ opacity: 1, scale: 1, x: 0 }}
            exit={anim.isEnabled ? { opacity: 0, scale: 0.9 } : undefined}
            transition={anim.isEnabled ? {
              delay: index * anim.staggerDelay,
              duration: anim.duration(0.2),
              ease: "easeOut",
            } : { duration: 0 }}
            className="shrink-0"
          >
            <Card
              clip={clip}
              index={index}
              isSelected={index === selectedIndex}
              onSelect={() => onSelect(index)}
              onPaste={onPaste}
            />
          </motion.div>
        ))}
      </AnimatePresence>
    </div>
  );
}
