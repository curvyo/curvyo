import { CircleHelp } from "lucide-react";

import type { OperationKind } from "@/lib/operationsView";

/** The dot ramp of Image (`ux-notes.md` 2.1): 2 x 2 squares on a 4-unit pitch, four columns
 * falling in density, 4, 3, 2 and 1 dots. */
const IMAGE_DOTS: readonly (readonly [number, number])[] = [
  [1, 1], [1, 5], [1, 9], [1, 13],
  [5, 1], [5, 5], [5, 13],
  [9, 1], [9, 9],
  [13, 5],
];

/**
 * The kind glyphs of the Operations tab (`specs/0101-laser-operations` `ux-notes.md` section 3):
 * 16 px, `currentColor`, 1.5 px stroke. The shape carries the kind; only Cut and Cut-out are red
 * (`--cut`), by the look of the canvas (customer 2026-11-10).
 */
export function OperationGlyph({ kind, size = 16 }: { kind: OperationKind; size?: number }) {
  const common = {
    width: size,
    height: size,
    viewBox: "0 0 16 16",
    fill: "none",
    stroke: "currentColor",
    strokeWidth: 1.5,
    "aria-hidden": true,
    focusable: false,
    className: "shrink-0",
  } as const;
  // The hatch of Engrave: three 45 degree lines inside the square (ux-notes section 3).
  const hatch = (
    <>
      <line x1="2" y1="6" x2="10" y2="14" strokeWidth={1} />
      <line x1="2" y1="2" x2="14" y2="14" strokeWidth={1} />
      <line x1="6" y1="2" x2="14" y2="10" strokeWidth={1} />
    </>
  );
  switch (kind) {
    case "cut":
      return (
        <svg {...common} style={{ color: "var(--cut)" }}>
          <rect x="2.25" y="2.25" width="11.5" height="11.5" />
        </svg>
      );
    case "score":
      return (
        <svg {...common}>
          <line x1="2" y1="8" x2="14" y2="8" strokeDasharray="0.01 3" strokeLinecap="round" />
        </svg>
      );
    case "engrave":
      return <svg {...common}>{hatch}</svg>;
    case "engrave_outline":
      return (
        <svg {...common}>
          {hatch}
          <rect x="0.75" y="0.75" width="14.5" height="14.5" />
        </svg>
      );
    case "cut_out":
      return (
        <svg {...common} style={{ color: "var(--cut)" }}>
          <rect x="2.25" y="2.25" width="11.5" height="11.5" strokeDasharray="4 2" />
        </svg>
      );
    case "image":
      return (
        <svg {...common} fill="currentColor" stroke="none">
          {IMAGE_DOTS.map(([x, y]) => (
            <rect key={`${x}-${y}`} x={x} y={y} width="2" height="2" />
          ))}
        </svg>
      );
    case "unknown":
      return <CircleHelp size={size} strokeWidth={1.5} absoluteStrokeWidth aria-hidden className="shrink-0" />;
  }
}

/**
 * The Test grid glyph of the Add test grid button (`specs/0028` `ux-notes.md` 2): nine outlined
 * squares, 3 x 3, ink, 16 px. Not the fixture Grid `#` of `0102`.
 */
export function TestGridGlyph({ size = 16 }: { size?: number }) {
  const squares = [0, 1, 2].flatMap((row) =>
    [0, 1, 2].map((column) => (
      <rect key={`${row}${column}`} x={0.75 + column * 5.5} y={0.75 + row * 5.5} width="3.5" height="3.5" />
    )),
  );
  return (
    <svg width={size} height={size} viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth={1.5} aria-hidden focusable={false} className="shrink-0">
      {squares}
    </svg>
  );
}

/**
 * The Tabs glyph (`specs/0139` `ux-notes.md` 4.1): a horizontal cut line in `--cut` with a 4 px gap
 * and an ink bar across the gap, the canvas tab look in small. `pressed` turns the red part to the
 * toggle's pressed foreground.
 */
export function TabsGlyph({ size = 16, pressed = false }: { size?: number; pressed?: boolean }) {
  return (
    <svg width={size} height={size} viewBox="0 0 16 16" fill="none" strokeWidth={1.5} aria-hidden focusable={false} className="shrink-0">
      <g stroke={pressed ? "currentColor" : "var(--cut)"}>
        <line x1="2" y1="8" x2="6" y2="8" />
        <line x1="10" y1="8" x2="14" y2="8" />
      </g>
      <line x1="8" y1="5" x2="8" y2="11" stroke="currentColor" />
    </svg>
  );
}

/**
 * The lead glyphs of the Path options group (`specs/0139` `ux-notes.md` 4.1): an edge at y 6 from
 * x 2 to 14; Line adds a stub from (5, 13) up to the edge, Arc a quarter circle from (2, 13) to
 * (6, 6) meeting the edge tangentially. `mirrored` puts the lead at the right (lead-out).
 */
export function LeadGlyph({ kind, mirrored = false, size = 16 }: { kind: "none" | "line" | "arc"; mirrored?: boolean; size?: number }) {
  return (
    <svg width={size} height={size} viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth={1.5} aria-hidden focusable={false} className="shrink-0">
      <g transform={mirrored ? "matrix(-1 0 0 1 16 0)" : undefined}>
        <line x1="2" y1="6" x2="14" y2="6" />
        {kind === "line" ? <line x1="5" y1="13" x2="5" y2="6" /> : null}
        {kind === "arc" ? <path d="M2 13 A4 7 0 0 1 6 6" /> : null}
      </g>
    </svg>
  );
}
