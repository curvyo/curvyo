// Zip archives in the browser's Add font (`specs/0083-font-info-and-zip` criteria 8 to 13 and 18,
// `adrs.md` decision 5): the archive is read by the engine's reader (the same limits as the
// desktop), each font checked as a loose file is, then the whole font folder is written in one
// IndexedDB transaction with no other await inside it, so a full storage keeps nothing of it.
import type { ArchiveResult } from "@/lib/fontLibraryText";
import { freeFolderName } from "@/lib/fontArchiveText";
import {
  type StoredDocument,
  type StoredFaces,
  type StoredFile,
  STORES,
  allFaces,
  database,
  isQuota,
  sha256,
  splitKey,
} from "@/lib/browserFonts";
import { WasmFontArchive, archive_folder_name, font_file_faces } from "@/wasm-bindings/curvyo_editor_wasm.js";

/** The largest archive read (256 MiB), checked before its bytes are read (criterion 15). */
const MAX_ARCHIVE_BYTES = 256 * 1024 * 1024;
/** The order a face offered twice is taken in (criterion 11). */
const KIND_ORDER = ["ttf", "otf", "ttc", "svg"];

interface Entry {
  name: string;
  file_name: string;
  class: "font" | "document" | "left_out" | "unsafe";
  kind: string | null;
}

/** Whether `file` is a zip archive by its name. */
export function isArchiveFile(file: File): boolean {
  return file.name.toLowerCase().endsWith(".zip");
}

/** What one archive gives: the records to write, or a refusal. */
interface Prepared {
  result: ArchiveResult;
  files: StoredFile[];
  faces: StoredFaces[];
  documents: StoredDocument[];
}

function stem(name: string): string {
  const dot = name.lastIndexOf(".");
  return dot > 0 ? name.slice(0, dot) : name;
}

/** Reads and checks one archive against the stored files and those prepared before it. */
async function prepare(file: File, known: StoredFaces[], takenFolders: string[]): Promise<Prepared> {
  const result: ArchiveResult = { name: file.name, added: 0, present: 0, documents: 0, unsafeEntries: 0, refused: null };
  const out: Prepared = { result, files: [], faces: [], documents: [] };
  if (file.size > MAX_ARCHIVE_BYTES) {
    result.refused = "over_limit";
    return out;
  }
  const bytes = await file.arrayBuffer().catch(() => null);
  if (!bytes) {
    result.refused = "unreadable";
    return out;
  }
  let archive: WasmFontArchive;
  try {
    archive = WasmFontArchive.open(new Uint8Array(bytes));
  } catch (code) {
    result.refused = code === "over_limit" ? "over_limit" : "unreadable";
    return out;
  }
  try {
    const entries = (JSON.parse(archive.entries()) as Entry[]).map((entry, index) => ({ ...entry, index }));
    result.unsafeEntries = entries.filter((e) => e.class === "unsafe").length;
    const fonts = entries
      .filter((e) => e.class === "font")
      .sort((a, b) => KIND_ORDER.indexOf(a.kind ?? "") - KIND_ORDER.indexOf(b.kind ?? "") || a.index - b.index);
    const read = (index: number): Uint8Array | null => {
      try {
        return archive.read(index);
      } catch (code) {
        if (code === "over_limit") {
          throw new Error("over_limit");
        }
        result.unsafeEntries += 1;
        return null;
      }
    };
    const folder = freeFolderName(archive_folder_name(stem(file.name)), takenFolders);
    const taken = new Set<string>();
    for (const entry of fonts) {
      const data = read(entry.index);
      if (!data) {
        continue;
      }
      const faces = JSON.parse(font_file_faces(data)) as StoredFaces["faces"];
      if (faces.length === 0) {
        continue;
      }
      const buffer = data.slice().buffer;
      const hash = await sha256(buffer);
      const names = faces.map((face) => `${face.family}\u0000${face.style}`);
      const twice = names.every((n) => taken.has(n));
      const stored = [...known, ...out.faces].some((o) => o.length === data.byteLength && o.sha256 === hash);
      if (twice || stored) {
        result.present += 1;
        continue;
      }
      names.forEach((n) => taken.add(n));
      const key = `${folder}/${entry.file_name}`;
      out.files.push({ name: key, bytes: buffer });
      out.faces.push({ name: key, faces, length: data.byteLength, sha256: hash });
    }
    if (out.files.length > 0) {
      for (const entry of entries.filter((e) => e.class === "document")) {
        const data = read(entry.index);
        if (data) {
          out.documents.push({ key: `${folder}/${entry.file_name}`, bytes: data.slice().buffer });
        }
      }
    }
    result.added = out.files.length;
    result.documents = out.documents.length;
  } catch (error) {
    result.refused = error instanceof Error && error.message === "over_limit" ? "over_limit" : "unreadable";
    out.files = [];
    out.faces = [];
    out.documents = [];
  } finally {
    archive.free();
  }
  return out;
}

/** Writes one prepared archive in one transaction: `full` when the storage had no room,
 * `failed` for any other storage error; either way the transaction keeps nothing. */
async function write(prepared: Prepared): Promise<"written" | "full" | "failed"> {
  try {
    const db = await database();
    await new Promise<void>((resolve, reject) => {
      const tx = db.transaction([STORES.files, STORES.faces, STORES.documents], "readwrite");
      prepared.files.forEach((f) => tx.objectStore(STORES.files).put(f));
      prepared.faces.forEach((f) => tx.objectStore(STORES.faces).put(f));
      prepared.documents.forEach((d) => tx.objectStore(STORES.documents).put(d));
      tx.oncomplete = () => resolve();
      tx.onerror = () => reject(tx.error);
      tx.onabort = () => reject(tx.error);
    });
    return "written";
  } catch (error) {
    return isQuota(error) ? "full" : "failed";
  }
}

/** Adds the archives `files`, one after the other, each as one font folder; `full` when the
 * storage had no room for one (that archive keeps nothing). */
export async function addBrowserFontArchives(files: File[]): Promise<{ archives: ArchiveResult[]; full: boolean }> {
  const archives: ArchiveResult[] = [];
  let full = false;
  for (const file of files) {
    const known = await allFaces();
    const takenFolders = known.map((f) => splitKey(f.name)[0] ?? f.name);
    const prepared = await prepare(file, known, takenFolders);
    const written = prepared.files.length > 0 ? await write(prepared) : "written";
    if (written !== "written") {
      full ||= written === "full";
      prepared.result.refused = written === "failed" ? "not_written" : null;
      prepared.result.added = 0;
      prepared.result.documents = 0;
    }
    archives.push(prepared.result);
  }
  return { archives, full };
}
