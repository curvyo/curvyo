// Share workspace's file work (`specs/0095-workspace` criterion 25, ADR 0037 decision 9), for the
// open workspace and for a workspace that is not open (`specs/0096-own-devices-sync` criterion 8,
// Sync these): every document read and prepared in memory before the room exists, and after the
// room took the snapshot the files with their new ids, the new id and the index. `workspace` names
// a workspace that is not open by its id; absent, the open one.

import { answerBytes } from "@/lib/binaryArgs";
import { mintPeerId } from "@/lib/editorSession";
import { invoke } from "@/platform/host";
import { WasmWorkspaceShare, workspace_restamp_record } from "@/wasm-bindings/curvyo_editor_wasm.js";

/** One scanned entry of the workspace. */
export interface ShareEntry {
  path: string[];
  kind: "folder" | "document";
}

/** What Share needs of the page besides the files. */
export interface ShareDeps {
  progress: (done: number, total: number) => void;
}

/** The prepared Share: the builder the session takes, and what the commit writes. */
export interface PreparedShare {
  builder: WasmWorkspaceShare;
  /** The new workspace id. */
  id: Uint8Array;
  ids: Map<string, string[]>;
  /** The bytes each document had when it was read, by its path as JSON. */
  originals: Map<string, Uint8Array>;
  /** Documents left out: unreadable, or a name the list refuses. */
  notAdded: string[];
}

function random(length: number): Uint8Array {
  const bytes = new Uint8Array(length);
  crypto.getRandomValues(bytes);
  return bytes;
}

export function toHex(bytes: Uint8Array): string {
  return Array.from(bytes, (b) => b.toString(16).padStart(2, "0")).join("");
}

function sameBytes(a: Uint8Array, b: Uint8Array): boolean {
  return a.length === b.length && a.every((byte, i) => byte === b[i]);
}

/** Reads and prepares every document of `entries` (criterion 25); the caller frees the builder. */
export async function prepareShare(
  entries: readonly ShareEntry[],
  deps: ShareDeps,
  workspace?: string,
): Promise<PreparedShare> {
  const id = random(16);
  const builder = new WasmWorkspaceShare(id, mintPeerId(), Date.now(), random(16));
  const ids = new Map<string, string[]>();
  const originals = new Map<string, Uint8Array>();
  for (const entry of entries.filter((e) => e.kind === "folder")) {
    builder.add_folder(JSON.stringify(entry.path));
  }
  const documents = entries.filter((e) => e.kind === "document");
  let done = 0;
  for (const entry of documents) {
    deps.progress(done, documents.length);
    const bytes = await invoke<ArrayBuffer>("read_workspace_document", { path: entry.path, workspace })
      .then(answerBytes)
      .catch(() => null);
    if (bytes) {
      originals.set(JSON.stringify(entry.path), bytes);
    }
    const document = random(16);
    const refusal = bytes ? builder.add_document(JSON.stringify(entry.path), bytes, document) : "unreadable";
    if (refusal === "") {
      ids.set(toHex(document), entry.path);
    }
    done += 1;
  }
  const notAdded = JSON.parse(builder.not_added_json()) as string[];
  builder.prepare(random(16));
  return { builder, id, ids, originals, notAdded };
}

/** After the room took the snapshot: the files with their new ids, the new workspace id and the
 * index. A file that changed on disk since it was read (a Save during the upload) is never put back
 * to the older bytes: it keeps what is on disk and only gets its record (`specs/0105`). */
export async function commitShare(prepared: PreparedShare, workspace?: string): Promise<void> {
  const { builder, id, ids, originals } = prepared;
  for (let index = 0; index < builder.file_count(); index += 1) {
    const key = builder.file_path(index);
    const path = JSON.parse(key) as string[];
    const shared = builder.file_bytes(index);
    const now = await invoke<ArrayBuffer>("read_workspace_document", { path, workspace })
      .then(answerBytes)
      .catch(() => null);
    const before = originals.get(key);
    const unchanged = now !== null && before !== undefined && sameBytes(now, before);
    const bytes = unchanged || now === null ? shared : workspace_restamp_record(now, shared);
    if (bytes.length > 0) {
      await invoke("replace_workspace_document", { path, workspace }, bytes).catch(() => undefined);
    }
  }
  await invoke("write_workspace_id", { id: toHex(id), workspace }).catch(() => undefined);
  await invoke("write_workspace_index", {
    documents: [...ids].map(([document, path]) => ({ document, path })),
    workspace,
  }).catch(() => undefined);
}

/** After the commit: the open document at `path` takes the record of the file Share wrote for it,
 * in its own session (`specs/0105` criterion 20). */
export function adoptOpenDocument(
  session: { adopt_workspace_document(file: Uint8Array): boolean } | null,
  builder: WasmWorkspaceShare,
  path: readonly string[] | null,
): void {
  if (!session || !path) {
    return;
  }
  const wanted = JSON.stringify(path);
  for (let index = 0; index < builder.file_count(); index += 1) {
    if (builder.file_path(index) === wanted) {
      session.adopt_workspace_document(builder.file_bytes(index));
      return;
    }
  }
}
