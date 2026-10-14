// The rows of the History tab (`specs/0020-undo-redo` criteria 32 to 46): parsing what the session
// returns, the words of a row (second line, tooltip, accessible name), clock times and the windowing
// and keyboard arithmetic of the virtualised list. Pure, so `tests/historyRows.test.ts` covers it.

/** The fixed height of a step row, px (criterion 39). */
export const ROW_HEIGHT_PX = 44;

/** The height of the pinned "Now" row, px. */
export const NOW_ROW_HEIGHT_PX = 32;

/** How many rows one request for rows asks for, and how many show at once on opening. */
export const PAGE_SIZE = 50;

/** An object a step touched, as the session reports it. */
export interface HistoryObject {
  id: unknown;
  shape: string;
  role: "created" | "deleted" | "changed" | "moved";
  alive: boolean;
  /** The first 6 characters of the object's id ("3f9a1c"). */
  short?: string;
}

/** What a row is: a step, an object undo or redo, the start of a clone, or the floor of a wiped
 * object history (`specs/0041-object-history`). */
export type RowClass = "step" | "undoes" | "redoes" | "clone-start" | "floor";

/** One row of the history list, as the session returns it. */
export interface HistoryRowData {
  /** The shown step id, at least seven characters. */
  id: string;
  /** The step's identity, `<peer>@<counter>`, sent back for the hover link. */
  step: string;
  label: string;
  name: string;
  /** False for a label the table does not know: the name is "Edit". */
  known: boolean;
  author: "you" | "earlier";
  /** Seconds since the epoch; 0 for an unknown time. */
  time: number;
  kind: "create" | "delete" | "replace" | "change" | "document";
  /** "3 paths", or empty when the step touched no object. */
  summary: string;
  /** "undone-by": taken back by an object undo of `undoneBy`; "branch": taken back and then left
   * by a newer step (`specs/0042-history-branches` criterion 4). */
  status: "applied" | "undone" | "undone-by" | "branch";
  /** For a row on a branch: "on branch from a1b2c3d, step 2 of 2" (criterion 9). */
  lane?: string;
  /** For a go to or a clone of a version: the version's shown id (`0042` criteria 10 and 22). */
  version?: string;
  undoneBy?: "you" | "earlier" | "";
  class?: RowClass;
  /** What the step did to the selected object (Object scope rows). */
  role?: "created" | "deleted" | "changed" | "moved";
  /** The short id of the object an inherited row was taken over from; empty for a row of its own. */
  from?: string;
  /** The full ids of the objects a created object came from (a Boolean's operands). */
  sources?: string[];
  objects: HistoryObject[];
}

/** Whether the row stands for a step that is not in effect. */
export function isUndone(row: HistoryRowData): boolean {
  return row.status !== "applied";
}

/** Reads the JSON array the session returns. */
export function parseRows(json: string): HistoryRowData[] {
  const value: unknown = JSON.parse(json);
  return Array.isArray(value) ? (value as HistoryRowData[]) : [];
}

const MONTHS = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];

const two = (n: number) => String(n).padStart(2, "0");

/** The time of a row: the clock time ("12:03"), with the date before it for steps older than today
 * ("9 Oct 12:03"), "Unknown time" for a step from a file with no times. Never relative, so a row
 * does not change under the pointer (criterion 39). */
export function formatRowTime(seconds: number, now: Date): string {
  if (seconds <= 0) {
    return "Unknown time";
  }
  const when = new Date(seconds * 1000);
  const clock = `${two(when.getHours())}:${two(when.getMinutes())}`;
  const sameDay =
    when.getFullYear() === now.getFullYear() &&
    when.getMonth() === now.getMonth() &&
    when.getDate() === now.getDate();
  return sameDay ? clock : `${when.getDate()} ${MONTHS[when.getMonth()]} ${clock}`;
}

