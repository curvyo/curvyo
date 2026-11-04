// The browser's key custody (`specs/0096-own-devices-sync` criteria 31, 34, 36, ADR 0040
// decision 9): the device identity, the group's and every room's keys as key records in
// IndexedDB `curvyo-keys`, each sealed by one non-extractable AES-GCM-256 key that never leaves
// the browser. The passkey backup (Frage 145 A, decision 13, `keyBackup.ts`) works behind this
// interface: it `load`s the identity to back it up, and a restore on a browser without a group
// `save`s it back before the group is joined (epoch keys return with the keyring wraps).

/** Where a device keeps its secrets: named byte strings. */
export interface DeviceKeyStore {
  /** The names of every stored secret (for a backup). */
  names(): Promise<string[]>;
  /** The secret `name`, `null` when there is none; throws [`UnreadableKeyRecord`] for a record
   * that is there but does not open (newer format, damaged, another wrap key). */
  load(name: string): Promise<Uint8Array | null>;
  save(name: string, bytes: Uint8Array): Promise<void>;
  /** Stores `bytes` as `name` only when no record of that name exists; `false` when one did
   * (another tab was first). */
  add(name: string, bytes: Uint8Array): Promise<boolean>;
  remove(name: string): Promise<void>;
}

/** A stored record that this build cannot open: never overwritten, never taken for absent. */
export class UnreadableKeyRecord extends Error {
  constructor(name: string) {
    super(`the stored key record "${name}" cannot be read in this browser`);
    this.name = "UnreadableKeyRecord";
  }
}

/** The format of one stored key record. */
export const KEY_RECORD_FORMAT_VERSION = 1;

/** One sealed secret as IndexedDB holds it (`format_version` 1). */
export interface KeyRecord {
  format_version: number;
  name: string;
  /** The 12-byte AES-GCM nonce. */
  iv: Uint8Array;
  /** The secret sealed with the record's name as associated data. */
  ciphertext: Uint8Array;
}

const DATABASE = "curvyo-keys";
const RECORDS = "records";
const WRAP = "wrap";
const WRAP_KEY = "aes-gcm-256";

/** Seals `bytes` under `key` as the record `name`; the name is bound as associated data, so a
 * record moved to another name does not open. */
export async function sealRecord(key: CryptoKey, name: string, bytes: Uint8Array, iv: Uint8Array): Promise<KeyRecord> {
  const sealed = await crypto.subtle.encrypt(
    { name: "AES-GCM", iv: iv as BufferSource, additionalData: new TextEncoder().encode(name) },
    key,
    bytes as BufferSource,
  );
  return { format_version: KEY_RECORD_FORMAT_VERSION, name, iv, ciphertext: new Uint8Array(sealed) };
}

/** Opens `record` with `key`; `null` for a newer format, a damaged record or another key. */
export async function openRecord(key: CryptoKey, record: KeyRecord): Promise<Uint8Array | null> {
  if (record.format_version !== KEY_RECORD_FORMAT_VERSION) {
    return null;
  }
  try {
    const opened = await crypto.subtle.decrypt(
      { name: "AES-GCM", iv: record.iv as BufferSource, additionalData: new TextEncoder().encode(record.name) },
      key,
      record.ciphertext as BufferSource,
    );
    return new Uint8Array(opened);
  } catch {
    return null;
  }
}

function request<T>(req: IDBRequest<T>): Promise<T> {
  return new Promise((resolve, reject) => {
    req.onsuccess = () => resolve(req.result);
    req.onerror = () => reject(req.error ?? new Error("indexeddb"));
  });
}

/** Resolves once `tx` committed (not when its request succeeded: a quota abort comes later). */
function committed(tx: IDBTransaction): Promise<void> {
  return new Promise((resolve, reject) => {
    tx.oncomplete = () => resolve();
    tx.onabort = () => reject(tx.error ?? new Error("aborted"));
    tx.onerror = () => reject(tx.error ?? new Error("indexeddb"));
  });
}

function openDatabase(): Promise<IDBDatabase> {
  const open = indexedDB.open(DATABASE, 1);
  open.onupgradeneeded = () => {
    open.result.createObjectStore(RECORDS, { keyPath: "name" });
    open.result.createObjectStore(WRAP);
  };
  return request(open);
}

/** The records in IndexedDB, sealed by a non-extractable key made on first use. */
export class IndexedDbKeyStore implements DeviceKeyStore {
  private database: Promise<IDBDatabase> | null = null;
  private wrapKey: Promise<CryptoKey> | null = null;

  private db(): Promise<IDBDatabase> {
    this.database ??= openDatabase();
    return this.database;
  }

