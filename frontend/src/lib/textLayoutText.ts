// The words and keys of the text layout controls (`specs/0079-text-layout-controls` criteria 1,
// 6, 7, 10, 13, 16, 26, 27b, 27c, 29; UX notes §1, §6, §10.1 and §10.3): the Alignment group, the
// eight fields of the text layout block, its fold, the "All characters" buttons and the kerning
// keys. No React, so node tests can check them; every value comes from the session.

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
  "taper",
  "horizontal-kerning",
  "vertical-kerning",
  "char-rotation",
] as const;
export type LayoutFieldCode = (typeof LAYOUT_FIELDS)[number];

/** Each field's label, which is also its accessible name (criterion 7). */
export const LAYOUT_FIELD_LABELS: Record<LayoutFieldCode, string> = {
  "letter-spacing": TEXT_LAYOUT_FIELD_NAMES.letterSpacing,
  "word-spacing": TEXT_LAYOUT_FIELD_NAMES.wordSpacing,
  "line-spacing": TEXT_LAYOUT_FIELD_NAMES.lineSpacing,
  "character-rotation": TEXT_LAYOUT_FIELD_NAMES.characterRotation,
  taper: TEXT_LAYOUT_FIELD_NAMES.taper,
  "horizontal-kerning": TEXT_LAYOUT_FIELD_NAMES.horizontalKerning,
  "vertical-kerning": TEXT_LAYOUT_FIELD_NAMES.verticalKerning,
  "char-rotation": TEXT_LAYOUT_FIELD_NAMES.charRotation,
};

/** The fields that act on single characters (criteria 26, 27b). */
function perCharacter(code: LayoutFieldCode): boolean {
  return code === "horizontal-kerning" || code === "vertical-kerning" || code === "char-rotation";
}

/** The fields in degrees. */
function inDegrees(code: LayoutFieldCode): boolean {
  return code === "character-rotation" || code === "char-rotation";
}

/** The field in percent (criterion 16). */
function inPercent(code: LayoutFieldCode): boolean {
  return code === "taper";
}

/** The fields with an "All characters" button beside them (criterion 27c). */
export function appliesToAll(code: LayoutFieldCode): boolean {
  return perCharacter(code);
}

/** The "All characters" button's accessible name and tooltip (criterion 27c). */
export const APPLY_TO_ALL_NAME = "Apply to all characters";

/**
 * The reset slot's own name, when it is not "Reset <field> to <default>": outside edit mode a
 * kerning field's reset clears both kerns of the texts, the per-character rotation's every
 * character's rotation (criteria 28, 27b).
 */
export function layoutResetLabel(code: LayoutFieldCode, editing: boolean): string | undefined {
  if (editing || !perCharacter(code)) {
    return undefined;
  }
  return code === "char-rotation" ? "Clear per-character rotation" : "Clear kerning";
}

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

/** The block's eight fields, or `null` when it is not shown (no text selected or edited). */
export function parseLayoutBlock(json: string): LayoutFieldState[] | null {
  try {
    const parsed = JSON.parse(json) as LayoutFieldState[] | null;
    return Array.isArray(parsed) && parsed.length === LAYOUT_FIELDS.length ? parsed : null;
  } catch {
    return null;
  }
}

/** The field's unit as drawn: mm, the degree sign, or "%". */
export function layoutUnit(field: LayoutFieldCode): string {
  if (inPercent(field)) {
    return "%";
  }
  return inDegrees(field) ? "°" : "mm";
}

/** The unit spelled out for `aria-valuetext`. */
export function layoutUnitWords(field: LayoutFieldCode): string {
  if (inPercent(field)) {
    return "percent";
  }
  return inDegrees(field) ? "degrees" : "millimetres";
}

