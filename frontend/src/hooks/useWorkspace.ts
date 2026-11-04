import { useCallback, useEffect, useRef, useState } from "react";

import type { EditorSession } from "@/hooks/useEditorSession";
import type { ProjectApi } from "@/hooks/useProject";
import { createSession } from "@/lib/editorSession";
import {
  ALERT_MS,
  type PlanAnswer,
  type ProbeData,
  type RecentWorkspaceData,
  SCAN_MS,
  STATUS_MS,
  UNDO_MOVE_MS,
  type WorkspaceAnswer,
  type WorkspaceOp,
  type WorkspaceRowData,
  type WorkspaceViewData,
  fileNameOf,
  isInside,
  movedPath,
  parsePlan,
  probeWith,
  parseWorkspaceView,
  pathKey,
  samePath,
  thumbnailKey,
} from "@/lib/workspaceRows";
import { invoke, isTauri } from "@/platform/host";
import { answerBytes } from "@/lib/binaryArgs";
import type { DeletedRow } from "@/lib/sharedTree";
import { endWorkspaceLink } from "@/lib/workspaceLink";
import {
  WasmWorkspace,
  workspace_copy,
  workspace_failure,
  workspace_open_refusal,
  workspace_unreadable,
} from "@/wasm-bindings/curvyo_editor_wasm.js";

/** What the action slot under the tree shows (ux-notes section 2, 3.2, 3.3). */
export type WorkspaceSlot =
  | { id: number; kind: "confirm"; text: string; path: string[]; op: WorkspaceOp; noTrash: string; status: string }
  | { id: number; kind: "undo"; text: string; inverse: WorkspaceOp }
  | { id: number; kind: "ask"; text: string; action: string; run: () => void }
  | { id: number; kind: "status" | "alert"; text: string };

/** A slot's content before it gets its id. */
type SlotBody<T = WorkspaceSlot> = T extends unknown ? Omit<T, "id"> : never;

/** The inline name field: a new entry in `folder`, or a rename of `path`. */
export type WorkspaceNaming =
  | { mode: "new-document" | "new-folder"; folder: string[]; error: string }
  | { mode: "rename"; path: string[]; initial: string; error: string };

/** What a document row shows in place of the thumbnail. */
export type ThumbnailState = { kind: "thumbnail"; url: string | null } | { kind: "unreadable"; text: string };

