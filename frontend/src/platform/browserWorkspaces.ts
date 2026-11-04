// The browser's workspaces (`specs/0096-own-devices-sync` criteria 35, 37 to 39): a synced
// workspace held in IndexedDB `curvyo-workspaces` instead of a folder, served through the same
// host commands the desktop answers for a folder, so the page runs the same workspace code. One
// key-value store: entries (folders and documents) under `<id>|e|<path>`, the marker's share files
// under `<id>|m|<name>`, kept copies of deleted documents under `<id>|d|<document>`, the index
// under `<id>|i`, and the workspaces' names under `w|<id>`. Paths are arrays of names as on the
// desktop; nothing here interprets a document.

import { container_share_members } from "@/wasm-bindings/curvyo_editor_wasm.js";

const DATABASE = "curvyo-workspaces";
const FILES = "files";

/** One entry of a workspace as stored. */
interface StoredEntry {
  kind: "folder" | "document";
  path: string[];
  bytes?: Uint8Array;
  modified: number;
}

interface StoredWorkspace {
  id: string;
  name: string;
}

/** What a full store answers: the browser's numbers for the alert (criterion 37). */
export class StorageFull extends Error {
  constructor() {
    super("full");
  }
}

let database: Promise<IDBDatabase> | null = null;
/** The workspace open in the editor, by id. */
let openId: string | null = null;
/** The workspace document open in the editor, for Save. */
let openDocument: string[] | null = null;
const fullListeners = new Set<(name: string) => void>();

/** Calls `listener` with a document's name when the browser refused to store it. */
export function onStorageFull(listener: (name: string) => void): () => void {
  fullListeners.add(listener);
  return () => fullListeners.delete(listener);
}

function request<T>(req: IDBRequest<T>): Promise<T> {
  return new Promise((resolve, reject) => {
    req.onsuccess = () => resolve(req.result);
    req.onerror = () => reject(req.error ?? new Error("indexeddb"));
  });
}

function db(): Promise<IDBDatabase> {
  database ??= (() => {
    const open = indexedDB.open(DATABASE, 1);
    open.onupgradeneeded = () => {
      open.result.createObjectStore(FILES);
    };
    return request(open);
  })();
  return database;
}

async function get<T>(key: string): Promise<T | undefined> {
  const store = (await db()).transaction(FILES).objectStore(FILES);
  return request(store.get(key) as IDBRequest<T | undefined>);
}

/** Resolves once `tx` committed; a refused write (also a quota abort) rejects. */
function committed(tx: IDBTransaction): Promise<void> {
  return new Promise((resolve, reject) => {
    tx.oncomplete = () => resolve();
    tx.onabort = () => reject(tx.error ?? new Error("aborted"));
    tx.onerror = () => reject(tx.error ?? new Error("indexeddb"));
  });
}

function isQuota(error: unknown): boolean {
  return (error instanceof DOMException || error instanceof Error) && error.name === "QuotaExceededError";
}

async function put(key: string, value: unknown, name = ""): Promise<void> {
  const tx = (await db()).transaction(FILES, "readwrite");
  const done = committed(tx);
  try {
    tx.objectStore(FILES).put(value, key);
    await done;
  } catch (error) {
    if (isQuota(error)) {
      for (const listener of fullListeners) {
        listener(name);
      }
      throw new StorageFull();
    }
    throw error;
  }
}

async function remove(key: string): Promise<void> {
  const tx = (await db()).transaction(FILES, "readwrite");
  const done = committed(tx);
  tx.objectStore(FILES).delete(key);
  await done;
}

/** Every key and value under `prefix`. */
async function below<T>(prefix: string): Promise<[string, T][]> {
  const store = (await db()).transaction(FILES).objectStore(FILES);
  const range = IDBKeyRange.bound(prefix, `${prefix}￿`);
  const [keys, values] = await Promise.all([
    request(store.getAllKeys(range)),
    request(store.getAll(range) as IDBRequest<T[]>),
  ]);
  return keys.map((key, index) => [String(key), values[index] as T]);
}

