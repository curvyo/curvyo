/**
 * The numbers of the ratio sliders (`specs/0047-transform-polish/` Part B, S1 and S2):
 * the range, the rounding of a position to a value and the rule of the typed entry. The
 * colour sliders do not use this: their values come from Rust (`rgbOf`).
 */

/** The inner ratio of a star: 0.01 to 0.99 in steps of 0.01. */
export const RATIO_RANGE = { min: 0.01, max: 0.99, step: 0.01 } as const;

export interface SliderRange {
  min: number;
  max: number;
  step: number;
}

/** The message of a refused typed ratio (criterion 22). */
export const RATIO_MESSAGE = "Enter a number from 0.01 to 0.99";

/** `value` clamped to the range and rounded to the nearest step. */
export function snapToStep(value: number, range: SliderRange): number {
  const clamped = Math.min(range.max, Math.max(range.min, value));
  const steps = Math.round((clamped - range.min) / range.step);
  // Six decimals shed the float noise of `0.01 * 3`.
  return Math.min(
    range.max,
    Math.max(range.min, Number((range.min + steps * range.step).toFixed(6))),
  );
}

/** The value at `x` px of a track `width` px wide (the whole width is the whole range). */
export function valueAtPosition(
  x: number,
  width: number,
  range: SliderRange,
): number {
  const fraction = width > 0 ? Math.min(1, Math.max(0, x / width)) : 0;
  return snapToStep(range.min + fraction * (range.max - range.min), range);
}

/** Where `value` lies on a track `width` px wide, in px. */
export function positionOfValue(
  value: number,
  width: number,
  range: SliderRange,
): number {
  return ((value - range.min) / (range.max - range.min)) * width;
}

/** A typed ratio: `{ ok: true, value }` rounded to two decimals, or `{ ok: false }`. The text
 * must be a finite number from 0.01 to 0.99 as typed, with "." or "," as the separator
 * ("0.456" gives 0.46, "0.995" is refused, criterion 22). */
export function parseRatioEntry(
  text: string,
): { ok: true; value: number } | { ok: false } {
  const trimmed = text.trim();
  if (!/^(\d+([.,]\d*)?|[.,]\d+)$/.test(trimmed)) {
    return { ok: false };
  }
  const typed = Number(trimmed.replace(",", "."));
  if (
    !Number.isFinite(typed) ||
    typed < RATIO_RANGE.min ||
    typed > RATIO_RANGE.max
  ) {
    return { ok: false };
  }
  return { ok: true, value: snapToStep(typed, RATIO_RANGE) };
}
