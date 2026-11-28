import { useCallback, useMemo } from "react";

import type { EditorSession } from "@/hooks/useEditorSession";
import { storeExtensions } from "@/lib/extensionsFile";
import { flushMachineWrites } from "@/lib/machineProfilesFile";
import { flushMaterialWrites } from "@/lib/materialsFile";
import type { CncView } from "@/lib/cncView";
import type { EmbroideryView } from "@/lib/embroideryView";
import type { PlotterView } from "@/lib/plotterView";
import { decodeMachinesView, type MachinePicker, type MachinesView } from "@/lib/machinesView";
import { NO_WORKBENCHES, type WorkbenchList } from "@/workbenches/registry";

/** The active laser page's values (`manufacturing_view_json`), `null` on another page. */
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
  /** The Machine group of every output page (ADR 0098 decision 4). */
  picker?: MachinePicker | null;
  /** The plotter page's own view (`specs/0159`), present on a plotter page only. */
  plotter?: PlotterView | null;
  /** The embroidery page's own view (`specs/0160`), present on an embroidery page only. */
  embroidery?: EmbroideryView | null;
  /** The CNC page's own view (`specs/0161`), present on a CNC page only. */
  cnc?: CncView | null;
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
    /** Default power in percent, one decimal (`specs/0150` criterion 1). */
    default_power: number;
    /** Default speed in mm/min, at most `max_feed`. */
    default_speed: number;
    laser_mode: boolean;
    /** The machine has a motorised Z axis (`specs/0111` criterion 22). */
    motorised_z: boolean;
    /** Air assist is on (`specs/0029` criterion 25). */
    air_assist: boolean;
    /** The Focus point mode's pulse (`specs/0118` 35 a): on, its power in percent, its length. */
    focus_pulse: boolean;
    pulse_power: number;
    pulse_length: number;
    /** The output format's name, "GRBL", or "unknown" for a damaged value (criterion 39). */
    output: string;
    width: number;
    height: number;
  };
  builtin: boolean;
  state: "differs" | "missing" | null;
  can_update: boolean;
  /** The profile's fixture count, "Fixtures in profile: N" (`specs/0102` 9.1). */
  fixtures_in_profile: number;
  /** How many fixture sets the profile holds (`0113`). */
  fixture_sets_in_profile: number;
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