const entryKey = (id: string, path: readonly string[]) => `${id}|e|${path.join("/")}`;

function workspaceOf(args?: Record<string, unknown>): string {
  const id = typeof args?.workspace === "string" ? args.workspace : openId;
  if (!id) {
    throw "not-found";
  }
  return id;
}

function now(): number {
  return Math.floor(Date.now() / 1000);
}

async function workspaces(): Promise<StoredWorkspace[]> {
  return (await below<StoredWorkspace>("w|")).map(([, value]) => value);
}

async function scan(id: string): Promise<{ path: string[]; kind: string; size: number; modified: number }[]> {
  const entries = await below<StoredEntry>(`${id}|e|`);
  return entries
    .map(([, entry]) => ({
      path: entry.path,
      kind: entry.kind,
      size: entry.bytes?.byteLength ?? 0,
      modified: entry.modified,
    }))
    .sort((a, b) => a.path.join("/").localeCompare(b.path.join("/")));
}

/** Runs a planned operation like `curvyo-storage-io`: nothing is replaced, a parent must exist. */
async function applyOp(id: string, op: { kind: string; path?: string[]; from?: string[]; to?: string[] }, bytes?: Uint8Array): Promise<void> {
  const parentExists = async (path: readonly string[]) =>
    path.length <= 1 || (await get<StoredEntry>(entryKey(id, path.slice(0, -1))))?.kind === "folder";
  switch (op.kind) {
    case "create-document":
    case "create-folder": {
      const path = op.path ?? [];
      if (path.length === 0 || !(await parentExists(path))) {
        throw "not-found";
      }
      if (await get(entryKey(id, path))) {
        throw "exists";
      }
      const entry: StoredEntry =
        op.kind === "create-folder"
          ? { kind: "folder", path, modified: now() }
          : { kind: "document", path, bytes: bytes ?? new Uint8Array(), modified: now() };
      await put(entryKey(id, path), entry, path[path.length - 1] ?? "");
      return;
    }
    case "move": {
      const from = op.from ?? [];
      const to = op.to ?? [];
      if (!(await get(entryKey(id, from)))) {
        throw "not-found";
      }
      if ((await get(entryKey(id, to))) || !(await parentExists(to))) {
        throw "exists";
      }
      const moved = await below<StoredEntry>(entryKey(id, from));
      for (const [key, entry] of moved) {
        if (key !== entryKey(id, from) && !key.startsWith(`${entryKey(id, from)}/`)) {
          continue;
        }
        const path = [...to, ...entry.path.slice(from.length)];
        await put(entryKey(id, path), { ...entry, path }, path[path.length - 1] ?? "");
        await remove(key);
      }
      if (openDocument && from.every((part, index) => openDocument?.[index] === part)) {
        openDocument = [...to, ...openDocument.slice(from.length)];
      }
      return;
    }
    case "trash": {
      // The browser has no OS trash: the entry and what is in it go.
      const path = op.path ?? [];
      for (const [key] of await below<StoredEntry>(entryKey(id, path))) {
        if (key === entryKey(id, path) || key.startsWith(`${entryKey(id, path)}/`)) {
          await remove(key);
        }
      }
      return;
    }
    default:
      throw "not-found";
  }
}

async function readDocument(id: string, path: readonly string[]): Promise<ArrayBuffer> {
  const entry = await get<StoredEntry>(entryKey(id, path));
  if (!entry || entry.kind !== "document" || !entry.bytes) {
    throw "not-found";
  }
  return entry.bytes.slice().buffer;
}

async function writeDocument(id: string, path: string[], bytes: Uint8Array): Promise<void> {
  if (!(await get<StoredEntry>(entryKey(id, path)))) {
    throw "not-found";
  }
  await put(entryKey(id, path), { kind: "document", path, bytes, modified: now() }, path[path.length - 1] ?? "");
}

