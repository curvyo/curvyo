// The glyphs of the Pattern block (`specs/0071-pattern-fill` UX notes §2, §4): the 16 px Pattern
// paint item and the ten 24 px generator tiles, static line drawings in `currentColor`.

import type { ReactNode } from "react";

const TILE = {
  width: 24,
  height: 24,
  viewBox: "0 0 24 24",
  fill: "none",
  stroke: "currentColor",
  strokeWidth: 1.5,
  strokeLinecap: "round",
  strokeLinejoin: "round",
  "aria-hidden": true,
} as const;

/** "Pattern fill": a square with three diagonal lines inside. */
export function PatternPaintIcon() {
  return (
    <svg width={16} height={16} viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth={1.5} aria-hidden>
      <rect x="2.5" y="2.5" width="11" height="11" />
      <path d="M2.5 8.5 8.5 2.5M2.5 13.5 13.5 2.5M8 13.5 13.5 8" />
    </svg>
  );
}

const GLYPHS: Record<string, ReactNode> = {
  hatch: <path d="M3 9 9 3M3 15 15 3M3 21 21 3M9 21 21 9M15 21 21 15" />,
  crosshatch: <path d="M3 9 9 3M3 15 15 3M3 21 21 3M9 21 21 9M15 21 21 15M3 15 9 21M3 9 15 21M3 3 21 21M9 3 21 15M15 3 21 9" />,
  grid: <path d="M8 3v18M12 3v18M16 3v18M3 8h18M3 12h18M3 16h18" />,
  dots: (
    <g fill="currentColor" stroke="none">
      {[
        [5, 6], [12, 6], [19, 6], [8.5, 12], [15.5, 12], [5, 18], [12, 18], [19, 18],
      ].map(([x, y]) => (
        <circle key={`${x}-${y}`} cx={x} cy={y} r="1.6" />
      ))}
    </g>
  ),
  honeycomb: (
    <path d="M7 4.5 10 6.25v3.5L7 11.5 4 9.75v-3.5ZM14 4.5l3 1.75v3.5l-3 1.75-3-1.75v-3.5ZM10.5 12.5l3 1.75v3.5l-3 1.75-3-1.75v-3.5Z" />
  ),
  brick_wall: <path d="M3 4h18v16H3ZM3 9.33h18M3 14.67h18M9 4v5.33M15 4v5.33M6 9.33v5.34M12 9.33v5.34M18 9.33v5.34M9 14.67V20M15 14.67V20" />,
  leaf_forest: (
    <path d="M4 10c1-4 5-5 6-5-1 1-1 5-6 5ZM12 20c-1-4 2-7 3-7 0 1 1 5-3 7ZM14 9c3-3 6-2 7-2-1 1-3 4-7 2Z" />
  ),
  sea_waves: (
    <path d="M2 12a5 5 0 0 1 10 0M5 12a2 2 0 0 1 4 0M12 12a5 5 0 0 1 10 0M15 12a2 2 0 0 1 4 0M7 20a5 5 0 0 1 10 0M10 20a2 2 0 0 1 4 0" />
  ),
  waves: <path d="M3 6c3-3 6 3 9 0s6 3 9 0M3 12c3-3 6 3 9 0s6 3 9 0M3 18c3-3 6 3 9 0s6 3 9 0" />,
  interpolation_waves: <path d="M3 5c6-2 12-2 18 0M3 10c6 0 12-1 18 1M3 15c6 2 12 0 18 2M3 20c6 4 12 0 18 2" />,
};

/** The tile glyph of the generator `id`. */
export function GeneratorGlyph({ id }: { id: string }) {
  return <svg {...TILE}>{GLYPHS[id]}</svg>;
}
