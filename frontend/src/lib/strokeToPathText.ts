/**
 * The words of Stroke to path (`specs/0085-stroke-to-path` criteria 22, 25 and 26, UX notes
 * "States" and "Command and notice"): the button's name, its tooltip and the notices. Rust sends
 * codes and counts; the sentences are written here, once.
 */

import { sharedRefusalText } from "./booleanText.ts";
import { textRefusalText } from "./textText.ts";

/** The button's label, accessible name and enabled tooltip. */
export const STROKE_TO_PATH_NAME = "Stroke to path";

/** The tooltip of the dimmed button (a state label, not a hint). */
export const STROKE_TO_PATH_DIMMED = "Stroke to path: nothing selected has a stroke.";

/** What `WasmSession.apply_stroke_to_path` did (`kind` is the outcome code). */
export interface StrokeToPathResult {
  kind: string;
  /** Objects or texts to blame for a refusal. */
  count: number;
  /** Objects selected. */
  of: number;
  strokes: number;
  unstroked: number;
  texts: number;
  unlinked: number;
  /** The one missing face of a refusal of texts. */
  face?: string;
}

/** The notice of one press, or `null` when nothing happened. */
export interface StrokeToPathNotice {
  text: string;
  alert: boolean;
}

/** The notice after a press (criterion 22, 25, 26), or `null` if nothing was done. */
export function strokeToPathNotice(result: StrokeToPathResult): StrokeToPathNotice | null {
  if (result.kind === "applied") {
    return { text: strokeToPathSuccessText(result), alert: false };
  }
  const refusal = textRefusalText(result) ?? sharedRefusalText(STROKE_TO_PATH_NAME, result);
  return refusal ? { text: refusal, alert: true } : null;
}

/** "Stroke to path: 3 strokes became filled outlines." and the sentences that apply, in order. */
export function strokeToPathSuccessText(result: StrokeToPathResult): string {
  let text =
    result.strokes === 1
      ? "Stroke to path: 1 stroke became a filled outline."
      : `Stroke to path: ${result.strokes} strokes became filled outlines.`;
  if (result.unstroked === 1) {
    text += " 1 object has no stroke and was left as it is.";
  } else if (result.unstroked > 1) {
    text += ` ${result.unstroked} objects have no stroke and were left as they are.`;
  }
  if (result.texts === 1) {
    text += " 1 text was converted to paths.";
  } else if (result.texts > 1) {
    text += ` ${result.texts} texts were converted to paths.`;
  }
  if (result.unlinked === 1) {
    text += " 1 clone was unlinked.";
  } else if (result.unlinked > 1) {
    text += ` ${result.unlinked} clones were unlinked.`;
  }
  return text;
}

/** Whether a key event is Ctrl+Alt+C (Cmd+Alt+C on macOS), not AltGr and not while typing or
 * composing (UX notes "Keyboard"). */
export function isStrokeToPathChord(event: {
  key: string;
  code?: string;
  ctrlKey: boolean;
  metaKey: boolean;
  altKey: boolean;
  shiftKey: boolean;
  isComposing?: boolean;
  getModifierState?: (key: string) => boolean;
}): boolean {
  if (!event.altKey || event.shiftKey || !(event.ctrlKey || event.metaKey) || event.isComposing) {
    return false;
  }
  if (event.getModifierState?.("AltGraph")) {
    return false;
  }
  // With Alt the key value may be a composed character ("ç" on macOS): read the physical key.
  return event.code === "KeyC" || event.key.toLowerCase() === "c";
}