export interface WorkspaceApi {
  /** A workspace is open. */
  open: boolean;
  name: string;
  view: WorkspaceViewData;
  /** The workspace path of the open document, or null. */
  openPath: string[] | null;
  recents: RecentWorkspaceData[];
  /** The refusal line under Open workspace…, or empty. */
  refusal: string;
  /** The Recent toggle of an open workspace shows the list in place of the tree. */
  showRecent: boolean;
  slot: WorkspaceSlot | null;
  naming: WorkspaceNaming | null;
  /** The row marked by Ctrl+X. */
  cut: string[] | null;
  thumbnails: ReadonlyMap<string, ThumbnailState>;
  pickWorkspace: () => void;
  openRecent: (index: number) => void;
  removeRecent: (index: number) => void;
  toggleRecent: () => void;
  close: (refocus: () => void) => void;
  setExpanded: (path: readonly string[], expanded: boolean) => void;
  openRow: (row: WorkspaceRowData, refocus: () => void) => void;
  startNaming: (naming: WorkspaceNaming) => void;
  /** Commits the name field; the refusal shown in the field, or empty once done. */
  commitName: (text: string) => Promise<string>;
  cancelNaming: () => void;
  duplicate: (path: readonly string[]) => void;
  askDelete: (path: readonly string[]) => void;
  confirmDelete: (refocus: () => void) => void;
  move: (path: readonly string[], folder: readonly string[]) => void;
  undoMove: () => void;
  dismissSlot: () => void;
  setCut: (path: readonly string[] | null) => void;
  loadThumbnails: (rows: readonly WorkspaceRowData[]) => void;
  /** The last scan's entries (`specs/0095-workspace` criterion 25: Share builds the list from
   * it). */
  scanned: () => ScannedEntryData[];
  /** Scans again now. */
  rescan: () => Promise<void>;
  /** Takes a host answer that opened a workspace (a joined one, criterion 27). */
  adoptOpened: (answer: WorkspaceAnswer) => void;
  /** Opens the document at `path` without the check (it was just closed by Share). */
  reopen: (path: string[]) => Promise<void>;
  /** Closes the workspace without the check (a join replaces it after the check). */
  closeNow: () => Promise<void>;
  /** A shared workspace's tree (criteria 35 to 41): the Deleted list's rows. */
  deleted: readonly DeletedRow[];
  /** Restore of a Deleted row, Add to workspace of a row outside it, Download again. */
  restore: (row: DeletedRow) => void;
  addToWorkspace: (path: readonly string[]) => void;
  downloadAgain: () => void;
  /** The shared tree's side (`useSharedTree`) registers here; `null` when it ends. */
  setShared: (shared: SharedTreeHooks | null) => void;
  /** The rows' states of a shared workspace (`WasmWorkspace.set_shared_states`). */
  setSharedStates: (json: string) => void;
  setDeleted: (rows: DeletedRow[]) => void;
  /** The open document's path in the workspace now. */
  openPathNow: () => string[] | null;
  /** The plan moved a file: the open document, the expanded folders and the cut mark follow. */
  followMove: (from: string[], to: string[]) => void;
  /** Shows a status line in the action slot. */
  showStatus: (text: string) => void;
  /** Shows an alert in the action slot. */
  showAlert: (text: string) => void;
  /** The block on a device removed from the workspace (criterion 44), empty when none. */
  removedText: string;
  /** Reads the recent workspaces again (a synced workspace arrived, `0096` ux-notes 4.4). */
  reloadRecents: () => void;
  showRemoved: (text: string) => void;
  dismissRemoved: () => void;
  /** Asks in the action slot: `text`, Cancel and `action`, which runs `run`. */
  ask: (text: string, action: string, run: () => void) => void;
}

/** What a shared workspace adds to the tree's actions (`useSharedTree`). */
export interface SharedTreeHooks {
  /** Runs a planned operation in a shared workspace (the host's operation and the list edit);
   * `null` when the workspace is not shared, else the failure code or `null` once done. */
  apply: (op: WorkspaceOp, bytes: Uint8Array | undefined, run: (bytes?: Uint8Array) => Promise<unknown>) => Promise<string | null> | null;
  restore: (row: DeletedRow) => void;
  addToWorkspace: (path: readonly string[]) => void;
  downloadAgain: () => void;
}

/** One entry of the host's scan. */
export interface ScannedEntryData {
  path: string[];
  kind: "folder" | "document";
  size: number;
  modified: number;
}

/** The local time's offset east of UTC, minutes. */
function offsetMinutes(): number {
  return -new Date().getTimezoneOffset();
}

/** The bytes of an empty document with the default page, as New makes it (`0001`). */
async function emptyDocumentBytes(): Promise<Uint8Array> {
  const session = await createSession();
  try {
    return session.pack();
  } finally {
    session.free();
  }
}

/**
 * The open workspace in the page (`specs/0095-workspace` criteria 1 to 24, `ux-notes.md`
 * sections 2, 3, 10 and 11): the host's folder and scan, the session's rows and plans, the open
 * document's place, the action slot, the name field and the thumbnails. The rules and the words
 * are Rust's; this runs the host commands in order and keeps the page told. Desktop only.
 */
