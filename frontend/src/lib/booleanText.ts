/**
 * The words of the boolean operations (`specs/0016-boolean-operations/`
 * criteria 1, 2, 16, 17, 29, 38; `specs/0054-combine-open-paths/` criteria
 * 21 to 23): names, rules, tooltip notes and notices.
 * Rust sends codes and counts; the sentences are written here, once.
 */

import { textRefusalOutlines, textRefusalText } from "./textText.ts";

/** The wire code of an operation, as `WasmSession.apply_boolean` takes it. */
export type BooleanOp =
  | "union"
  | "difference"
  | "intersection"
  | "exclusion"
  | "reverse_difference";

/** The rail order (criterion 1). */
export const BOOLEAN_OPS: readonly BooleanOp[] = [
  "union",
  "difference",
  "intersection",
  "exclusion",
  "reverse_difference",
];

/** What the rail section shows (`WasmSession.boolean_availability`). */
export interface BooleanAvailabilityState {
  /** Dimmed: fewer than two objects selected, or not the Select tool. */
  needsTwo: boolean;
  /** How many open outlines a press closes with a straight line. */
  open: number;
}

/** What `WasmSession.apply_boolean` did (`kind` is the outcome code). */
export interface BooleanResult {
  kind: string;
  count: number;
  of: number;
  compound: boolean;
  /** Open outlines closed with a straight line. */
  closedOpen: number;
  /** Open outlines that enclosed no area and were removed. */
  removedOpen: number;
  /** The one missing face of a refusal of texts (`specs/0074-text-objects` criterion 29). */
  face?: string;
}

const NAMES: Record<BooleanOp, string> = {
  union: "Union",
  difference: "Difference",
  intersection: "Intersection",
  exclusion: "Exclusion",
  reverse_difference: "Reverse difference",
};

const RULES: Record<BooleanOp, string> = {
  union: "Everything covered by any selected object.",
  difference: "The lowest selected object minus the others.",
  intersection: "Only what every selected object covers.",
  exclusion: "Areas covered an odd number of times.",
  reverse_difference: "The top selected object minus the others.",
};

const EMPTY_TEXTS: Record<BooleanOp, string> = {
  union: "Union is empty.",
  difference: "Difference is empty: the lowest object is covered completely.",
  intersection: "Intersection is empty: the selected objects share no area.",
  exclusion: "Exclusion is empty: no area is covered an odd number of times.",
  reverse_difference: "Reverse difference is empty: the top object is covered completely.",
};

export const NOTHING_CHANGED = "Nothing was changed.";

/** The operation's name; also its accessible name. */
export function operationName(op: BooleanOp): string {
  return NAMES[op];
}

/** The second tooltip line. */
export function operationRule(op: BooleanOp): string {
  return RULES[op];
}

/** The third tooltip line (criteria 1 and 2). `selectTool` is whether the
 * Select tool is active: with another tool the buttons are dimmed and the
 * note says which tool to take. */
export function tooltipNote(availability: BooleanAvailabilityState, selectTool: boolean): string {
  if (availability.needsTwo) {
    const note = "Select two or more objects.";
    return selectTool ? note : `${note} Use the Select tool.`;
  }
  if (availability.open > 0) {
    return closesOpenNote(availability.open);
  }
  return "Replaces the selection.";
}

/** The tooltip note for `open` open outlines a press closes (`0054` criterion 23). */
export function closesOpenNote(open: number): string {
  return `Closes ${plural(open, "open outline", "open outlines")} with a straight line.`;
}

export function plural(count: number, one: string, many: string): string {
  return count === 1 ? `1 ${one}` : `${count} ${many}`;
}

/** The notice after a success (criterion 29), or `null` if nothing was done. */
export function successText(op: BooleanOp, result: BooleanResult): string | null {
  if (result.kind !== "applied") {
    return null;
  }
  const became = result.compound ? "1 compound path" : "1 path";
  return `${NAMES[op]}: ${result.count} objects became ${became}.${openOutlineSentences(result.closedOpen, result.removedOpen)}`;
}

/** The sentences about open outlines a Boolean operation closed (`0054` criteria 21 and 22), each
 * with a leading space, or "" for none. */
export function openOutlineSentences(closed: number, removed: number): string {
  let text = "";
  if (closed === 1) {
    text += " 1 open outline was closed with a straight line.";
  } else if (closed > 1) {
    text += ` ${closed} open outlines were closed with a straight line.`;
  }
  if (removed === 1) {
    text += " 1 open outline enclosed no area and was removed.";
  } else if (removed > 1) {
    text += ` ${removed} open outlines enclosed no area and were removed.`;
  }
  return text;
}

/** The refusals the rail commands share (`0016` criteria 16 and 17, `0035` criterion 11): no
 * area, out of range, groups. `name` is the command's name. `null` for any other result. */
export function sharedRefusalText(
  name: string,
  result: { kind: string; count: number; of: number },
): string | null {
  const objects = plural(result.of, "selected object", "selected objects");
  switch (result.kind) {
    case "no_area": {
      const verb = result.count === 1 ? "has" : "have";
      // "1 of 1" reads wrong (`0068` UX notes, "Refusals").
      const who = result.of === 1 ? "The selected object" : `${result.count} of ${objects}`;
      return `${name} needs shapes that enclose an area. ${who} ${verb} no area. ${NOTHING_CHANGED}`;
    }
    case "out_of_range": {
      const verb = result.count === 1 ? "reaches" : "reach";
      const who = result.of === 1 ? "The selected object" : `${result.count} of ${objects}`;
      return `${name} works only within 10 km of the point 0, 0. ${who} ${verb} further. ${NOTHING_CHANGED}`;
    }
    case "groups":
      // The operations do not look through groups (`0023` criterion 28).
      return `${name} does not work on groups. Ungroup first. ${NOTHING_CHANGED}`;
    default:
      return null;
  }
}

/** The notice after a refusal (criteria 15 to 17), or `null` for a result
 * that is no refusal (`applied`, `ignored`, `needs_two`). */
export function refusalText(op: BooleanOp, result: BooleanResult): string | null {
  const texts = textRefusalText(result);
  if (texts) {
    return texts;
  }
  if (result.kind === "empty") {
    return `${EMPTY_TEXTS[op]} ${NOTHING_CHANGED}`;
  }
  return sharedRefusalText(NAMES[op], result);
}

/** Whether a refusal outlines objects on the canvas (criterion 16). */
export function refusalOutlinesObjects(result: BooleanResult): boolean {
  return (
    textRefusalOutlines(result.kind) ||
    result.kind === "no_area" ||
    result.kind === "out_of_range" ||
    result.kind === "groups"
  );
}
