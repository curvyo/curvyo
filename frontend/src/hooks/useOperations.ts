import { useCallback, useEffect, useMemo, useState } from "react";

import type { WasmSession } from "@/lib/editorSession";
import type { DropBadge } from "@/components/DropBadgePill";
import { assignWord, digitRow } from "@/lib/operationRows";
import {
  COUNT_WIRE,
  decodeOperationId,
  decodeOperationsText,
  operationIdKey,
  operationIdText,
  type CountKey,
  type OperationId,
  type OperationsView,
} from "@/lib/operationsView";

/** What the Operations tab reads from the editor: the session and its re-read signal. */
export interface OperationsEditor {
  getSession: () => WasmSession | null;
  syncRevision: number;
  sync: () => void;
  /** The canvas: a digit press assigns the selection only while the canvas has the focus. */
  containerRef: { readonly current: HTMLElement | null };
  /** Items selected on the canvas. */
  selectionCount: number;
}

/** The Operations tab's state and commands (`specs/0101-laser-operations` `adrs.md` decision 5;
 * the session holds the rules, this hook only calls them and re-reads the view). */
export interface OperationsApi {
  /** The decoded view of the active laser page; `null` on any other page or with the extension off. */
  view: OperationsView | null;
  /** The notice of the last command (criterion 10), `null` when it had none. */
  notice: string | null;
  /** The key (`peer:counter`) of the row a canvas drag is over, `null` otherwise. */
  dropKey: string | null;
  /** The "Assign to <name>" badge beside the pointer over a row (`ux-notes.md` 6.3), `null` otherwise. */
  dropBadge: DropBadge | null;
  /** Adds an operation of `kind` with the selection assigned; the new id, `null` when refused. */
  press: (kind: string) => OperationId | null;
  rename: (id: OperationId, name: string) => boolean;
  remove: (id: OperationId) => boolean;
  setShow: (id: OperationId, on: boolean) => boolean;
  setOutput: (id: OperationId, on: boolean) => boolean;
  setCutsLast: (on: boolean) => boolean;
  /** Moves the operation to the index `to` in the run order shown. */
  move: (id: OperationId, to: number) => boolean;
  /** Ticks (`on`) or unticks the selection in the operation. */
  tick: (id: OperationId, on: boolean) => boolean;
  /** Holds (`on`) or drops a clone member, by its clone, in the operation. */
  setCloneMember: (id: OperationId, clone: OperationId, member: OperationId, on: boolean) => boolean;
  /** Selects the items of a summary count on the canvas (criterion 17). */
  selectItems: (count: CountKey) => boolean;
  /** Draws the output page as drawn (`true`) or by its operations (`false`) (criterion 18). */
  setDrawn: (drawn: boolean) => boolean;
}

interface Snapshot {
  view: OperationsView | null;
  notice: string | null;
}

function readSnapshot(session: WasmSession | null): Snapshot {
  if (!session) return { view: null, notice: null };
  return {
    view: decodeOperationsText(session.operations_view_json()),
    notice: session.operations_notice() ?? null,
  };
}

/** The row under a point of the screen, by its key; `null` off the rows. */
function rowKeyAt(x: number, y: number): string | null {
  const element = document.elementFromPoint(x, y);
  return element?.closest<HTMLElement>("[data-row-key]")?.dataset.rowKey ?? null;
}

