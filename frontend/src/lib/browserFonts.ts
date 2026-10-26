// The browser build's Curvyo fonts (`specs/0075-font-library` criteria 28 to 31, `adrs.md`
// decision 7): font files the maker adds are kept in this site's IndexedDB, never uploaded.
// Each Add is one transaction, so a full storage keeps nothing of it. The faces of a file are
// read by wasm when it is added and kept in a store of their own with the file's length and
// SHA-256, so listing and the "already there" test read no font bytes; the bytes are read only
// when a face is used. Since `specs/0083-font-info-and-zip` a file may sit in a font folder: its
// key is `<folder>/<file>` (top-level files keep their bare name), and the folder's documents are
// kept in a store of their own under the same keys (`adrs.md` decision 4).
import type { AddResult } from "@/lib/fontLibraryText";
import { requestFontStorageKeep } from "@/lib/fontStorageKeep";
import { font_file_faces } from "@/wasm-bindings/curvyo_editor_wasm.js";

const DB_NAME = "curvyo-fonts";
/** The bytes, by file name. */
const FILES = "files";
/** The faces, length and SHA-256 of each file, by file name. */
const FACES = "faces";
/** The documents of font folders, by `<folder>/<file>` (`0083`). */
const DOCUMENTS = "documents";
/** Version 2 split the bytes from the faces (review of 2026-10-20); version 1 existed only on the
 * unmerged branch, so its store is dropped. Version 3 adds the documents store and nothing else
 * (`0083` adrs.md decision 4). */
const DB_VERSION = 3;
/** The host id prefix of a face in this store. */
export const BROWSER_FONT_PREFIX = "idb:";
/** The largest font file read (64 MiB, ADR 0020 §2). */
const MAX_FONT_BYTES = 64 * 1024 * 1024;
/** `.svg` holds SVG fonts (`specs/0076-single-line-fonts` criterion 3). */
const FONT_EXTENSIONS = [".ttf", ".otf", ".ttc", ".svg"];

interface StoredFace {
  index: number;
  family: string;
  style: string;
  os2: string | null;
  /** `"single_line"` for an SVG font face (`0076`); absent for an outline face. */
  kind?: string;
}

export interface StoredFile {
  name: string;
  bytes: ArrayBuffer;
}

export interface StoredFaces {
  name: string;
  faces: StoredFace[];
  length: number;
  sha256: string;
}

export interface StoredDocument {
  key: string;
  bytes: ArrayBuffer;
}

/** A listed face, as the desktop's `font_folder_list` gives it. */
export interface BrowserListedFace {
  id: string;
  family: string;
  style: string;
  index: number;
  os2: string | null;
  stamp: string;
  kind?: string;
  file: string;
  size: number;
  folder?: string;
}

/** A font folder and its documents, as the desktop lists them. */
export interface ListedFontFolder {
  name: string;
  documents: { name: string; size: number }[];
}

/** The folder and file of a store key: `Lobster/OFL.txt` -> `["Lobster", "OFL.txt"]`. */
export function splitKey(key: string): [string | null, string] {
  const slash = key.indexOf("/");
  return slash < 0 ? [null, key] : [key.slice(0, slash), key.slice(slash + 1)];
}


export function request<T>(req: IDBRequest<T>): Promise<T> {
  return new Promise((resolve, reject) => {
    req.onsuccess = () => resolve(req.result);
    req.onerror = () => reject(req.error);
  });
}

let opened: Promise<IDBDatabase> | null = null;

export function database(): Promise<IDBDatabase> {
  opened ??= new Promise((resolve, reject) => {
    const req = indexedDB.open(DB_NAME, DB_VERSION);
    req.onupgradeneeded = (event) => {
      const db = req.result;
      if (event.oldVersion < 2) {
        for (const store of [FILES, FACES]) {
          if (db.objectStoreNames.contains(store)) {
            db.deleteObjectStore(store);
          }
          db.createObjectStore(store, { keyPath: "name" });
        }
      }
      if (!db.objectStoreNames.contains(DOCUMENTS)) {
        db.createObjectStore(DOCUMENTS, { keyPath: "key" });
      }
    };
    req.onsuccess = () => resolve(req.result);
    req.onerror = () => {
      opened = null;
      reject(req.error);
    };
  });
  return opened;
}

/** The store names, for the archive add's one transaction. */
export const STORES = { files: FILES, faces: FACES, documents: DOCUMENTS } as const;

/** Every stored document (licence and readme files are small). */
async function allDocuments(): Promise<StoredDocument[]> {
  const db = await database();
  return request(db.transaction(DOCUMENTS).objectStore(DOCUMENTS).getAll() as IDBRequest<StoredDocument[]>);
}