/** The full local date and time with seconds, for the tooltip: "9 Oct 2026 12:03:41". */
export function formatFullTime(seconds: number): string {
  if (seconds <= 0) {
    return "Unknown time";
  }
  const when = new Date(seconds * 1000);
  return `${when.getDate()} ${MONTHS[when.getMonth()]} ${when.getFullYear()} ${two(when.getHours())}:${two(when.getMinutes())}:${two(when.getSeconds())}`;
}

/** "You" or "Earlier" in a row. */
export function authorWord(row: HistoryRowData): string {
  return row.author === "you" ? "You" : "Earlier";
}

/** "You" or "Earlier session" in the tooltip and the accessible name. */
export function authorLong(row: HistoryRowData): string {
  return row.author === "you" ? "You" : "Earlier session";
}

/** The status word of a row, empty while it is in effect: "Undone" for a step taken back with
 * Ctrl+Z, "Undone by You" for one taken back by an object undo (`0041` criterion 8), "Branch" for
 * one on a branch (`0042` criterion 4). */
export function statusWord(row: HistoryRowData): string {
  if (row.status === "undone-by") {
    return row.undoneBy === "earlier" ? "Undone by Earlier session" : "Undone by You";
  }
  if (row.status === "branch") {
    return "Branch";
  }
  return row.status === "undone" ? "Undone" : "";
}

/** The words that start line 2 and are set in semibold: the status word and, for an inherited
 * row, "from 3f9a1c". */
export function leadWords(row: HistoryRowData): string {
  const from = row.from ? `from ${row.from}` : "";
  return [statusWord(row), from].filter((part) => part !== "").join(" · ");
}

/** The second line of a row: status word (only when not Applied), summary, author and time,
 * separated by " · " (criterion 39). */
export function secondLine(row: HistoryRowData, now: Date): string {
  return [leadWords(row), secondLineRest(row, now)].filter((part) => part !== "").join(" · ");
}

/** The second line without its status and flag words: summary, author and time. A row that is
 * "Undone by <author>" leaves the author out when it is the step's own, and a row with such a
 * word or with "from <id>" puts the summary last, so a line that does not fit loses the summary
 * and keeps the author and the time (`0041` UX review). */
export function secondLineRest(row: HistoryRowData, now: Date): string {
  const summary = rowDisplay(row).summary;
  const author =
    row.status === "undone-by" && row.undoneBy === row.author ? "" : authorWord(row);
  const time = formatRowTime(row.time, now);
  const parts =
    (row.status === "undone-by" || row.from) && !row.version
      ? [author, time, summary]
      : [summary, author, time];
  return parts.filter((part) => part !== "").join(" · ");
}

/** What a row shows as its name and summary. The wipe of one object's history is titled "Wipe
 * history" with the object on line 2, so the id is not the part an ellipsis cuts. */
export function rowDisplay(row: HistoryRowData): { name: string; summary: string } {
  // A go to or a clone keeps the version's full id on line 2, where an ellipsis does not reach it
  // (`0042` UX review): "Go to version" over "a1b2c3d · Resize".
  if (row.version) {
    const v = row.version;
    if (row.class === "clone-start") {
      return { name: row.name.replace(` at version ${v}`, ""), summary: `at version ${v}` };
    }
    const name = row.name.endsWith(` ${v}`) ? row.name.slice(0, -(v.length + 1)) : row.name;
    return { name, summary: [v, row.summary].filter((part) => part !== "").join(" · ") };
  }
  const wiped = /^Wipe history of (.+)$/.exec(row.name);
  if (row.label === "wipe_object_history" && wiped) {
    return { name: "Wipe history", summary: (wiped[1] ?? "").replace(", ", " ") };
  }
  return { name: row.name, summary: row.summary };
}

/** The tooltip: "Move, 3 paths. You, 9 Oct 2026 12:03:41. Id a1b2c3d.", plus the raw label of an
 * unknown old label. */
