import { useCallback, useMemo } from "react";

import type { WasmSession } from "@/lib/editorSession";
import { decodeJson, type MarksView, type PrintAndCutView } from "@/lib/printAndCutView";

/** What Print and cut reads from the editor: the session and its re-read signal. */
export interface PrintAndCutEditor {
  getSession: () => WasmSession | null;
  syncRevision: number;
  sync: () => void;
}

/**
 * Print and cut's state and commands (`specs/0121-print-and-cut`): Add registration marks on a
 * Drawing page, the Registration marks block, and the Print and cut group of a laser page. The
 * session holds every rule; each command is one session call, then one re-read.
 */
export interface PrintAndCutApi {
  /** Whether the Document tab shows Add registration marks. */
  marksOffered: boolean;
  addMarks: () => void;
  /** The Registration marks block, `null` when the selection is no marks object. */
  marks: MarksView | null;
  setMarkSize: (diameter: number, line: number) => boolean;
  addMark: () => void;
  /** The Print and cut group, `null` when it is absent. */
  view: PrintAndCutView | null;
  chooseSheet: (id: unknown) => void;
  measure: (mark: number, x: number, y: number) => boolean;
  takeHead: (mark: number, x: number, y: number) => boolean;
  clear: (mark: number) => void;
  setTolerance: (mm: number) => boolean;
  /** The refusal text of Apply, empty on success. */
  apply: () => string;
  reset: () => void;
}

export function usePrintAndCut(editor: PrintAndCutEditor): PrintAndCutApi {
  const { getSession, syncRevision, sync } = editor;
  const read = useMemo(() => {
    void syncRevision; // The session's state changed: read the views again.
    const session = getSession();
    return {
      marksOffered: session?.registration_marks_offered() ?? false,
      marks: decodeJson<MarksView>(session?.registration_marks_view()),
      view: decodeJson<PrintAndCutView>(session?.print_and_cut_view()),
    };
  }, [getSession, syncRevision]);

  const run = useCallback(
    <T,>(call: (session: WasmSession) => T, fallback: T): T => {
      const session = getSession();
      if (!session) return fallback;
      const result = call(session);
      sync();
      return result;
    },
    [getSession, sync],
  );

  return {
    ...read,
    addMarks: () => run((s) => s.add_registration_marks(), false),
    setMarkSize: (diameter, line) => run((s) => s.set_registration_mark_size(diameter, line), false),
    addMark: () => run((s) => s.add_registration_mark(), false),
    chooseSheet: (id) => run((s) => s.print_and_cut_choose_sheet(JSON.stringify(id)), undefined),
    measure: (mark, x, y) => run((s) => s.print_and_cut_measure(mark, x, y), false),
    takeHead: (mark, x, y) => run((s) => s.print_and_cut_take_head_position(mark, x, y), false),
    clear: (mark) => run((s) => s.print_and_cut_clear(mark), false),
    setTolerance: (mm) => run((s) => s.print_and_cut_tolerance(mm), false),
    apply: () => run((s) => s.print_and_cut_apply(), ""),
    reset: () => run((s) => s.print_and_cut_reset(), false),
  };
}
