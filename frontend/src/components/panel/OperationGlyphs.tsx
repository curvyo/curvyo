import { CircleHelp } from "lucide-react";

import type { OperationKind } from "@/lib/operationsView";

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
    case "unknown":
      return <CircleHelp size={size} strokeWidth={1.5} absoluteStrokeWidth aria-hidden className="shrink-0" />;
  }
}
