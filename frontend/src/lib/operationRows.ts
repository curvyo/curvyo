/**
 * The row logic of the Operations tab (`specs/0101-laser-operations` `ux-notes.md`): the two
 * sections, the accessible names, the summary lines, the reorder limit at the section line, the
 * focus moves and the digit route. Pure functions over the decoded view (`operationsView.ts`); the
 * component only draws what these return.
 */

import {
  operationIdKey,
  type CountKey,
  type OperationKind,
  type OperationRow,
  type OperationSection,
  type OperationsView,
  type TickState,
} from "./operationsView.ts";

/** The kind word, the row's second line and its accessible name (`ux-notes.md` section 4.2). */
export const KIND_WORDS: Record<OperationKind, string> = {
  cut: "Cut",
  score: "Score",
  engrave: "Engrave",
  engrave_outline: "Engrave + Outline",
  cut_out: "Cut-out",
  unknown: "Unknown operation",
};

/** The add buttons' accessible names (`ux-notes.md` section 10). */
export const ADD_LABELS = [
  ["cut", "Add Cut"],
  ["score", "Add Score"],
  ["engrave", "Add Engrave"],
  ["engrave_outline", "Add Engrave + Outline"],
  ["cut_out", "Add Cut-out"],
] as const;


/** How far Page Up and Page Down move the focus (`ux-notes.md` section 9). */
export const ROW_JUMP = 5;

/** "1 item" or "4 items". */
export function itemsText(count: number): string {
  return count === 1 ? "1 item" : `${count} items`;
}

/** The row's accessible name: "Cut 1, Cut, 4 items", with ", Output off" when Output is off. */
export function rowName(row: OperationRow): string {
  if (row.kind === "unknown") return `${row.name}, unknown operation`;
  const off = row.output ? "" : ", Output off";
  return `${row.name}, ${KIND_WORDS[row.kind]}, ${itemsText(row.items)}${off}`;
}

/** A stable React key of a row: the node id. */
export function rowKey(row: OperationRow): string {
  return operationIdKey(row.id);
}

/** The row's section for the grouping; an unknown operation keeps the Marking place it has in the
 * run order (the session's rule). */
export function sectionOf(row: OperationRow): OperationSection {
  return row.section ?? "marking";
}

/** A group of rows: with Cuts last on, one per section; `section` is null for the flat list. */
export interface RowGroup {
  section: OperationSection | null;
  /** The row and its place in the whole list (the number of the digit route is `index + 1`). */
  rows: Array<{ row: OperationRow; index: number }>;
}

/**
 * The list's groups. With Cuts last on, Marking and Cutting, both always present (an empty
 * section keeps its header and its drop slot, `ux-notes.md` 4.1); with it off, one flat group.
 */
export function listGroups(view: OperationsView): RowGroup[] {
  const indexed = view.rows.map((row, index) => ({ row, index }));
  if (!view.cutsLast) {
    return [{ section: null, rows: indexed }];
  }
  return (["marking", "cutting"] as const).map((section) => ({
    section,
    rows: indexed.filter((entry) => sectionOf(entry.row) === section),
  }));
}

function count(n: number, one: string, many: string): string {
  return n === 1 ? one : many;
}

/** One summary line: its text, and the count it selects when pressed (`null`: not a button). */
export interface SummaryLine {
  text: string;
  count: CountKey | null;
}

/** The summary's lines (criterion 17): the run line, then only the lines whose count is above 0. */
export function summaryLines(view: OperationsView): SummaryLine[] {
  if (view.rows.length === 0) {
    return [{ text: "Nothing runs. Assign items to an operation.", count: null }];
  }
  const s = view.summary;
  const lines: SummaryLine[] = [
    {
      text: `Runs ${s.operations} ${count(s.operations, "operation", "operations")}, ${s.running} ${count(s.running, "item", "items")}.`,
      count: null,
    },
  ];
  if (s.notAssigned > 0) {
    lines.push({ text: `${s.notAssigned} ${count(s.notAssigned, "item", "items")} not assigned.`, count: "notAssigned" });
  }
  if (s.runsWhileHidden > 0) {
    lines.push({
      text: `${s.runsWhileHidden} ${count(s.runsWhileHidden, "item runs", "items run")} while hidden.`,
      count: "runsWhileHidden",
    });
  }
  if (s.outputOff > 0) {
    lines.push({
      text: `${s.outputOff} ${count(s.outputOff, "item sits", "items sit")} in operations with Output off.`,
      count: "outputOff",
    });
  }
  return lines;
}

/** The height of a header row, of an operation row, and of its open details (`ux-notes.md` 4.1,
 * 4.2, 4.4): fixed, so the window and the scrollbar agree. */
export const HEADER_HEIGHT = 24;
/** One operation row: 40 px (`ux-notes.md` 4.2); 48 px with a coarse pointer, by CSS. */
export const ROW_HEIGHT = 40;

/** One entry of the list as it is drawn: a section header, or an operation row. */
export type ListEntry =
  | { kind: "header"; section: OperationSection; height: number }
  | { kind: "row"; row: OperationRow; index: number; height: number };

