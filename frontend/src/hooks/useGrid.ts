import { useCallback, useMemo } from "react";

import type { WasmSession } from "@/lib/editorSession";

/** The page corner the grid starts at and the rulers count from (`specs/0090-grid` criterion 16). */
export type OriginCorner = "top_left" | "top_right" | "bottom_left" | "bottom_right";

const CORNERS: readonly OriginCorner[] = ["top_left", "top_right", "bottom_left", "bottom_right"];

/** The active page's grid as the Grid & Rulers panel shows it (`specs/0090-grid` criteria 2, 3, 18). */
export interface GridView {
  pageName: string;
  shown: boolean;
  mode: "adaptive" | "fixed";
  scale: "linear" | "log";
  spacing: string;
  subdivision: string;
  angle: string;
  origin: OriginCorner;
  unit: string;
}

/** A typed Grid field. */
export type GridField = "spacing" | "subdivision" | "angle";

/** Grid API for the rail entry and the Grid panel. */
export interface GridApi {
  view: GridView;
  /** Shift+click on the rail entry: shows or hides the active page's grid. Returns the new state. */
  toggle: () => boolean;
  setShown: (shown: boolean) => boolean;
  setMode: (mode: GridView["mode"]) => boolean;
  setScale: (scale: GridView["scale"]) => boolean;
  /** A typed field: "committed", "unchanged" or "invalid:<key>". */
  setField: (field: GridField, text: string) => string;
  /** The origin corner buttons: one step per change. */
  setOrigin: (corner: OriginCorner) => boolean;
}

/** The notices of a refused field (`specs/0090-grid` UX notes §2), keyed by the session's outcome. */
export const GRID_MESSAGES = {
  spacing_number: "Enter a number greater than 0",
  spacing_positive: "Spacing must be greater than 0",
  subdivision: "Enter a whole number, 2 or more",
  number: "Enter a number",
} as const;

const DEFAULT_VIEW: GridView = {
  pageName: "Page 1",
  shown: false,
  mode: "adaptive",
  scale: "linear",
  spacing: "5",
  subdivision: "5",
  angle: "0",
  origin: "top_left",
  unit: "mm",
};

function readGrid(session: WasmSession | null): GridView {
  if (!session) {
    return DEFAULT_VIEW;
  }
  const raw = session.active_page_grid();
  const view: GridView = {
    pageName: raw.page_name,
    shown: raw.shown,
    mode: raw.mode === "fixed" ? "fixed" : "adaptive",
    scale: raw.scale === "log" ? "log" : "linear",
    spacing: raw.spacing,
    subdivision: raw.subdivision,
    angle: raw.angle,
    origin: CORNERS.find((corner) => corner === raw.origin) ?? "top_left",
    unit: raw.unit,
  };
  raw.free();
  return view;
}

interface EditorHandle {
  getSession: () => WasmSession | null;
  syncRevision: number;
  sync: () => void;
}

/**
 * The active page's grid and its commands (`specs/0090-grid`). The snapshot is re-read whenever
 * the session says its UI state changed (`syncRevision`), so undo, redo, merges and a page switch
 * show at once.
 */
export function useGrid(editor: EditorHandle): GridApi {
  const { getSession, syncRevision, sync } = editor;
  const view = useMemo(() => {
    void syncRevision; // Host revisions invalidate this authoritative document snapshot.
    return readGrid(getSession());
  }, [getSession, syncRevision]);

  const run = useCallback(
    <T,>(fallback: T, call: (session: WasmSession) => T): T => {
      const session = getSession();
      if (!session) {
        return fallback;
      }
      const result = call(session);
      sync();
      return result;
    },
    [getSession, sync],
  );

  const toggle = useCallback(() => run(false, (s) => s.toggle_active_page_grid()), [run]);
  const setShown = useCallback(
    (shown: boolean) => run(false, (s) => s.set_active_page_grid_shown(shown)),
    [run],
  );
  const setMode = useCallback(
    (mode: GridView["mode"]) => run(false, (s) => s.set_active_page_grid_mode(mode)),
    [run],
  );
  const setScale = useCallback(
    (scale: GridView["scale"]) => run(false, (s) => s.set_active_page_grid_scale(scale)),
    [run],
  );
  const setField = useCallback(
    (field: GridField, text: string) =>
      run("unchanged", (s) => s.set_active_page_grid_field(field, text)),
    [run],
  );
  const setOrigin = useCallback(
    (corner: OriginCorner) => run(false, (s) => s.set_active_page_origin(corner)),
    [run],
  );

  return { view, toggle, setShown, setMode, setScale, setField, setOrigin };
}
