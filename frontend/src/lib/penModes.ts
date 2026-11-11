// The Pen's drawing modes as the Pen bar offers them (`specs/0110-pen-modes-variable-stroke`
// criterion 1, UX notes "Pen bar"): one icon button each, the name in tooltip and accessible name.

/** A Pen mode code as the session reads and writes it. */
export type PenModeCode = "bezier" | "spiro" | "bspline" | "lines" | "freehand";

/** The modes in bar order; Bézier is the default. */
export const PEN_MODES: readonly { value: PenModeCode; label: string; tooltip: string }[] = [
  { value: "bezier", label: "Bézier", tooltip: "Bézier: click for corners, drag for curves" },
  { value: "spiro", label: "Spiro", tooltip: "Spiro: a smooth curve through every point" },
  { value: "bspline", label: "B-spline", tooltip: "B-spline: points pull the curve like magnets" },
  { value: "lines", label: "Lines", tooltip: "Lines: straight segments" },
  { value: "freehand", label: "Freehand", tooltip: "Freehand: drag to draw" },
];

/** Whether `mode` shows the Smoothing slider: Freehand alone. */
export function showsSmoothing(mode: string): boolean {
  return mode === "freehand";
}