/** The page kinds read and the laser page commands; each command syncs the editor. */
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
      void flushMaterialWrites(session, sync);
      return result;
    },
    [getSession, sync],
  );
  const profiles = useMemo<MachineProfileRow[]>(() => {
    void syncRevision;
    const session = getSession();
    return session ? (JSON.parse(session.machine_profiles_json()) as MachineProfileRow[]) : [];
  }, [getSession, syncRevision]);
  const embroideryProfiles = useMemo<MachineProfileRow[]>(() => {
    void syncRevision;
    const session = getSession();
    return session ? (JSON.parse(session.embroidery_profiles_json()) as MachineProfileRow[]) : [];
  }, [getSession, syncRevision]);
  const cncProfiles = useMemo<MachineProfileRow[]>(() => {
    void syncRevision;
    const session = getSession();
    return session ? (JSON.parse(session.cnc_profiles_json()) as MachineProfileRow[]) : [];
  }, [getSession, syncRevision]);
  const machinesView = useMemo<MachinesView | null>(() => {
    void syncRevision;
    const session = getSession();
    return session ? decodeMachinesView(session.machines_view_json()) : null;
  }, [getSession, syncRevision]);
  const plotterProfiles = useMemo<MachineProfileRow[]>(() => {
    void syncRevision;
    const session = getSession();
    return session ? (JSON.parse(session.plotter_profiles_json()) as MachineProfileRow[]) : [];
  }, [getSession, syncRevision]);
  return {
    list,
    view,
    /** The machine list of the Add page group, the starting profile chosen. */
    profiles,
    /** The machine lists of the other families' kinds, by process (`specs/0159` criterion 2). */
    familyProfiles: {
      "curvyo.cutter": plotterProfiles,
      "curvyo.embroidery": embroideryProfiles,
      "curvyo.cnc": cncProfiles,
    } as Record<string, MachineProfileRow[]>,
    /** The mat of a plotter page (`specs/0159`, Fixtures tab); each returns what the session says. */
    plotter: {
      setMat: (size: string) => run((session) => session.set_plotter_mat(size)) ?? false,
      setGrip: (grip: string) => run((session) => session.set_plotter_grip(grip)) ?? false,
      setGrid: (on: boolean) => run((session) => session.set_plotter_mat_grid(on)) ?? false,
    },
    /** The hoop of an embroidery page (`specs/0160`, Fixtures tab). */
    embroidery: {
      setHoop: (key: string) => run((session) => session.set_embroidery_hoop(key)) ?? false,
    },
    /** The stock and tools of a CNC page (`specs/0161`, Fixtures and Tools tabs). */
    cnc: {
      setStock: (field: string, text: string) => run((session) => session.edit_cnc_stock(field, text)) ?? "",
      addTool: () => run((session) => session.add_cnc_tool(randomBytes())) ?? "",
      setTool: (id: string, field: string, text: string) => run((session) => session.edit_cnc_tool(id, field, text)) ?? "",
      removeTool: (id: string) => run((session) => session.remove_cnc_tool(id)) ?? false,
    },
    /** The Machine group of any output page (ADR 0098 decision 4): pick, reset and save. */
    output: {
      pick: (id: string) => run((session) => session.pick_output_profile(id)) ?? false,
      reset: () => run((session) => session.reset_output_profile()) ?? false,
      saveAs: () => run((session) => session.save_output_profile(randomBytes(), Date.now())) ?? "",
    },
    /** The Machines tab (ADR 0098 decisions 4 and 5): its view and its edits. */
    machines: {
      view: machinesView,
      /** A field from typed text: `{ notice }`, `{ error }` or `{}`. */
      set: (id: string, field: string, text: string) =>
        JSON.parse(run((session) => session.set_machine_value(id, field, text, Date.now())) ?? "{}") as {
          notice?: string;
          error?: string;
        },
      duplicate: (id: string) => run((session) => session.duplicate_machine(id, randomBytes(), Date.now())) ?? "",
      /** "+ Machine" of a family section: a copy of its built-in, one step; the new id. */
      add: (family: string) => run((session) => session.add_machine(family, randomBytes(), Date.now())) ?? "",
      rename: (id: string, name: string) => run((session) => session.rename_machine(id, name, Date.now())) ?? false,
      /** The Connection group of laser machine `id`: a library write without a step; the notice. */
      setConnection: (id: string, address: string, port: string, baud: number) =>
        run((session) => session.set_machine_connection_of(id, address, port, baud, Date.now())) ?? "",
      /** The size offer of laser machine `id` (`specs/0131`), read on every render. */
      sizeApi: (id: string) => ({
        sizeOffer: (x: number, y: number): [number, number] | null => {
          const offer = getSession()?.machine_size_offer_of(id, x, y);
          return offer !== undefined && offer.length === 2 ? [offer[0], offer[1]] : null;
        },
        sizeReported: (x: number, y: number): boolean => getSession()?.machine_size_reported_of(id, x, y) ?? false,
        useSize: (x: number, y: number) => run((session) => session.use_machine_size_of(id, x, y, Date.now())) ?? false,
      }),
      remove: (id: string) => run((session) => session.delete_machine(id)) ?? false,
      follow: () => run((session) => session.follow_machine()) ?? false,
      keep: () => {
        run((session) => session.keep_machine_pages());
      },
    },
    /** Add page with the Kind switch (`specs/0098` criteria 1 to 4); the new page's id. */
    addPage: (kind: string, profile: string) => {
      // The drawing tools' context ends before the canvas changes owner.
      editor.setTool("select");
      return run((session) => session.add_page_of_kind(kind, profile)) ?? "";
    },
    /** Place page (`specs/0100` criterion 15): drawing page `id` at the active laser page's
     * centre. Whether it was placed. */
    placePage: (id: string) => run((session) => session.place_page_on_active(id)) ?? false,
    setExtensionEnabled: (id: string, enabled: boolean) => {
      const text = run((session) => session.set_extension_enabled(id, enabled));
      if (text !== undefined) {
        storeExtensions(text);
      }
    },
  };
}
export type WorkbenchesApi = ReturnType<typeof useWorkbenches>;
