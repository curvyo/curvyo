// A shared workspace in the page (`specs/0094-workspace` criteria 25 to 33, ADR 0036 decisions 8
// and 9): Share workspace, joining by a `2.` link into a chosen folder, reconnecting a shared
// workspace when its folder opens, and the files the session hands out (the join's documents and
// the marker). Every rule and word is Rust's; this carries bytes between the session and the host.

import { useCallback, useEffect, useRef, useState } from "react";

import type { CollaborationApi } from "@/hooks/useCollaboration";
import type { EditorSession } from "@/hooks/useEditorSession";
import type { ProjectApi } from "@/hooks/useProject";
import type { ScannedEntryData, WorkspaceApi } from "@/hooks/useWorkspace";
import { useSharedTree } from "@/hooks/useSharedTree";
import { useWorkspaceWriter } from "@/hooks/useWorkspaceWriter";
import { mintPeerId } from "@/lib/editorSession";
import { openOldRoom, pushMoved } from "@/lib/movedPush";
import type { SharedScan } from "@/lib/sharedTree";
import type { WorkspaceAnswer } from "@/lib/workspaceRows";
import { invoke } from "@/platform/host";
import { answerBytes } from "@/lib/binaryArgs";
import {
  WasmWorkspaceShare,
  invite_link_server,
  invite_link_workspace,
  join_folder_refusal_text,
  normalize_server_address,
  workspace_removed_text,
  workspace_share_take_in,
  type WasmSession,
} from "@/wasm-bindings/curvyo_editor_wasm.js";

/** How often the marker's files are written at most (ms). */
const MARKER_MS = 2000;
/** How often the session's hand-outs are collected (ms). */
const PUMP_MS = 250;
/** How long Share waits for the server's first answer (ms). */
const SHARE_WAIT_MS = 15_000;

/** Where Share workspace stands. */
export type SharePhase =
  | { kind: "idle" }
  | { kind: "uploading"; done: number; total: number }
  | { kind: "waiting" }
  | { kind: "failed"; text: string };

/** The folder chosen for a joined workspace, or why it was refused. */
export interface JoinFolder {
  path: string;
  refusal: string;
}

/** The documents shared on their own that Share takes in or leaves out (criteria 42, 49). */
export interface ShareCheck {
  takenIn: string[];
  outside: string[];
}

const NO_CHECK: ShareCheck = { takenIn: [], outside: [] };

export interface WorkspaceSyncApi {
  share: SharePhase;
  /** What the last `checkShare` found, for the Share confirm. */
  shareCheck: ShareCheck;
  /** Finds the documents shared on their own before the Share confirm names them. */
  checkShare: () => Promise<void>;
  /** Names of documents Share left out (shared on their own, unreadable). */
  notAdded: string[];
  /** Shares the open workspace (after the unsaved-changes check). */
  shareWorkspace: () => void;
  joinFolder: JoinFolder;
  /** Choose folder… of a workspace join. */
  chooseJoinFolder: () => void;
  /** Joins the workspace of a `2.` link into the chosen folder. */
  joinWorkspace: (link: string) => void;
  /** The open workspace is a shared one (its marker names a room, or the session carries its
   * link): the Share tab shows no `0092` Document block for it. */
  sharedOpen: boolean;
}

function random(length: number): Uint8Array {
  const bytes = new Uint8Array(length);
  crypto.getRandomValues(bytes);
  return bytes;
}

function toHex(bytes: Uint8Array): string {
  return Array.from(bytes, (b) => b.toString(16).padStart(2, "0")).join("");
}

function fromHex(hex: string): Uint8Array {
  const bytes = new Uint8Array(hex.length / 2);
  for (let i = 0; i < bytes.length; i += 1) {
    bytes[i] = Number.parseInt(hex.slice(2 * i, 2 * i + 2), 16);
  }
  return bytes;
}

/** The last component of a path the host shows. */
function folderName(path: string): string {
  const parts = path.split(/[\\/]/).filter((part) => part !== "");
  return parts[parts.length - 1] ?? path;
}

/** A document's name as rows show it: its file name without `.curvyo`. */
function documentName(path: readonly string[]): string {
  return (path[path.length - 1] ?? "").replace(/\.curvyo$/i, "");
}

