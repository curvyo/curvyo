import { useCallback, useEffect, useState } from "react";

import type { WasmSession } from "@/lib/editorSession";

/** What the ellipse group and the "Curve" field show (`specs/0021-ellipse-arcs-and-shaping/`
 * criteria 3, 26, 44): read from the session after every sync. A `*Grey` string is empty
 * while the control applies, else the reason (`"curve"`, `"no-arc"`, `"arc"`). */
export interface ArcBarState {
  ellipseShown: boolean;
  /** `"pie"`, `"arc"`, `"chord"`, `"mixed"`, or `""` while no selected ellipse has an arc. */
  kind: string;
  kindGrey: string;
  start: number;
  startMixed: boolean;
  end: number;
  endMixed: boolean;
  anglesGrey: string;
  curveShown: boolean;
  /** Percent. */
  curve: number;
  curveMixed: boolean;
  curveGrey: string;
  curveLimited: boolean;
  /** The stored percent, for the "limited" tag's tooltip. */
  curveStored: number;
}

export const EMPTY_ARC_BAR_STATE: ArcBarState = {
  ellipseShown: false,
  kind: "",
  kindGrey: "",
  start: 0,
  startMixed: false,
  end: 0,
  endMixed: false,
  anglesGrey: "",
  curveShown: false,
  curve: 0,
  curveMixed: false,
  curveGrey: "",
  curveLimited: false,
  curveStored: 0,
};

export interface ArcBarApi {
  state: ArcBarState;
  setKind: (kind: "pie" | "arc" | "chord") => void;
  makeWhole: () => void;
  /** Enter in "Start" or "End": `"committed"`, `"unchanged"` or `"invalid:number"`. */
  setAngle: (end: boolean, text: string) => string;
  /** Enter in "Curve": as [`setAngle`]. */
  setCurve: (text: string) => string;
  previewCurve: (percent: number) => void;
  commitCurve: () => void;
  cancelCurve: () => void;
}

function read(session: WasmSession): ArcBarState {
  const view = session.arc_bar_state();
  const state: ArcBarState = {
    ellipseShown: view.ellipse_shown,
    kind: view.kind,
    kindGrey: view.kind_grey,
    start: view.start,
    startMixed: view.start_mixed,
    end: view.end,
    endMixed: view.end_mixed,
    anglesGrey: view.angles_grey,
    curveShown: view.curve_shown,
    curve: view.curve,
    curveMixed: view.curve_mixed,
    curveGrey: view.curve_grey,
    curveLimited: view.curve_limited,
    curveStored: view.curve_stored,
  };
  view.free();
  return state;
}

function same(a: ArcBarState, b: ArcBarState): boolean {
  return (Object.keys(a) as (keyof ArcBarState)[]).every((key) => a[key] === b[key]);
}

/**
 * The Select bar's ellipse group and "Curve" field in the frontend: forwards every edit to
 * the session, re-reads the state after each sync (`revision`) and speaks the session's
 * announcement of the edit (criterion 65). Every rule is the session's.
 */
export function useArcBar(
  getSession: () => WasmSession | null,
  onChange: () => void,
  announceText: (text: string) => void,
  revision: number,
): ArcBarApi {
  const [state, setState] = useState<ArcBarState>(EMPTY_ARC_BAR_STATE);

  useEffect(() => {
    const session = getSession();
    if (!session) {
      return;
    }
    const next = read(session);
    setState((previous) => (same(previous, next) ? previous : next));
  }, [getSession, revision]);

  /** Runs `edit`, syncs and speaks what the session announces. */
  const run = useCallback(
    <T,>(edit: (session: WasmSession) => T, fallback: T): T => {
      const session = getSession();
      if (!session) {
        return fallback;
      }
      const result = edit(session);
      onChange();
      const announcement = session.take_transform_announcement();
      if (announcement !== "") {
        announceText(announcement);
      }
      return result;
    },
    [getSession, onChange, announceText],
  );

  const setKind = useCallback(
    (kind: "pie" | "arc" | "chord") => {
      run((session) => session.set_selected_arc_kind(kind), "");
    },
    [run],
  );
  const makeWhole = useCallback(() => {
    run((session) => session.make_selected_ellipses_whole(), "");
  }, [run]);
  const setAngle = useCallback(
    (end: boolean, text: string) =>
      run((session) => session.set_selected_arc_angle(end, text), "unchanged"),
    [run],
  );
  const setCurve = useCallback(
    (text: string) => run((session) => session.set_selected_curve(text), "unchanged"),
    [run],
  );
  const previewCurve = useCallback(
    (percent: number) => {
      getSession()?.preview_selected_curve(percent);
      onChange();
    },
    [getSession, onChange],
  );
  const commitCurve = useCallback(() => {
    run((session) => session.commit_selected_curve(), undefined);
  }, [run]);
  const cancelCurve = useCallback(() => {
    getSession()?.cancel_selected_curve();
    onChange();
  }, [getSession, onChange]);

  return { state, setKind, makeWhole, setAngle, setCurve, previewCurve, commitCurve, cancelCurve };
}
