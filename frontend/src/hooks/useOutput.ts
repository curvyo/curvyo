import { useCallback, useMemo } from "react";

import { useJobExport, type JobExportEditor } from "@/hooks/useJobExport";
import { useSendJob, type SendApi } from "@/hooks/useSendJob";
import type { WasmSession } from "@/lib/editorSession";
import { operationIdText, type OperationId } from "@/lib/operationsView";
import { decodeOutputGroupText, type OutputGroup, type StartValue } from "@/lib/outputView";

/** What the output group reads from the editor. */
export interface OutputEditor extends JobExportEditor {
  syncRevision: number;
}

/**
 * The output group's state and commands (`specs/0029-laser-job-preview-and-output` UX notes 6.3;
 * the session holds every rule, this hook only calls it and re-reads the view): the decoded group
 * of the active laser page, the Start and fixture material picks, the check buttons' selection and
 * Export job file. It does not grow `useEditorSession`.
 */
export interface OutputApi {
  /** The group of the active laser page; `null` on any other page. */
  view: OutputGroup | null;
  /** The fixture jobs' material, `""` removes the pick. */
  setFixtureMaterial: (id: string) => boolean;
  /** The Start pick: one step "Start: <option>". */
  setStart: (value: StartValue) => boolean;
  /** A check's count: selects these items on the canvas. */
  selectItems: (ids: OperationId[]) => boolean;
  /** The total's "M items": selects the running items (`specs/0101` criterion 17). */
  selectRunning: () => boolean;
  /** The Route toggle in the label of laser page `page`. */
  toggleRoute: (page: string) => boolean;
  /** Export job file. */
  exportJob: () => Promise<void>;
  /** Send to machine and the connector's state (`specs/0125`). */
  send: SendApi;
}

/** The output group's API for `editor`; `projectFile` names the project for the file name. */
export function useOutput(
  editor: OutputEditor,
  projectFile: () => string | null,
  onError: (message: string, title: string) => void,
): OutputApi {
  const { getSession, sync, syncRevision } = editor;
  const view = useMemo(() => {
    void syncRevision; // The session's state changed: read the view again.
    const session = getSession();
    return session ? decodeOutputGroupText(session.output_group_json()) : null;
  }, [getSession, syncRevision]);
  const run = useCallback(
    (call: (session: WasmSession) => boolean): boolean => {
      const session = getSession();
      if (!session) return false;
      const result = call(session);
      sync();
      return result;
    },
    [getSession, sync],
  );
  const setFixtureMaterial = useCallback((id: string) => run((s) => s.set_fixture_material(id)), [run]);
  const setStart = useCallback((value: StartValue) => run((s) => s.set_start(value)), [run]);
  const selectItems = useCallback(
    (ids: OperationId[]) => run((s) => s.select_output_items(JSON.stringify(ids.map(operationIdText)))),
    [run],
  );
  const selectRunning = useCallback(() => run((s) => s.select_operation_items("running")), [run]);
  const toggleRoute = useCallback((page: string) => run((s) => s.toggle_toolpath_view(page)), [run]);
  const exportJob = useJobExport(editor, projectFile, onError);
  const send = useSendJob(editor);
  return { view, setFixtureMaterial, setStart, selectItems, selectRunning, toggleRoute, exportJob, send };
}
