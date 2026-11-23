/**
 * The device cache of the AI models (`specs/0135` criteria 63 to 65, ADR 0076 §5, §8): one model
 * file per pinned SHA-256, written as 4 MiB chunks while it downloads, so a cancelled or broken
 * download resumes at its stored length and is never held whole in memory until it is complete.
 * A file is `complete` only after its pin was checked; a partial file is never read for a Run.
 *
 * The browser (and the desktop webview) keep it in IndexedDB under the same `persist()` request as
 * the fonts. The store is an interface so the download is tested with the memory store.
 */

/** 4 MiB per chunk record. */
export const CHUNK_BYTES = 4 * 1024 * 1024;

/** What the device holds of one model file. */
export type StoredModel =
  | { state: "absent" }
  | { state: "partial"; bytes: number }
  | { state: "complete"; bytes: number };

/** The cache of model files, by their pinned SHA-256. */
export interface ModelStore {
  /** What is held of `sha256`. */
  status(sha256: string): Promise<StoredModel>;
  /** Appends `bytes` at `offset`; refused unless `offset` is the stored length. */
  append(sha256: string, offset: number, bytes: Uint8Array): Promise<void>;
  /** The stored bytes, whole (to verify before commit, or to load a complete file). */
  read(sha256: string): Promise<Uint8Array | null>;
  /** Marks the file complete (after its pin matched). */
  commit(sha256: string): Promise<void>;
  /** Removes the file and its chunks (Remove, a checksum failure, a restart). */
  remove(sha256: string): Promise<void>;
}

/** A store in memory: the tests' and a fallback when IndexedDB is not there. */
export function memoryModelStore(): ModelStore {
  const files = new Map<string, { chunks: Uint8Array[]; complete: boolean }>();
  const length = (sha: string) => (files.get(sha)?.chunks ?? []).reduce((n, c) => n + c.length, 0);
  return {
    status: async (sha) => {
      const file = files.get(sha);
      if (!file) return { state: "absent" };
      return { state: file.complete ? "complete" : "partial", bytes: length(sha) };
    },
    append: async (sha, offset, bytes) => {
      if (offset !== length(sha)) throw new Error("not at the stored length");
      const file = files.get(sha) ?? { chunks: [], complete: false };
      file.chunks.push(bytes.slice());
      files.set(sha, file);
    },
    read: async (sha) => {
      const file = files.get(sha);
      if (!file) return null;
      return concat(file.chunks);
    },
    commit: async (sha) => {
      const file = files.get(sha);
      if (file) file.complete = true;
    },
    remove: async (sha) => {
      files.delete(sha);
    },
  };
}

/** `parts` as one array. */
export function concat(parts: Uint8Array[]): Uint8Array {
  const out = new Uint8Array(parts.reduce((n, p) => n + p.length, 0));
  let at = 0;
  for (const part of parts) {
    out.set(part, at);
    at += part.length;
  }
  return out;
}

const DB_NAME = "curvyo-models";
const DB_VERSION = 1;
const CHUNKS = "chunks";
const FILES = "files";

interface FileRecord {
  bytes: number;
  chunks: number;
  complete: boolean;
}

function request<T>(req: IDBRequest<T>): Promise<T> {
  return new Promise((resolve, reject) => {
    req.onsuccess = () => resolve(req.result);
    req.onerror = () => reject(req.error);
  });
}

let opened: Promise<IDBDatabase> | null = null;

function database(): Promise<IDBDatabase> {
  opened ??= new Promise((resolve, reject) => {
    const req = indexedDB.open(DB_NAME, DB_VERSION);
    req.onupgradeneeded = () => {
      if (!req.result.objectStoreNames.contains(CHUNKS)) req.result.createObjectStore(CHUNKS);
      if (!req.result.objectStoreNames.contains(FILES)) req.result.createObjectStore(FILES);
    };
    req.onsuccess = () => resolve(req.result);
    req.onerror = () => {
      opened = null;
      reject(req.error);
    };
  });
  return opened;
}

async function write(change: (files: IDBObjectStore, chunks: IDBObjectStore) => void): Promise<void> {
  const tx = (await database()).transaction([FILES, CHUNKS], "readwrite");
  change(tx.objectStore(FILES), tx.objectStore(CHUNKS));
  await new Promise<void>((resolve, reject) => {
    tx.oncomplete = () => resolve();
    tx.onerror = () => reject(tx.error);
    tx.onabort = () => reject(tx.error ?? new Error("the browser refused the write"));
  });
}

async function fileRecord(sha: string): Promise<FileRecord | null> {
  const db = await database();
  const value: unknown = await request(db.transaction(FILES).objectStore(FILES).get(sha));
  if (
    typeof value === "object" &&
    value !== null &&
    typeof (value as FileRecord).bytes === "number" &&
    typeof (value as FileRecord).chunks === "number" &&
    typeof (value as FileRecord).complete === "boolean"
  ) {
    return value as FileRecord;
  }
  return null;
}

/** The models of this device in IndexedDB: a file record and one record per chunk. */
export const indexedDbModelStore: ModelStore = {
  status: async (sha) => {
    const file = await fileRecord(sha);
    if (!file) return { state: "absent" };
    return { state: file.complete ? "complete" : "partial", bytes: file.bytes };
  },
  append: async (sha, offset, bytes) => {
    const file = (await fileRecord(sha)) ?? { bytes: 0, chunks: 0, complete: false };
    if (offset !== file.bytes || file.complete) throw new Error("not at the stored length");
    await write((files, chunks) => {
      chunks.put(bytes.slice().buffer, `${sha}/${file.chunks}`);
      files.put({ bytes: file.bytes + bytes.length, chunks: file.chunks + 1, complete: false }, sha);
    });
  },
  read: async (sha) => {
    const file = await fileRecord(sha);
    if (!file) return null;
    const db = await database();
    const store = db.transaction(CHUNKS).objectStore(CHUNKS);
    const parts: Uint8Array[] = [];
    for (let i = 0; i < file.chunks; i++) {
      const value: unknown = await request(store.get(`${sha}/${i}`));
      if (!(value instanceof ArrayBuffer)) return null;
      parts.push(new Uint8Array(value));
    }
    return concat(parts);
  },
  commit: async (sha) => {
    const file = await fileRecord(sha);
    if (!file) return;
    await write((files) => files.put({ ...file, complete: true }, sha));
  },
  remove: async (sha) => {
    const file = await fileRecord(sha);
    await write((files, chunks) => {
      for (let i = 0; i < (file?.chunks ?? 0); i++) chunks.delete(`${sha}/${i}`);
      files.delete(sha);
    });
  },
};