/** Whether the own room of the document file `bytes` can be reached for its `Moved` record. */
async function probeOldRoom(bytes: Uint8Array, collab: CollaborationApi): Promise<boolean> {
  const room = await openOldRoom(bytes, collab.name, (id) => collab.deviceKeys(id, () => undefined));
  room?.close();
  return room !== null;
}

/** The workspace id (hex) named by a marker's `share.json`. */
function markerRoom(shareJson: number[]): string {
  try {
    const parsed = JSON.parse(new TextDecoder().decode(new Uint8Array(shareJson))) as { room?: string };
    return parsed.room ?? "";
  } catch {
    return "";
  }
}

interface MarkerPayload {
  share_json: number[];
  keyring_log: number[];
  list: number[];
}

/** The connection state the session's Share view reports. */
function shareState(session: WasmSession): string {
  try {
    return (JSON.parse(session.share_view_json()) as { state?: string }).state ?? "";
  } catch {
    return "";
  }
}

export function useWorkspaceSync(
  editor: EditorSession,
  project: ProjectApi,
  collab: CollaborationApi,
  workspace: WorkspaceApi,
): WorkspaceSyncApi {
  const { getSession } = editor;
  const refs = useRef({ project, collab, workspace });
  useEffect(() => {
    refs.current = { project, collab, workspace };
  });
  const [share, setShare] = useState<SharePhase>({ kind: "idle" });
  const [notAdded, setNotAdded] = useState<string[]>([]);
  const [shareCheck, setShareCheck] = useState<ShareCheck>(NO_CHECK);
  const [joinFolder, setJoinFolder] = useState<JoinFolder>({ path: "", refusal: "" });
  // The workspace (by name) found shared; another or a closed one is not.
  const [sharedName, setSharedName] = useState("");
  /** While Share runs, the marker is not written: the files are committed only on success. */
  const sharingRef = useRef(false);
  /** The workspace id of a join, until the chosen folder became the workspace. */
  const joiningRef = useRef<string | null>(null);
  const joinedIndexRef = useRef(new Map<string, string[]>());
  const writingRef = useRef<Promise<void>>(Promise.resolve());
  const lastMarkerRef = useRef(0);
  /** The session whose removal from the workspace was handled. */
  const removedRef = useRef<WasmSession | null>(null);
  // The retries after a passphrase reach the latest callbacks.
  const resumeRef = useRef<() => Promise<void>>(async () => undefined);
  const runShareRef = useRef<() => Promise<void>>(async () => undefined);
  const startJoinRef = useRef<(link: string, folder: string) => Promise<void>>(async () => undefined);

  /** Writes `index.json` for the documents named. */
  const writeIndex = useCallback(async (documents: Map<string, string[]>) => {
    await invoke("write_workspace_index", {
      documents: [...documents].map(([document, path]) => ({ document, path })),
    }).catch(() => undefined);
  }, []);

  /** A shared workspace whose folder opened reconnects from its marker (criterion 33). */
  const resume = useCallback(async () => {
    const session = getSession();
    if (!session || session.has_workspace_link() || joiningRef.current) {
      return;
    }
    const marker = await invoke<MarkerPayload | null>("workspace_marker_share").catch(() => null);
    if (!marker) {
      return;
    }
    setSharedName(refs.current.workspace.name);
    const room = markerRoom(marker.share_json);
    const keys = await refs.current.collab.deviceKeys(room, () => void resumeRef.current());
    const current = getSession();
    if (!keys || !current || current.has_workspace_link()) {
      return;
    }
    current.resume_workspace(
      new Uint8Array(marker.share_json),
      new Uint8Array(marker.keyring_log),
      new Uint8Array(marker.list),
      refs.current.workspace.name,
      refs.current.collab.name,
      JSON.stringify(keys.keys),
      fromHex(keys.identity),
      random(32),
      random(32),
      Date.now(),
    );
    refs.current.collab.drive(current);
  }, [getSession]);


  const { pump: pumpWriter, stop: stopWriter } = useWorkspaceWriter(getSession, collab.drive, writingRef);
  useSharedTree(
    getSession,
    project,
    workspace,
    writingRef,
    (room) => refs.current.collab.deviceKeys(room, () => undefined),
    collab.name,
  );

  const opened = workspace.open ? workspace.name : "";
  useEffect(() => {
    if (opened !== "") {
      void resume();
      return undefined;
    }
    // A closed workspace's waiting records stay in its room: the worker ends.
    stopWriter();
    return undefined;
  }, [opened, resume, stopWriter]);

  /** The join's folders and documents, written in order (criterion 28). */
  const writeJoined = useCallback(
    (writes: { kind: string; path: string[]; document: string; bytes: Uint8Array }[]) => {
      writingRef.current = writingRef.current.then(async () => {
        const id = joiningRef.current;
        if (id) {
          const answer = await invoke<WorkspaceAnswer>("start_joined_workspace", { id }).catch(
            (): WorkspaceAnswer => ({ kind: "refused", code: "not-found" }),
          );
          joiningRef.current = null;
          if (answer.kind !== "opened") {
            return;
          }
          refs.current.workspace.adoptOpened(answer);
          setJoinFolder({ path: "", refusal: "" });
        }
        for (const write of writes) {
          if (write.kind === "deleted") {
            // A deleted document's state from the joined snapshot: its kept copy (criterion 40).
            await invoke("write_deleted_document", { document: write.document }, write.bytes).catch(() => undefined);
            getSession()?.workspace_document_written(write.document);
            continue;
          }
          const op =
            write.kind === "folder"
              ? { kind: "create-folder", path: write.path }
              : { kind: "create-document", path: write.path };
          // A folder or file that is already there was written before: it counts as written.
          await (write.kind === "folder"
            ? invoke("apply_workspace_op", { op })
            : invoke("apply_workspace_op", { op }, write.bytes)
          ).catch(() => undefined);
          if (write.kind === "document") {
            joinedIndexRef.current.set(write.document, write.path);
            getSession()?.workspace_document_written(write.document);
          }
        }
        void refs.current.workspace.rescan();
      });
    },
    [getSession],
  );

  // The session's hand-outs: the join's files and the marker.
  useEffect(() => {
    const timer = window.setInterval(() => {
      const session = getSession();
      if (!session) {
        return;
      }
      if (session.workspace_removed() && removedRef.current !== session) {
        // Criterion 44: the workspace is a local one now; every document stays.
        removedRef.current = session;
        setSharedName("");
        void invoke("clear_workspace_marker_share").catch(() => undefined);
        refs.current.workspace.showRemoved(workspace_removed_text());
      }
      if (!session.has_workspace_link()) {
        // A join that was cancelled or refused leaves nothing to wait for.
        joiningRef.current = null;
        return;
      }
      if (!session.workspace_removed() && refs.current.workspace.open) {
        setSharedName(refs.current.workspace.name);
      }
      const writes: { kind: string; path: string[]; document: string; bytes: Uint8Array }[] = [];
      for (let write = session.next_workspace_write(); write; write = session.next_workspace_write()) {
        writes.push({
          kind: write.kind,
          path: JSON.parse(write.path) as string[],
          document: write.document,
          bytes: write.bytes,
        });
        write.free();
      }
      if (writes.length > 0) {
        writeJoined(writes);
      }
      pumpWriter(session);
      const now = Date.now();
      if (sharingRef.current || joiningRef.current || now - lastMarkerRef.current < MARKER_MS) {
        return;
      }
      if (!refs.current.workspace.open) {
        return;
      }
      const marker = session.take_workspace_marker();
      if (!marker) {
        return;
      }
      lastMarkerRef.current = now;
      const payload = {
        share_json: Array.from(marker.share_json),
        keyring_log: Array.from(marker.keyring_log),
        list: Array.from(marker.list),
      };
      marker.free();
      writingRef.current = writingRef.current.then(async () => {
        await invoke("write_workspace_marker_share", { marker: payload }).catch(() => undefined);
        const joined = joinedIndexRef.current;
        if (joined.size > 0) {
          joinedIndexRef.current = new Map();
          await writeIndex(joined);
        }
      });
    }, PUMP_MS);
    return () => window.clearInterval(timer);
  }, [getSession, pumpWriter, writeIndex, writeJoined]);

  /** Waits for the shared workspace's first connection; "" once connected, else the reason. */
  const connected = useCallback(
    () =>
      new Promise<string>((resolve) => {
        const started = Date.now();
        const timer = window.setInterval(() => {
          const session = getSession();
          const state = session ? shareState(session) : "";
          if (state === "connected") {
            window.clearInterval(timer);
            resolve("");
          } else if (state === "not-connected" || Date.now() - started > SHARE_WAIT_MS) {
            window.clearInterval(timer);
            session?.leave_sync();
            if (session) {
              refs.current.collab.drive(session);
            }
            resolve("cannot reach the server.");
          }
        }, 100);
      }),
    [getSession],
  );

  /** Share workspace (criterion 25): every document read and prepared in memory, the room made,
   * the snapshot uploaded; only then the files, the new id and the marker are written. */
  const runShare = useCallback(async () => {
    const { workspace: ws, collab } = refs.current;
    const server = normalize_server_address(collab.server);
    const reopen = ws.openPath;
    const entries: ScannedEntryData[] = ws.scanned();
    const documents = entries.filter((e) => e.kind === "document");
    sharingRef.current = true;
    try {
      await refs.current.project.resetToEmpty();
      // The device's identity first: a document shared on this device alone is taken in.
      const keys = await collab.deviceKeys("", () => void runShareRef.current());
      if (!keys) {
        setShare({ kind: "idle" });
        return;
      }
      const id = random(16);
      const builder = new WasmWorkspaceShare(id, mintPeerId(), Date.now(), random(16));
      builder.set_identity(fromHex(keys.identity));
      const ids = new Map<string, string[]>();
      const originals = new Map<string, Uint8Array>();
      /** Documents to take in whose own room cannot be reached: they stay out (Proposal). */
      const unreachable: string[] = [];
      try {
        for (const entry of entries.filter((e) => e.kind === "folder")) {
          builder.add_folder(JSON.stringify(entry.path));
        }
        let done = 0;
        for (const entry of documents) {
          setShare({ kind: "uploading", done, total: documents.length });
          const bytes = await invoke<ArrayBuffer>("read_workspace_document", { path: entry.path }).then(answerBytes).catch(() => null);
          if (bytes) {
            originals.set(JSON.stringify(entry.path), bytes);
          }
          const document = random(16);
          const reachable =
            !bytes ||
            workspace_share_take_in(bytes, fromHex(keys.identity)) !== "allowed" ||
            (await probeOldRoom(bytes, collab));
          if (!reachable) {
            unreachable.push(documentName(entry.path));
          }
          const refusal = !reachable
            ? "unreachable"
            : bytes
              ? builder.add_document(JSON.stringify(entry.path), bytes, document)
              : "unreadable";
          if (refusal === "") {
            ids.set(toHex(document), entry.path);
          }
          done += 1;
        }
        setNotAdded([...(JSON.parse(builder.not_added_json()) as string[]), ...unreachable]);
        builder.prepare(random(16));
        const session = getSession();
        if (!session) {
          setShare({ kind: "idle" });
          return;
        }
        setShare({ kind: "waiting" });
        const error = session.share_workspace(
          server,
          collab.name,
          ws.name,
          builder,
          fromHex(keys.identity),
          random(32),
          random(32),
          Date.now(),
        );
        collab.drive(session);
        const failure = error || (await connected());
        if (failure) {
          setShare({ kind: "failed", text: `Could not share: ${failure}` });
          return;
        }
        for (let index = 0; index < builder.file_count(); index += 1) {
          await invoke(
            "replace_workspace_document",
            { path: JSON.parse(builder.file_path(index)) as string[] },
            builder.file_bytes(index),
          ).catch(() => undefined);
        }
        // ADR 0036 decision 10: each document taken in ends its own room with `Moved`.
        for (const taken of JSON.parse(builder.taken_in_json()) as { path: string[]; document: string }[]) {
          const original = originals.get(JSON.stringify(taken.path));
          if (original) {
            await pushMoved(original, toHex(id), taken.document, collab.name, (room) =>
              collab.deviceKeys(room, () => undefined),
            );
          }
        }
        await invoke("write_workspace_id", { id: toHex(id) }).catch(() => undefined);
        await writeIndex(ids);
      } finally {
        builder.free();
      }
      sharingRef.current = false;
      lastMarkerRef.current = 0;
      setShare({ kind: "idle" });
      void ws.rescan();
      if (reopen) {
        await ws.reopen(reopen);
      }
    } finally {
      sharingRef.current = false;
    }
  }, [connected, getSession, writeIndex]);

  const checkShare = useCallback(async () => {
    const scan = await invoke<SharedScan>("scan_shared_workspace").catch(() => null);
    const candidates = scan?.documents.filter((d) => d.keyring) ?? [];
    if (candidates.length === 0) {
      setShareCheck(NO_CHECK);
      return;
    }
    const keys = await refs.current.collab.deviceKeys("", () => undefined);
    if (!keys) {
      setShareCheck(NO_CHECK);
      return;
    }
    const found: ShareCheck = { takenIn: [], outside: [] };
    for (const candidate of candidates) {
      const bytes = await invoke<ArrayBuffer>("read_workspace_document", { path: candidate.path })
        .then(answerBytes)
        .catch(() => null);
      const kind = bytes ? workspace_share_take_in(bytes, fromHex(keys.identity)) : "plain";
      if (kind === "allowed") {
        found.takenIn.push(documentName(candidate.path));
      } else if (kind === "refused") {
        found.outside.push(documentName(candidate.path));
      }
    }
    setShareCheck(found);
  }, []);

  const shareWorkspace = useCallback(() => {
    setShare({ kind: "idle" });
    setNotAdded([]);
    refs.current.project.replaceThen("share-workspace", () => runShareRef.current());
  }, []);

  const chooseJoinFolder = useCallback(() => {
    void invoke<{ kind: string; path?: string; code?: string }>("pick_join_folder")
      .then((answer) => {
        if (answer.kind === "picked") {
          setJoinFolder({ path: answer.path ?? "", refusal: "" });
        } else if (answer.kind === "refused") {
          setJoinFolder({ path: "", refusal: join_folder_refusal_text(answer.code ?? "") });
        }
      })
      .catch(() => setJoinFolder({ path: "", refusal: join_folder_refusal_text("not-writable") }));
  }, []);

  /** Joins a workspace (criterion 27): the open workspace and document close after the check;
   * the chosen folder becomes the workspace once the first file arrives. */
  const startJoin = useCallback(
    async (link: string, folder: string) => {
      const { collab } = refs.current;
      const keys = await collab.deviceKeys("", () => void startJoinRef.current(link, folder));
      const session = getSession();
      if (!keys || !session) {
        return;
      }
      joiningRef.current = invite_link_workspace(link);
      // A joiner without a server address takes the link's, so the tab shows where it syncs.
      if (collab.server === "") {
        collab.setServer(invite_link_server(link));
      }
      joinedIndexRef.current = new Map();
      const refusal = session.join_workspace(
        link,
        collab.name,
        folderName(folder),
        fromHex(keys.identity),
        random(32),
        random(32),
        Date.now(),
      );
      if (refusal) {
        joiningRef.current = null;
        setJoinFolder((current) => ({ ...current, refusal }));
      }
      collab.drive(session);
    },
    [getSession],
  );
  useEffect(() => {
    resumeRef.current = resume;
    runShareRef.current = runShare;
    startJoinRef.current = startJoin;
  });

  const joinWorkspace = useCallback(
    (link: string) => {
      const folder = joinFolder.path;
      if (folder === "") {
        return;
      }
      refs.current.project.replaceThen("join", async () => {
        const ws = refs.current.workspace;
        if (ws.open) {
          await ws.closeNow();
        } else {
          await refs.current.project.resetToEmpty();
        }
        await startJoinRef.current(link, folder);
      });
    },
    [joinFolder.path],
  );

  const sharedOpen = opened !== "" && sharedName === opened;
  return {
    share,
    shareCheck,
    checkShare,
    notAdded,
    shareWorkspace,
    joinFolder,
    chooseJoinFolder,
    joinWorkspace,
    sharedOpen,
  };
}
