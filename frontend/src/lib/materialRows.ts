/**
 * The row logic of the Materials tab (`specs/0112-test-grid-and-materials` `ux-notes.md` 7 and 8):
 * accessible names, line 2, the armed texts, the order keys, the window heights and the notice
 * texts. Pure functions over the decoded view (`materialsView.ts`).
 */
import type {
  LaserBlock,
  LoadedMaterialRow,
  MaterialRow,
  MaterialSet,
  TestRow,
} from "./materialsView.ts";
import type { SetKind } from "./operationSettingsView.ts";
import { thicknessText } from "./operationSettingsRows.ts";

/** A row is 40 px high (48 on a coarse pointer, set by the row's own class). */
export const MATERIAL_ROW_HEIGHT = 40;

/** Above this many rows the list is windowed (UX 12). */
export const MATERIAL_WINDOW_ABOVE = 60;

/** The longest material name a notice shows whole. */
export const NOTICE_NAME_MAX = 24;

/** The longest note (UX 8.4). */
export const NOTE_MAX = 500;

/** The words of the three kinds. */
export const KIND_WORD: Record<SetKind, string> = { cut: "Cut", score: "Score", engrave: "Engrave" };

/** The kinds in a laser block's order. */
export const KIND_ORDER: readonly SetKind[] = ["cut", "score", "engrave"];

/** A name cut with an ellipsis where a notice would run over 70 characters. */
export function shortName(name: string): string {
  const chars = Array.from(name);
  return chars.length <= NOTICE_NAME_MAX ? name : `${chars.slice(0, NOTICE_NAME_MAX - 1).join("")}…`;
}

/** The name a row shows on line 1. */
export function rowTitle(row: MaterialRow): string {
  switch (row.state) {
    case "loaded":
      return row.name;
    case "damaged":
      return "Damaged material";
    default:
      return row.name ?? "Material";
  }
}

/** The row's accessible name. */
export function materialRowName(row: MaterialRow): string {
  switch (row.state) {
    case "loaded":
      return `${row.name}, ${thicknessText(row.thicknessMm)}, ${row.line}`;
    case "damaged":
      return `Damaged material, ${row.reason}`;
    case "newer":
      return `${rowTitle(row)}, needs a newer Curvyo`;
    case "older":
      return `${rowTitle(row)}, has an older format`;
  }
}

/** Line 2 of a row: the counts, the reason or the format words. */
export function line2Text(row: MaterialRow): string {
  switch (row.state) {
    case "loaded":
      return row.line;
    case "damaged":
      return row.reason;
    case "newer":
      return "Needs a newer Curvyo";
    case "older":
      return "Has an older format";
  }
}

/** Whether the row has a trash: a newer or older record is never touched. */
export function hasTrash(row: MaterialRow): boolean {
  return row.state === "loaded" || row.state === "damaged";
}

/** Whether a press on the row unfolds its body. */
export function opensBody(row: MaterialRow): row is LoadedMaterialRow {
  return row.state === "loaded";
}

/** "1 test", "3 tests". */
function testsText(count: number): string {
  return count === 1 ? "1 test" : `${count} tests`;
}

/** The armed status of a material trash: "Press again to delete Birch 3 mm. Its 3 tests go with it." */
export function armText(row: MaterialRow): string {
  const base = `Press again to delete ${rowTitle(row)}.`;
  if (row.state !== "loaded" || row.tests === 0) return base;
  return `${base} Its ${testsText(row.tests)} ${row.tests === 1 ? "goes" : "go"} with it.`;
}

/** The armed status of a set trash. */
export function setArmText(kind: SetKind, laserName: string): string {
  return `Press again to delete the ${KIND_WORD[kind]} set of ${laserName}.`;
}

/** The armed status of a test trash. */
export function testArmText(id: string, photo: boolean): string {
  return `Press again to delete test ${id}.${photo ? " Its photo goes with it." : ""}`;
}

/** The armed status of the photo trash. */
export function photoArmText(): string {
  return "Press again to remove the photo.";
}

/** The arm target of a set's trash. */
export function setArmKey(id: string, laser: string, kind: SetKind): string {
  return `material-set:${id}:${laser}:${kind}`;
}

/** The status after a delete. */
export function deletedText(name: string): string {
  return `${name} deleted.`;
}

/** What a key on a material row does. */
export type MaterialKeyAction = "move" | "rename" | "arm" | "remove" | "open" | "none";

