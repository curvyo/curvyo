// The rows of the File tab's workspace tree (`specs/0094-workspace` criteria 5, 6, 13 to 19,
// `ux-notes.md` section 3): reading what the session and the host answer, the row heights and the
// window of the virtualised list, and small rules on paths. Pure, so `tests/workspaceRows.test.ts`
// covers it. The order, the words and the plans are Rust's; this only reads them.

/** A folder row, px (36 with a coarse pointer). */
export const FOLDER_ROW_PX = 32;
const COARSE_FOLDER_ROW_PX = 36;
/** A document row, px (48 with a coarse pointer). */
export const DOCUMENT_ROW_PX = 44;
const COARSE_DOCUMENT_ROW_PX = 48;
/** A document row "Not in the shared workspace", with Add to workspace on a third line, px (72
 * with a coarse pointer, whose button is 36 px). */
export const OUTSIDE_ROW_PX = 64;
const COARSE_OUTSIDE_ROW_PX = 72;

/** At most this many row elements are in the page (as the Layers list, `0050` criterion 48). */
export const MAX_ROW_ELEMENTS = 60;

/** The indent per level, px, and the deepest indent drawn (ux-notes section 3). */
const INDENT_PX = 12;
const MAX_INDENT_LEVELS = 6;

/** How long the move notice with Undo move stays, ms (criterion 16). */
export const UNDO_MOVE_MS = 10_000;
/** How long a status line stays, ms. */
export const STATUS_MS = 3000;
/** How long a refusal stays, ms. */
export const ALERT_MS = 8000;
/** How often the folder is scanned, ms (ADR 0036 decision 2). */
export const SCAN_MS = 2000;
/** Thumbnails load after the list rests this long, ms (ux-notes section 3). */
export const THUMBNAIL_REST_MS = 100;
/** A collapsed folder under a drag opens after this long, ms. */
export const HOVER_EXPAND_MS = 600;
/** A drag starts past this many px. */
export const DRAG_THRESHOLD_PX = 4;

/** One row as the session returns it. */
export interface WorkspaceRowData {
  path: readonly string[];
  kind: "folder" | "document";
  name: string;
  level: number;
  posinset: number;
  setsize: number;
  expanded: boolean;
  count: number;
  /** The time of the last change ("12:03"); empty for a folder. */
  time: string;
  /** The accessible name. */
  label: string;
  /** A shared workspace's row state: `outside`, `missing`, `receiving`, or empty. */
  state?: string;
}

/** The header line and the shown rows. */
export interface WorkspaceViewData {
  documents: number;
  header: string;
  rows: readonly WorkspaceRowData[];
}

const EMPTY_VIEW: WorkspaceViewData = { documents: 0, header: "0 documents", rows: [] };

/** Reads the view JSON; an unreadable text is an empty tree. */
export function parseWorkspaceView(json: string): WorkspaceViewData {
  try {
    return { ...EMPTY_VIEW, ...(JSON.parse(json) as Partial<WorkspaceViewData>) };
  } catch {
    return EMPTY_VIEW;
  }
}

/** A file operation as the host's `apply_workspace_op` reads it. */
export type WorkspaceOp =
  | { kind: "create-document"; path: string[] }
  | { kind: "create-folder"; path: string[] }
  | { kind: "move"; from: string[]; to: string[] }
  | { kind: "trash"; path: string[] };

/** A planned operation with its words, or the reason it is refused. */
export type PlanAnswer =
  | {
      kind: "planned";
      op: WorkspaceOp;
      name: string;
      status: string;
      confirm: string;
      noTrash: string;
    }
  | { kind: "refused"; text: string };

/** Reads the session's plan; anything without an operation is a refusal. */
export function parsePlan(json: string): PlanAnswer {
  try {
    const value = JSON.parse(json) as {
      op?: WorkspaceOp;
      name?: string;
      status?: string;
      confirm?: string;
      noTrash?: string;
      refusal?: string;
    };
    if (value.op) {
      return {
        kind: "planned",
        op: value.op,
        name: value.name ?? "",
        status: value.status ?? "",
        confirm: value.confirm ?? "",
        noTrash: value.noTrash ?? "",
      };
    }
    return { kind: "refused", text: value.refusal ?? "" };
  } catch {
    return { kind: "refused", text: "" };
  }
}

/** What a row shows of a document file. */
export type ProbeData = { kind: "readable"; png: Uint8Array<ArrayBuffer> | null } | { kind: "unreadable"; code: string };

/** The codes of a probe that found a file it cannot read. */
const UNREADABLE_CODES = new Set(["not-curvyo", "damaged"]);

/** Reads the host's probe: the thumbnail's raw bytes (empty: none), or the error code; `null`
 * when the probe failed for another reason (the file is gone). */
