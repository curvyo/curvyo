// A shared workspace's live tree in the page (`specs/0095-workspace` criteria 35 to 41, ADR 0037
// decision 6): the maker's file operations recorded in the shared list, and every 2 s the host's
// scan turned by the session's plan into file operations, row states, the index and the Deleted
// list. The plan and its words are Rust's; this runs the host commands in order.

import { type MutableRefObject, useCallback, useEffect, useRef } from "react";

import type { ProjectApi } from "@/hooks/useProject";
import type { SharedTreeHooks, WorkspaceApi } from "@/hooks/useWorkspace";
import { type RoomKeys, openOldRoom } from "@/lib/movedPush";
import { type DeletedRow, type DiskStep, type SharedScan, parseDeleted, parseDiskPlan, sharedStates, trashChoice } from "@/lib/sharedTree";
import { SCAN_MS, type WorkspaceOp, fileNameOf } from "@/lib/workspaceRows";
import { invoke } from "@/platform/host";
import {
  type WasmSession,
  workspace_added_text,
  workspace_deleted_while_open_text,
  workspace_follow_failed,
  workspace_restored_text,
  workspace_take_in_confirm_text,
  workspace_take_in_refused_text,
  workspace_take_in_unreachable_text,
} from "@/wasm-bindings/curvyo_editor_wasm.js";

/** The alert when the old room could not be told (Proposal: the document is in the workspace
 * anyway; its own link keeps admitting until a member of it reads `Moved`). */
const MOVED_FAILED = "Could not reach the document's own room. Its old link may still work.";

function random(length: number): Uint8Array {
  const bytes = new Uint8Array(length);
  crypto.getRandomValues(bytes);
  return bytes;
}

function toHex(bytes: Uint8Array): string {
  return Array.from(bytes, (b) => b.toString(16).padStart(2, "0")).join("");
}

/** The local time's offset east of UTC, minutes. */
function offsetMinutes(): number {
  return -new Date().getTimezoneOffset();
}

/** The session when it carries a shared workspace that has finished joining. */
function sharedSession(getSession: () => WasmSession | null): WasmSession | null {
  const session = getSession();
  return session?.has_workspace_link() && session.workspace_id_hex() !== "" ? session : null;
}

