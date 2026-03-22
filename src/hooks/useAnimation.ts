import { useMemo } from "react";

/**
 * Returns animation duration/transition configs scaled by the animation speed multiplier.
 * When speed is 0, all durations are 0 (instant).
 * Respects prefers-reduced-motion by setting speed to 0.
 */
export function useAnimation(speed: number = 1.0) {
  const effectiveSpeed = useMemo(() => {
    // Respect prefers-reduced-motion
    if (typeof window !== "undefined" && window.matchMedia("(prefers-reduced-motion: reduce)").matches) {
      return 0;
    }
    return Math.max(0, speed);
  }, [speed]);

  const isEnabled = effectiveSpeed > 0;

  return useMemo(
    () => ({
      isEnabled,
      speed: effectiveSpeed,
      // Scaled duration helper
      duration: (base: number) => (isEnabled ? base * effectiveSpeed : 0),
      // Spring transition for card selection
      spring: isEnabled
        ? { type: "spring" as const, stiffness: 400, damping: 25, mass: 0.8 }
        : { duration: 0 },
      // Ease-out for show
      easeOut: (base: number) =>
        isEnabled
          ? { duration: base * effectiveSpeed, ease: [0.0, 0.0, 0.2, 1] as [number, number, number, number] }
          : { duration: 0 },
      // Ease-in for hide
      easeIn: (base: number) =>
        isEnabled
          ? { duration: base * effectiveSpeed, ease: [0.4, 0.0, 1, 1] as [number, number, number, number] }
          : { duration: 0 },
      // Stagger delay per child
      staggerDelay: isEnabled ? 0.03 * effectiveSpeed : 0,
    }),
    [isEnabled, effectiveSpeed],
  );
}
