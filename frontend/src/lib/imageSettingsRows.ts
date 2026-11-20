/**
 * The field rules of the Image settings group (`specs/0123-raster-engraving` `ux-notes.md` 2.2,
 * 2.4, 8): widths, Up and Down steps and names. Pure functions over the decoded group
 * (`operationSettingsView.ts`); the session holds the ranges and answers with the chip text.
 */
import type { ImageFieldName } from "./operationSettingsView.ts";

/** Look of one field: its width in px (P34), the Up and Down step and the Shift step. */
export interface ImageFieldLook {
  width: number;
  step: number;
  shiftStep: number;
  decimals: number;
  /** The accessible name. */
  name: string;
}

export const IMAGE_FIELD_LOOK: Record<ImageFieldName, ImageFieldLook> = {
  threshold: { width: 124, step: 1, shiftStep: 10, decimals: 0, name: "Threshold, percent" },
  lowest_power: { width: 132, step: 1, shiftStep: 10, decimals: 0, name: "Lowest power, percent" },
  brightness: { width: 120, step: 1, shiftStep: 10, decimals: 0, name: "Brightness, percent" },
  contrast: { width: 108, step: 1, shiftStep: 10, decimals: 0, name: "Contrast, percent" },
  gamma: { width: 100, step: 0.01, shiftStep: 0.1, decimals: 2, name: "Gamma" },
  // The screens' fields (`specs/0135` criteria 42, 43; UX notes §10): 136 px holds "Line pitch"
  // with a two-decimal value and "mm"; with Angle 104 the row stays inside the 256 px column.
  screen_size: { width: 136, step: 0.05, shiftStep: 0.5, decimals: 2, name: "Cell size or line pitch, millimetres" },
  screen_angle: { width: 104, step: 1, shiftStep: 15, decimals: 0, name: "Screen angle, degrees" },
};

/** A number as an Image field shows it: at most two decimals, no trailing zeros. */
export function imageFieldText(value: number): string {
  return String(Math.round(value * 100) / 100);
}

/** The Up and Down step on typed text (Shift: the big step); `null` for another key or no number. */
export function imageFieldStep(name: ImageFieldName): (text: string, key: string, shift: boolean) => string | null {
  const look = IMAGE_FIELD_LOOK[name];
  return (text, key, shift) => {
    if (key !== "ArrowUp" && key !== "ArrowDown") return null;
    const value = Number(text.trim().replace(",", "."));
    if (text.trim() === "" || !Number.isFinite(value)) return null;
    const next = value + (key === "ArrowUp" ? 1 : -1) * (shift ? look.shiftStep : look.step);
    return imageFieldText(Number(next.toFixed(look.decimals)));
  };
}
