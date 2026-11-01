/**
 * Constraint glyphs of the Sketch workbench (`specs/0066-constraint-sketcher` AC24, AC32): one
 * set used in the rail (20px), and in the canvas symbol badges (12px). Single-colour line
 * redrawings in a 16px grid of the FreeCAD Sketcher constraint icons. This file as a whole is
 * licensed under LGPL-2.1-or-later (not AGPL-3.0-or-later), text in `docs/licenses/LGPL-2.1.txt`;
 * source, files and licence are recorded in `docs/third-party-assets.md`. Midpoint, On axis, the
 * aligned Dimension ruler and the Horizontal and Vertical distance rulers follow FreeCAD's
 * `Constraint_PointOnMidPoint`, `Constraint_PointOnObject`, `Constraint_Length`,
 * `Constraint_HorizontalDistance` and `Constraint_VerticalDistance`.
 */
import type { ReactNode } from "react";

const GLYPHS: Record<string, ReactNode> = {
  coincident: <>
    <circle cx="8" cy="8" r="2" fill="currentColor" stroke="none" />
    <path d="M2.5 2.5 5 5M13.5 2.5 11 5M2.5 13.5 5 11M13.5 13.5 11 11" />
  </>,
  horizontal: <path d="M2 8h12" />,
  vertical: <path d="M8 2v12" />,
  parallel: <path d="M2.5 13 8 3M8 13l5.5-10" />,
  perpendicular: <path d="M2 13.5h12M8 13.5V2.5" />,
  tangent: <path d="M4 13a9 9 0 0 1 9-9M3.5 9.8 9.8 3.5" />,
  equal: <path d="M3 6h10M3 10h10" />,
  fixed: <>
    <rect x="3.5" y="7" width="9" height="7" rx="1" />
    <path d="M5.5 7V5a2.5 2.5 0 0 1 5 0v2" />
  </>,
  symmetric: <path d="M2.5 4 6.5 8l-4 4M13.5 4l-4 4 4 4" />,
  midpoint: <>
    <path d="M2.5 13.5 13.5 2.5M2.5 10v3.5H6M13.5 6V2.5H10" />
    <circle cx="8" cy="8" r="2" fill="currentColor" stroke="none" />
  </>,
  "on-axis": <>
    <path d="M3 14A11 11 0 0 1 14 3" />
    <circle cx="6.2" cy="6.2" r="2" fill="currentColor" stroke="none" />
  </>,
  // FreeCAD's Constraint_Length is the aligned (diagonal) distance; Constraint_HorizontalDistance
  // and Constraint_VerticalDistance measure along one axis (AC44).
  dimension: <path d="M1.5 10.5l4 4M10.5 1.5l4 4M4.5 11.5l7-7M7 11.5H4.5V9M9 4.5h2.5V7" />,
  "horizontal-distance": <path d="M2 4v8M14 4v8M3 8h10M5 6 3 8l2 2M11 6l2 2-2 2" />,
  "vertical-distance": <path d="M4 2h8M4 14h8M8 3v10M6 5l2-2 2 2M6 11l2 2 2-2" />,
};
// A point on a line, circle, arc or ellipse is FreeCAD's PointOnObject, as on axis (AC37).
GLYPHS["point-on"] = GLYPHS["on-axis"];

/** The glyph named `name` at `size` pixels, drawn in `currentColor` with a 1.5px stroke. */
export function ConstraintIcon({ name, size }: { name: string; size: number }) {
  return <svg width={size} height={size} viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth={1.5}
    strokeLinecap="round" strokeLinejoin="round" aria-hidden focusable="false">{GLYPHS[name] ?? GLYPHS.dimension}</svg>;
}
