import { useCallback, useState } from "react";

import type { WasmSession } from "@/lib/editorSession";
import { decodeOperationId, operationIdText, type OperationId } from "@/lib/operationsView";

/** What the operation tools read from the editor: the session and its re-read signal. */
export interface OperationToolsEditor {
  getSession: () => WasmSession | null;
  sync: () => void;
}

/** The row a kind press last wrote to; `seq` makes two presses on the same row two events. */
export interface PressedRow {
  id: OperationId;
  seq: number;
  /** A reveal from the Output tab: the Operations tab is drawn after it, so it replays it. */
  reveal?: boolean;
}

/**
 * The laser operation tools' commands (`specs/0111-laser-operation-tools` Part C): the Select
 * bar's kind buttons, the unfolded row, the options of its lower part and the Operations view
 * toggle. The session holds every rule; this hook only calls it and re-reads the view. It is the
 * only caller of these bindings and does not grow `useEditorSession`.
 */
export interface OperationToolsApi {
  /** A kind button press, `shift` held for a new operation; the row written to, `null` when
   * nothing changed. */
  press: (kind: string, shift: boolean) => OperationId | null;
  /** The row of the last press, for the tab to select and scroll to; `null` before the first. */
  pressed: PressedRow | null;
  /** The polite status of the last press ("Cut 3 added."), for the tab's sr-only region. */
  status: string;
  /** A press on a row: it unfolds, or folds when it was unfolded. */
  unfold: (id: OperationId) => boolean;
  /** Types `value` into an option field or presses a toggle; the chip text, empty when taken. */
  edit: (id: OperationId, option: string, value: number) => string;
  /** A pick of the Dither mode of Image row `id` (`specs/0123`); whether it was written. */
  setDither: (id: OperationId, mode: string) => boolean;
  /** The Operations view toggle in the label of laser page `page`. */
  toggleView: (page: string) => boolean;
  /** A check's operation button of the Output tab (`specs/0029`): the row is selected and stays
   * unfolded; whether it exists on the active page. */
  reveal: (id: OperationId) => boolean;
  /** Escape on a kind button inside an entered group: the session leaves one level, as Escape
   * on the canvas does; whether it left. */
  leaveGroup: () => boolean;
  /** The Tabs toggle of the Select bar (`specs/0139` UX 4.2); whether it changed. */
  setTabsTool: (on: boolean) => boolean;
  /** The Tabs pill of the item part of row `id`: Own or the operation's; whether it wrote. */
  setTabsOwn: (id: OperationId, own: boolean) => boolean;
  /** The current tab's width in mm, `null` for the operation's; the chip text, empty when taken. */
  setTabWidth: (mm: number | null) => string;
}

/** The operation tools' API for `editor`. */
export function useOperationTools(editor: OperationToolsEditor): OperationToolsApi {
  const { getSession, sync } = editor;
  const [pressed, setPressed] = useState<PressedRow | null>(null);
  const [status, setStatus] = useState("");

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
    (kind: string, shift: boolean) => {
      const session = getSession();
      if (!session) return null;
      const text = session.press_operation_kind(kind, shift);
      const id = text === undefined ? null : decodeOperationId(text);
      // The status is read either way, so a polite text never waits for the next press.
      const said = session.take_operations_status();
      if (said !== "") setStatus(said);
      if (id) setPressed((previous) => ({ id, seq: (previous?.seq ?? 0) + 1 }));
      sync();
      return id;
    },
    [getSession, sync],
  );
  const unfold = useCallback(
    (id: OperationId) => run(false, (s) => s.unfold_operation_row(operationIdText(id))),
    [run],
  );
  const edit = useCallback(
    (id: OperationId, option: string, value: number) =>
      run("", (s) => s.edit_operation_option(operationIdText(id), option, value)),
    [run],
  );
  const setDither = useCallback(
    (id: OperationId, mode: string) => run(false, (s) => s.set_operation_dither(operationIdText(id), mode)),
    [run],
  );
  const reveal = useCallback(
    (id: OperationId) => {
      const done = run(false, (s) => s.reveal_operation_row(operationIdText(id)));
      if (done) setPressed((previous) => ({ id, seq: (previous?.seq ?? 0) + 1, reveal: true }));
      return done;
    },
    [run],
  );
  const toggleView = useCallback((page: string) => run(false, (s) => s.toggle_operations_view(page)), [run]);

  const leaveGroup = useCallback(
    () =>
      run(
        false,
        (s) => s.key_down("Escape", false, false, false, false, false, false, performance.now()) === "escape-group",
      ),
    [run],
  );

  const setTabsTool = useCallback((on: boolean) => run(false, (s) => s.set_tabs_tool(on)), [run]);
  const setTabsOwn = useCallback(
    (id: OperationId, own: boolean) => run(false, (s) => s.set_selection_tabs_own(operationIdText(id), own)),
    [run],
  );
  const setTabWidth = useCallback(
    (mm: number | null) => run("", (s) => s.set_current_tab_width(mm ?? Number.NaN)),
    [run],
  );

  return { press, pressed, status, unfold, edit, setDither, toggleView, reveal, leaveGroup, setTabsTool, setTabsOwn, setTabWidth };
}
