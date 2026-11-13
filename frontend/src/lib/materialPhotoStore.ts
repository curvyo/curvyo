// The browser build's material photos (ADR 0058 §1, §5; `specs/0112-test-grid-and-materials`
// criteria 27, 31, 33): kept in this site's IndexedDB `curvyo-material-photos` under the desktop
// file name `<record id>.<test id>.jpg|png`, never uploaded. The commands mirror the desktop's
// `read_material_photo`, `write_material_photo` and `remove_material_photo`, with the same name
// rule and the same 1 MiB limit.

/** The database and its one store, keyed by file name. */
const DB_NAME = "curvyo-material-photos";
const STORE = "photos";
const DB_VERSION = 1;
/** The most bytes of a photo (ADR 0058 §5, `MAX_PHOTO_BYTES` of `curvyo-laser-core`). */
export const MAX_PHOTO_BYTES = 1024 * 1024;

/** `<record id>.<test id>.jpg|png`: a version 4 UUID in lower case, five of `0-9A-Z`. */
const PHOTO_NAME =
  /^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}\.[0-9A-Z]{5}\.(jpg|png)$/;

/** Whether `name` is a photo file name; any other name is refused before the store. */
export function isPhotoFileName(name: string): boolean {
  return PHOTO_NAME.test(name);
}

/** Where the browser keeps photo bytes; IndexedDB in the page, a double in the tests. */
export interface PhotoStore {
  get(name: string): Promise<Uint8Array | null>;
  put(name: string, bytes: Uint8Array): Promise<void>;
  delete(name: string): Promise<void>;
}

function checkedName(args?: Record<string, unknown>): string {
  const name = String(args?.name ?? "");
  if (!isPhotoFileName(name)) {
    throw new Error("not a photo file name");
  }
  return name;
}

/** The browser stand-ins of the desktop's photo commands over `store`: the read answers an
 * `ArrayBuffer` as Tauri does for raw bytes and rejects with `not-found` when there is none. */
export function browserPhotoCommands(store: PhotoStore) {
  return {
    read_material_photo: async (args?: Record<string, unknown>): Promise<ArrayBuffer> => {
      const bytes = await store.get(checkedName(args));
      if (bytes === null) {
        throw new Error("not-found");
      }
      return bytes.slice().buffer;
    },
    write_material_photo: async (args?: Record<string, unknown>): Promise<void> => {
      const name = checkedName(args);
      const bytes = args?.bytes;
      if (!(bytes instanceof Uint8Array)) {
        throw new Error("no photo bytes");
      }
      if (bytes.byteLength > MAX_PHOTO_BYTES) {
        throw new Error("it is over 1 MiB");
      }
      await store.put(name, bytes);
    },
    remove_material_photo: async (args?: Record<string, unknown>): Promise<void> =>
      store.delete(checkedName(args)),
  };
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
      if (!req.result.objectStoreNames.contains(STORE)) {
        req.result.createObjectStore(STORE);
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

/** One transaction on the store; it resolves when the browser has committed it. */
async function write(change: (store: IDBObjectStore) => void): Promise<void> {
  const tx = (await database()).transaction(STORE, "readwrite");
  change(tx.objectStore(STORE));
  await new Promise<void>((resolve, reject) => {
    tx.oncomplete = () => resolve();
    tx.onerror = () => reject(tx.error);
    tx.onabort = () => reject(tx.error ?? new Error("the browser refused the write"));
  });
}

/** The photos of this browser in IndexedDB; a stored value that is not bytes reads as none. */
export const indexedDbPhotoStore: PhotoStore = {
  get: async (name) => {
    const db = await database();
    const value: unknown = await request(db.transaction(STORE).objectStore(STORE).get(name));
    return value instanceof ArrayBuffer ? new Uint8Array(value) : null;
  },
  put: (name, bytes) => write((store) => store.put(bytes.slice().buffer, name)),
  delete: (name) => write((store) => store.delete(name)),
};