/** The target index of a move key, `null` for a key that does not move. */
export function moveTarget(count: number, index: number, key: string): number | null {
  if (count < 2) return null;
  switch (key) {
    case "ArrowUp":
      return index > 0 ? index - 1 : null;
    case "ArrowDown":
      return index < count - 1 ? index + 1 : null;
    case "Home":
      return index > 0 ? 0 : null;
    case "End":
      return index < count - 1 ? count - 1 : null;
    default:
      return null;
  }
}

/** The key action on a row's name button: Alt+arrows move, F2 renames, Delete arms (and removes when armed), Enter or Space open. */
export function materialKeyAction(key: string, alt: boolean, armed: boolean): MaterialKeyAction {
  if (alt && (key === "ArrowUp" || key === "ArrowDown" || key === "Home" || key === "End")) return "move";
  if (key === "F2") return "rename";
  if (key === "Delete") return armed ? "remove" : "arm";
  if (key === "Enter" || key === " ") return armed ? "remove" : "open";
  return "none";
}

/** The window item heights: a row is 40 px, the unfolded one carries its body. */
export function materialHeights(rows: readonly MaterialRow[], unfoldedId: string | null, body: number): number[] {
  return rows.map((row) => MATERIAL_ROW_HEIGHT + (row.state === "loaded" && row.id === unfoldedId ? body : 0));
}

/** The undo notice of a record that could not be written back. */
export function undoFailureText(direction: "undo" | "redo", name: string | null, reason: string): string {
  return `Could not ${direction} ${name === null ? "the material" : shortName(name)}: ${reason}.`;
}

/** The notice of a failed save of a record. */
export function saveFailureText(name: string | null, reason: string): string {
  return `Could not save ${name === null ? "the material" : shortName(name)}: ${reason}.`;
}

/** The accessible name of a test row: "Test K7Q2M, Cut, Lasered 12 Nov, Diode 20 W, with photo". */
export function testRowName(test: TestRow): string {
  return `Test ${test.id}, ${KIND_WORD[test.kind]}, ${test.line.replace(/ · /g, ", ")}${test.photo ? ", with photo" : ""}`;
}

/** The count at a folded laser header: "2 sets", "1 set", "No sets". */
export function laserCountText(block: LaserBlock): string {
  const n = block.sets.length;
  return n === 0 ? "No sets" : n === 1 ? "1 set" : `${n} sets`;
}

/** The fields of a set in the order Power, Passes, Speed, Interval with their units. */
export type SetField = "power" | "passes" | "speed" | "interval";

/** The number of `field` in `set`, `null` for an Interval a set does not have. */
export function setFieldValue(set: MaterialSet, field: SetField): number | null {
  switch (field) {
    case "power":
      return set.numbers.powerPct;
    case "passes":
      return set.numbers.passes;
    case "speed":
      return set.numbers.speedMmMin;
    case "interval":
      return set.intervalMm;
  }
}

/** The unit inside a set field. */
export const SET_FIELD_UNIT: Record<SetField, string> = { power: "%", passes: "×", speed: "mm/min", interval: "mm" };

/** The word of a set field. */
export const SET_FIELD_WORD: Record<SetField, string> = {
  power: "Power",
  passes: "Passes",
  speed: "Speed",
  interval: "Interval",
};

/** A typed field as the number the session takes; `null` for text that is no number. */
export function typedNumber(text: string): number | null {
  const trimmed = text.trim().replace(",", ".");
  if (trimmed === "") return null;
  const value = Number(trimmed);
  return Number.isFinite(value) ? value : null;
}

/** A set number as its field shows it: no trailing zeros. */
export function numberText(value: number): string {
  return String(Math.round(value * 100) / 100);
}

/** The fields a set of `kind` shows. */
export function setFields(kind: SetKind): SetField[] {
  return kind === "engrave" ? ["power", "passes", "speed", "interval"] : ["power", "passes", "speed"];
}

/** The set heading's accessible name: "Cut set of Diode 20 W, Test K7Q2M". */
export function setHeadingName(set: MaterialSet, laserName: string): string {
  return `${KIND_WORD[set.kind]} set of ${laserName}, ${set.source}`;
}

/** The longest material name (UX 7.2). */
export const NAME_MAX = 60;

/** The refusal of the New material form before the session is asked, `null` when it may be added. */
export function newMaterialRefusal(name: string, thickness: string): { field: "name" | "thickness"; text: string } | null {
  const chars = Array.from(name.trim()).length;
  if (chars < 1 || chars > NAME_MAX) return { field: "name", text: "Enter a name." };
  const mm = typedNumber(thickness);
  if (mm === null || mm < 0.1 || mm > 100) return { field: "thickness", text: "Thickness is 0.1 to 100 mm." };
  return null;
}
