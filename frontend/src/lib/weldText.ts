/**
 * The words and the Gap rules of Weld (`specs/0068-laserable-lettering/` criteria 1, 2, 10, 23,
 * 27, 28 and its UX notes §2 to §5): the tooltip, the Gap field's value, steps, units and storage,
 * and the notices. Rust sends codes and counts; the sentences are written here, once.
 */

import { openOutlineSentences, plural, sharedRefusalText } from "./booleanText.ts";
import { FLIP_DISABLED_LINE } from "./flipText.ts";
import { parseSize, textRefusalOutlines, textRefusalText } from "./textText.ts";

/** The button's accessible name and tooltip name line. */
export const WELD_NAME = "Weld";

/** The tooltip's rule line. */
export const WELD_RULE = "Joins overlapping shapes into one outline.";

/** The third, muted tooltip line while nothing is selected (the Flip wording). */
export const WELD_DIMMED_LINE = FLIP_DISABLED_LINE;

/** The Gap field's accessible name and tooltip name line, and its rule line. */
export const WELD_GAP_NAME = "Weld gap";
export const WELD_GAP_RULE = "Closes gaps narrower than this.";

/** The per-user setting's key (`adrs.md` decision 7). */
export const WELD_GAP_KEY = "curvyo.weld-gap-mm";

/** The default Gap, and the range. */
export const WELD_GAP_DEFAULT_MM = 0.1;
export const WELD_GAP_MAX_MM = 1;

/** What `apply_weld` did (`kind` is the outcome code). */
export interface WeldResult {
  kind: string;
  /** Objects or texts to blame for a refusal. */
  count: number;
  /** Objects selected. */
  of: number;
  textsOnly: boolean;
  compound: boolean;
  pieces: number;
  dashed: boolean;
  closedOpen: number;
  removedOpen: number;
  mixedStyles: boolean;
  /** The one missing face of a refusal of texts. */
  face?: string;
}

/** The Gap as the field shows it: up to two decimals, trailing zeros trimmed. */
export function gapText(mm: number): string {
  return String(Math.round(mm * 100) / 100);
}

/** A typed Gap in mm (units of `0052`), rounded to 0.01 mm as the field shows it ("0.004 in"
 * is 0.1), or `null` for text that is not a number. */
export function parseGap(text: string): number | null {
  const mm = parseSize(text);
  return mm === null ? null : Math.round(mm * 100) / 100;
}

/** The text a blur of the Gap field commits: the edit, unless Escape discarded it just before
 * (Escape returns focus to the canvas, and that blur must not save what Escape threw away). */
export function gapBlurCommit(draft: string | null, escaped: boolean): string | null {
  return escaped ? null : draft;
}

/** The Gap after an arrow key: 0.01 mm, Shift 0.1 mm, stopped at the limits without a notice. */
export function stepGap(mm: number, up: boolean, shift: boolean): number {
  const step = shift ? 0.1 : 0.01;
  const next = Math.round((mm + (up ? step : -step)) * 100) / 100;
  return Math.min(WELD_GAP_MAX_MM, Math.max(0, next));
}

/** The notice under the field after a typed value was set to a limit (criterion 2), or `null`. */
export function gapClampNotice(clamp: string): string | null {
  if (clamp === "largest") {
    return "Gap set to 1 mm, the largest gap.";
  }
  if (clamp === "smallest") {
    return "Gap set to 0 mm, the smallest gap.";
  }
  return null;
}

/** The stored Gap, or the default when there is none or the storage cannot be read. */
export function readStoredGap(storage: Pick<Storage, "getItem"> | null): number {
  try {
    const value = Number(storage?.getItem(WELD_GAP_KEY) ?? Number.NaN);
    return Number.isFinite(value) && value >= 0 && value <= WELD_GAP_MAX_MM ? value : WELD_GAP_DEFAULT_MM;
  } catch {
    return WELD_GAP_DEFAULT_MM;
  }
}

/** Stores the Gap; a failure keeps the value for the session only. */
export function storeGap(storage: Pick<Storage, "setItem"> | null, mm: number): void {
  try {
    storage?.setItem(WELD_GAP_KEY, String(mm));
  } catch {
    // The value stays for this session.
  }
}

/** The notice after a success (criteria 10 and 23, UX notes §4), or `null` if nothing was done. */
export function weldSuccessText(result: WeldResult): string | null {
  if (result.kind !== "applied") {
    return null;
  }
  const noun = result.textsOnly ? ["text", "texts"] : ["object", "objects"];
  const kind = result.compound ? "compound path" : "path";
  let text = `Weld: ${plural(result.of, noun[0], noun[1])} became 1 ${kind}, ${plural(result.pieces, "piece", "pieces")}.`;
  if (result.dashed) {
    text += " Dashes were welded as solid lines.";
  }
  text += openOutlineSentences(result.closedOpen, result.removedOpen);
  if (result.mixedStyles) {
    text += " It uses the style of the lowest object.";
  }
  return text;
}

/** The notice after a refusal (criteria 27 and 28), or `null` for a result that is no refusal. */
export function weldRefusalText(result: WeldResult): string | null {
  return textRefusalText(result) ?? sharedRefusalText(WELD_NAME, result);
}

/** Whether a refusal outlines objects on the canvas. */
export function weldRefusalOutlines(result: WeldResult): boolean {
  return result.kind === "no_area" || result.kind === "out_of_range" || textRefusalOutlines(result.kind);
}
