/**
 * The Crop bar's view and its frame's arithmetic (`specs/0135-image-optimisation` criterion 5,
 * `ux-notes.md` §3.9): the decoder of `crop_bar_json` (`curvyo-editor-wasm/src/session/image_adjust/crop_bar.rs`)
 * and the pure geometry of a drag of the frame or a handle in the picture's unit square. The
 * session decides what is written; a drag only proposes whole percents.
 */
import { decodeStepFields, type StepField } from "./adjustmentsView.ts";

/** Left, top, width, height in whole % of the whole picture. */
export type CropPct = [number, number, number, number];

/** The map of the unit square to canvas px: `x = a·u + c·v + e`, `y = b·u + d·v + f`. */
export type Quad = [number, number, number, number, number, number];

export interface CropBarView {
  row: number;
  pct: CropPct;
  quad: Quad;
  /** The working image's size, the bytes of `before_after_pixels("before")`. */
  width: number;
  height: number;
  fields: StepField[];
}

/** The eight handles and the frame's inside. */
export type CropGrip = "tl" | "t" | "tr" | "r" | "br" | "b" | "bl" | "l" | "inside";

export const CROP_GRIPS: readonly Exclude<CropGrip, "inside">[] = ["tl", "t", "tr", "r", "br", "b", "bl", "l"];

function whole(value: unknown, max: number): value is number {
  return Number.isSafeInteger(value) && (value as number) >= 0 && (value as number) <= max;
}

/** The bar from its JSON text; `null` while closed or of the wrong shape. */
export function decodeCropBarText(text: string): CropBarView | null {
  let value: unknown;
  try {
    value = JSON.parse(text);
  } catch {
    return null;
  }
  if (value === null || typeof value !== "object") return null;
  const v = value as Record<string, unknown>;
  const pct = v.pct;
  const quad = v.quad;
  if (
    !whole(v.row, 1_000) ||
    !Array.isArray(pct) ||
    pct.length !== 4 ||
    !pct.every((p) => whole(p, 100)) ||
    !Array.isArray(quad) ||
    quad.length !== 6 ||
    !quad.every((q) => typeof q === "number" && Number.isFinite(q)) ||
    !whole(v.width, 1 << 20) ||
    !whole(v.height, 1 << 20)
  ) {
    return null;
  }
  const fields = decodeStepFields(v.fields);
  if (!fields) return null;
  return {
    row: v.row,
    pct: pct as CropPct,
    quad: quad as Quad,
    width: v.width,
    height: v.height,
    fields,
  };
}

/** A unit-square point on the canvas. */
export function toScreen(q: Quad, u: number, v: number): [number, number] {
  return [q[0] * u + q[2] * v + q[4], q[1] * u + q[3] * v + q[5]];
}

/** A canvas point in the unit square (`null` for a degenerate map). */
export function toUnit(q: Quad, x: number, y: number): [number, number] | null {
  const det = q[0] * q[3] - q[1] * q[2];
  if (Math.abs(det) < 1e-12) return null;
  const dx = x - q[4];
  const dy = y - q[5];
  return [(q[3] * dx - q[2] * dy) / det, (-q[1] * dx + q[0] * dy) / det];
}

/** The unit-square point of a grip of `pct`. */
export function gripPoint(pct: CropPct, grip: Exclude<CropGrip, "inside">): [number, number] {
  const [l, t, w, h] = pct.map((p) => p / 100);
  const x = grip.includes("l") ? l : grip.includes("r") ? l + w : l + w / 2;
  const y = grip.startsWith("t") ? t : grip.startsWith("b") ? t + h : t + h / 2;
  return [x, y];
}

const clamp = (v: number, lo: number, hi: number) => Math.min(hi, Math.max(lo, v));

/**
 * The crop after dragging `grip` of `start` by (`du`, `dv`) in the unit square: a corner moves
 * two sides (Shift keeps the ratio), a side one, the inside moves the frame; whole %, at least
 * 1 %, inside the picture.
 */
export function dragCrop(start: CropPct, grip: CropGrip, du: number, dv: number, keepRatio = false): CropPct {
  const [l, t, w, h] = start;
  const dx = Math.round(du * 100);
  const dy = Math.round(dv * 100);
  if (grip === "inside") {
    return [clamp(l + dx, 0, 100 - w), clamp(t + dy, 0, 100 - h), w, h];
  }
  let left = l;
  let top = t;
  let right = l + w;
  let bottom = t + h;
  if (grip.includes("l")) left = clamp(l + dx, 0, right - 1);
  if (grip.includes("r")) right = clamp(right + dx, left + 1, 100);
  if (grip.startsWith("t")) top = clamp(t + dy, 0, bottom - 1);
  if (grip.startsWith("b")) bottom = clamp(bottom + dy, top + 1, 100);
  if (keepRatio && grip.length === 2 && w > 0 && h > 0) {
    // The width leads; the height follows the start's ratio, inside the picture.
    const ratio = h / w;
    const width = right - left;
    const height = clamp(Math.round(width * ratio), 1, grip.startsWith("t") ? bottom : 100 - top);
    if (grip.startsWith("t")) top = bottom - height;
    else bottom = top + height;
  }
  return [left, top, right - left, bottom - top];
}
