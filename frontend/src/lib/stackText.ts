/**
 * The words of Fracture and Flatten (`specs/0037-fracture-and-flatten/` criteria 1 to 4, 9, 11,
 * 13 and its UX notes): names, rule lines and notices. The tooltip note is the Boolean card's.
 * Rust sends codes and counts; the sentences are written here, once.
 */

import { textRefusalOutlines, textRefusalText } from "./textText.ts";
import {
  BOOLEAN_OPS,
  type BooleanOp,
  NOTHING_CHANGED,
  sharedRefusalText,
} from "./booleanText.ts";

/** The two commands, as `WasmSession.apply_fracture` and `apply_flatten` name them. */
export type StackCommand = "fracture" | "flatten";

/** The rail order: buttons six and seven of the Boolean card. */
export const STACK_COMMANDS: readonly StackCommand[] = ["fracture", "flatten"];

/** A button of the Boolean card: one of the five operations, one of the two stack commands, or
 * Weld (`specs/0068-laserable-lettering/`). */
export type BooleanCardCommand = BooleanOp | StackCommand | "weld";

/** The Boolean card in rail order: Union to Reverse difference, Fracture and Flatten, then
 * Weld. */
export const BOOLEAN_CARD_COMMANDS: readonly BooleanCardCommand[] = [
  ...BOOLEAN_OPS,
  ...STACK_COMMANDS,
  "weld",
];

/** Whether `command` is Fracture or Flatten. */
export function isStackCommand(command: BooleanCardCommand): command is StackCommand {
  return command === "fracture" || command === "flatten";
}

/** What `apply_fracture` or `apply_flatten` did (`kind` is the outcome code). */
export interface StackResult {
  kind: string;
  /** Objects to blame for a refusal. */
  count: number;
  /** Objects selected. */
  of: number;
  /** Objects replaced or removed. */
  removed: number;
  /** Objects made. */
  pieces: number;
  /** Objects covered completely and removed (Flatten). */
  hidden: number;
  /** A trimmed object has a stroke that now follows the trim (Flatten). */
  strokesFollow: boolean;
  /** The one missing face of a refusal of texts (`specs/0074-text-objects` criterion 29). */
  face?: string;
  closedOpen: number;
  removedOpen: number;
}

const NAMES: Record<StackCommand, string> = {
  fracture: "Fracture",
  flatten: "Flatten",
};

const RULES: Record<StackCommand, string> = {
  fracture: "Cuts overlapping shapes into pieces.",
  flatten: "Trims each shape to what shows.",
};

/** The command's name; also its accessible name. */
export function stackName(command: StackCommand): string {
  return NAMES[command];
}

/** The second tooltip line. */
export function stackRule(command: StackCommand): string {
  return RULES[command];
}

/** The notice after a refusal (criteria 2 to 4), or `null` for a result that is no refusal. */
export function stackRefusalText(command: StackCommand, result: StackResult): string | null {
  const name = NAMES[command];
  const texts = textRefusalText(result);
  if (texts) {
    return texts;
  }
  switch (result.kind) {
    case "no_overlap":
      return `${name}: the selected shapes do not overlap. ${NOTHING_CHANGED}`;
    case "too_many":
      return `${name} would make more than 5000 pieces. ${NOTHING_CHANGED}`;
    default:
      return sharedRefusalText(name, result);
  }
}

/** Whether a refusal outlines objects on the canvas (criterion 2). */
export function stackRefusalOutlinesObjects(result: StackResult): boolean {
  return (
    result.kind === "no_area" ||
    result.kind === "out_of_range" ||
    result.kind === "groups" ||
    textRefusalOutlines(result.kind)
  );
}
