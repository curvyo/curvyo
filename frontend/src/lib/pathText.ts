/**
 * The words of the Path card (`specs/0035-combine-and-break-apart/` criteria 2, 7, 9 to 11,
 * `specs/0048-split-compound-path/` criteria 2, 7 to 9, 12 and 13, `specs/0036-cut-at-crossings/`
 * criteria 1, 7 to 10): names, rules, tooltip notes and notices. Rust sends codes and counts; the sentences are written here, once.
 */

import { NOTHING_CHANGED, plural, sharedRefusalText } from "./booleanText.ts";

/** The commands of the Path card, in the card's order. */
export type PathCommand = "combine" | "split" | "break_apart" | "cut";

export const PATH_COMMANDS: readonly PathCommand[] = ["combine", "split", "break_apart", "cut"];

/** What the Path card shows (`WasmSession.path_availability`). */
export interface PathAvailabilityState {
  /** Combine is dimmed: fewer than two objects selected, or not the Select tool. */
  needsTwo: boolean;
  /** How many selected objects are open paths. */
  open: number;
  /** How many objects are selected (0 when `needsTwo`). */
  of: number;
  /** Split and Break apart are enabled: the selection holds a compound path. */
  hasCompound: boolean;
  /** Cut is enabled: something is selected. */
  canCut: boolean;
}

/** What `WasmSession.apply_combine` did (`kind` is the outcome code). */
export interface CombineResult {
  kind: string;
  count: number;
  of: number;
  holes: number;
  stylesDiffer: boolean;
}

/** What `WasmSession.apply_split` did (`kind` is the outcome code). */
export interface SplitResult {
  kind: string;
  compounds: number;
  pieces: number;
  withHoles: boolean;
  leftAlone: number;
  /** Objects selected (for the out-of-range sentence). */
  of?: number;
}

/** What `WasmSession.apply_break_apart` did (`kind` is the outcome code). */
export interface BreakApartResult {
  kind: string;
  compounds: number;
  pieces: number;
  withHoles: boolean;
}

/** What `WasmSession.apply_cut` did (`kind` is the outcome code). `count` paths became `pieces`
 * pieces; for a refusal `count` objects are to blame, `of` objects are selected. */
export interface CutResult {
  kind: string;
  count: number;
  of: number;
  pieces: number;
}

const NAMES: Record<PathCommand, string> = {
  combine: "Combine",
  split: "Split",
  break_apart: "Break apart",
  cut: "Cut",
};

const RULES: Record<PathCommand, string> = {
  combine: "One object. Inner shapes become holes.",
  split: "Separates unconnected parts. Holes stay.",
  break_apart: "Every outline becomes its own object.",
  cut: "Cuts paths open where they cross or touch.",
};

const USE_SELECT = "Use the Select tool.";

/** The command's name; also its accessible name. */
export function commandName(command: PathCommand): string {
  return NAMES[command];
}

/** The second tooltip line (under 45 characters, so it never wraps). */
export function commandRule(command: PathCommand): string {
  return RULES[command];
}

/** Whether the button is dimmed. */
export function commandDimmed(command: PathCommand, availability: PathAvailabilityState): boolean {
  switch (command) {
    case "combine":
      return availability.needsTwo;
    case "cut":
      return !availability.canCut;
    default:
      return !availability.hasCompound;
  }
}

/** The third tooltip line, first match wins (criterion 2). `selectTool` is whether the Select
 * tool is active: with another tool the buttons are dimmed and the note only says which tool to
 * take. */
export function commandNote(
  command: PathCommand,
  availability: PathAvailabilityState,
  selectTool: boolean,
): string {
  // With another tool the session reports nothing as selected, so a sentence about the selection
  // would be false; the tool is the one reason (UX review of 0048 and 0036, 2026-10-12).
  if (!selectTool) {
    return USE_SELECT;
  }
  if (command === "combine") {
    if (availability.needsTwo) {
      return "Select two or more closed shapes.";
    }
    if (availability.open > 0) {
      const verb = availability.open === 1 ? "is" : "are";
      return `Needs closed paths: ${availability.open} of ${availability.of} selected ${verb} open.`;
    }
    return "Replaces the selection.";
  }
  if (command === "cut") {
    return availability.canCut
      ? "Pieces get no fill. Replaces the selection."
      : "Select one or more paths.";
  }
  if (!availability.hasCompound) {
    return "Select a compound path.";
  }
  return command === "split"
    ? "Replaces the selection."
    : "Holes become filled shapes. Use Split to keep them.";
}

/** A success notice lasts 3 s, or 5 s when its sentence is longer than 70 characters (criterion
 * 7); `undefined` is the default of the kind. */
export function noticeMs(text: string): number | undefined {
  return text.length > 70 ? 5000 : undefined;
}

/** The notice after a Combine, or `null` if nothing was done (criterion 7). */
export function combineSuccessText(result: CombineResult): string | null {
  if (result.kind !== "applied") {
    return null;
  }
  const holes = result.holes === 0 ? "" : ` with ${plural(result.holes, "hole", "holes")}`;
  const style = result.stylesDiffer ? " It uses the style of the lowest object." : "";
  return `Combine: ${result.count} objects became 1 compound path${holes}.${style}`;
}

/** The notice after a refused Combine (criteria 9 to 11), or `null` for a result that is no
 * refusal (`applied`, `ignored`, `needs_two`). */
