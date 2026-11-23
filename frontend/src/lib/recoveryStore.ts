// Where the recovery copies live (`specs/0136-autosave-and-restore` criteria 3, 9, 11, 12, ADR
// 0079 decisions 3 and 8): the desktop host's commands over the app's local data folder, and in
// the browser one IndexedDB database `curvyo-recovery` (the copy and its record in one
// transaction, each asset once) with a Web Lock held for the life of the tab. Nothing leaves the
// machine (criterion 10). The rules (schedule, record, prune, check) are Rust's; this moves bytes.

import { answerBytes } from "@/lib/binaryArgs";
import {
  type BrowserBackend,
  type RecoveryListing,
  type RecoveryRead,
  type RecoveryStore,
  RecoveryWriteError,
} from "@/lib/recoveryTypes";
import { invoke } from "@/platform/host";

export {
  type BrowserBackend,
  type RecoveryListing,
  type RecoveryRead,
  type RecoveryRow,
  type RecoveryRules,
  type RecoveryStore,
  RecoveryWriteError,
  browserRecoveryStore,
  mintRecoveryKey,
} from "@/lib/recoveryTypes";

// ---- the desktop ------------------------------------------------------------------------------

/** The desktop store: the host's commands (`curvyo-app/src/recovery.rs`). */
export const desktopRecoveryStore: RecoveryStore = {
  list: () => invoke<RecoveryListing>("recovery_list"),
  begin: (key) => invoke<void>("recovery_begin", { key }),
  writeAsset: (key, name, bytes) => failing(invoke<void>("recovery_write_asset", { key, name }, bytes)),
  writeCopy: (key, record, copy) => failing(invoke<void>("recovery_write_copy", { key, record }, copy)),
  deleteCopy: (key) => invoke<void>("recovery_delete_copy", { key }),
  end: (key) => invoke<void>("recovery_end", { key }),
  read: async (key) => {
    try {
      const copy = answerBytes(await invoke<ArrayBuffer>("recovery_read_copy", { key }));
      const names = await invoke<string[]>("recovery_asset_names", { key });
      const assets: RecoveryRead["assets"] = [];
      for (const name of names) {
        try {
          assets.push({ name, bytes: answerBytes(await invoke<ArrayBuffer>("recovery_read_asset", { key, name })) });
        } catch {
          // A missing asset does not refuse the copy: the project opens as a file with a missing
          // asset opens (ADR 0079 §5).
        }
      }
      return { copy, assets };
    } catch {
      return null;
    }
  },
  fileBase: async (path) => {
    const base = await invoke<unknown>("recovery_file_base", { path }).catch(() => null);
    return base ? JSON.stringify(base) : "";
  },
};

/** A host error as the code the words know: the host answers `disk-full`, `no-permission` or its
 * own reason. */
function failing<T>(call: Promise<T>): Promise<T> {
  return call.catch((error: unknown) => {
    throw new RecoveryWriteError(typeof error === "string" ? error : String((error as Error)?.message ?? error));
  });
}

const DB_NAME = "curvyo-recovery";
const COPIES = "copies";
const ASSETS = "assets";

function request<T>(req: IDBRequest<T>): Promise<T> {
  return new Promise((resolve, reject) => {
    req.onsuccess = () => resolve(req.result);
    req.onerror = () => reject(req.error);
  });
}

let opened: Promise<IDBDatabase> | null = null;

function database(): Promise<IDBDatabase> {
  opened ??= new Promise((resolve, reject) => {
    const req = indexedDB.open(DB_NAME, 1);
    req.onupgradeneeded = () => {
      for (const name of [COPIES, ASSETS]) {
        if (!req.result.objectStoreNames.contains(name)) {
          req.result.createObjectStore(name);
        }
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

/** One transaction over both stores; it resolves when the browser has committed it. */
async function transaction(change: (copies: IDBObjectStore, assets: IDBObjectStore) => void): Promise<void> {
  const tx = (await database()).transaction([COPIES, ASSETS], "readwrite");
  change(tx.objectStore(COPIES), tx.objectStore(ASSETS));
  await new Promise<void>((resolve, reject) => {
    tx.oncomplete = () => resolve();
    tx.onerror = () => reject(tx.error);
    tx.onabort = () => reject(tx.error ?? new Error("the browser refused the write"));
  });
}

const assetRange = (key: string) => IDBKeyRange.bound(`${key}/`, `${key}/￿`);

/** The Web Locks this tab holds, by key; a lock lives until its promise settles. */
const lockReleases = new Map<string, () => void>();

/** IndexedDB and Web Locks as the browser backend. */
export const indexedDbBackend: BrowserBackend = {
  putCopy: (key, record, copy) =>
    transaction((copies) => {
      copies.put({ record, copy: copy.slice().buffer }, key);
    }),
  putAsset: async (key, name, bytes) => {
    const db = await database();
    const id = `${key}/${name}`;
    if ((await request(db.transaction(ASSETS).objectStore(ASSETS).count(id))) > 0) {
      return;
    }
    await transaction((_copies, assets) => {
      assets.put(bytes.slice().buffer, id);
    });
  },
  deleteCopy: (key) =>
    transaction((copies, assets) => {
      copies.delete(key);
      assets.delete(assetRange(key));
    }),
  entries: async () => {
    const db = await database();
    const tx = db.transaction([COPIES, ASSETS]);
    const keys = (await request(tx.objectStore(COPIES).getAllKeys())) as string[];
    const out: Awaited<ReturnType<BrowserBackend["entries"]>> = [];
    for (const key of keys) {
      const stored = (await request(tx.objectStore(COPIES).get(key))) as { record: string; copy: ArrayBuffer } | undefined;
      if (!stored) {
        continue;
      }
      const assets = (await request(tx.objectStore(ASSETS).getAll(assetRange(key)))) as ArrayBuffer[];
      out.push({
        key,
        record: stored.record,
        copy: new Uint8Array(stored.copy),
        assetBytes: assets.reduce((sum, asset) => sum + asset.byteLength, 0),
      });
    }
    return out;
  },
  assets: async (key) => {
    const db = await database();
    const store = db.transaction(ASSETS).objectStore(ASSETS);
    const names = (await request(store.getAllKeys(assetRange(key)))) as string[];
    const values = (await request(store.getAll(assetRange(key)))) as ArrayBuffer[];
    return names.map((id, index) => ({ name: id.slice(key.length + 1), bytes: new Uint8Array(values[index]) }));
  },
  hold: (key) => {
    if (lockReleases.has(key)) {
      return Promise.resolve(true);
    }
    const locks = navigator.locks;
    if (!locks) {
      return Promise.resolve(true);
    }
    return new Promise<boolean>((resolve) => {
      void locks.request(`curvyo-recovery-${key}`, { ifAvailable: true }, (lock) => {
        if (!lock) {
          resolve(false);
          return undefined;
        }
        resolve(true);
        return new Promise<void>((release) => lockReleases.set(key, release));
      });
    });
  },
  release: (key) => {
    lockReleases.get(key)?.();
    lockReleases.delete(key);
  },
  held: async () => {
    const state = await navigator.locks?.query();
    const prefix = "curvyo-recovery-";
    return new Set(
      (state?.held ?? []).flatMap((lock) => (lock.name?.startsWith(prefix) ? [lock.name.slice(prefix.length)] : [])),
    );
  },
};