export function useSharedTree(
  getSession: () => WasmSession | null,
  project: ProjectApi,
  workspace: WorkspaceApi,
  writingRef: MutableRefObject<Promise<void>>,
  keysFor: (room: string) => Promise<RoomKeys | null>,
  label: string,
): void {
  const refs = useRef({ project, workspace, keysFor, label });
  useEffect(() => {
    refs.current = { project, workspace, keysFor, label };
  });
  const runningRef = useRef(false);
  const indexRef = useRef("");
  const deletedRef = useRef("");
  const failedRef = useRef("");
  const wasSharedRef = useRef(false);

  /** One file operation of the plan: the path of a document still arriving, `failed` when the
   * host refused it (the later steps depend on it), else null. */
  const runStep = useCallback(async (step: DiskStep): Promise<string[] | "failed" | null> => {
    const { project: p, workspace: ws } = refs.current;
    const done = (call: Promise<unknown>) =>
      call.then(
        () => true,
        () => false,
      );
    switch (step.kind) {
      case "create-folder":
        return (await done(invoke("apply_workspace_op", { op: { kind: "create-folder", path: step.path } })))
          ? null
          : "failed";
      case "move": {
        if (!(await done(invoke("apply_workspace_op", { op: { kind: "move", from: step.from, to: step.to } })))) {
          return "failed";
        }
        ws.followMove(step.from, step.to);
        return null;
      }
      case "write": {
        // Restore writes the kept copy; a new document is written by the background writer.
        const restored = await done(invoke("restore_workspace_document", { document: step.document, path: step.path }));
        return restored ? null : step.path;
      }
      case "keep":
        return (await done(
          invoke("keep_workspace_document", { path: step.path, document: step.document, trash: false }),
        ))
          ? null
          : "failed";
      case "trash-document": {
        const choice = trashChoice(step.path, ws.openPathNow(), p.dirty);
        if (choice === "hold") {
          p.setDeletedStatus(workspace_deleted_while_open_text(step.by));
          return null;
        }
        if (choice === "close-then-trash") {
          await p.resetToEmpty();
        }
        return (await done(
          invoke("keep_workspace_document", { path: step.path, document: step.document, trash: true }),
        ))
          ? null
          : "failed";
      }
      case "trash":
        return (await done(invoke("apply_workspace_op", { op: { kind: "trash", path: step.path } })))
          ? null
          : "failed";
    }
  }, []);

  /** Brings the folder to the list once (criteria 35 to 41). */
  const tick = useCallback(async () => {
    const session = sharedSession(getSession);
    const { workspace: ws } = refs.current;
    if (!session || !ws.open) {
      if (wasSharedRef.current) {
        wasSharedRef.current = false;
        ws.setSharedStates(sharedStates(null, []));
        ws.setDeleted([]);
        deletedRef.current = "";
      }
      return;
    }
    const scan = await invoke<SharedScan>("scan_shared_workspace").catch(() => null);
    const current = sharedSession(getSession);
    if (!scan || !current) {
      return;
    }
    const plan = parseDiskPlan(current.workspace_disk_plan(JSON.stringify(scan), Date.now(), random(16)));
    if (!plan) {
      return;
    }
    wasSharedRef.current = true;
    const receiving: string[][] = [];
    let failed: DiskStep | null = null;
    for (const step of plan.steps) {
      const waiting = await runStep(step);
      if (waiting === "failed") {
        failed = step;
        break;
      }
      if (waiting) {
        receiving.push(waiting);
      }
    }
    if (failed) {
      // The index keeps the last places that hold; the next scan plans again from the disk.
      const key = JSON.stringify(failed);
      if (key !== failedRef.current) {
        failedRef.current = key;
        ws.showAlert(workspace_follow_failed());
      }
      await ws.rescan();
      return;
    }
    failedRef.current = "";
    const index = JSON.stringify(plan.settled);
    if (index !== indexRef.current) {
      indexRef.current = index;
      await invoke("write_workspace_index", { documents: plan.settled }).catch(() => undefined);
    }
    ws.setSharedStates(sharedStates(plan, receiving));
    const deleted = current.workspace_deleted_json(Date.now(), offsetMinutes());
    if (deleted !== deletedRef.current) {
      deletedRef.current = deleted;
      ws.setDeleted(parseDeleted(deleted));
    }
    if (plan.steps.length > 0) {
      await ws.rescan();
    }
  }, [getSession, runStep]);

  /** Runs a tick after the writes queued before it, never two at once. */
  const runTick = useCallback(() => {
    if (runningRef.current) {
      return;
    }
    runningRef.current = true;
    writingRef.current = writingRef.current.then(tick).finally(() => {
      runningRef.current = false;
    });
  }, [tick, writingRef]);

  useEffect(() => {
    const timer = window.setInterval(runTick, SCAN_MS);
    return () => window.clearInterval(timer);
  }, [runTick]);

  /** The list edit of a shared workspace: empty when done. */
  const edit = useCallback(
    (session: WasmSession, value: object) => session.workspace_list_edit(JSON.stringify(value), Date.now(), random(16)),
    [],
  );

  const apply = useCallback(
    (op: WorkspaceOp, bytes: Uint8Array | undefined, run: (bytes?: Uint8Array) => Promise<unknown>) => {
      const session = sharedSession(getSession);
      if (!session) {
        return null;
      }
      return (async (): Promise<string | null> => {
        switch (op.kind) {
          case "create-document": {
            // The list goes out before the document's first records (criterion 35).
            const document = toHex(random(16));
            if (edit(session, { kind: "create-document", path: op.path, document }) !== "") {
              return "not-found";
            }
            await run(session.workspace_adopt_document(document, bytes ?? new Uint8Array()));
            return null;
          }
          case "create-folder":
            await run();
            edit(session, { kind: "create-folder", path: op.path });
            return null;
          case "move":
            await run();
            edit(session, { kind: "move", from: op.from, to: op.to });
            return null;
          case "trash":
            // A shared deletion (criterion 40): every member's plan trashes it, this one's too.
            edit(session, { kind: "delete", path: op.path });
            runTick();
            return null;
        }
      })();
    },
    [edit, getSession, runTick],
  );

  const restore = useCallback(
    (row: DeletedRow) => {
      const session = sharedSession(getSession);
      if (!session) {
        return;
      }
      if (edit(session, { kind: "restore", node: row.node }) === "") {
        refs.current.workspace.showStatus(workspace_restored_text(row.name));
        runTick();
      }
    },
    [edit, getSession, runTick],
  );

  /** Add to workspace (criterion 41): the file gets a fresh document id and joins the list. */
  const addToWorkspace = useCallback(
    (path: readonly string[]) => {
      void (async () => {
        const session = sharedSession(getSession);
        if (!session) {
          return;
        }
        const bytes = await invoke<ArrayBuffer>("read_workspace_document", { path }).catch(() => null);
        if (!bytes) {
          return;
        }
        const original = new Uint8Array(bytes);
        const ws = refs.current.workspace;
        const name = fileNameOf([...path]).replace(/\.curvyo$/i, "");
        const check = JSON.parse(session.workspace_take_in_check(original)) as { kind: string; people?: number };
        if (check.kind === "refused") {
          // Criterion 49: it keeps syncing on its own room.
          ws.showAlert(workspace_take_in_refused_text(name, ws.name));
          return;
        }
        const add = async () => {
          const current = sharedSession(getSession);
          if (!current) {
            return;
          }
          // ADR 0037 decision 10 (Proposal): the old room must be reachable before anything
          // changes; otherwise the document is not taken in and keeps syncing on its own.
          const oldRoom =
            check.kind === "allowed"
              ? await openOldRoom(original, refs.current.label, (room) => refs.current.keysFor(room))
              : null;
          if (check.kind === "allowed" && !oldRoom) {
            ws.showAlert(workspace_take_in_unreachable_text(name));
            return;
          }
          const document = toHex(random(16));
          if (edit(current, { kind: "create-document", path, document }) !== "") {
            oldRoom?.close();
            return;
          }
          const adopted = current.workspace_adopt_document(document, original);
          if (oldRoom) {
            // The state is in the workspace, then `Moved` ends the old room.
            const stored = await oldRoom.push(current.workspace_id_hex(), document);
            oldRoom.close();
            if (!stored) {
              ws.showAlert(MOVED_FAILED);
            }
          }
          await invoke("replace_workspace_document", { path }, adopted).catch(() => undefined);
          ws.showStatus(workspace_added_text(name));
          runTick();
        };
        if (check.kind === "allowed") {
          ws.ask(workspace_take_in_confirm_text(name, check.people ?? 0, ws.name), "Take in", () => void add());
        } else {
          await add();
        }
      })();
    },
    [edit, getSession, runTick],
  );

  const downloadAgain = useCallback(() => {
    sharedSession(getSession)?.workspace_download_again();
  }, [getSession]);

  useEffect(() => {
    const hooks: SharedTreeHooks = { apply, restore, addToWorkspace, downloadAgain };
    workspace.setShared(hooks);
    return () => workspace.setShared(null);
  }, [addToWorkspace, apply, downloadAgain, restore, workspace]);
}
