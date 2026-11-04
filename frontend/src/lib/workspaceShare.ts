// Share workspace's file work (`specs/0095-workspace` criterion 25, ADR 0037 decision 9), for the
// open workspace and for a workspace that is not open (`specs/0096-own-devices-sync` criterion 8,
// Sync these): every document read and prepared in memory before the room exists, and after the
// room took the snapshot the files with their new ids, the `Moved` records, the new id and the
// index. `workspace` names a workspace that is not open by its id; absent, the open one.

import { answerBytes } from "@/lib/binaryArgs";
import { mintPeerId } from "@/lib/editorSession";
import { type RoomKeys, openOldRoom, pushMoved } from "@/lib/movedPush";
import { invoke } from "@/platform/host";
import { WasmWorkspaceShare, workspace_share_take_in } from "@/wasm-bindings/curvyo_editor_wasm.js";

/** One scanned entry of the workspace. */
export interface ShareEntry {
  path: string[];
  kind: "folder" | "document";
}

/** What Share needs of the page besides the files. */
export interface ShareDeps {
  label: string;
  keysFor: (room: string) => Promise<RoomKeys | null>;
  progress: (done: number, total: number) => void;
}

/** The prepared Share: the builder the session takes, and what the commit writes. */
export interface PreparedShare {
  builder: WasmWorkspaceShare;
  /** The new workspace id. */
  id: Uint8Array;
  ids: Map<string, string[]>;
  originals: Map<string, Uint8Array>;
  /** Documents left out: shared with others, unreadable, or their own room unreachable. */
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

function fromHex(hex: string): Uint8Array {
  const bytes = new Uint8Array(hex.length / 2);
  for (let i = 0; i < bytes.length; i += 1) {
    bytes[i] = Number.parseInt(hex.slice(2 * i, 2 * i + 2), 16);
  }
  return bytes;
}

/** A document's name as rows show it: its file name without `.curvyo`. */
function documentName(path: readonly string[]): string {
  return (path[path.length - 1] ?? "").replace(/\.curvyo$/i, "");
}

/** Reads and prepares every document of `entries` (criterion 25); the caller frees the builder. */
export async function prepareShare(
  entries: readonly ShareEntry[],
  identity: string,
  deps: ShareDeps,
  workspace?: string,
): Promise<PreparedShare> {
  const id = random(16);
  const builder = new WasmWorkspaceShare(id, mintPeerId(), Date.now(), random(16));
  builder.set_identity(fromHex(identity));
  const ids = new Map<string, string[]>();
  const originals = new Map<string, Uint8Array>();
  const unreachable: string[] = [];
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
    let reachable = true;
    if (bytes && workspace_share_take_in(bytes, fromHex(identity)) === "allowed") {
      // A document shared on this device alone is taken in once its own room is reachable.
      const room = await openOldRoom(bytes, deps.label, deps.keysFor);
      room?.close();
      reachable = room !== null;
    }
    if (!reachable) {
      unreachable.push(documentName(entry.path));
    }
    const refusal = !reachable ? "unreachable" : bytes ? builder.add_document(JSON.stringify(entry.path), bytes, document) : "unreadable";
    if (refusal === "") {
      ids.set(toHex(document), entry.path);
    }
    done += 1;
  }
  const notAdded = [...(JSON.parse(builder.not_added_json()) as string[]), ...unreachable];
  builder.prepare(random(16));
  return { builder, id, ids, originals, notAdded };
}

/** After the room took the snapshot: the files with their new ids, `Moved` for each document
 * taken in (ADR 0037 decision 10), the new workspace id and the index. */
export async function commitShare(prepared: PreparedShare, deps: ShareDeps, workspace?: string): Promise<void> {
  const { builder, id, ids, originals } = prepared;
  for (let index = 0; index < builder.file_count(); index += 1) {
    await invoke(
      "replace_workspace_document",
      { path: JSON.parse(builder.file_path(index)) as string[], workspace },
      builder.file_bytes(index),
    ).catch(() => undefined);
  }
  for (const taken of JSON.parse(builder.taken_in_json()) as { path: string[]; document: string }[]) {
    const original = originals.get(JSON.stringify(taken.path));
    if (original) {
      await pushMoved(original, toHex(id), taken.document, deps.label, deps.keysFor);
    }
  }
  await invoke("write_workspace_id", { id: toHex(id), workspace }).catch(() => undefined);
  await invoke("write_workspace_index", {
    documents: [...ids].map(([document, path]) => ({ document, path })),
    workspace,
  }).catch(() => undefined);
}