export function rowTooltip(row: HistoryRowData): string {
  const head = row.summary === "" ? row.name : `${row.name}, ${row.summary}`;
  const status = row.status === "applied" ? "" : ` ${statusWord(row)}.`;
  const text = `${head}.${status} ${authorLong(row)}, ${formatFullTime(row.time)}. Id ${row.id}.`;
  const sources =
    row.sources && row.sources.length > 0 ? ` Sources: ${row.sources.join(", ")}.` : "";
  const lineage = row.from ? ` Taken over from ${row.from}.` : "";
  const full = `${text}${sources}${lineage}`;
  return row.known ? full : `${full} Label: ${row.label}.`;
}

/** The accessible name: "Move, 3 paths, You, 12:03, id a1b2c3d, undone". */
export function rowAriaName(row: HistoryRowData, now: Date): string {
  return [
    row.name,
    row.summary,
    authorLong(row),
    formatRowTime(row.time, now),
    `id ${row.id}`,
    row.status === "branch" ? (row.lane ?? "") : statusWord(row).toLowerCase(),
    row.from ? `from ${row.from}` : "",
  ]
    .filter((part) => part !== "")
    .join(", ");
}

/** The range of rows to draw for a scroll position: those in view plus `overscan` on each side. */
export function visibleRange(
  scrollTop: number,
  viewportHeight: number,
  total: number,
  overscan = 5,
): { first: number; last: number } {
  if (total <= 0) {
    return { first: 0, last: -1 };
  }
  const top = Math.min(scrollTop, Math.max(0, total * ROW_HEIGHT_PX - viewportHeight));
  const first = Math.max(0, Math.floor(top / ROW_HEIGHT_PX) - overscan);
  const last = Math.min(
    total - 1,
    Math.ceil((top + viewportHeight) / ROW_HEIGHT_PX) + overscan,
  );
  return { first, last };
}

/** The pages (of [`PAGE_SIZE`] rows) that hold the rows `first` to `last`. */
export function pagesFor(first: number, last: number): number[] {
  if (last < first) {
    return [];
  }
  const pages: number[] = [];
  for (let page = Math.floor(first / PAGE_SIZE); page <= Math.floor(last / PAGE_SIZE); page += 1) {
    pages.push(page);
  }
  return pages;
}

/** Where the roving focus goes for a key (criterion 42); `null` for a key that does not move it. */
export function nextFocus(key: string, current: number, total: number, pageRows: number): number | null {
  if (total <= 0) {
    return null;
  }
  const clamp = (n: number) => Math.max(0, Math.min(total - 1, n));
  switch (key) {
    case "ArrowDown":
      return clamp(current + 1);
    case "ArrowUp":
      return clamp(current - 1);
    case "Home":
      return 0;
    case "End":
      return total - 1;
    case "PageDown":
      return clamp(current + pageRows);
    case "PageUp":
      return clamp(current - pageRows);
    default:
      return null;
  }
}

/** The scroll position that keeps the same row under the pointer when `added` rows arrive at the
 * top of a list that is scrolled away from it (criterion 44); a list at the top stays at the top. */
export function scrollAfterInsert(scrollTop: number, added: number): number {
  return scrollTop <= 0 ? 0 : scrollTop + added * ROW_HEIGHT_PX;
}

/** The text of the footer line (criterion 45). */
export const SAVED_IN_FILE_LINE = "The history is saved in the project file.";

/** The question of the inline confirm that replaces the "Wipe history" button (criterion 47). */
export function wipeQuestion(steps: number): string {
  const what = steps === 1 ? "the 1 step" : `all ${steps} steps`;
  return `Remove ${what} from this file? The drawing stays as it is. This cannot be undone.`;
}

/** How long "Wipe" ignores presses after the confirm opened, ms (criterion 47). */
export const WIPE_ARM_MS = 400;

/** The notice after a wipe (UX notes, "Notices"). */
export function wipedNotice(steps: number): string {
  return `History wiped. ${steps} ${steps === 1 ? "step" : "steps"} removed.`;
}

/** The line that takes the button's place in a shared document (criterion 50). */
export const WIPE_SHARED_LINE = "Wiping is not available for shared documents yet.";