export function combineRefusalText(result: CombineResult): string | null {
  const objects = plural(result.of, "selected object", "selected objects");
  switch (result.kind) {
    case "touching":
      return `Combine needs shapes that do not touch. ${result.count} of ${objects} touch each other. Use Union to merge overlapping shapes. ${NOTHING_CHANGED}`;
    case "self_touching": {
      const verb = result.count === 1 ? "does" : "do";
      return `Combine needs shapes that do not cross themselves. ${result.count} of ${objects} ${verb}. ${NOTHING_CHANGED}`;
    }
    default:
      return sharedRefusalText("Combine", result);
  }
}

/** Whether a refused Combine outlines objects on the canvas (criteria 9 to 11). */
export function combineOutlinesObjects(result: CombineResult): boolean {
  return ["open_paths", "no_area", "out_of_range", "touching", "self_touching", "groups"].includes(
    result.kind,
  );
}

const NO_COMPOUND = "needs a compound path.";

/** The notice after a Split, or `null` if nothing was done (`0048` criterion 8). */
export function splitSuccessText(result: SplitResult): string | null {
  if (result.kind !== "applied") {
    return null;
  }
  const compounds = plural(result.compounds, "compound path", "compound paths");
  let text = `Split: ${compounds} became ${result.pieces} objects.`;
  if (result.withHoles) {
    text += " Holes stayed with their piece.";
  }
  if (result.leftAlone === 1) {
    text += " 1 compound path is one piece and was left as it is.";
  } else if (result.leftAlone > 1) {
    text += ` ${result.leftAlone} compound paths are one piece each and were left as they are.`;
  }
  return text;
}

/** The notice after a refused Split (`0048` criteria 7 and 9), or `null` for a result that is
 * no refusal. A compound path of one piece outlines nothing; a group is the whole selection's
 * fault, so the session outlines the leaves of the groups. */
export function splitRefusalText(result: SplitResult): string | null {
  switch (result.kind) {
    case "no_compound":
      return `Split ${NO_COMPOUND} ${NOTHING_CHANGED}`;
    case "groups":
      return `Split does not work on groups. Ungroup first. ${NOTHING_CHANGED}`;
    case "one_piece":
      return result.compounds === 1
        ? `Nothing to split. The compound path is one piece with its holes. Use Break apart to release the holes. ${NOTHING_CHANGED}`
        : `Nothing to split. The ${result.compounds} selected compound paths are one piece each, with their holes. Use Break apart to release the holes. ${NOTHING_CHANGED}`;
    case "out_of_range":
      return sharedRefusalText("Split", {
        kind: result.kind,
        count: result.compounds,
        of: result.of ?? result.compounds,
      });
    default:
      return null;
  }
}

/** The notice after a Break apart, or `null` if nothing was done (`0048` criterion 12). */
export function breakApartSuccessText(result: BreakApartResult): string | null {
  if (result.kind !== "applied") {
    return null;
  }
  const compounds = plural(result.compounds, "compound path", "compound paths");
  const text = `Break apart: ${compounds} became ${result.pieces} objects.`;
  return result.withHoles ? `${text} Holes became filled shapes.` : text;
}

/** The notice after a refused Break apart (`0048` criterion 13), or `null` for a result that is
 * no refusal. */
export function breakApartRefusalText(result: BreakApartResult): string | null {
  switch (result.kind) {
    case "no_compound":
      return `Break apart ${NO_COMPOUND} ${NOTHING_CHANGED}`;
    case "groups":
      return `Break apart does not work on groups. Ungroup first. ${NOTHING_CHANGED}`;
    default:
      return null;
  }
}

/** The notice after a Cut, or `null` if nothing was done (`0036` criterion 7). */
export function cutSuccessText(result: CutResult): string | null {
  if (result.kind !== "applied") {
    return null;
  }
  const paths = plural(result.count, "path", "paths");
  // Closed paths cut once each give as many pieces as paths: the picture does not change, so the
  // notice says what happened to them.
  if (result.pieces === result.count) {
    return result.count === 1 ? "Cut: 1 path was cut open." : `Cut: ${paths} were cut open.`;
  }
  return `Cut: ${paths} became ${plural(result.pieces, "piece", "pieces")}.`;
}

/** The notice after a refused Cut (`0036` criteria 8 to 10), or `null` for a result that is no
 * refusal. */
export function cutRefusalText(result: CutResult): string | null {
  switch (result.kind) {
    case "no_crossing":
      return result.of === 1
        ? `The selected path does not cross itself. ${NOTHING_CHANGED}`
        : `The selected paths do not cross each other or themselves. ${NOTHING_CHANGED}`;
    case "compound":
      return `Cut does not work on compound paths. Split or Break apart first. ${NOTHING_CHANGED}`;
    case "too_many":
      return `Cut would make more than 10000 pieces. ${NOTHING_CHANGED}`;
    case "none_selected":
      return `Cut needs one or more paths. ${NOTHING_CHANGED}`;
    default:
      return sharedRefusalText("Cut", result);
  }
}

/** Whether a refused Cut outlines objects on the canvas: the groups' leaves, the compound paths
 * and the objects out of range; "do not cross" outlines nothing. */
export function cutOutlinesObjects(result: CutResult): boolean {
  return ["groups", "compound", "out_of_range"].includes(result.kind);
}