/** The faces records of every stored file; no font bytes are read. */
export async function allFaces(): Promise<StoredFaces[]> {
  const db = await database();
  return request(db.transaction(FACES).objectStore(FACES).getAll() as IDBRequest<StoredFaces[]>);
}

/** The SHA-256 of `bytes` as hex. */
export async function sha256(bytes: ArrayBuffer): Promise<string> {
  const digest = new Uint8Array(await crypto.subtle.digest("SHA-256", bytes));
  return Array.from(digest, (b) => b.toString(16).padStart(2, "0")).join("");
}

/** The font folders of the stored files and documents, documents A to Z ignoring case. */
function foldersOf(files: StoredFaces[], documents: StoredDocument[]): ListedFontFolder[] {
  const folders = new Map<string, { name: string; size: number }[]>();
  for (const file of files) {
    const [folder] = splitKey(file.name);
    if (folder !== null && !folders.has(folder)) {
      folders.set(folder, []);
    }
  }
  for (const doc of documents) {
    const [folder, name] = splitKey(doc.key);
    if (folder !== null) {
      folders.set(folder, [...(folders.get(folder) ?? []), { name, size: doc.bytes.byteLength }]);
    }
  }
  return [...folders.entries()].map(([name, docs]) => ({
    name,
    documents: docs.sort((a, b) => a.name.toLowerCase().localeCompare(b.name.toLowerCase())),
  }));
}

/** The Curvyo block of the browser: every face of every stored file, and the font folders. */
export async function listBrowserFonts(): Promise<{
  faces: BrowserListedFace[];
  skipped: string[];
  folders: ListedFontFolder[];
}> {
  const files = await allFaces().catch(() => []);
  const documents = await allDocuments().catch(() => []);
  const faces = files.flatMap((file) => {
    const [folder, name] = splitKey(file.name);
    return file.faces.map((face) => ({
      id: `${BROWSER_FONT_PREFIX}${file.name}`,
      family: face.family,
      style: face.style,
      index: face.index,
      os2: face.os2,
      kind: face.kind,
      // Length and content: a file removed and added again under its name with other bytes of
      // the same length is checked again (review of 2026-10-20).
      stamp: `${file.length}-${file.sha256}`,
      file: name,
      size: file.length,
      ...(folder !== null ? { folder } : {}),
    }));
  });
  return { faces, skipped: [], folders: foldersOf(files, documents) };
}

/** The bytes of document `name` of font folder `folder`. */
export async function browserDocumentBytes(folder: string, name: string): Promise<ArrayBuffer> {
  const db = await database();
  const doc = await request(
    db.transaction(DOCUMENTS).objectStore(DOCUMENTS).get(`${folder}/${name}`) as IDBRequest<StoredDocument | undefined>,
  );
  if (!doc) {
    throw new Error("this document is no longer stored");
  }
  return doc.bytes;
}

/** The bytes of the stored file behind host id `id`. */
export async function browserFontBytes(id: string): Promise<ArrayBuffer> {
  const db = await database();
  const name = id.slice(BROWSER_FONT_PREFIX.length);
  const file = await request(db.transaction(FILES).objectStore(FILES).get(name) as IDBRequest<StoredFile | undefined>);
  if (!file) {
    throw new Error("this font is no longer listed");
  }
  return file.bytes;
}

/** The first free name for `name` among `taken`, compared by Unicode lower case: `name`, else
 * `<stem>-2.<ext>`, `-3`, ... (the rule of `curvyo-storage-io`'s Add). */
export function freeFontName(name: string, taken: string[]): string {
  const isTaken = (candidate: string) => taken.some((t) => t.toLowerCase() === candidate.toLowerCase());
  if (!isTaken(name)) {
    return name;
  }
  const dot = name.lastIndexOf(".");
  const [stem, ext] = dot > 0 ? [name.slice(0, dot), name.slice(dot)] : [name, ""];
  for (let n = 2; ; n += 1) {
    const candidate = `${stem}-${n}${ext}`;
    if (!isTaken(candidate)) {
      return candidate;
    }
  }
}

/** Whether a storage error means the quota is full (criterion 30). */
export function isQuota(error: unknown): boolean {
  return error instanceof DOMException && (error.name === "QuotaExceededError" || error.code === 22);
}

/** The result of an Add in the browser; `full` when the storage had no room (nothing kept). */
export type BrowserAddResult = AddResult & { full: boolean };