/** How many rows arrived at the top between two reads of the count. Pure and idempotent on purpose:
 * the hook must not count inside a state updater, which React runs twice in development. */
export function rowsAdded(seenTotal: number, total: number): number {
  return Math.max(0, total - seenTotal);
}

/** A step an object undo or redo would act on, as the session names it. */
export interface ObjectStepInfo {
  id: string;
  name: string;
  author: "you" | "earlier";
  time: number;
}

/** The subject of the Object scope: what the selected object is called, the steps its two buttons
 * act on and what a wipe would hide. */
export interface ObjectHistoryInfo {
  /** Changes when another object is selected. */
  key: string;
  kind: string;
  /** The first 6 characters of the object's id. */
  id: string;
  fullId: string;
  undo: ObjectStepInfo | null;
  redo: ObjectStepInfo | null;
  /** The rows a wipe would hide. */
  wipeCount: number;
  total: number;
}

/** Reads what `object_history_info` returns; `null` for none or several selected objects. */
export function parseObjectInfo(json: string): ObjectHistoryInfo | null {
  if (json === "") {
    return null;
  }
  return JSON.parse(json) as ObjectHistoryInfo;
}

/** The tooltip of a step button of the Object scope: "Undo: Resize, You, 12:03 (Ctrl+U)", "Redo:
 * Resize (Ctrl+Shift+U)" (`0041` criterion 16). */
export function objectButtonTooltip(
  verb: "Undo" | "Redo",
  step: ObjectStepInfo,
  shortcut: string,
  now: Date,
): string {
  if (verb === "Redo") {
    return `Redo: ${step.name} (${shortcut})`;
  }
  const who = step.author === "you" ? "You" : "Earlier session";
  return `Undo: ${step.name}, ${who}, ${formatRowTime(step.time, now)} (${shortcut})`;
}

/** The question of the inline confirm of a wipe of one object's history (`0041` criterion 29). */
export function objectWipeQuestion(steps: number): string {
  const what = steps === 1 ? "the 1 earlier step" : `the ${steps} earlier steps`;
  return `Hide ${what} of this object? They stay in the document history. The file does not get smaller.`;
}

/** The notice after a wipe of one object's history. */
export function objectWipedNotice(steps: number): string {
  return `Hid ${steps} earlier ${steps === 1 ? "step" : "steps"} of this object.`;
}

/** The row of an object whose timeline the file does not hold (a very old file). */
export const NO_OBJECT_STEPS_LINE = "No steps recorded for this object.";

/** The History tab's scope. */
export type HistoryScope = "document" | "object";

/** The scope that shows: the Object scope exists only with exactly one object selected, and a
 * press on Document is kept while the selection stays one object (`0041` criterion 1). */
export function effectiveScope(selectionCount: number, choice: HistoryScope): HistoryScope {
  return selectionCount === 1 ? choice : "document";
}

/** The choice to keep after the selection changed: leaving the single-object state resets it to
 * Object for the next time. */
export function choiceAfterSelection(selectionCount: number, choice: HistoryScope): HistoryScope {
  return selectionCount === 1 ? choice : "object";
}

/** What an open wipe confirm belongs to: a change of selection, of the object the Object scope is
 * about, of the number of rows or of the scope closes it without writing (`0041` criterion 29).
 * Two objects with the same number of steps must still differ, so the object is part of it. */
export function wipeConfirmKey(state: {
  selectionCount: number;
  subjectKey: string;
  total: number;
  scope: HistoryScope;
}): string {
  return `${state.scope}|${state.selectionCount}|${state.subjectKey}|${state.total}`;
}

/** Whether a kept scroll position can be put back now: one is waiting, the list has rows, and
 * they are the rows of the wanted scope and object (not those of the scope just left, whose
 * shorter list would clamp the position to the top). */
export function canRestoreScroll(state: {
  pending: number | null;
  total: number;
  loadedFor: string;
  wanted: string;
}): boolean {
  return state.pending !== null && state.total > 0 && state.loadedFor === state.wanted;
}