/** The reset target as text ("0 mm", "Auto", "0°", "0 %"). */
export function layoutDefaultText(field: LayoutFieldCode): string {
  if (field === "line-spacing") {
    return "Auto";
  }
  if (inPercent(field)) {
    return "0 %";
  }
  return inDegrees(field) ? "0°" : "0 mm";
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

/** The tooltip's value: "1.5 mm", "12.4 mm (Auto)", "−12°", "20 %", "Mixed". */
export function layoutTipValue(field: LayoutFieldCode, state: LayoutFieldState): string {
  if (state.mixed) {
    return "Mixed";
  }
  if (inPercent(field)) {
    return `${state.text} %`;
  }
  if (inDegrees(field)) {
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
  if (inPercent(field)) {
    // One range for every text (criterion 16): "Taper set to 50 %, the largest taper."
    const percent = `${mmText(outcome.mm ?? 0)} %`;
    if (outcome.kind === "max") {
      return `${name} set to ${percent}, the largest taper.`;
    }
    if (outcome.kind === "min") {
      return `${name} set to ${percent}, the smallest taper.`;
    }
    return outcome.kind === "limited"
      ? `${name} limited on ${outcome.limited ?? 0} of ${outcome.of ?? 0} texts.`
      : null;
  }
  const value = `${mmText(outcome.mm ?? 0)}${inDegrees(field) ? "°" : " mm"}`;
  switch (outcome.kind) {
    case "max":
      return `${name} set to ${value}, the largest for this size.`;
    case "min":
      return `${name} set to ${value}, the smallest for this size.`;
    case "limited": {
      // A rotation has no limit by size; the clause is only for the lengths.
      const why = inDegrees(field) ? "" : ", to fit its size";
      return `${name} limited on ${outcome.limited ?? 0} of ${outcome.of ?? 0} texts${why}.`;
    }
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

/** The kern an Alt+arrow key asks for in edit mode (criterion 29): `[along, across, coarse]`
 * steps for Alt+Left, Alt+Right (along −1, 1) and Alt+Up, Alt+Down (across 1, −1), Shift for the
 * coarse step; `null` for any other key. */
export function kernKey(
  key: string,
  alt: boolean,
  shift: boolean,
  ctrl: boolean,
): [number, number, boolean] | null {
  if (!alt || ctrl) {
    return null;
  }
  const steps: Record<string, [number, number]> = {
    ArrowLeft: [-1, 0],
    ArrowRight: [1, 0],
    ArrowUp: [0, 1],
    ArrowDown: [0, -1],
  };
  const step = steps[key];
  return step ? [step[0], step[1], shift] : null;
}

/** The parsed outcome JSON; an unreadable answer is "unchanged". */
export function parseLayoutOutcome(json: string): LayoutOutcome {
  try {
    return JSON.parse(json) as LayoutOutcome;
  } catch {
    return { kind: "unchanged" };
  }
}

/** The storage key of the block's fold; the value is the JSON boolean `true` while it is open. */
export const TEXT_LAYOUT_OPEN_KEY = "curvyo.text-layout-open";

/** The part of `Storage` the fold uses, so a test can hand in a fake. */
export interface FoldStorage {
  getItem(key: string): string | null;
  setItem(key: string, value: string): void;
}

/** The webview's `localStorage`, or `undefined` when even reaching it throws. */
function defaultStorage(): FoldStorage | undefined {
  try {
    return window.localStorage;
  } catch {
    return undefined;
  }
}

/** Whether the block was open when the maker last folded it; a new user starts closed
 * (customer decision 2026-10-25). Anything unreadable means closed. */
export function readLayoutOpen(storage: FoldStorage | undefined = defaultStorage()): boolean {
  try {
    return storage?.getItem(TEXT_LAYOUT_OPEN_KEY) === "true";
  } catch {
    return false;
  }
}

/** Remembers whether the block is open. A failing write is ignored. */
export function writeLayoutOpen(open: boolean, storage: FoldStorage | undefined = defaultStorage()): void {
  try {
    storage?.setItem(TEXT_LAYOUT_OPEN_KEY, JSON.stringify(open));
  } catch {
    /* Not kept; the next start shows the block closed. */
  }
}

/** The fields whose value differs from the default (or is Mixed), in block order: what the
 * folded header's dot stands for ("Changed: Letter spacing, Taper"). */
export function changedLayoutFields(fields: readonly LayoutFieldState[]): string[] {
  return LAYOUT_FIELDS.filter((_, index) => fields[index]?.resettable).map(
    (code) => LAYOUT_FIELD_LABELS[code],
  );
}

/** The header's description while it is folded and a value is changed, else `null`. */
export function foldChangedText(open: boolean, fields: readonly LayoutFieldState[]): string | null {
  if (open) {
    return null;
  }
  const changed = changedLayoutFields(fields);
  return changed.length > 0 ? `Changed: ${changed.join(", ")}` : null;
}
