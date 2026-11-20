// The rules of the font sync that need no wasm and no host (`specs/0116-library-sync` criteria
// 16 to 19): which listed faces make one file, which files make a family, whether a family's
// toggle reads on, and which of a font folder's documents travel. Pure functions, so the tests
// load them without a bundler.

/** A listed face of the Curvyo block, as the host lists it. */
export interface ListedFontFace {
  id: string;
  family: string;
  index: number;
  stamp?: string;
  file?: string;
  size?: number;
  folder?: string | null;
}

/** One font file of the Curvyo folder: its faces share the host id. */
export interface FontFileInfo {
  id: string;
  file: string;
  folder: string | null;
  size: number;
  stamp: string;
  /** The families of its faces, in listed order; the first carries the file's metadata (plan P26). */
  families: string[];
}

/** The files of a Curvyo listing, one per host id. */
export function groupFiles(faces: ListedFontFace[]): FontFileInfo[] {
  const files = new Map<string, FontFileInfo>();
  for (const face of faces) {
    const known = files.get(face.id);
    if (known) {
      if (!known.families.includes(face.family)) {
        known.families.push(face.family);
      }
      continue;
    }
    files.set(face.id, {
      id: face.id,
      file: face.file ?? "",
      folder: face.folder ?? null,
      size: face.size ?? 0,
      stamp: face.stamp ?? "",
      families: [face.family],
    });
  }
  return [...files.values()];
}

/** The files that hold a face of `family`. */
export function filesOfFamily(files: FontFileInfo[], family: string): FontFileInfo[] {
  return files.filter((file) => file.families.includes(family));
}

/** A row of the group's library view. */
export interface FontToggleRow {
  id: string;
  sync: boolean;
}

/** Whether the family's toggle reads on: it has files, every one is hashed and has an entry that
 * is on. A file without an entry never synced and reads off (UX notes section 1, F5). */
export function familySyncOn(
  files: FontFileInfo[],
  hashes: ReadonlyMap<string, string>,
  rows: readonly FontToggleRow[],
): boolean {
  if (files.length === 0) {
    return false;
  }
  return files.every((file) => {
    const hash = hashes.get(file.id);
    return hash !== undefined && rows.some((row) => row.id === hash && row.sync);
  });
}

/** Whether any file of the family has an entry that is on: a press then switches the whole
 * family off, so a family part of whose files was refused can still be switched off. */
export function familySyncAny(
  files: FontFileInfo[],
  hashes: ReadonlyMap<string, string>,
  rows: readonly FontToggleRow[],
): boolean {
  return files.some((file) => {
    const hash = hashes.get(file.id);
    return hash !== undefined && rows.some((row) => row.id === hash && row.sync);
  });
}

/** Whether a file with `hash` and no entry in `rows` starts on by itself (criterion 4): not when
 * its switch-on was refused, and not when the font was deleted on another device while this copy
 * stayed (the maker's own install): it would be published to the whole group again. */
export function startsOnItsOwn(
  hash: string,
  rows: readonly FontToggleRow[],
  refused: ReadonlySet<string>,
  deletedElsewhere: { has(hash: string): boolean },
): boolean {
  return !rows.some((row) => row.id === hash) && !refused.has(hash) && !deletedElsewhere.has(hash);
}

/** The listed files that go to the trash when the font `hash` was deleted elsewhere: only the
 * ones written through sync, by their host id, never the maker's own install or a copy of the
 * same bytes in another folder. */
export function syncedIdsToTrash(
  listed: readonly string[],
  hashes: ReadonlyMap<string, string>,
  hash: string,
  viaSync: { has(id: string): boolean },
): string[] {
  return listed.filter((id) => hashes.get(id) === hash && viaSync.has(id));
}

/** The files with `hash` that were not listed `before`: what a sync write just created. */
export function newIdsOfHash(
  listed: readonly string[],
  before: ReadonlySet<string>,
  hashes: ReadonlyMap<string, string>,
  hash: string,
): string[] {
  return listed.filter((id) => !before.has(id) && hashes.get(id) === hash);
}

/** A set of strings (file hashes or host ids) kept in the browser's storage (a missing or blocked
 * storage keeps it in memory only): the files that came through sync, the fonts deleted
 * elsewhere, and the refusals already told. */
export class HashSetStore {
  private readonly hashes = new Set<string>();
  private readonly key: string;
  private readonly storage: Pick<Storage, "getItem" | "setItem"> | null;

  constructor(key: string, storage: Pick<Storage, "getItem" | "setItem"> | null) {
    this.key = key;
    this.storage = storage;
    try {
      const parsed: unknown = JSON.parse(storage?.getItem(key) ?? "[]");
      for (const hash of Array.isArray(parsed) ? parsed : []) {
        if (typeof hash === "string") {
          this.hashes.add(hash);
        }
      }
    } catch {
      // No storage, or text that is not ours: start empty.
    }
  }

  has(hash: string): boolean {
    return this.hashes.has(hash);
  }

  add(hash: string): void {
    if (!this.hashes.has(hash)) {
      this.hashes.add(hash);
      this.save();
    }
  }

  delete(hash: string): void {
    if (this.hashes.delete(hash)) {
      this.save();
    }
  }

  /** Keeps only the members in `keep`. */
  retain(keep: ReadonlySet<string>): void {
    let dropped = false;
    for (const member of [...this.hashes]) {
      if (!keep.has(member)) {
        this.hashes.delete(member);
        dropped = true;
      }
    }
    if (dropped) {
      this.save();
    }
  }

  private save(): void {
    try {
      this.storage?.setItem(this.key, JSON.stringify([...this.hashes]));
    } catch {
      // Blocked or full: the set stays for this session.
    }
  }
}

/** The documents of a font folder that travel with a font (criterion 18): the text documents; a
 * PDF is not read as text on the desktop and stays where it is. */
export function travellingDocuments(folder: string | null, folders: { name: string; documents: { name: string; size: number }[] }[]): string[] {
  if (folder === null) {
    return [];
  }
  const found = folders.find((entry) => entry.name === folder);
  return (found?.documents ?? []).filter((doc) => !doc.name.toLowerCase().endsWith(".pdf")).map((doc) => doc.name);
}

/** Fired when a synced font changed the font library file, so the Fonts tab reads it again. */
export const FONT_LIBRARY_CHANGED = "curvyo:font-library-changed";
