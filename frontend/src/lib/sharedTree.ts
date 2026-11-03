// A shared workspace's live tree in the page (`specs/0094-workspace` criteria 35 to 41): what the
// host's scan and the session's disk plan carry, and the small rules the page applies when it
// runs the plan's steps. Pure, so `tests/sharedTree.test.ts` covers it; the plan is Rust's.

import { samePath } from "./workspaceRows.ts";

/** The host's scan of a shared workspace (`scan_shared_workspace`). */
export interface SharedScan {
  entries: unknown[];
  folders: string[][];
  documents: { path: string[]; share: string | null; keyring: boolean }[];
  known: { document: string; path: string[] }[];
}

/** One file operation of the session's disk plan. */
export type DiskStep =
  | { kind: "create-folder"; path: string[] }
  | { kind: "move"; from: string[]; to: string[] }
  | { kind: "write"; path: string[]; document: string }
  | { kind: "keep"; path: string[]; document: string }
  | { kind: "trash-document"; path: string[]; document: string; by: string }
  | { kind: "trash"; path: string[] };

/** The session's disk plan. */
export interface DiskPlan {
  steps: DiskStep[];
  missing: string[][];
  outside: string[][];
  blocked: string[][];
  settled: { document: string; path: string[] }[];
}

/** Reads the session's plan; empty or unreadable text is no plan (no shared workspace). */
export function parseDiskPlan(json: string): DiskPlan | null {
  if (json === "") {
    return null;
  }
  try {
    const value = JSON.parse(json) as Partial<DiskPlan>;
    return {
      steps: value.steps ?? [],
      missing: value.missing ?? [],
      outside: value.outside ?? [],
      blocked: value.blocked ?? [],
      settled: value.settled ?? [],
    };
  } catch {
    return null;
  }
}

/** What happens to a deleted document's file (criterion 40): trashed; the open document is
 * closed first; with unsaved changes it is held until saved elsewhere or discarded. */
export type TrashChoice = "trash" | "close-then-trash" | "hold";

export function trashChoice(path: readonly string[], openPath: readonly string[] | null, dirty: boolean): TrashChoice {
  if (!samePath(path, openPath)) {
    return "trash";
  }
  return dirty ? "hold" : "close-then-trash";
}

/** Where a workspace document keeps its file, as the session answers (`workspace_document_place`). */
export type DocumentPlace = { kind: "file"; path: string[] } | { kind: "deleted" } | { kind: "unknown" };

export function parsePlace(json: string): DocumentPlace {
  if (json === "") {
    return { kind: "unknown" };
  }
  try {
    const value = JSON.parse(json) as { path?: string[]; deleted?: boolean };
    if (value.deleted) {
      return { kind: "deleted" };
    }
    return value.path ? { kind: "file", path: value.path } : { kind: "unknown" };
  } catch {
    return { kind: "unknown" };
  }
}

/** One row of the Deleted list (`ux-notes.md` section 5). */
export interface DeletedRow {
  node: string;
  name: string;
  kind: "folder" | "document";
  document: string;
  line: string;
}

export function parseDeleted(json: string): DeletedRow[] {
  try {
    return JSON.parse(json) as DeletedRow[];
  } catch {
    return [];
  }
}

/** The row states the tree shows for a plan (`WasmWorkspace.set_shared_states`). */
export function sharedStates(plan: DiskPlan | null, receiving: string[][]): string {
  if (!plan) {
    return JSON.stringify({ shared: false });
  }
  return JSON.stringify({ shared: true, outside: plan.outside, missing: plan.missing, receiving });
}
