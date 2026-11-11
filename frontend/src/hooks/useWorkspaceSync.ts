// A shared workspace in the page (`specs/0095-workspace` criteria 25 to 33, ADR 0037 decisions 8
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
import { claimOpenRoom, endedBySwitchOff, openRoomNow, releaseOpenRoom } from "@/lib/backgroundWorkspace";
import { parseEnded } from "@/lib/endedRecord";
import { STALL_MS } from "@/lib/stallWatch";
import { adoptOpenDocument, commitShare, prepareShare } from "@/lib/workspaceShare";
import type { WorkspaceAnswer } from "@/lib/workspaceRows";
import { invoke } from "@/platform/host";
import {
  invite_link_server,
  invite_link_workspace,
  join_folder_refusal_text,
  normalize_server_address,
  workspace_removed_text,
  devices_no_longer_synced_notice,
  devices_text,
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


export interface WorkspaceSyncApi {
  share: SharePhase;
  /** Names of documents Share left out (unreadable, or a name the list refuses). */
  notAdded: string[];
  /** Shares the open workspace (after the unsaved-changes check). */
  shareWorkspace: () => void;
  /** Cancels a running Share: nothing is written, the open document comes back. */
  cancelShare: () => void;
  joinFolder: JoinFolder;
  /** Choose folder… of a workspace join. */
  chooseJoinFolder: () => void;
  /** Joins the workspace of a `2.` link into the chosen folder. */
  joinWorkspace: (link: string) => void;
  /** The open workspace is a shared one (its marker names a room, or the session carries its
   * link). */
  sharedOpen: boolean;
}

function random(length: number): Uint8Array {
  const bytes = new Uint8Array(length);
  crypto.getRandomValues(bytes);
  return bytes;
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
  const { getSession, showHistoryNotice } = editor;
  const refs = useRef({ project, collab, workspace });
  useEffect(() => {
    refs.current = { project, collab, workspace };
  });
  const [share, setShare] = useState<SharePhase>({ kind: "idle" });
  const [notAdded, setNotAdded] = useState<string[]>([]);
  const [joinFolder, setJoinFolder] = useState<JoinFolder>({ path: "", refusal: "" });
  // The workspace (by name) found shared; another or a closed one is not.
  const [sharedName, setSharedName] = useState("");
  /** While Share runs, the marker is not written: the files are committed only on success. */
  const sharingRef = useRef(false);
  /** The Share run that may go on; Cancel and the watchdog count it up. */
  const shareRunRef = useRef(0);
  /** The workspace id of a join, until the chosen folder became the workspace. */
  const joiningRef = useRef<string | null>(null);
  const joinedIndexRef = useRef(new Map<string, string[]>());
  const writingRef = useRef<Promise<void>>(Promise.resolve());
  const lastMarkerRef = useRef(0);
  /** The session whose removal from the workspace was handled. */
  const removedRef = useRef<WasmSession | null>(null);
  // The retries after a passphrase reach the latest callbacks.
  const resumeRef = useRef<() => Promise<void>>(async () => undefined);
  const runShareRef = useRef<(run: number) => Promise<void>>(async () => undefined);
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
    // A synced workspace held in the background hands over first: its marker is written, its
    // socket closed (`specs/0096-own-devices-sync`, ADR 0040 decision 7).
    await claimOpenRoom(room);
    const fresh = await invoke<MarkerPayload | null>("workspace_marker_share").catch(() => null);
    if (fresh) {
      marker.share_json = fresh.share_json;
      marker.keyring_log = fresh.keyring_log;
      marker.list = fresh.list;
    }
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


  const writerNotice = useCallback((text: string) => showHistoryNotice(text, 8000, "canvas", "alert"), [showHistoryNotice]);
  const { pump: pumpWriter, stop: stopWriter } = useWorkspaceWriter(getSession, collab.drive, writingRef, writerNotice);
  useSharedTree(getSession, project, workspace, writingRef);

  const opened = workspace.open ? workspace.name : "";
  useEffect(() => {
    if (opened !== "") {
      void resume();
      return undefined;
    }
    // A closed workspace's waiting records stay in its room: the worker ends.
    stopWriter();
    releaseOpenRoom();
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
        const id = openRoomNow();
        // A switch-off drops the share; a removal or leave keeps the marker's ended record and
        // every document's record for the way back (`specs/0097` criteria 15 and 20).
        const ended = Array.from(session.workspace_ended_json());
        const switchedOff =
          id !== "" && endedBySwitchOff(id, session.share_removed_by_hex(), parseEnded(ended)?.left ?? false);
        void (switchedOff || ended.length === 0
          ? invoke("clear_workspace_marker_share")
          : invoke("end_workspace_marker_share", { ended_json: ended })
        ).catch(() => undefined);
        const left = parseEnded(ended)?.left ?? false;
        if (switchedOff) {
          // Switched off for my devices (0096 ux-notes 6.4): a notice, the copy stays.
          showHistoryNotice(devices_no_longer_synced_notice(refs.current.workspace.name), 5000, "canvas", "status");
        } else if (!left) {
          // A leave is told by the session in its own words (`specs/0097` ux-notes 5.2).
          refs.current.workspace.showRemoved(workspace_removed_text());
        }
      }
      if (!session.workspace_removed() && removedRef.current === session && session.has_workspace_link()) {
        // Sync again resumed it (`specs/0097` criterion 21): a later end is handled again.
        removedRef.current = null;
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
  }, [getSession, pumpWriter, showHistoryNotice, writeIndex, writeJoined]);

  /** Waits for the shared workspace's first connection; "" once connected, else the reason
   * ("cancelled" once Share was cancelled; the caller leaves the room then). */
  const connected = useCallback(
    (run: number) =>
      new Promise<string>((resolve) => {
        const started = Date.now();
        const timer = window.setInterval(() => {
          const session = getSession();
          const state = session ? shareState(session) : "";
          if (shareRunRef.current !== run) {
            window.clearInterval(timer);
            resolve("cancelled");
          } else if (state === "connected") {
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
   * the snapshot uploaded; only then the files, the new id and the marker are written. It runs on
   * the open document's session and keeps it: the open document takes its record in place, so
   * nothing closes or reopens and the Share tab stays (`specs/0105` criterion 20). */
  const runShare = useCallback(async (run: number) => {
    const { workspace: ws, collab } = refs.current;
    const server = normalize_server_address(collab.server);
    const entries: ScannedEntryData[] = ws.scanned();
    /** Cancelled meanwhile (Cancel, the switch turned off, the watchdog): nothing is written. */
    const cancelled = () => shareRunRef.current !== run;
    /** A run that did not finish leaves no link of its attempt on the session. */
    const abandon = (session: WasmSession) => {
      session.leave_sync();
      collab.drive(session);
      session.drop_ended_link();
    };
    sharingRef.current = true;
    try {
      // The device's identity first: the genesis names it.
      const keys = await collab.deviceKeys("", () => void runShareRef.current(run));
      if (cancelled()) {
        return;
      }
      if (!keys) {
        setShare({ kind: "idle" });
        return;
      }
      const deps = {
        progress: (done: number, total: number) => {
          if (shareRunRef.current === run) {
            setShare({ kind: "uploading", done, total });
          }
        },
      };
      const prepared = await prepareShare(entries, deps);
      const { builder } = prepared;
      try {
        if (cancelled()) {
          return;
        }
        setNotAdded(prepared.notAdded);
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
        const failure = error || (await connected(run));
        if (failure === "cancelled" || (failure === "" && shareRunRef.current !== run)) {
          // The room may exist on the server now; this device leaves it and nothing is written.
          abandon(session);
          return;
        }
        if (failure) {
          abandon(session);
          setShare({ kind: "failed", text: `Could not share: ${failure}` });
          return;
        }
        // The open document takes its record first, so any Save from now on writes it.
        adoptOpenDocument(getSession(), builder, ws.openPathNow());
        await commitShare(prepared);
      } finally {
        builder.free();
      }
      lastMarkerRef.current = 0;
      setShare({ kind: "idle" });
      void ws.rescan();
    } finally {
      sharingRef.current = false;
    }
  }, [connected, getSession]);

  const shareWorkspace = useCallback(() => {
    shareRunRef.current += 1;
    setShare({ kind: "idle" });
    setNotAdded([]);
    void runShareRef.current(shareRunRef.current);
  }, []);

  /** Cancels a Share that runs: back to idle at once; the run stops at its next step and leaves a
   * room it made (customer feedback on PR #281). */
  const cancelShare = useCallback(() => {
    shareRunRef.current += 1;
    setShare({ kind: "idle" });
  }, []);

  // A Share that made no progress for `STALL_MS` fails with its reason and can be tried again.
  const shareKey = share.kind === "uploading" ? `uploading:${share.done}` : share.kind;
  useEffect(() => {
    if (share.kind !== "uploading" && share.kind !== "waiting") {
      return undefined;
    }
    const timer = window.setTimeout(() => {
      shareRunRef.current += 1;
      setShare({ kind: "failed", text: devices_text("share-stalled") });
    }, STALL_MS);
    return () => window.clearTimeout(timer);
  }, [share.kind, shareKey]);

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

  /** Joins on the session that is open: an open workspace closes (its link ends), the open
   * document stays, so the Share tab stays (`specs/0105` criterion 20). */
  const joinWorkspace = useCallback(
    (link: string) => {
      const folder = joinFolder.path;
      if (folder === "") {
        return;
      }
      void (async () => {
        const ws = refs.current.workspace;
        if (ws.open) {
          await ws.closeNow();
        }
        await startJoinRef.current(link, folder);
      })();
    },
    [joinFolder.path],
  );

  const sharedOpen = opened !== "" && sharedName === opened;
  return {
    share,
    notAdded,
    shareWorkspace,
    cancelShare,
    joinFolder,
    chooseJoinFolder,
    joinWorkspace,
    sharedOpen,
  };
}