/** The list flattened in drawing order (headers with Cuts last on), each with its height. */
export function listEntries(view: OperationsView, _openKey: string | null = null): ListEntry[] {
  const entries: ListEntry[] = [];
  for (const group of listGroups(view)) {
    if (group.section !== null) {
      entries.push({ kind: "header", section: group.section, height: HEADER_HEIGHT });
    }
    for (const { row, index } of group.rows) {
      entries.push({ kind: "row", row, index, height: ROW_HEIGHT });
    }
  }
  return entries;
}

/** A tick press adds the selection to the operation unless every selected item is in it: a mixed
 * tick becomes full on press and is never cleared directly (`ux-notes.md` 6.2). */
export function tickPressAdds(tick: TickState): boolean {
  return tick !== "all";
}

/**
 * The row an Alt+Up (`step` -1) or Alt+Down (`step` 1) moves row `index` to, or `null` where the
 * move is refused: at the list's end, across the section line with Cuts last on, and for an
 * unknown operation (it can only be renamed or deleted).
 */
export function neighbourIndex(view: OperationsView, index: number, step: -1 | 1): number | null {
  const row = view.rows[index];
  const target = view.rows[index + step];
  if (!row || !target || row.kind === "unknown") return null;
  if (view.cutsLast && sectionOf(target) !== sectionOf(row)) return null;
  return index + step;
}

/** The row the focus goes to for a navigation key, `null` for a key that does not move it. */
export function nextFocusIndex(count: number, current: number, key: string): number | null {
  if (count === 0) return null;
  const last = count - 1;
  switch (key) {
    case "ArrowDown":
      return Math.min(current + 1, last);
    case "ArrowUp":
      return Math.max(current - 1, 0);
    case "Home":
      return 0;
    case "End":
      return last;
    case "PageDown":
      return Math.min(current + ROW_JUMP, last);
    case "PageUp":
      return Math.max(current - ROW_JUMP, 0);
    default:
      return null;
  }
}

/** The operation a digit 1 to 9 names (the number column of the rows); `null` past the rows. */
export function digitRow(view: OperationsView, digit: number): OperationRow | null {
  if (!Number.isInteger(digit) || digit < 1 || digit > 9) return null;
  return view.rows[digit - 1] ?? null;
}

/** A run of consecutive entries drawn in one section group (`ux-notes.md` 4.3). */
export interface SectionRun {
  /** The section's name, or `null` with Cuts last off (one flat list, no groups). */
  section: OperationSection | null;
  entries: ListEntry[];
}

/** The entries split into the runs of one section each, so each run is one group of the list. An
 * unknown operation's row stays in the Marking run, as in `listGroups`. A window that starts
 * inside a section keeps each row's own section. */
export function sectionRuns(entries: ListEntry[], grouped: boolean): SectionRun[] {
  if (!grouped) {
    return entries.length === 0 ? [] : [{ section: null, entries }];
  }
  const runs: SectionRun[] = [];
  for (const entry of entries) {
    const section: OperationSection = entry.kind === "header" ? entry.section : sectionOf(entry.row);
    const last = runs[runs.length - 1];
    if (last && last.section === section && entry.kind === "row") {
      last.entries.push(entry);
    } else if (last && last.section === section && entry.kind === "header" && last.entries.length === 0) {
      last.entries.push(entry);
    } else {
      runs.push({ section, entries: [entry] });
    }
  }
  return runs;
}

/** The word of the drop badge of a canvas drag over an operation row (`ux-notes.md` 6.3). */
export function assignWord(name: string): string {
  return `Assign to ${name}`;
}

/** Whether the Δ move readout is drawn: never while the pointer is over an operation row
 * (`ux-notes.md` 6.3). */
export function liveReadoutVisible<T>(readout: T | null, overRow: boolean): boolean {
  return readout !== null && !overRow;
}

/** Where the 2 px insertion line of a reorder drag sits (ux-notes 4.6): the top edge of the landing
 * row when the row moves up, its bottom edge when it moves down; none where it already is. */
export function insertionEdge(source: number, target: number): "top" | "bottom" | null {
  if (target === source) return null;
  return target < source ? "top" : "bottom";
}

/** The ghost's top in CSS pixels: it follows the pointer vertically, keeping the grab point, and
 * stays inside the list's box (ux-notes 4.6). */
export function ghostTop(
  pointerY: number,
  grabOffset: number,
  listTop: number,
  listBottom: number,
  height: number,
): number {
  return Math.min(Math.max(pointerY - grabOffset, listTop), listBottom - height);
}

/** What a key on a row's name does (ux-notes 4.4, 4.5, 9): Alt+arrows move the row, F2 renames,
 * Delete arms the trash and, on the armed row, removes it; Enter removes only the armed row. */
export type NameKeyAction = "step-up" | "step-down" | "rename" | "arm" | "remove" | "none";

export function nameKeyAction(key: string, altKey: boolean, armed: boolean): NameKeyAction {
  if (altKey && key === "ArrowUp") return "step-up";
  if (altKey && key === "ArrowDown") return "step-down";
  if (key === "F2") return "rename";
  if (key === "Delete") return armed ? "remove" : "arm";
  if (key === "Enter" && armed) return "remove";
  return "none";
}

/** Whether the focus goes back to the row when a rename ends (ux-notes 4.4, 15): Enter and Escape
 * keep it on the row; leaving the field by a press elsewhere (a blur) does not take it back. */
export function focusAfterRename(reason: "enter" | "escape" | "blur"): boolean {
  return reason !== "blur";
}