  private key(): Promise<CryptoKey> {
    this.wrapKey ??= (async () => {
      const db = await this.db();
      const found = await request(db.transaction(WRAP).objectStore(WRAP).get(WRAP_KEY) as IDBRequest<CryptoKey | undefined>);
      if (found) {
        return found;
      }
      // Non-extractable: the browser keeps the key material; script can only use it.
      const made = await crypto.subtle.generateKey({ name: "AES-GCM", length: 256 }, false, ["encrypt", "decrypt"]);
      const tx = db.transaction(WRAP, "readwrite");
      const done = committed(tx);
      // Another tab may have made one a moment ago: its key wins.
      tx.objectStore(WRAP).add(made, WRAP_KEY).onerror = (event) => event.preventDefault();
      await done.catch(() => undefined);
      const kept = await request(db.transaction(WRAP).objectStore(WRAP).get(WRAP_KEY) as IDBRequest<CryptoKey | undefined>);
      return kept ?? made;
    })();
    return this.wrapKey;
  }

  async names(): Promise<string[]> {
    const db = await this.db();
    const keys = await request(db.transaction(RECORDS).objectStore(RECORDS).getAllKeys());
    return keys.map(String);
  }

  async load(name: string): Promise<Uint8Array | null> {
    const db = await this.db();
    const record = await request(db.transaction(RECORDS).objectStore(RECORDS).get(name) as IDBRequest<KeyRecord | undefined>);
    if (!record) {
      return null;
    }
    const opened = await openRecord(await this.key(), record);
    if (!opened) {
      throw new UnreadableKeyRecord(name);
    }
    return opened;
  }

  private async sealed(name: string, bytes: Uint8Array): Promise<KeyRecord> {
    const iv = new Uint8Array(12);
    crypto.getRandomValues(iv);
    return sealRecord(await this.key(), name, bytes, iv);
  }

  async save(name: string, bytes: Uint8Array): Promise<void> {
    const record = await this.sealed(name, bytes);
    const tx = (await this.db()).transaction(RECORDS, "readwrite");
    const done = committed(tx);
    tx.objectStore(RECORDS).put(record);
    await done;
  }

  async add(name: string, bytes: Uint8Array): Promise<boolean> {
    const record = await this.sealed(name, bytes);
    const tx = (await this.db()).transaction(RECORDS, "readwrite");
    const done = committed(tx);
    let added = true;
    // A record of that name (another tab's) makes `add` fail: it is kept.
    tx.objectStore(RECORDS).add(record).onerror = (event) => {
      added = false;
      event.preventDefault();
    };
    return done.then(
      () => added,
      () => false,
    );
  }

  async remove(name: string): Promise<void> {
    const tx = (await this.db()).transaction(RECORDS, "readwrite");
    const done = committed(tx);
    tx.objectStore(RECORDS).delete(name);
    await done;
  }
}

/** The answer of `read_device_keys` in the desktop host's shape. */
export type DeviceKeysAnswer =
  | { status: "ok"; identity: string; keys: [number, string][] }
  | { status: "failed"; message: string };

/** The device identity, made on first use: only when no record exists, never over one that does
 * not open; of two tabs making it at once, the first one's is kept. */
export async function deviceIdentity(store: DeviceKeyStore): Promise<Uint8Array> {
  const found = await store.load("identity");
  if (found) {
    return found;
  }
  const fresh = new Uint8Array(64);
  crypto.getRandomValues(fresh);
  await store.add("identity", fresh);
  const kept = await store.load("identity");
  if (!kept) {
    throw new Error("the device identity could not be stored");
  }
  return kept;
}

/** The device identity (64 bytes as hex) and a room's keys in the desktop host's answer shape,
 * kept in `store` (criterion 34). A record that does not open is a refusal, not a fresh start:
 * the device keeps its place in the group (ADR 0040 decision 9). */
export async function readDeviceKeys(store: DeviceKeyStore, room: string): Promise<DeviceKeysAnswer> {
  try {
    const identity = await deviceIdentity(store);
    const keys = room === "" ? null : await store.load(`room:${room}`);
    return {
      status: "ok",
      identity: Array.from(identity, (b) => b.toString(16).padStart(2, "0")).join(""),
      keys: keys ? (JSON.parse(new TextDecoder().decode(keys)) as [number, string][]) : [],
    };
  } catch (error) {
    return { status: "failed", message: error instanceof Error ? error.message : String(error) };
  }
}

/** Keeps a room's epoch keys `[[epoch, hex]]` in `store`. */
export function writeRoomKeys(store: DeviceKeyStore, room: string, keys: [number, string][]): Promise<void> {
  return store.save(`room:${room}`, new TextEncoder().encode(JSON.stringify(keys)));
}

/** This browser's key custody: one store per page. */
export const browserKeyStore: DeviceKeyStore = new IndexedDbKeyStore();
