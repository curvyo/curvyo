import { useCallback, useMemo } from "react";

import type { EditorSession } from "@/hooks/useEditorSession";
import { storeExtensions } from "@/lib/extensionsFile";
import { flushMachineWrites } from "@/lib/machineProfilesFile";
import { NO_WORKBENCHES, type WorkbenchList } from "@/workbenches/registry";

/** The active manufacturing's values (`manufacturing_view_json`). */
export interface ManufacturingView {
  id: string;
  name: string;
  process: string;
  /** The status bar's right block: machine and work area. */
  status: string;
  /** The Machine block's values (`specs/0026-machine-profile`), `null` for a workbench without one. */
  machine: MachineView | null;
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

/** A corner of the output page, as stored. */
export type OriginCorner = "top_left" | "top_right" | "bottom_left" | "bottom_right";

/** One row of the machine profile list. */
export interface MachineProfileRow {
  id: string;
  name: string;
  width: number;
  height: number;
  builtin: boolean;
  chosen: boolean;
}

/** The Machine block's values: the copy, its state against the library, the library rows. */
export interface MachineView {
  copy: {
    id: string | null;
    name: string;
    origin: OriginCorner;
    max_feed: number;
    max_power: number;
    laser_mode: boolean;
    width: number;
    height: number;
  };
  builtin: boolean;
  state: "differs" | "missing" | null;
  can_update: boolean;
  name_notice: string | null;
  connection: { connector_address: string; controller_port: string; baud_rate: number } | null;
  library_notices: string[];
  profiles: MachineProfileRow[];
}

/** 16 random bytes for a new profile id (core has no random source, ADR 0038 §5). */
function randomBytes(): Uint8Array {
  const bytes = new Uint8Array(16);
  crypto.getRandomValues(bytes);
  return bytes;
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
      void flushMachineWrites(session, sync);
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
    setWorkArea: (width: number, height: number) =>
      run((session) => session.set_work_area(width, height)) ?? "",
    machine: {
      pick: (id: string) => run((session) => session.pick_machine_profile(id)),
      setName: (text: string) => run((session) => session.set_machine_name(text)) ?? "",
      setOrigin: (corner: OriginCorner) => run((session) => session.set_machine_origin(corner)),
      setMaxFeed: (value: number) => run((session) => session.set_machine_max_feed(value)) ?? "",
      setMaxPower: (value: number) => run((session) => session.set_machine_max_power(value)) ?? "",
      setLaserMode: (on: boolean) => run((session) => session.set_machine_laser_mode(on)),
      reset: () => run((session) => session.reset_machine_profile()),
      saveAs: () => run((session) => session.save_machine_profile(randomBytes(), Date.now())) ?? "",
      update: () => run((session) => session.update_machine_profile(Date.now())) ?? "",
      duplicate: (id: string) =>
        run((session) => session.duplicate_machine_profile(id, randomBytes(), Date.now())) ?? "",
      rename: (id: string, name: string) =>
        run((session) => session.rename_machine_profile(id, name, Date.now())) ?? "",
      remove: (id: string) => run((session) => session.delete_machine_profile(id)),
      setConnection: (address: string, port: string, baud: number) =>
        run((session) => session.set_machine_connection(address, port, baud, Date.now())) ?? "",
    },
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
