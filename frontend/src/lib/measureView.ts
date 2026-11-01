/** The Measure tool's overlay as Rust lays it out (`specs/0070-measure-and-dimension`): the
 * dimension texts at their screen positions, the mode label and the readout chip. */
export interface MeasureLabel {
  id: string;
  text: string;
  name: string;
  x: number;
  y: number;
  linked: boolean;
  selected: boolean;
  preview: boolean;
}

export interface MeasureChip {
  lines: string[];
  x: number;
  y: number;
}

export interface MeasureView {
  dimensions: MeasureLabel[];
  mode: MeasureChip | null;
  readout: MeasureChip | null;
  /** The value of the last pin or placement, for the polite live region (UX notes §3). */
  speech: string;
}

export const EMPTY_MEASURE_VIEW: MeasureView = { dimensions: [], mode: null, readout: null, speech: "" };

/** The view of `json`, or the empty view for text that is not one. */
export function readMeasureView(json: string): MeasureView {
  try {
    const raw = JSON.parse(json) as Partial<MeasureView> | null;
    if (!raw || typeof raw !== "object") return EMPTY_MEASURE_VIEW;
    return {
      dimensions: Array.isArray(raw.dimensions) ? raw.dimensions : [],
      mode: raw.mode ?? null,
      readout: raw.readout ?? null,
      speech: typeof raw.speech === "string" ? raw.speech : "",
    };
  } catch {
    return EMPTY_MEASURE_VIEW;
  }
}

/** Where a chip goes: 12px up and right of its point, flipped below and left near the edges
 * (UX notes §3), as CSS left and top of its box of `size`. */
export function chipPlace(
  at: { x: number; y: number },
  size: { width: number; height: number },
  canvas: { width: number; height: number },
  offset = 12,
): { left: number; top: number } {
  let left = at.x + offset;
  let top = at.y - offset - size.height;
  if (left + size.width > canvas.width) left = at.x - offset - size.width;
  if (top < 0) top = at.y + offset;
  return { left: Math.max(0, left), top: Math.max(0, Math.min(top, canvas.height - size.height)) };
}
