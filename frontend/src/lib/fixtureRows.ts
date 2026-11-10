/**
 * The row logic of the Fixtures block (`specs/0102-laser-fixtures` `ux-notes.md` sections 3, 4,
 * 7.2 and 11): the add kinds at the limits, the accessible names, the second line, the slot count
 * and the fit words. Pure functions over the decoded view (`fixturesView.ts`); the component only
 * draws what these return.
 */

import type { FixtureKind, FixtureRow, FixturesView } from "./fixturesView.ts";

/** The add group's kinds in the order of the spec; Grid is left out while the page has one. */
export const ADD_KINDS = ["grid", "holder", "clamp", "stop", "screw"] as const satisfies readonly FixtureKind[];

/** The slot limit of a holder (criterion 19): the add button is gone at this count. */
export const MAX_SLOTS = 50;

/** The word of each kind (the second line and the accessible name). */
export const KIND_WORDS: Record<FixtureKind, string> = {
  grid: "Grid",
  holder: "Holder",
  clamp: "Clamp",
  stop: "Stop",
  screw: "Screw",
  unknown: "Unknown fixture",
};

/** The add buttons' accessible names (`ux-notes.md` section 11). */
export const ADD_LABELS: Record<(typeof ADD_KINDS)[number], string> = {
  grid: "Add Grid",
  holder: "Add Holder",
  clamp: "Add Clamp",
  stop: "Add Stop",
  screw: "Add Screw",
};

/** The kinds the add group shows: Grid only while the page has none (`ux-notes.md` D6). */
export function addKinds(view: FixturesView): Array<(typeof ADD_KINDS)[number]> {
  return ADD_KINDS.filter((kind) => kind !== "grid" || view.canAddGrid);
}

/** A row's stable React key: its node id. */
export function rowKey(row: FixtureRow): string {
  return row.id;
}

/** The accessible name: "Holder 1, Holder, on the bed", or "... not on the bed". */
export function rowName(row: FixtureRow): string {
  if (row.kind === "unknown") return `${row.name}, unknown fixture`;
  return `${row.name}, ${KIND_WORDS[row.kind]}, ${row.active ? "on the bed" : "not on the bed"}`;
}

/** The second line: the kind word, a holder's slot count, and "not on the bed" when Active is off. */
export function rowLine(row: FixtureRow): string {
  const parts: string[] = [KIND_WORDS[row.kind]];
  if (row.kind === "holder") {
    parts.push(`${row.slots.length} ${row.slots.length === 1 ? "slot" : "slots"}`);
  }
  if (row.kind !== "unknown" && !row.active) {
    parts.push("not on the bed");
  }
  return parts.join(" · ");
}

/** "2 of 50": the slot count of the holder's expanded block. */
export function slotCountText(count: number): string {
  return `${count} of ${MAX_SLOTS}`;
}

/** "Slot <N>" with the lowest N no slot of the holder has (`ux-notes.md` section 4.2). */
export function slotName(names: readonly string[]): string {
  const taken = new Set(names);
  let n = 1;
  while (taken.has(`Slot ${n}`)) n += 1;
  return `Slot ${n}`;
}

/** The Drop badge's words over a fit: "Fits Slot 2", or "Fits Slot 2, turned" (`ux-notes.md` 7.2). */
export function fitWord(slot: string, turned: boolean): string {
  return turned ? `Fits ${slot}, turned` : `Fits ${slot}`;
}

/** What a key on a fixture row asks for (`ux-notes.md` section 10). */
export type RowKeyAction = { kind: "focus"; index: number } | { kind: "rename" } | { kind: "arm" } | { kind: "remove" };

/** Rows Page Up and Page Down move (`ux-notes.md` section 10). */
const PAGE_ROWS = 5;

/**
 * The action a key on row `index` of `count` rows asks for, or null for a key the row keeps. The
 * focus moves do not wrap; Delete arms the trash, and Delete or Enter on the armed row removes it.
 */
export function rowKeyAction(key: string, index: number, count: number, armed: boolean): RowKeyAction | null {
  if (count <= 0) return null;
  const focus = (to: number): RowKeyAction => ({ kind: "focus", index: Math.max(0, Math.min(count - 1, to)) });
  switch (key) {
    case "ArrowDown":
      return focus(index + 1);
    case "ArrowUp":
      return focus(index - 1);
    case "PageDown":
      return focus(index + PAGE_ROWS);
    case "PageUp":
      return focus(index - PAGE_ROWS);
    case "Home":
      return focus(0);
    case "End":
      return focus(count - 1);
    case "F2":
      return { kind: "rename" };
    case "Delete":
      return armed ? { kind: "remove" } : { kind: "arm" };
    case "Enter":
      return armed ? { kind: "remove" } : null;
    default:
      return null;
  }
}

/** The row that is the list's one Tab stop: the focused row while it exists, else the selected one,
 * else the first; null for an empty list. */
export function tabStopRow(ids: readonly string[], selected: string | null, focused: string | null): string | null {
  if (focused !== null && ids.includes(focused)) return focused;
  if (selected !== null && ids.includes(selected)) return selected;
  return ids[0] ?? null;
}

/** The fit cue the session sends during a move (`fixture_fit_cue_json`): the slot's full id, its name
 * and whether the item turns. `null` for the JSON null or any other shape. */
export interface FitCue {
  slot: string;
  name: string;
  turned: boolean;
}

export function decodeFitCue(text: string): FitCue | null {
  try {
    const value: unknown = JSON.parse(text);
    if (typeof value !== "object" || value === null) return null;
    const { slot, name, turned } = value as Record<string, unknown>;
    if (typeof slot !== "string" || typeof name !== "string" || typeof turned !== "boolean") return null;
    return { slot, name, turned };
  } catch {
    return null;
  }
}

/** One Up or Down step of a field (`ux-notes.md` section 4.1): one unit, Shift ten. */
export function stepValue(value: number, direction: "up" | "down", shift: boolean): number {
  const step = shift ? 10 : 1;
  return direction === "up" ? value + step : value - step;
}
