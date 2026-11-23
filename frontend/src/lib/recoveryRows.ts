// The rows of the recovery block in the File tab (`specs/0136-autosave-and-restore` `ux-notes.md`
// sections 3 to 7): the local time the words need, the row heights, and the keys of the roving
// list. Pure, so `tests/recoveryRows.test.ts` covers it. The words themselves are Rust's
// (`recovery_row_view`); this only carries what they need.

/** A row, px (60 with a third line, `ux-notes.md` section 3). */
export const ROW_PX = 44;
export const ROW_THREE_LINES_PX = 60;

/** A local date and time as `curvyo-ui-core`'s `LocalTime` reads it. */
export interface LocalParts {
  year: number;
  month: number;
  day: number;
  hour: number;
  minute: number;
}

/** `ms` since the epoch in the page's zone. */
export function localParts(ms: number): LocalParts {
  const date = new Date(ms);
  return {
    year: date.getFullYear(),
    month: date.getMonth() + 1,
    day: date.getDate(),
    hour: date.getHours(),
    minute: date.getMinutes(),
  };
}

/** The words of one row, as `recovery_row_view` answers. */
export interface RowView {
  name: string;
  line2: string;
  line3: string | null;
  label: string;
  restore_label: string;
  discard_label: string;
  discard_armed_label: string;
  subject: string;
  recovered_name: string;
}

/** Reads the answer of `recovery_row_view`. */
export function parseRowView(text: string): RowView {
  return JSON.parse(text) as RowView;
}

/** What a key does on the focused row of a list of `count` rows. */
export type RowKey =
  | { kind: "move"; to: number }
  | { kind: "restore" }
  | { kind: "to-button" }
  | { kind: "to-row" }
  | { kind: "arm" }
  | { kind: "escape" }
  | { kind: "none" };

/** The roving list's keys: Up, Down, Home, End without wrap; Enter restores; Right goes to the
 * Restore button and Left back; Delete arms; Escape disarms or leaves for the canvas. A damaged
 * row has no Restore, so Enter and Right do nothing there. */
export function rowKey(key: string, index: number, count: number, restorable: boolean): RowKey {
  switch (key) {
    case "ArrowDown":
      return { kind: "move", to: Math.min(index + 1, count - 1) };
    case "ArrowUp":
      return { kind: "move", to: Math.max(index - 1, 0) };
    case "Home":
      return { kind: "move", to: 0 };
    case "End":
      return { kind: "move", to: count - 1 };
    case "Enter":
      return restorable ? { kind: "restore" } : { kind: "none" };
    case "ArrowRight":
      return restorable ? { kind: "to-button" } : { kind: "none" };
    case "ArrowLeft":
      return { kind: "to-row" };
    case "Delete":
      return { kind: "arm" };
    case "Escape":
      return { kind: "escape" };
    default:
      return { kind: "none" };
  }
}

/** Where the focus goes after row `index` of `count` went: the next row, else the previous one,
 * else `null` (the File tab button, `ux-notes.md` section 6). */
export function focusAfterRemoval(index: number, count: number): number | null {
  if (count <= 1) {
    return null;
  }
  return index < count - 1 ? index : index - 1;
}

/** The help line's path cut at the start with a leading "..." to fit `max` characters. */
export function cutPath(path: string, max: number): string {
  return path.length <= max ? path : `...${path.slice(path.length - (max - 3))}`;
}