export function parseProbe(outcome: { png: Uint8Array<ArrayBuffer> } | { error: unknown }): ProbeData | null {
  if ("png" in outcome) {
    return { kind: "readable", png: outcome.png.length > 0 ? outcome.png : null };
  }
  const code = String(outcome.error);
  return UNREADABLE_CODES.has(code) ? { kind: "unreadable", code } : null;
}

/** Probes the document at `path` through `call` (the host's raw `probe_workspace_document`). */
export function probeWith(call: () => Promise<Uint8Array<ArrayBuffer>>): Promise<ProbeData | null> {
  return call().then(
    (png) => parseProbe({ png }),
    (error: unknown) => parseProbe({ error }),
  );
}

/** How opening a workspace ended, as the host answers. */
export type WorkspaceAnswer =
  | { kind: "opened"; name: string; entries: unknown[] }
  | { kind: "cancelled" }
  | { kind: "refused"; code: string };

/** One recent workspace. */
export interface RecentWorkspaceData {
  name: string;
  path: string;
  found: boolean;
}

/** The height of a row, px. */
export function rowHeight(row: Pick<WorkspaceRowData, "kind" | "state">, coarse: boolean): number {
  if (row.kind === "folder") {
    return coarse ? COARSE_FOLDER_ROW_PX : FOLDER_ROW_PX;
  }
  if (row.state === "outside") {
    return coarse ? COARSE_OUTSIDE_ROW_PX : OUTSIDE_ROW_PX;
  }
  return coarse ? COARSE_DOCUMENT_ROW_PX : DOCUMENT_ROW_PX;
}

/** The top of every row and, last, the height of the whole list, px. */
export function rowOffsets(rows: readonly Pick<WorkspaceRowData, "kind" | "state">[], coarse: boolean): number[] {
  const offsets = [0];
  for (const row of rows) {
    offsets.push(offsets[offsets.length - 1] + rowHeight(row, coarse));
  }
  return offsets;
}

/** The index of the row at `y` px from the list's top. */
export function rowAt(offsets: readonly number[], y: number): number {
  let low = 0;
  let high = offsets.length - 2;
  while (low < high) {
    const middle = Math.ceil((low + high) / 2);
    if (offsets[middle] <= y) {
      low = middle;
    } else {
      high = middle - 1;
    }
  }
  return Math.max(0, low);
}

/** The rows a scroll position shows, with a few more on each side, never more than
 * [`MAX_ROW_ELEMENTS`] in all. */
export function rowWindow(
  offsets: readonly number[],
  scrollTop: number,
  viewportHeight: number,
): { first: number; count: number } {
  const total = offsets.length - 1;
  if (total <= 0) {
    return { first: 0, count: 0 };
  }
  const height = offsets[total];
  const top = Math.max(0, Math.min(scrollTop, height - viewportHeight));
  const firstShown = rowAt(offsets, top);
  const lastShown = rowAt(offsets, Math.max(top, top + viewportHeight - 1));
  const shown = lastShown - firstShown + 1;
  const overscan = Math.max(0, Math.floor((MAX_ROW_ELEMENTS - shown) / 2));
  const first = Math.max(0, firstShown - overscan);
  const count = Math.min(total - first, MAX_ROW_ELEMENTS, lastShown + overscan + 1 - first);
  return { first, count };
}

/** A stable key of a path. */
export function pathKey(path: readonly string[]): string {
  return JSON.stringify(path);
}

/** Whether two paths name the same entry. */
export function samePath(a: readonly string[] | null, b: readonly string[] | null): boolean {
  return a !== null && b !== null && a.length === b.length && a.every((part, index) => part === b[index]);
}

/** Whether `path` is `folder` or lies below it. */
export function isInside(path: readonly string[], folder: readonly string[]): boolean {
  return path.length >= folder.length && folder.every((part, index) => path[index] === part);
}

/** The folder that holds `path`. */
export function parentOf(path: readonly string[]): string[] {
  return path.slice(0, -1);
}

/** `path` after its entry or a folder above it moved from `from` to `to`. */
export function movedPath(path: readonly string[], from: readonly string[], to: readonly string[]): string[] {
  return isInside(path, from) ? [...to, ...path.slice(from.length)] : [...path];
}

/** The folder New document acts on: the focused folder, a focused document's folder, else the
 * root (ux-notes section 3.2). */
export function targetFolder(row: Pick<WorkspaceRowData, "kind" | "path"> | null): string[] {
  if (!row) {
    return [];
  }
  return row.kind === "folder" ? [...row.path] : parentOf(row.path);
}

/** The key of a row's thumbnail: a changed file loads it again. */
export function thumbnailKey(path: readonly string[], time: string): string {
  return `${pathKey(path)}@${time}`;
}

/** The indent of a row, px: 12 per level below the top, at most six levels. */
export function indentPx(level: number): number {
  return Math.min(Math.max(level - 1, 0), MAX_INDENT_LEVELS) * INDENT_PX;
}

/** The file name of a document path's last component. */
export function fileNameOf(path: readonly string[]): string {
  return path[path.length - 1] ?? "";
}