/** Adds `files` (criteria 21 to 24 and 28): each is read and checked by wasm, the same bytes
 * again are "already there", a taken name gets `-2`; the new files are written in one
 * transaction. */
export async function addBrowserFontFiles(files: File[]): Promise<BrowserAddResult> {
  requestFontStorageKeep();
  const result: BrowserAddResult = { added: [], present: [], notFonts: [], full: false };
  const existing = await allFaces();
  const fresh: { file: StoredFile; faces: StoredFaces }[] = [];
  for (const file of files) {
    const lower = file.name.toLowerCase();
    if (file.size > MAX_FONT_BYTES || !FONT_EXTENSIONS.some((ext) => lower.endsWith(ext))) {
      result.notFonts.push(file.name);
      continue;
    }
    const bytes = await file.arrayBuffer().catch(() => null);
    const faces: StoredFace[] = bytes ? (JSON.parse(font_file_faces(new Uint8Array(bytes))) as StoredFace[]) : [];
    if (!bytes || faces.length === 0) {
      result.notFonts.push(file.name);
      continue;
    }
    const hash = await sha256(bytes);
    const known = [...existing, ...fresh.map((f) => f.faces)];
    if (known.some((other) => other.length === bytes.byteLength && other.sha256 === hash)) {
      result.present.push(file.name);
      continue;
    }
    const name = freeFontName(file.name, known.map((f) => f.name));
    fresh.push({
      file: { name, bytes },
      faces: { name, faces, length: bytes.byteLength, sha256: hash },
    });
  }
  if (fresh.length === 0) {
    return result;
  }
  const db = await database();
  try {
    await new Promise<void>((resolve, reject) => {
      const tx = db.transaction([FILES, FACES], "readwrite");
      for (const entry of fresh) {
        tx.objectStore(FILES).put(entry.file);
        tx.objectStore(FACES).put(entry.faces);
      }
      tx.oncomplete = () => resolve();
      tx.onerror = () => reject(tx.error);
      tx.onabort = () => reject(tx.error);
    });
    result.added = fresh.map((entry) => entry.file.name);
  } catch (error) {
    if (!isQuota(error)) {
      throw error;
    }
    result.full = true;
  }
  return result;
}

/** Deletes the stored files of a family's faces (criterion 25): a file is deleted only when
 * every face of it is among `faces`, else it is kept. A font folder left without a file loses
 * its documents in the same transaction (`0083` criterion 14). */
export async function removeBrowserFontFiles(
  faces: { id: string; index: number }[],
): Promise<{ removed: string[]; kept: string[]; documents: number }> {
  const files = await allFaces();
  const documents = await allDocuments();
  const removed: string[] = [];
  const kept: string[] = [];
  const db = await database();
  const tx = db.transaction([FILES, FACES, DOCUMENTS], "readwrite");
  for (const id of new Set(faces.map((face) => face.id))) {
    const name = id.slice(BROWSER_FONT_PREFIX.length);
    const file = files.find((f) => f.name === name);
    if (!file) {
      continue;
    }
    const whole = file.faces.every((face) => faces.some((f) => f.id === id && f.index === face.index));
    if (whole) {
      tx.objectStore(FILES).delete(name);
      tx.objectStore(FACES).delete(name);
      removed.push(name);
    } else {
      kept.push(name);
    }
  }
  let gone = 0;
  const touched = new Set(removed.map((name) => splitKey(name)[0]).filter((f): f is string => f !== null));
  for (const folder of touched) {
    const left = files.some((f) => splitKey(f.name)[0] === folder && !removed.includes(f.name));
    if (!left) {
      for (const doc of documents.filter((d) => splitKey(d.key)[0] === folder)) {
        tx.objectStore(DOCUMENTS).delete(doc.key);
        gone += 1;
      }
    }
  }
  await new Promise<void>((resolve, reject) => {
    tx.oncomplete = () => resolve();
    tx.onerror = () => reject(tx.error);
  });
  return { removed, kept, documents: gone };
}

/** The browser's file picker for Add font (criterion 28): several fonts and zip archives
 * (`0083` criterion 7); `null` when cancelled. */
export function pickBrowserFontFiles(): Promise<File[] | null> {
  return new Promise((resolve) => {
    const input = document.createElement("input");
    input.type = "file";
    input.multiple = true;
    input.accept = [...FONT_EXTENSIONS, "image/svg+xml", ".zip", "application/zip"].join(",");
    input.onchange = () => resolve(input.files && input.files.length > 0 ? Array.from(input.files) : null);
    input.addEventListener("cancel", () => resolve(null));
    input.click();
  });
}