function bytesArg(args?: Record<string, unknown>): Uint8Array {
  const bytes = args?.bytes;
  return bytes instanceof Uint8Array ? bytes : new Uint8Array();
}

/** The marker's three share files of `id`, `null` when it is not shared. */
async function marker(id: string): Promise<{ share_json: number[]; keyring_log: number[]; list: number[] } | null> {
  const share = await get<Uint8Array>(`${id}|m|share.json`);
  if (!share) {
    return null;
  }
  const keyring = (await get<Uint8Array>(`${id}|m|keyring.log`)) ?? new Uint8Array();
  const list = (await get<Uint8Array>(`${id}|m|list.loro`)) ?? new Uint8Array();
  return { share_json: Array.from(share), keyring_log: Array.from(keyring), list: Array.from(list) };
}

/** A free name among the browser's workspaces: `wanted`, then "`wanted` 2", ... */
async function freeName(wanted: string): Promise<string> {
  const taken = new Set((await workspaces()).map((w) => w.name.toLowerCase()));
  let name = wanted.trim() || "Workspace";
  for (let number = 2; taken.has(name.toLowerCase()); number += 1) {
    name = `${wanted} ${number}`;
  }
  return name;
}

async function rows(): Promise<{ name: string; path: string; found: boolean }[]> {
  return (await workspaces()).map((w) => ({ name: w.name, path: w.id, found: true }));
}