/** The Operations tab's API for `editor`. Every call is one session command, then one re-read. */
export function useOperations(editor: OperationsEditor): OperationsApi {
  const { getSession, syncRevision, sync, containerRef, selectionCount } = editor;
  const snapshot = useMemo(() => {
    void syncRevision; // The session's UI state changed: read the view again.
    return readSnapshot(getSession());
  }, [getSession, syncRevision]);
  const [dropKey, setDropKey] = useState<string | null>(null);
  const [dropBadge, setDropBadge] = useState<DropBadge | null>(null);

  const run = useCallback(
    <T,>(fallback: T, call: (session: WasmSession) => T): T => {
      const session = getSession();
      if (!session) return fallback;
      const result = call(session);
      sync();
      return result;
    },
    [getSession, sync],
  );

  const press = useCallback(
    (kind: string) =>
      run<OperationId | null>(null, (s) => {
        const text = s.press_operation_kind(kind);
        return text === undefined ? null : decodeOperationId(text);
      }),
    [run],
  );
  const rename = useCallback(
    (id: OperationId, name: string) => run(false, (s) => s.rename_operation(operationIdText(id), name)),
    [run],
  );
  const remove = useCallback(
    (id: OperationId) => run(false, (s) => s.delete_operation(operationIdText(id))),
    [run],
  );
  const setShow = useCallback(
    (id: OperationId, on: boolean) => run(false, (s) => s.set_operation_show(operationIdText(id), on)),
    [run],
  );
  const setOutput = useCallback(
    (id: OperationId, on: boolean) => run(false, (s) => s.set_operation_output(operationIdText(id), on)),
    [run],
  );
  const setCutsLast = useCallback((on: boolean) => run(false, (s) => s.set_cuts_last(on)), [run]);
  const move = useCallback(
    (id: OperationId, to: number) => run(false, (s) => s.move_operation(operationIdText(id), to)),
    [run],
  );
  const tick = useCallback(
    (id: OperationId, on: boolean) => run(false, (s) => s.set_selection_in_operation(operationIdText(id), on)),
    [run],
  );
  const setCloneMember = useCallback(
    (id: OperationId, clone: OperationId, member: OperationId, on: boolean) =>
      run(false, (s) =>
        s.set_clone_member_in_operation(operationIdText(id), operationIdText(clone), operationIdText(member), on),
      ),
    [run],
  );
  const selectItems = useCallback(
    (count: CountKey) => run(false, (s) => s.select_operation_items(COUNT_WIRE[count])),
    [run],
  );
  const setDrawn = useCallback((drawn: boolean) => run(false, (s) => s.set_operations_drawn(drawn)), [run]);

  // A canvas drag that ends over an operation row assigns the selection to it, in one step, and
  // the drag is cancelled first (Escape: no move, no step), so the items do not move. The window
  // capture phase runs before the canvas, so the canvas never sees this release.
  useEffect(() => {
    const dragging = () => {
      const session = getSession();
      return session !== null && selectionCount > 0 && session.pointer_is_down() ? session : null;
    };
    const rowAt = (key: string | null) =>
      key === null ? undefined : snapshot.view?.rows.find((r) => operationIdKey(r.id) === key);
    const onMove = (event: PointerEvent) => {
      const key = dragging() ? rowKeyAt(event.clientX, event.clientY) : null;
      setDropKey((previous) => (previous === key ? previous : key));
      const row = rowAt(key);
      setDropBadge(
        row ? { kind: "assign", word: assignWord(row.name), refused: false, x: event.clientX + 16, y: event.clientY + 16 } : null,
      );
    };
    const onUp = (event: PointerEvent) => {
      const session = dragging();
      const key = session ? rowKeyAt(event.clientX, event.clientY) : null;
      setDropKey(null);
      setDropBadge(null);
      const row = key === null ? undefined : snapshot.view?.rows.find((r) => operationIdKey(r.id) === key);
      if (!session || !row) return;
      event.stopPropagation();
      session.key_down("Escape", false, false, false, false, false, false, performance.now());
      // A drop only adds (`specs/0101` criterion 8); a tick press is the one that toggles.
      session.set_selection_in_operation(operationIdText(row.id), true);
      sync();
    };
    window.addEventListener("pointermove", onMove, true);
    window.addEventListener("pointerup", onUp, true);
    return () => {
      window.removeEventListener("pointermove", onMove, true);
      window.removeEventListener("pointerup", onUp, true);
    };
  }, [getSession, selectionCount, snapshot.view, sync]);

  // A digit 1 to 9 on the canvas toggles the selection in that operation (criterion 8, Proposal).
  // The canvas's own key handler runs first: a digit it has taken is `defaultPrevented` here.
  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      if (
        event.defaultPrevented ||
        event.ctrlKey ||
        event.metaKey ||
        event.altKey ||
        event.shiftKey ||
        event.target !== containerRef.current ||
        !/^[1-9]$/.test(event.key)
      ) {
        return;
      }
      const session = getSession();
      const view = snapshot.view;
      if (!session || !view || digitRow(view, Number(event.key)) === null) {
        return;
      }
      if (session.toggle_selection_by_number(Number(event.key))) {
        event.preventDefault();
        sync();
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [containerRef, getSession, snapshot.view, sync]);

  return {
    view: snapshot.view,
    notice: snapshot.notice,
    dropKey,
    dropBadge,
    press,
    rename,
    remove,
    setShow,
    setOutput,
    setCutsLast,
    move,
    tick,
    setCloneMember,
    selectItems,
    setDrawn,
  };
}