export function useWorkspace(editor: EditorSession, project: ProjectApi): WorkspaceApi {
  const projectRef = useRef(project);
  const editorRef = useRef(editor);
  useEffect(() => {
    projectRef.current = project;
    editorRef.current = editor;
  });

  const workspaceRef = useRef<WasmWorkspace | null>(null);
  const [name, setName] = useState("");
  const [view, setView] = useState<WorkspaceViewData>(parseWorkspaceView(""));
  const [openPath, setOpenPathState] = useState<string[] | null>(null);
  const openPathRef = useRef<string[] | null>(null);
  const setOpenPath = useCallback((path: string[] | null) => {
    openPathRef.current = path;
    setOpenPathState(path);
  }, []);
  const [recents, setRecents] = useState<RecentWorkspaceData[]>([]);
  const [refusal, setRefusal] = useState("");
  const [showRecent, setShowRecent] = useState(false);
  const [slot, setSlotState] = useState<WorkspaceSlot | null>(null);
  const slotRef = useRef<WorkspaceSlot | null>(null);
  const slotCounter = useRef(0);
  const slotTimer = useRef(0);
  const [naming, setNaming] = useState<WorkspaceNaming | null>(null);
  const [cut, setCutState] = useState<string[] | null>(null);
  const [thumbnails, setThumbnails] = useState<ReadonlyMap<string, ThumbnailState>>(new Map());
  const pendingThumbs = useRef(new Set<string>());
  /** The view of each document left in this workspace session (criterion 10). */
  const views = useRef(new Map<string, string>());
  /** A shared workspace's side of the actions (criteria 35 to 41), registered by `useSharedTree`. */
  const sharedRef = useRef<SharedTreeHooks | null>(null);
  const [deleted, setDeleted] = useState<readonly DeletedRow[]>([]);
  const [removedText, setRemovedText] = useState("");


  const nameRef = useRef(name);

  const showSlot = useCallback(
    (next: SlotBody | null, ms?: number) => {
      window.clearTimeout(slotTimer.current);
      slotCounter.current += 1;
      const value = next ? ({ ...next, id: slotCounter.current } as WorkspaceSlot) : null;
      slotRef.current = value;
      setSlotState(value);
      if (value && ms) {
        slotTimer.current = window.setTimeout(() => {
          if (slotRef.current?.id === value.id) {
            slotRef.current = null;
            setSlotState(null);
          }
        }, ms);
      }
    },
    [],
  );
  const alert = useCallback((text: string) => showSlot({ kind: "alert", text }, ALERT_MS), [showSlot]);

  const titleOf = useCallback((path: readonly string[]) => [nameRef.current, ...path].join("/"), []);

  /** Re-reads the rows; the open document's file is checked against the scan (criterion 7). */
  const refreshView = useCallback(() => {
    const workspace = workspaceRef.current;
    if (!workspace) {
      return;
    }
    setView(parseWorkspaceView(workspace.rows_json(Date.now() / 1000, offsetMinutes())));
    const path = openPathRef.current;
    if (path) {
      const missing = !workspace.contains(pathKey(path));
      projectRef.current.setFileState(fileNameOf(path), titleOf(path), missing);
    }
  }, [titleOf]);

  const scannedRef = useRef<ScannedEntryData[]>([]);
  const rescan = useCallback(async () => {
    const workspace = workspaceRef.current;
    if (!workspace) {
      return;
    }
    try {
      const entries = await invoke<ScannedEntryData[]>("scan_open_workspace");
      if (workspaceRef.current === workspace) {
        scannedRef.current = entries;
      }
      if (workspaceRef.current === workspace && workspace.set_scan(JSON.stringify(entries))) {
        refreshView();
      }
    } catch {
      // The folder is gone or unreadable: the tree stays as last seen.
    }
  }, [refreshView]);

  const loadRecents = useCallback(() => {
    if (!isTauri) {
      return;
    }
    void invoke<RecentWorkspaceData[]>("recent_workspaces")
      .then(setRecents)
      .catch(() => setRecents([]));
  }, []);

  useEffect(() => loadRecents(), [loadRecents]);

  const forgetThumbnails = useCallback(() => {
    setThumbnails((current) => {
      for (const state of current.values()) {
        if (state.kind === "thumbnail" && state.url) {
          URL.revokeObjectURL(state.url);
        }
      }
      return new Map();
    });
    pendingThumbs.current.clear();
  }, []);

  const clearWorkspace = useCallback(() => {
    workspaceRef.current?.free();
    workspaceRef.current = null;
    nameRef.current = "";
    setName("");
    setView(parseWorkspaceView(""));
    setOpenPath(null);
    setNaming(null);
    setCutState(null);
    showSlot(null);
    forgetThumbnails();
    views.current.clear();
    setDeleted([]);
    setRemovedText("");
  }, [forgetThumbnails, setOpenPath, showSlot]);

  /** Takes what the host answered to an open. */
  const adopt = useCallback(
    (answer: WorkspaceAnswer) => {
      if (answer.kind === "cancelled") {
        return;
      }
      if (answer.kind === "refused") {
        setRefusal(workspace_open_refusal(answer.code));
        loadRecents();
        return;
      }
      clearWorkspace();
      const workspace = new WasmWorkspace(answer.name);
      workspace.set_scan(JSON.stringify(answer.entries));
      scannedRef.current = answer.entries as ScannedEntryData[];
      workspaceRef.current = workspace;
      nameRef.current = answer.name;
      setName(answer.name);
      setRefusal("");
      setShowRecent(false);
      refreshView();
      loadRecents();
    },
    [clearWorkspace, loadRecents, refreshView],
  );

  /** Opens a workspace by `ask` (the picker or a recent row); with one open, the check comes
   * first and the open document closes only when another workspace did open. */
  const openBy = useCallback(
    (ask: () => Promise<WorkspaceAnswer>) => {
      const run = async () => {
        let answer: WorkspaceAnswer;
        try {
          answer = await ask();
        } catch {
          answer = { kind: "refused", code: "not-found" };
        }
        if (answer.kind === "opened" && workspaceRef.current) {
          endWorkspaceLink(editorRef.current.getSession());
          await projectRef.current.resetToEmpty();
        }
        adopt(answer);
      };
      if (workspaceRef.current) {
        projectRef.current.replaceThen("open-workspace", run);
      } else {
        void run();
      }
    },
    [adopt],
  );

  const pickWorkspace = useCallback(
    () => openBy(() => invoke<WorkspaceAnswer>("pick_workspace")),
    [openBy],
  );

  const openRecent = useCallback(
    (index: number) => {
      if (recents[index] && !recents[index].found) {
        return;
      }
      openBy(() => invoke<WorkspaceAnswer>("open_recent_workspace", { index }));
    },
    [openBy, recents],
  );

  const removeRecent = useCallback((index: number) => {
    void invoke<RecentWorkspaceData[]>("remove_recent_workspace", { index })
      .then(setRecents)
      .catch(() => undefined);
  }, []);

  /** Closes the workspace without the check (it ran already). */
  const closeNow = useCallback(async () => {
    // Closing a shared workspace disconnects its room (criterion 21).
    endWorkspaceLink(editorRef.current.getSession());
    await projectRef.current.resetToEmpty();
    await invoke("close_workspace").catch(() => undefined);
    clearWorkspace();
    setShowRecent(false);
    loadRecents();
  }, [clearWorkspace, loadRecents]);

  const close = useCallback(
    (refocus: () => void) => {
      projectRef.current.replaceThen("close-workspace", closeNow, refocus);
    },
    [closeNow],
  );

  // The folder is scanned every 2 s while open: other programs' changes show (criterion 7).
  const isOpen = name !== "";
  useEffect(() => {
    if (!isOpen) {
      return undefined;
    }
    const timer = window.setInterval(() => void rescan(), SCAN_MS);
    return () => window.clearInterval(timer);
  }, [isOpen, rescan]);

  // A document opened another way (New, Open…, Close, a join) is no longer the tree's.
  useEffect(() => {
    const path = openPathRef.current;
    if (path && project.info.path !== titleOf(path)) {
      setOpenPath(null);
    }
  }, [project.info.path, setOpenPath, titleOf]);

  const setExpanded = useCallback(
    (path: readonly string[], expanded: boolean) => {
      if (workspaceRef.current?.set_expanded(pathKey(path), expanded)) {
        refreshView();
      }
    },
    [refreshView],
  );

  /** Opens the document at `path` in the window (criteria 9, 10, 12). */
  const openDocument = useCallback(
    async (path: string[]) => {
      const { getSession, sync } = editorRef.current;
      const previous = openPathRef.current;
      const leaving = getSession();
      if (previous && leaving) {
        views.current.set(pathKey(previous), leaving.saved_view_text());
      }
      let bytes: Uint8Array;
      try {
        bytes = await invoke<ArrayBuffer>("read_workspace_document", { path }).then(answerBytes);
      } catch (code) {
        alert(workspace_failure(String(code)));
        void rescan();
        return;
      }
      const opened = await projectRef.current.openWorkspaceDocument(
        path,
        fileNameOf(path),
        titleOf(path),
        bytes,
      );
      if (!opened) {
        // The row says why from now on (ux-notes section 10).
        void probeWith(() => invoke<ArrayBuffer>("probe_workspace_document", { path }).then(answerBytes)).then((probe) => {
          const text = probe?.kind === "unreadable" ? workspace_unreadable(probe.code) : workspace_unreadable("damaged");
          setThumbnails((current) => new Map(current).set(pathKey(path), { kind: "unreadable", text }));
        });
        return;
      }
      setOpenPath(path);
      const saved = views.current.get(pathKey(path));
      const session = getSession();
      if (saved && session?.restore_view_text(saved)) {
        sync();
      }
      refreshView();
    },
    [alert, refreshView, rescan, setOpenPath, titleOf],
  );

  const openRow = useCallback(
    (row: WorkspaceRowData, refocus: () => void) => {
      if (row.kind !== "document" || samePath(row.path, openPathRef.current)) {
        return;
      }
      const unreadable = [...thumbnails.entries()].find(
        ([key, state]) => state.kind === "unreadable" && (key === pathKey(row.path) || key.startsWith(`${pathKey(row.path)}@`)),
      );
      if (unreadable && unreadable[1].kind === "unreadable") {
        // Nothing opens; the slot gives the full reason (ux-notes section 3).
        alert(unreadable[1].text);
        return;
      }
      projectRef.current.replaceThen("switch", () => openDocument([...row.path]), refocus);
    },
    [alert, openDocument, thumbnails],
  );

  /** A file moved from `from` to `to`: the open document, the expanded folders and the cut mark
   * follow it. */
  const followMove = useCallback(
    (from: string[], to: string[]) => {
      workspaceRef.current?.moved(pathKey(from), pathKey(to));
      const path = openPathRef.current;
      if (path && isInside(path, from)) {
        const moved = movedPath(path, from, to);
        setOpenPath(moved);
        projectRef.current.setFileState(fileNameOf(moved), titleOf(moved), false);
      }
      setCutState((current) => (current && isInside(current, from) ? null : current));
      refreshView();
    },
    [refreshView, setOpenPath, titleOf],
  );

  /** Runs a planned operation; the alert of a failure, or null when done. In a shared workspace
   * the list records it too. */
  const apply = useCallback(
    async (op: WorkspaceOp, bytes?: Uint8Array): Promise<string | null> => {
      const run = (with_bytes?: Uint8Array) =>
        with_bytes ? invoke("apply_workspace_op", { op }, with_bytes) : invoke("apply_workspace_op", { op });
      try {
        const shared = sharedRef.current?.apply(op, bytes, run) ?? null;
        if (shared) {
          const failed = await shared;
          if (failed) {
            return failed;
          }
        } else {
          await run(bytes);
        }
      } catch (code) {
        return String(code);
      } finally {
        await rescan();
      }
      if (op.kind === "move") {
        followMove(op.from, op.to);
      }
      return null;
    },
    [followMove, rescan],
  );

  const plan = useCallback((request: object): PlanAnswer => {
    const workspace = workspaceRef.current;
    return workspace ? parsePlan(workspace.plan(JSON.stringify(request))) : { kind: "refused", text: "" };
  }, []);

  const commitName = useCallback(
    async (text: string): Promise<string> => {
      const current = naming;
      if (!current) {
        return "";
      }
      const answer =
        current.mode === "rename"
          ? plan({ kind: "rename", path: current.path, name: text })
          : plan({ kind: current.mode, folder: current.folder, name: text });
      if (answer.kind === "refused") {
        setNaming({ ...current, error: answer.text });
        return answer.text;
      }
      if (current.mode === "rename" && answer.op.kind === "move" && samePath(answer.op.from, answer.op.to)) {
        setNaming(null);
        return "";
      }
      const bytes = current.mode === "new-document" ? await emptyDocumentBytes() : undefined;
      const failed = await apply(answer.op, bytes);
      setNaming(null);
      if (failed) {
        alert(workspace_failure(failed));
        return "";
      }
      if (current.mode !== "rename") {
        showSlot({ kind: "status", text: answer.status }, STATUS_MS);
      }
      if (current.mode === "new-folder" || current.mode === "new-document") {
        setExpanded(current.folder, true);
      }
      if (current.mode === "new-document" && answer.op.kind === "create-document") {
        const path = answer.op.path;
        projectRef.current.replaceThen("switch", () => openDocument(path));
      }
      return "";
    },
    [alert, apply, naming, openDocument, plan, setExpanded, showSlot],
  );

  const duplicate = useCallback(
    (path: readonly string[]) => {
      void (async () => {
        const answer = plan({ kind: "duplicate", path });
        if (answer.kind === "refused") {
          alert(answer.text);
          return;
        }
        let copy: Uint8Array;
        try {
          copy = workspace_copy(await invoke<ArrayBuffer>("read_workspace_document", { path }).then(answerBytes));
        } catch (error) {
          alert(typeof error === "string" ? workspace_failure(error) : String(error));
          return;
        }
        const failed = await apply(answer.op, copy);
        if (failed) {
          alert(workspace_failure(failed));
          return;
        }
        showSlot({ kind: "status", text: answer.status }, STATUS_MS);
        if (answer.op.kind === "create-document") {
          setNaming({ mode: "rename", path: answer.op.path, initial: answer.name, error: "" });
        }
      })();
    },
    [alert, apply, plan, showSlot],
  );

  const askDelete = useCallback(
    (path: readonly string[]) => {
      const answer = plan({ kind: "delete", path });
      if (answer.kind === "refused") {
        alert(answer.text);
        return;
      }
      showSlot({
        kind: "confirm",
        text: answer.confirm,
        path: [...path],
        op: answer.op,
        noTrash: answer.noTrash,
        status: answer.status,
      });
    },
    [alert, plan, showSlot],
  );

  const confirmDelete = useCallback(
    (refocus: () => void) => {
      const pending = slotRef.current;
      if (pending?.kind !== "confirm") {
        return;
      }
      const trash = async () => {
        const failed = await apply(pending.op);
        if (failed === "no-trash") {
          showSlot({ kind: "alert", text: pending.noTrash }, ALERT_MS);
        } else if (failed) {
          alert(workspace_failure(failed));
        } else {
          showSlot({ kind: "status", text: pending.status }, STATUS_MS);
        }
        // The rows are the rescanned ones by the next frame: the focus goes to the row that took
        // the deleted one's place, else the one before it, else New document.
        requestAnimationFrame(refocus);
      };
      const open = openPathRef.current;
      if (open && isInside(open, pending.path)) {
        // The open document closes first, after the unsaved-changes check (criterion 17).
        projectRef.current.replaceThen(
          "delete-open",
          async () => {
            await projectRef.current.resetToEmpty();
            setOpenPath(null);
            await trash();
          },
          refocus,
        );
        return;
      }
      void trash();
    },
    [alert, apply, setOpenPath, showSlot],
  );

  const move = useCallback(
    (path: readonly string[], folder: readonly string[]) => {
      void (async () => {
        const answer = plan({ kind: "move", path, folder });
        if (answer.kind === "refused") {
          alert(answer.text);
          return;
        }
        const failed = await apply(answer.op);
        if (failed) {
          alert(workspace_failure(failed));
          return;
        }
        if (answer.op.kind === "move") {
          showSlot(
            { kind: "undo", text: answer.status, inverse: { kind: "move", from: answer.op.to, to: answer.op.from } },
            UNDO_MOVE_MS,
          );
        }
      })();
    },
    [alert, apply, plan, showSlot],
  );

  const undoMove = useCallback(() => {
    const pending = slotRef.current;
    if (pending?.kind !== "undo") {
      return;
    }
    showSlot(null);
    void apply(pending.inverse).then((failed) => {
      if (failed) {
        alert(workspace_failure(failed));
      }
    });
  }, [alert, apply, showSlot]);

  const loadThumbnails = useCallback((rows: readonly WorkspaceRowData[]) => {
    for (const row of rows) {
      if (row.kind !== "document") {
        continue;
      }
      const key = thumbnailKey(row.path, row.time);
      if (pendingThumbs.current.has(key)) {
        continue;
      }
      pendingThumbs.current.add(key);
      void probeWith(() => invoke<ArrayBuffer>("probe_workspace_document", { path: row.path }).then(answerBytes)).then(
        (probe: ProbeData | null) => {
          if (!probe) {
            pendingThumbs.current.delete(key);
            return;
          }
          const state: ThumbnailState =
            probe.kind === "unreadable"
              ? { kind: "unreadable", text: workspace_unreadable(probe.code) }
              : {
                  kind: "thumbnail",
                  url: probe.png ? URL.createObjectURL(new Blob([probe.png], { type: "image/png" })) : null,
                };
          setThumbnails((current) => new Map(current).set(key, state));
        },
      );
    }
  }, []);

  // The workspace ends with the page: its blob URLs go with it.
  useEffect(() => () => workspaceRef.current?.free(), []);

  return {
    open: isOpen,
    name,
    view,
    openPath,
    recents,
    refusal,
    showRecent,
    slot,
    naming,
    cut,
    thumbnails,
    pickWorkspace,
    openRecent,
    removeRecent,
    toggleRecent: () => setShowRecent((shown) => !shown),
    close,
    setExpanded,
    openRow,
    startNaming: (next: WorkspaceNaming) => {
      showSlot(null);
      setNaming(next);
    },
    commitName,
    cancelNaming: () => setNaming(null),
    duplicate,
    askDelete,
    confirmDelete,
    move,
    undoMove,
    dismissSlot: () => showSlot(null),
    setCut: (path: readonly string[] | null) => setCutState(path ? [...path] : null),
    loadThumbnails,
    scanned: () => scannedRef.current,
    rescan,
    adoptOpened: adopt,
    reopen: openDocument,
    closeNow,
    deleted,
    restore: (row: DeletedRow) => sharedRef.current?.restore(row),
    addToWorkspace: (path: readonly string[]) => sharedRef.current?.addToWorkspace(path),
    downloadAgain: () => sharedRef.current?.downloadAgain(),
    setShared: (shared: SharedTreeHooks | null) => {
      sharedRef.current = shared;
    },
    setSharedStates: (json: string) => {
      if (workspaceRef.current?.set_shared_states(json)) {
        refreshView();
      }
    },
    setDeleted,
    openPathNow: () => openPathRef.current,
    followMove,
    showStatus: (text: string) => showSlot({ kind: "status", text }, STATUS_MS),
    showAlert: alert,
    removedText,
    reloadRecents: loadRecents,
    showRemoved: setRemovedText,
    dismissRemoved: () => setRemovedText(""),
    ask: (text: string, action: string, run: () => void) => showSlot({ kind: "ask", text, action, run }),
  };
}
