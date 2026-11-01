/**
 * The words of Weld (`specs/0068-laserable-lettering/` criteria 1, 5, 21, 23, 24 and its UX
 * notes "Tooltip", "Notices" and "Refusals"): name, rule, tooltip note and notices. Rust sends
 * codes, counts and the formatted length; the sentences are written here, once.
 */

import {
  closesOpenNote,
  sharedRefusalText,
} from "./booleanText.ts";
import { textRefusalOutlines, textRefusalText } from "./textText.ts";

/** The button's accessible name and tooltip name line. */
export const WELD_NAME = "Weld";

/** The tooltip's rule line. */
export const WELD_RULE = "Keeps each visible line once, in one object.";

/** What the Weld button shows (`WasmSession.weld_availability`). */
export interface WeldAvailabilityState {
  /** Dimmed: nothing selected, or not the Select tool. */
  needsOne: boolean;
  /** How many open outlines a press closes with a straight line. */
  open: number;
}

/** What `WasmSession.apply_weld` did (`kind` is the outcome code). */
export interface WeldResult {
  kind: string;
  /** Objects or texts to blame for a refusal. */
  count: number;
  /** Objects selected. */
  of: number;
  textsOnly: boolean;
  compound: boolean;
  /** The result's line length in the display unit, as "140.0 mm". */
  lengthText: string;
  /** Shapes covered completely and removed. */
  hidden: number;
  closedOpen: number;
  removedOpen: number;
  mixedStyles: boolean;
  fillSwitchedOff: boolean;
  strokeSwitchedOn: boolean;
  /** The one missing face of a refusal of texts (`specs/0074-text-objects` criterion 29). */
  face?: string;
}

/** The third tooltip line, first match wins: Weld's own "one or more" note, the shared
 * open-outline note, else "Replaces the selection.". */
export function weldTooltipNote(availability: WeldAvailabilityState, selectTool: boolean): string {
  if (availability.needsOne) {
    const note = "Select one or more objects.";
    return selectTool ? note : `${note} Use the Select tool.`;
  }
  if (availability.open > 0) {
    return closesOpenNote(availability.open);
  }
  return "Replaces the selection.";
}

/** The notice after a refusal (criteria 23 and 24), or `null` for a result that is no refusal. */
export function weldRefusalText(result: WeldResult): string | null {
  return textRefusalText(result) ?? sharedRefusalText(WELD_NAME, result);
}

/** Whether a refusal outlines objects on the canvas. */
export function weldRefusalOutlines(result: WeldResult): boolean {
  return (
    result.kind === "no_area" || result.kind === "out_of_range" || textRefusalOutlines(result.kind)
  );
}
