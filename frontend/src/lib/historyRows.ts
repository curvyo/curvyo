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
}

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
  status: "applied" | "undone";
  objects: HistoryObject[];
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

/** The status word of a row, empty while it is in effect. */
export function statusWord(row: HistoryRowData): string {
  return row.status === "undone" ? "Undone" : "";
}

/** The second line of a row: status word (only when not Applied), summary, author and time,
 * separated by " · " (criterion 39). */
export function secondLine(row: HistoryRowData, now: Date): string {
  return [statusWord(row), secondLineRest(row, now)].filter((part) => part !== "").join(" · ");
}

/** The second line without its status word: summary, author and time. */
export function secondLineRest(row: HistoryRowData, now: Date): string {
  return [row.summary, authorWord(row), formatRowTime(row.time, now)]
    .filter((part) => part !== "")
    .join(" · ");
}

/** The tooltip: "Move, 3 paths. You, 9 Oct 2026 12:03:41. Id a1b2c3d.", plus the raw label of an
 * unknown old label. */
export function rowTooltip(row: HistoryRowData): string {
  const head = row.summary === "" ? row.name : `${row.name}, ${row.summary}`;
  const text = `${head}. ${authorLong(row)}, ${formatFullTime(row.time)}. Id ${row.id}.`;
  return row.known ? text : `${text} Label: ${row.label}.`;
}

/** The accessible name: "Move, 3 paths, You, 12:03, id a1b2c3d, undone". */
export function rowAriaName(row: HistoryRowData, now: Date): string {
  return [
    row.name,
    row.summary,
    authorLong(row),
    formatRowTime(row.time, now),
    `id ${row.id}`,
    row.status === "undone" ? "undone" : "",
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
