import { useCallback, useEffect, useMemo, useState } from "react";

import type { WasmSession } from "@/lib/editorSession";
import { operationIdText, type OperationId } from "@/lib/operationsView";
import type { SetKind } from "@/lib/operationSettingsView";
import { decodeAddResult, decodeTestGridText, type TestGridView } from "@/lib/testGridView";

/** What the Grid block reads from the editor: the session and its re-read signal. */
export interface TestGridEditor {
  getSession: () => WasmSession | null;
  syncRevision: number;
  sync: () => void;
}

/**
 * The test grid's commands (`specs/0028-material-test-library` `ux-notes.md` sections 4 to 7; the
 * session holds every rule, this hook only calls it and re-reads the view). It does not grow
 * `useEditorSession`.
 */
export interface TestGridApi {
  /** The grid the canvas selection or the pressed row names; `null` when none shows. */
  view: TestGridView | null;
  /** The refusal of the last Add test grid, shown 8 s under the button. */
  notice: string | null;
  /** Add test grid: the new row, `null` when refused (then `notice` says why). */
  add: () => OperationId | null;
  /** A grid row pressed in the list; `null` when another row is. */
  selectRow: (row: OperationId | null) => void;
  /** Types `typed` into field `key`; the chip text, empty when taken. */
  setField: (key: string, typed: string) => string;
  setKind: (kind: SetKind) => string;
  setLabels: (on: boolean) => string;
  rebuild: () => boolean;
  pressCell: (column: number, row: number) => boolean;
  /** The block's "Select": the grid becomes the canvas selection. */
  selectOnCanvas: () => boolean;
}

const NOTICE_MS = 8000;

/** The Grid block's API for `editor`. */
export function useTestGrid(editor: TestGridEditor): TestGridApi {
  const { getSession, syncRevision, sync } = editor;
  const view = useMemo(() => {
    void syncRevision;
    const session = getSession();
    return session ? decodeTestGridText(session.test_grid_view_json()) : null;
  }, [getSession, syncRevision]);
  const [notice, setNotice] = useState<string | null>(null);

  useEffect(() => {
    if (notice === null) return;
    const timer = window.setTimeout(() => setNotice(null), NOTICE_MS);
    return () => window.clearTimeout(timer);
  }, [notice]);

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

  const add = useCallback(
    () =>
      run<OperationId | null>(null, (s) => {
        const result = decodeAddResult(s.press_add_test_grid());
        setNotice(result.ok ? null : result.text);
        return result.ok ? result.row : null;
      }),
    [run],
  );
  const selectRow = useCallback(
    (row: OperationId | null) => run(undefined, (s) => s.select_test_grid_row(row ? operationIdText(row) : "")),
    [run],
  );
  const setField = useCallback((key: string, typed: string) => run("", (s) => s.set_test_grid_field(key, typed)), [run]);
  const setKind = useCallback((kind: SetKind) => run("", (s) => s.set_test_grid_kind(kind)), [run]);
  const setLabels = useCallback((on: boolean) => run("", (s) => s.set_test_grid_labels(on)), [run]);
  const rebuild = useCallback(() => run(false, (s) => s.rebuild_test_grid()), [run]);
  const pressCell = useCallback((column: number, row: number) => run(false, (s) => s.press_test_cell(column, row)), [run]);
  const selectOnCanvas = useCallback(() => run(false, (s) => s.select_test_grid_on_canvas()), [run]);

  return { view, notice, add, selectRow, setField, setKind, setLabels, rebuild, pressCell, selectOnCanvas };
}