/** The browser's stand-ins of the workspace and device-group host commands. */
export const browserWorkspaceCommands: Record<string, (args?: Record<string, unknown>) => unknown> = {
  recent_workspaces: () => rows(),
  remove_recent_workspace: () => rows(),
  pick_workspace: () => ({ kind: "cancelled" }),
  open_recent_workspace: async (args) => {
    const all = await workspaces();
    const chosen = all[Number(args?.index ?? -1)];
    if (!chosen) {
      return { kind: "refused", code: "not-found" };
    }
    openId = chosen.id;
    return { kind: "opened", name: chosen.name, entries: await scan(chosen.id) };
  },
  close_workspace: () => {
    openId = null;
    openDocument = null;
  },
  scan_open_workspace: (args) => scan(workspaceOf(args)),
  apply_workspace_op: (args) =>
    applyOp(workspaceOf(args), args?.op as { kind: string }, args?.bytes instanceof Uint8Array ? args.bytes : undefined),
  read_workspace_document: (args) => readDocument(workspaceOf(args), args?.path as string[]),
  confirm_workspace_document_opened: (args) => {
    openDocument = (args?.path as string[] | undefined) ?? null;
  },
  // The row shows no thumbnail in the browser: an empty answer is "readable, none".
  probe_workspace_document: async (args) => {
    await readDocument(workspaceOf(args), args?.path as string[]);
    return new ArrayBuffer(0);
  },
  workspace_marker_share: (args) => marker(workspaceOf(args)),
  write_workspace_marker_share: async (args) => {
    const id = workspaceOf(args);
    const given = args?.marker as { share_json: number[]; keyring_log: number[]; list: number[] };
    await put(`${id}|m|share.json`, Uint8Array.from(given.share_json));
    await put(`${id}|m|keyring.log`, Uint8Array.from(given.keyring_log));
    await put(`${id}|m|list.loro`, Uint8Array.from(given.list));
  },
  clear_workspace_marker_share: async (args) => {
    const id = workspaceOf(args);
    for (const name of ["share.json", "keyring.log", "list.loro"]) {
      await remove(`${id}|m|${name}`);
    }
  },
  // A browser workspace is created with its room's id: Share never gives it a new one.
  write_workspace_id: () => undefined,
  replace_workspace_document: (args) => writeDocument(workspaceOf(args), args?.path as string[], bytesArg(args)),
  write_workspace_index: (args) => put(`${workspaceOf(args)}|i`, args?.documents ?? []),
  scan_shared_workspace: async (args) => {
    const id = workspaceOf(args);
    const entries = await below<StoredEntry>(`${id}|e|`);
    const documents = entries
      .filter(([, entry]) => entry.kind === "document")
      .map(([, entry]) => {
        const members = JSON.parse(container_share_members(entry.bytes ?? new Uint8Array())) as {
          share: string | null;
          keyring: boolean;
        };
        return { path: entry.path, share: members.share, keyring: members.keyring };
      });
    return {
      entries: await scan(id),
      folders: entries.filter(([, entry]) => entry.kind === "folder").map(([, entry]) => entry.path),
      documents,
      known: (await get<{ document: string; path: string[] }[]>(`${id}|i`)) ?? [],
    };
  },
  keep_workspace_document: async (args) => {
    const id = workspaceOf(args);
    const path = args?.path as string[];
    const bytes = await readDocument(id, path);
    await put(`${id}|d|${String(args?.document)}`, new Uint8Array(bytes));
    if (args?.trash) {
      await remove(entryKey(id, path));
    }
  },
  restore_workspace_document: async (args) => {
    const id = workspaceOf(args);
    const kept = await get<Uint8Array>(`${id}|d|${String(args?.document)}`);
    if (!kept) {
      throw "not-found";
    }
    await applyOp(id, { kind: "create-document", path: args?.path as string[] }, kept);
  },
  read_deleted_document: async (args) => {
    const kept = await get<Uint8Array>(`${workspaceOf(args)}|d|${String(args?.document)}`);
    if (!kept) {
      throw "not-found";
    }
    return kept.slice().buffer;
  },
  write_deleted_document: (args) => put(`${workspaceOf(args)}|d|${String(args?.document)}`, bytesArg(args)),
  register_synced_workspace: async (args) => {
    const id = String(args?.id ?? "");
    if (!(await get<StoredWorkspace>(`w|${id}`))) {
      throw "not-found";
    }
    return id;
  },
  create_synced_workspace: async (args) => {
    const id = String(args?.id ?? "");
    if (!(await get<StoredWorkspace>(`w|${id}`))) {
      await put(`w|${id}`, { id, name: await freeName(String(args?.name ?? "")) } satisfies StoredWorkspace);
    }
    return { kind: "picked", path: id };
  },
  // A browser holds no copies outside its store (criterion 14 is the desktop's).
  find_workspace_copy: () => null,
  workspace_summary: async (args) => {
    const id = String(args?.path ?? "");
    const entries = await scan(id);
    const shared = await marker(id);
    return {
      id,
      documents: entries.filter((e) => e.kind === "document").length,
      shared: shared !== null,
      keyring: shared ? shared.keyring_log.map((b) => b.toString(16).padStart(2, "0")).join("") : "",
    };
  },
  read_device_group: async () => (await get<unknown>("group")) ?? null,
  write_device_group: (args) => put("group", args?.group),
  remove_device_group: () => remove("group"),
  // No folder: workspaces arrive in this browser's storage (ux-notes 9).
  prepare_synced_folder: () => ({ kind: "picked", path: "browser" }),
  pick_synced_folder: () => ({ kind: "picked", path: "browser" }),
};

/** Save of a workspace document open in the browser writes it into the store; `null` when no
 * workspace document is open (the browser then downloads, as before). */
export async function saveOpenDocument(bytes: Uint8Array): Promise<{ kind: "saved"; file_name: string; path: string } | null> {
  if (!openId || !openDocument) {
    return null;
  }
  await writeDocument(openId, openDocument, bytes);
  return { kind: "saved", file_name: openDocument[openDocument.length - 1] ?? "", path: openDocument.join("/") };
}

/** The project is no longer the open workspace document (New, Open, a reset). */
export function forgetOpenDocument(): void {
  openDocument = null;
}

/** The names of the browser's workspaces, for the storage line (criterion 37). */
export async function workspaceNames(): Promise<string[]> {
  return (await workspaces()).map((w) => w.name);
}
