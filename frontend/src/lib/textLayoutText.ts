// The words and keys of the text layout controls (`specs/0079-text-layout-controls` criteria 1,
// 7, 10, 13; UX notes §1 and §6): the Alignment group and the four fields of the text layout
// block. No React, so node tests can check them; every value comes from the session.

import { TEXT_LAYOUT_FIELD_NAMES } from "./sliderNames.ts";

/** The four alignments, in the group's order. */
export const ALIGN_CODES = ["left", "centre", "right", "justify"] as const;
export type AlignCode = (typeof ALIGN_CODES)[number];

/** Each button's accessible name and tooltip (criterion 1). */
export const ALIGN_NAMES: Record<AlignCode, string> = {
  left: "Align left",
  centre: "Align centre",
  right: "Align right",
  justify: "Justify",
};

/** The session's alignment code: one of the four, or "" when the subject texts differ. */
export function parseAlign(code: string): AlignCode | null {
  return (ALIGN_CODES as readonly string[]).includes(code) ? (code as AlignCode) : null;
}

/** The button holding the Tab stop: the pressed one, else Align left (UX notes §1). */
export function alignTabStop(pressed: AlignCode | null): number {
  return pressed ? ALIGN_CODES.indexOf(pressed) : 0;
}

/** Where an arrow, Home or End moves focus in the group: no wrap, focus only (UX notes §1). */
export function alignFocusMove(key: string, index: number): number | null {
  const last = ALIGN_CODES.length - 1;
  switch (key) {
    case "ArrowLeft":
    case "ArrowUp":
      return Math.max(0, index - 1);
    case "ArrowRight":
    case "ArrowDown":
      return Math.min(last, index + 1);
    case "Home":
      return 0;
    case "End":
      return last;
    default:
      return null;
  }
}

/** The tooltip's second line: "Mixed" while the subject texts differ and none is pressed. */
export function alignTooltipLine(pressed: AlignCode | null): string | null {
  return pressed ? null : "Mixed";
}

/** The fields of the block, in order, with the session's names. */
export const LAYOUT_FIELDS = [
  "letter-spacing",
  "word-spacing",
  "line-spacing",
  "character-rotation",
] as const;
export type LayoutFieldCode = (typeof LAYOUT_FIELDS)[number];

/** Each field's label, which is also its accessible name (criterion 7). */
export const LAYOUT_FIELD_LABELS: Record<LayoutFieldCode, string> = {
  "letter-spacing": TEXT_LAYOUT_FIELD_NAMES.letterSpacing,
  "word-spacing": TEXT_LAYOUT_FIELD_NAMES.wordSpacing,
  "line-spacing": TEXT_LAYOUT_FIELD_NAMES.lineSpacing,
  "character-rotation": TEXT_LAYOUT_FIELD_NAMES.characterRotation,
};

/** What a field shows, from the session's block JSON. */
export interface LayoutFieldState {
  text: string;
  value: number;
  mixed: boolean;
  auto: boolean;
  barFrom: number;
  barTo: number;
  zero: number | null;
  dragBase: number;
  dragMin: number;
  dragLimit: number;
  trackMin: number;
  trackMax: number;
  resettable: boolean;
}

/** The block's four fields, or `null` when it is not shown (no text selected or edited). */
export function parseLayoutBlock(json: string): LayoutFieldState[] | null {
  try {
    const parsed = JSON.parse(json) as LayoutFieldState[] | null;
    return Array.isArray(parsed) && parsed.length === LAYOUT_FIELDS.length ? parsed : null;
  } catch {
    return null;
  }
}

/** The field's unit as drawn: mm, or the degree sign. */
export function layoutUnit(field: LayoutFieldCode): string {
  return field === "character-rotation" ? "°" : "mm";
}

/** The unit spelled out for `aria-valuetext`. */
export function layoutUnitWords(field: LayoutFieldCode): string {
  return field === "character-rotation" ? "degrees" : "millimetres";
}

/** The reset target as text ("0 mm", "Auto", "0°"). */
export function layoutDefaultText(field: LayoutFieldCode): string {
  if (field === "line-spacing") {
    return "Auto";
  }
  return field === "character-rotation" ? "0°" : "0 mm";
}

/** The value as the tooltip and `aria-valuetext` read it: "1.5 millimetres", "−12 degrees",
 * "12.4 millimetres, automatic", "Mixed" (UX notes §0.3, §6). */
export function layoutValueText(field: LayoutFieldCode, state: LayoutFieldState): string {
  if (state.mixed) {
    return "Mixed";
  }
  const words = `${state.text} ${layoutUnitWords(field)}`;
  return state.auto ? `${words}, automatic` : words;
}

/** The tooltip's value: "1.5 mm", "12.4 mm (Auto)", "−12°", "Mixed". */
export function layoutTipValue(field: LayoutFieldCode, state: LayoutFieldState): string {
  if (state.mixed) {
    return "Mixed";
  }
  if (field === "character-rotation") {
    return `${state.text}°`;
  }
  return state.auto ? `${state.text} mm (Auto)` : `${state.text} mm`;
}

/** What the session answered to a typed value, a reset or a commit. */
export interface LayoutOutcome {
  kind: "set" | "unchanged" | "invalid" | "max" | "min" | "limited";
  mm?: number;
  limited?: number;
  of?: number;
}

function mmText(mm: number): string {
  return `${Math.round(mm * 100) / 100 + 0}`.replace("-", "−");
}

/** The notice under the field, or `null` (criterion 7, UX notes §0.4); at most 70 characters. */
export function layoutNotice(field: LayoutFieldCode, outcome: LayoutOutcome): string | null {
  const name = LAYOUT_FIELD_LABELS[field];
  switch (outcome.kind) {
    case "max":
      return `${name} set to ${mmText(outcome.mm ?? 0)} mm, the largest for this size.`;
    case "min":
      return `${name} set to ${mmText(outcome.mm ?? 0)} mm, the smallest for this size.`;
    case "limited":
      return `${name} limited on ${outcome.limited ?? 0} of ${outcome.of ?? 0} texts, to fit its size.`;
    default:
      return null;
  }
}

/** The answer a typed value gives the shared entry field: `"committed"`, `"unchanged"` or
 * `"invalid:number"`. */
export function entryAnswer(outcome: LayoutOutcome): string {
  switch (outcome.kind) {
    case "invalid":
      return "invalid:number";
    case "unchanged":
      return "unchanged";
    default:
      return "committed";
  }
}

/** The parsed outcome JSON; an unreadable answer is "unchanged". */
export function parseLayoutOutcome(json: string): LayoutOutcome {
  try {
    return JSON.parse(json) as LayoutOutcome;
  } catch {
    return { kind: "unchanged" };
  }
}
