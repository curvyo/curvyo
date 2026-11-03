import { useCallback, useMemo } from "react";

import type { EditorSession } from "@/hooks/useEditorSession";
import { storeExtensions } from "@/lib/extensionsFile";
import { NO_WORKBENCHES, type WorkbenchList } from "@/workbenches/registry";

/** The active manufacturing's values (`manufacturing_view_json`). */
export interface ManufacturingView {
  id: string;
  name: string;
  process: string;
  /** The status bar's right block: machine and work area. */
  status: string;
  /** Host-bound values of `Params` blocks by key (`host.work_width` ...). */
  values: Record<string, number | string>;
  /** The texts of the `Notice` blocks that apply, by notice id. */
  notices: Record<string, string>;
  fixtures: {
    id: string;
    name: string;
    kind: string | null;
    icon: string | null;
    visible: boolean;
    values: Record<string, number | string>;
  }[];
  read_only_notice: { seq: number; text: string; x: number | null; y: number | null } | null;
}

/** The workbench registry read and the manufacturing commands; each command syncs the editor. */
export function useWorkbenches(editor: EditorSession) {
  const { getSession, sync, syncRevision } = editor;
  const list = useMemo<WorkbenchList>(() => {
    void syncRevision;
    const session = getSession();
    return session ? (JSON.parse(session.workbenches_json()) as WorkbenchList) : NO_WORKBENCHES;
  }, [getSession, syncRevision]);
  const view = useMemo<ManufacturingView | null>(() => {
    void syncRevision;
    const session = getSession();
    return session ? (JSON.parse(session.manufacturing_view_json()) as ManufacturingView | null) : null;
  }, [getSession, syncRevision]);
  const run = useCallback(
    <T,>(command: (session: NonNullable<ReturnType<typeof getSession>>) => T): T | undefined => {
      const session = getSession();
      if (!session) {
        return undefined;
      }
      const result = command(session);
      sync();
      return result;
    },
    [getSession, sync],
  );
  return {
    list,
    view,
    switchTo: (id: string) => {
      if (id === list.active) return;
      // The drawing tools' context ends before the canvas changes owner.
      editor.setTool("select");
      run((session) => session.set_workbench(id));
    },
    addManufacturing: (process: string) => run((session) => session.add_manufacturing(process)) ?? "",
    deleteManufacturing: (id: string) => run((session) => session.delete_manufacturing(id)),
    setWorkArea: (width: number, height: number) => run((session) => session.set_work_area(width, height)),
    addFixture: (kind: string) => run((session) => session.add_fixture(kind)),
    deleteFixture: (id: string) => run((session) => session.delete_fixture(id)),
    setFixtureVisible: (id: string, visible: boolean) =>
      run((session) => session.set_fixture_visible(id, visible)),
    setFixtureLength: (id: string, key: string, mm: number) =>
      run((session) => session.set_fixture_length(id, key, mm)),
    fitPage: () => run((session) => session.fit_output_page()),
    setExtensionEnabled: (id: string, enabled: boolean) => {
      const text = run((session) => session.set_extension_enabled(id, enabled));
      if (text !== undefined) {
        storeExtensions(text);
      }
    },
  };
}
export type WorkbenchesApi = ReturnType<typeof useWorkbenches>;
