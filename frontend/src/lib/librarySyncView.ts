/**
 * The decoder of the library sync's view (`specs/0116-library-sync`, the group glue's
 * `library_view_json`): the per-item toggles of the machines and materials and the library
 * stream's state, every field checked at runtime. A value of the wrong shape decodes to `null`
 * and the rows show the greyed toggle rather than a guess. The row rules drawn from the view
 * (the glyph, the tooltip's second line) and the first-sync confirm's rule (criterion 3) are
 * here as pure functions, so the tests load them without a bundler.
 */

export interface LibraryToggleRow {
  id: string;
  sync: boolean;
}

/** The group's library view as the rows read it. */
export interface LibrarySyncView {
  machines: LibraryToggleRow[];
  materials: LibraryToggleRow[];
  /** Whether the group session is connected. */
  online: boolean;
  /** Whether every library change went out and the relay acknowledged it. */
  acked: boolean;
  /** The toggle tooltip's second line, formatted in Rust ("In sync with 2 devices."). */
  state: string;
}

function isObject(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}

function rows(value: unknown): LibraryToggleRow[] | null {
  if (!Array.isArray(value)) {
    return null;
  }
  const rows: LibraryToggleRow[] = [];
  for (const row of value) {
    if (!isObject(row) || typeof row.id !== "string" || typeof row.sync !== "boolean") {
      return null;
    }
    rows.push({ id: row.id, sync: row.sync });
  }
  return rows;
}

/** The view of `text`, or `null` on any wrong shape. */
export function decodeLibrarySync(text: string): LibrarySyncView | null {
  let parsed: unknown;
  try {
    parsed = JSON.parse(text);
  } catch {
    return null;
  }
  if (!isObject(parsed)) {
    return null;
  }
  const machines = rows(parsed.machines);
  const materials = rows(parsed.materials);
  if (
    machines === null ||
    materials === null ||
    typeof parsed.online !== "boolean" ||
    typeof parsed.acked !== "boolean" ||
    typeof parsed.state !== "string"
  ) {
    return null;
  }
  return { machines, materials, online: parsed.online, acked: parsed.acked, state: parsed.state };
}

/** Whether the item `id` of `kind` syncs: an item without an entry starts on (Frage 214 A). */
export function librarySyncOn(view: LibrarySyncView | null, kind: "machines" | "materials", id: string): boolean {
  return view?.[kind].find((row) => row.id === id)?.sync ?? true;
}

/** The glyph the toggle shows (UX notes §1): `RefreshCw` while this item transfers, `CloudOff`
 * while offline or refused, else `CloudSync`; an off or greyed toggle always keeps the cloud. */
export type LibrarySyncGlyph = "cloud" | "transferring" | "offline";

export function librarySyncGlyph(on: boolean, greyed: boolean, online: boolean, acked: boolean): LibrarySyncGlyph {
  if (!on || greyed) {
    return "cloud";
  }
  return online ? (acked ? "cloud" : "transferring") : "offline";
}

/** The first-sync confirm's answer (criterion 3): confirming runs the first sync (every item
 * on), declining syncs nothing and can be done later. */
export function firstSyncAnswer(confirm: boolean, firstSync: () => void): void {
  if (confirm) {
    firstSync();
  }
}
