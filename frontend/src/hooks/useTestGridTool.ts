import { useCallback, useEffect, useMemo, useState, type RefObject } from "react";

import type { WasmSession } from "@/lib/editorSession";
import { decodeTestGridText, type Param, type TestGridBarView } from "@/lib/testGridView";
import type { SetKind } from "@/lib/operationSettingsView";

/** What the Test grid tool reads from the editor: the session, its re-read signal and the tool. */
export interface TestGridEditor {
  getSession: () => WasmSession | null;
  syncRevision: number;
  sync: () => void;
}

/**
 * The Test grid tool's commands (`specs/0112-test-grid-and-materials` `ux-notes.md` 3, 4; the
 * session holds every rule, this hook only calls it and re-reads the view). The only caller of
 * the grid bindings; it does not grow `useEditorSession`.
 */
export interface TestGridToolApi {
  /** The tool is offered: a laser page. */
  available: boolean;
  /** The bar of the selected grid or of the tool's draft; `null` when none shows. */
  view: TestGridBarView | null;
  /** The refusal of the last press (a bed or font cause), shown under the bar. */
  pressNotice: string | null;
  /** The sr-only status ("New test id 8PL4D."), the last one taken. */
  status: string;
  /** Enter with the tool active and the canvas focused: places at the first free spot. */
  placeAtFreeSpot: () => void;
  /** Types `typed` into field `key`; the cause, empty when taken. */
  setField: (key: string, typed: string) => string;
  setKind: (kind: SetKind) => string;
  setAxis: (side: "columns" | "rows", param: Param) => string;
  setCrosshatch: (on: boolean) => string;
  /** A pick of the dropdown, `""` for none; the cause, empty when taken. */
  pickMaterial: (id: string) => string;
  /** Re-reads the view after the canvas changed it (a press, a selection). */
  refresh: () => void;
}

/** 64 random bytes for new test ids (`crypto.getRandomValues`). */
function randomBytes(): Uint8Array {
  const bytes = new Uint8Array(64);
  crypto.getRandomValues(bytes);
  return bytes;
}

/** The Test grid tool's API for `editor`; `active` is whether the tool is the active one. */
export function useTestGridTool(
  editor: TestGridEditor,
  active: boolean,
  container: RefObject<HTMLElement | null>,
): TestGridToolApi {
  const { getSession, syncRevision, sync } = editor;
  const view = useMemo(() => {
    void syncRevision;
    const session = getSession();
    return session ? decodeTestGridText(session.test_grid_view_json()) : null;
  }, [getSession, syncRevision]);
  const available = useMemo(() => {
    void syncRevision;
    return getSession()?.test_grid_tool_available() ?? false;
  }, [getSession, syncRevision]);
  const [status, setStatus] = useState("");

  // New test ids draw from bytes the host feeds: a stock whenever the tool or a grid shows and
  // after every id.
  const stock = view?.id ?? (active ? "" : null);
  useEffect(() => {
    if (stock === null) return;
    getSession()?.feed_test_grid_random(randomBytes());
  }, [getSession, stock]);

  // The refusal of a press on the canvas (the pointer path or Enter) shows after the re-read.
  const pressNotice = useMemo(() => {
    void syncRevision;
    return getSession()?.test_grid_notice() ?? null;
  }, [getSession, syncRevision]);

  const run = useCallback(
    (call: (session: WasmSession) => string): string => {
      const session = getSession();
      if (!session) return "";
      const cause = call(session);
      const spoken = session.take_test_grid_status();
      if (spoken) setStatus(spoken);
      sync();
      return cause;
    },
    [getSession, sync],
  );

  const placeAtFreeSpot = useCallback(() => {
    const session = getSession();
    if (!session) return;
    session.test_grid_press(0, 0, true);
    sync();
  }, [getSession, sync]);
  // Enter on the focused canvas with the tool active places a grid at the first free spot (G2).
  useEffect(() => {
    const element = container.current;
    if (!active || !element) return undefined;
    const onKey = (event: KeyboardEvent) => {
      if (event.key !== "Enter" || event.target !== element || event.repeat) return;
      event.preventDefault();
      placeAtFreeSpot();
    };
    element.addEventListener("keydown", onKey);
    return () => element.removeEventListener("keydown", onKey);
  }, [active, container, placeAtFreeSpot]);

  const setField = useCallback((key: string, typed: string) => run((s) => s.set_test_grid_field(key, typed)), [run]);
  const setKind = useCallback((kind: SetKind) => run((s) => s.set_test_grid_kind(kind)), [run]);
  const setAxis = useCallback((side: "columns" | "rows", param: Param) => run((s) => s.set_test_grid_axis(side, param)), [run]);
  const setCrosshatch = useCallback((on: boolean) => run((s) => s.set_test_grid_crosshatch(on)), [run]);
  const pickMaterial = useCallback((id: string) => run((s) => s.pick_test_grid_material(id)), [run]);

  return { available, view, pressNotice, status, placeAtFreeSpot, setField, setKind, setAxis, setCrosshatch, pickMaterial, refresh: sync };
}
