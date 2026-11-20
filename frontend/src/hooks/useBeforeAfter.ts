import { useCallback, useMemo, useState } from "react";

import type { OperationId } from "@/lib/operationsView";
import { operationIdText } from "@/lib/operationsView";
import type { WasmSession } from "@/lib/editorSession";
import { type BeforeAfterView, decodeBeforeAfterText } from "@/lib/beforeAfterView";

interface EditorHandle {
  getSession: () => WasmSession | null;
  syncRevision: number;
  sync: () => void;
}

/** The pane takes half of the canvas area (`ux-notes.md` §7, P18). */
const PANE_SHARE = 0.5;

/** The Before and after pane (`specs/0135-image-optimisation` criteria 11 to 13). */
export interface BeforeAfterApi {
  view: BeforeAfterView | null;
  open: boolean;
  /** The header button of the group, or the pane's own Close. */
  toggle: () => boolean;
  close: () => void;
  setView: (view: "split" | "side_by_side") => void;
  pickOperation: (id: OperationId) => void;
  /** The RGBA8 bytes of one half. */
  pixels: (half: "before" | "after") => Uint8ClampedArray;
  /** Full view chosen (it lies over the whole canvas area). */
  full: boolean;
  toggleFull: () => void;
  /** The dragged share of the canvas area. */
  split: number;
  setSplit: (split: number | undefined) => void;
  /** Re-reads after a change the hook did not make. */
  syncRevision: number;
}

/**
 * The pane's host state: its view JSON, re-read after every sync (the job's ticks sync, so the
 * live picture is replaced by the dots as they arrive), its switches and its pixels. The zoom
 * and pan are the pane component's. Full view and the dragged split are kept for the session only.
 */
export function useBeforeAfter(editor: EditorHandle): BeforeAfterApi {
  const { getSession, syncRevision, sync } = editor;
  const [local, setLocal] = useState(0);
  const [full, setFull] = useState(false);
  const [dragged, setDragged] = useState<number | undefined>(undefined);
  // eslint-disable-next-line react-hooks/exhaustive-deps
  const view = useMemo(() => decodeBeforeAfterText(getSession()?.before_after_view_json() ?? ""), [getSession, syncRevision, local]);
  const run = useCallback(
    <T,>(none: T, call: (session: WasmSession) => T): T => {
      const session = getSession();
      if (!session) return none;
      const result = call(session);
      setLocal((n) => n + 1);
      sync();
      return result;
    },
    [getSession, sync],
  );
  const toggle = useCallback(() => run(false, (s) => s.toggle_before_after()), [run]);
  const close = useCallback(() => run(undefined, (s) => s.close_before_after()), [run]);
  const setView = useCallback((next: "split" | "side_by_side") => run(false, (s) => s.set_before_after_view(next)), [run]);
  const pickOperation = useCallback(
    (id: OperationId) => run(false, (s) => s.pick_before_after_operation(operationIdText(id))),
    [run],
  );
  const pixels = useCallback(
    (half: "before" | "after") => new Uint8ClampedArray(getSession()?.before_after_pixels(half) ?? []),
    [getSession],
  );
  const toggleFull = useCallback(() => setFull((on) => !on), []);
  return {
    view,
    open: view?.open ?? false,
    toggle,
    close,
    setView: (next) => void setView(next),
    pickOperation: (id) => void pickOperation(id),
    pixels,
    full,
    toggleFull,
    split: dragged ?? PANE_SHARE,
    setSplit: setDragged,
    syncRevision,
  };
}
