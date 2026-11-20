/**
 * The pure rules of the Adjustments group (`specs/0135-image-optimisation` `ux-notes.md` §3):
 * the Strength slider's keys and track, the three bands of the step list, the fold state of the
 * rows across an add, a move and a remove, and the texts the group says itself. The session holds
 * every other rule (ranges, the pipeline place, what a preset derives).
 */
import type { StepRow } from "./adjustmentsView.ts";

/** The slider's range: whole percent. */
export const STRENGTH_RANGE = { min: 0, max: 100, step: 1 } as const;

/** The refusal of a typed Strength out of range (`ux-notes.md` §3.3). */
export const STRENGTH_MESSAGE = "Between 0 and 100 %";

/** `value` clamped to 0 to 100 and rounded to a whole percent; a NaN gives 0. */
export function clampStrength(value: number): number {
  if (!Number.isFinite(value)) return STRENGTH_RANGE.min;
  return Math.min(STRENGTH_RANGE.max, Math.max(STRENGTH_RANGE.min, Math.round(value)));
}

/** The Strength after a key (criterion 26: arrows 1, Page Up and Down 10, Home 0, End 100), or
 * `null` for a key the slider does not take. */
export function strengthAfterKey(current: number, key: string): number | null {
  switch (key) {
    case "ArrowLeft":
    case "ArrowDown":
      return clampStrength(current - 1);
    case "ArrowRight":
    case "ArrowUp":
      return clampStrength(current + 1);
    case "PageDown":
      return clampStrength(current - 10);
    case "PageUp":
      return clampStrength(current + 10);
    case "Home":
      return STRENGTH_RANGE.min;
    case "End":
      return STRENGTH_RANGE.max;
    default:
      return null;
  }
}

/** The Strength at `x` px of a track `width` px wide. */
export function strengthAtPosition(x: number, width: number): number {
  if (!(width > 0) || Number.isNaN(x)) return STRENGTH_RANGE.min;
  return clampStrength((Math.min(1, Math.max(0, x / width))) * STRENGTH_RANGE.max);
}

/** Where `value` lies on a track `width` px wide, in px. */
export function strengthPosition(value: number, width: number): number {
  return (clampStrength(value) / STRENGTH_RANGE.max) * width;
}

/** A typed Strength: a whole number from 0 to 100 (a "%" and spaces are allowed), else `null`. */
export function parseStrength(text: string): number | null {
  const trimmed = text.trim().replace(/\s*%$/, "");
  if (!/^\d{1,3}$/.test(trimmed)) return null;
  const value = Number(trimmed);
  return value <= STRENGTH_RANGE.max ? value : null;
}

/** A step list band (`ux-notes.md` §3.7): A Crop, Resize, Upscale, AI denoise; B the tone and
 * detail steps; C Remove background. Slice 1 has band B only; an unknown row has none. */
export type Band = "B" | "unknown";

/** The band of a row. */
export function bandOf(row: Pick<StepRow, "kind">): Band {
  return row.kind === "unknown" ? "unknown" : "B";
}

/** Whether a hairline separates row `index` from the one before it: the bands differ. */
export function bandBreak(rows: readonly StepRow[], index: number): boolean {
  return index > 0 && bandOf(rows[index - 1]) !== bandOf(rows[index]);
}

/** The rows a drag from `from` may land between: the insertion slots (0 to n) inside the
 * source row's band, as an inclusive range of slots; an unknown row does not drag. */
export function dropSlots(rows: readonly StepRow[], from: number): { first: number; last: number } | null {
  const band = bandOf(rows[from]);
  if (band === "unknown") return null;
  let first = from;
  while (first > 0 && bandOf(rows[first - 1]) === band) first -= 1;
  let last = from;
  while (last < rows.length - 1 && bandOf(rows[last + 1]) === band) last += 1;
  return { first, last: last + 1 };
}

/** The place the row at `from` takes when it is dropped in `slot` (the gap before row `slot`):
 * its new index. */
export function landingIndex(from: number, slot: number): number {
  return slot > from ? slot - 1 : slot;
}

/** The unfolded rows (by index) after row `from` moved to `to`: the open state follows the row. */
export function openAfterMove(open: ReadonlySet<number>, from: number, to: number): Set<number> {
  const next = new Set<number>();
  for (const index of open) {
    if (index === from) next.add(to);
    else if (from < to && index > from && index <= to) next.add(index - 1);
    else if (from > to && index >= to && index < from) next.add(index + 1);
    else next.add(index);
  }
  return next;
}

/** The unfolded rows after row `at` was removed. */
export function openAfterRemove(open: ReadonlySet<number>, at: number): Set<number> {
  const next = new Set<number>();
  for (const index of open) {
    if (index < at) next.add(index);
    else if (index > at) next.add(index - 1);
  }
  return next;
}

/** The unfolded rows after a row was inserted at `at` (that row unfolded, its neighbours kept). */
export function openAfterInsert(open: ReadonlySet<number>, at: number): Set<number> {
  const next = new Set<number>([at]);
  for (const index of open) next.add(index >= at ? index + 1 : index);
  return next;
}

/** The index a new step took: the first place where the kinds of `after` differ from `before`
 * (the session inserts at the pipeline place), else the end; `null` when nothing was added. */
export function insertedAt(before: readonly string[], after: readonly string[]): number | null {
  if (after.length !== before.length + 1) return null;
  for (let i = 0; i < before.length; i += 1) {
    if (before[i] !== after[i]) return i;
  }
  return before.length;
}

/** The step kinds of a list of rows, for `insertedAt`. */
export function kindsOf(rows: readonly StepRow[]): string[] {
  return rows.map((row) => row.kind);
}

/** The shown text of a field value: the session's decimals, nothing else. */
export function fieldText(value: number, decimals: number): string {
  return value.toFixed(decimals);
}

/** The Up and Down step of a typed field text (Shift: ten steps), at the field's decimals. */
export function fieldStep(decimals: number): (text: string, key: string, shift: boolean) => string | null {
  const unit = 10 ** -decimals;
  return (text, key, shift) => {
    if (key !== "ArrowUp" && key !== "ArrowDown") return null;
    const value = Number(text.trim().replace(",", "."));
    if (text.trim() === "" || !Number.isFinite(value)) return null;
    const next = value + (key === "ArrowUp" ? 1 : -1) * (shift ? 10 : 1) * unit;
    return next.toFixed(decimals);
  };
}

/** The text of a typed field: a number with "." or ",", else `NaN`. */
export function typedField(text: string): number {
  const trimmed = text.trim().replace(",", ".");
  return trimmed === "" ? Number.NaN : Number(trimmed);
}
