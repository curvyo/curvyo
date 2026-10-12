import { useCallback, useState } from "react";

import type { WasmSession } from "@/lib/editorSession";
import { pencilSampleValues, penReadoutText } from "@/lib/penInput";

/** The Pencil's pressure curve (`specs/0032-pen-tablet-input` criterion 14). */
export type PencilCurve = "soft" | "linear" | "firm";

/** What the Pencil bar shows (criteria 14 and 15). Session state: never saved. */
export interface PencilBarState {
  /** A pen has been seen in this session: the bar is shown. */
  penSeen: boolean;
  /** A stroke with pressure has been drawn: Min, Max and Curve are shown. */
  pressureSeen: boolean;
  /** "Pen: pressure 0.62, tilt 12° / −3°" while the last reading came from a pen, else null. */
  readout: string | null;
  minimumMm: number;
  maximumMm: number;
  curve: PencilCurve;
}

export interface PencilApi {
  pencilBar: PencilBarState;
  /** Enter in the Min field: `"committed"`, `"unchanged"` or `"invalid:number"`. */
  setPencilMinimum: (text: string) => string;
  /** Enter in the Max field: as the Min field. */
  setPencilMaximum: (text: string) => string;
  setPencilCurve: (curve: PencilCurve) => void;
}

const INITIAL: PencilBarState = {
  penSeen: false,
  pressureSeen: false,
  readout: null,
  minimumMm: 0.2,
  maximumMm: 2,
  curve: "linear",
};

function readBar(session: WasmSession): PencilBarState {
  const view = session.pencil_bar();
  const bar: PencilBarState = {
    penSeen: view.pen_seen,
    pressureSeen: view.pressure_seen,
    readout: view.has_readout ? penReadoutText(view.pressure, view.tilt_x, view.tilt_y) : null,
    minimumMm: view.minimum_mm,
    maximumMm: view.maximum_mm,
    curve: view.curve as PencilCurve,
  };
  view.free();
  return bar;
}

function sameBar(a: PencilBarState, b: PencilBarState): boolean {
  return (
    a.penSeen === b.penSeen &&
    a.pressureSeen === b.pressureSeen &&
    a.readout === b.readout &&
    a.minimumMm === b.minimumMm &&
    a.maximumMm === b.maximumMm &&
    a.curve === b.curve
  );
}

/** Parses a typed width in mm; the session clamps it. */
function parseWidth(text: string): number | null {
  const value = Number(text.trim().replace(",", "."));
  return text.trim() !== "" && Number.isFinite(value) ? value : null;
}

/**
 * The Pencil's frontend half: one wasm call per pointer event with all its samples
 * (`specs/0032-pen-tablet-input` criteria 1 and 2) and the Pencil bar's state and settings.
 */
export function usePencil(getSession: () => WasmSession | null) {
  const [pencilBar, setPencilBar] = useState<PencilBarState>(INITIAL);

  const sync = useCallback(() => {
    const session = getSession();
    if (!session) {
      return;
    }
    const next = readBar(session);
    setPencilBar((previous) => (sameBar(previous, next) ? previous : next));
  }, [getSession]);

  /** Hands a canvas pointer event to the Pencil. */
  const input = useCallback(
    (phase: "down" | "move" | "up", event: React.PointerEvent<HTMLCanvasElement>) => {
      const session = getSession();
      if (!session) {
        return;
      }
      const bounds = event.currentTarget.getBoundingClientRect();
      const values = pencilSampleValues(event.nativeEvent, bounds, phase === "move");
      session.pencil_samples(phase, event.pointerType, event.buttons, values);
      sync();
    },
    [getSession, sync],
  );

  const setWidth = useCallback(
    (text: string, current: number, apply: (session: WasmSession, mm: number) => void) => {
      const session = getSession();
      const value = parseWidth(text);
      if (!session) {
        return "unchanged";
      }
      if (value === null) {
        return "invalid:number";
      }
      if (value === current) {
        return "unchanged";
      }
      apply(session, value);
      sync();
      return "committed";
    },
    [getSession, sync],
  );

  const setPencilMinimum = useCallback(
    (text: string) =>
      setWidth(text, pencilBar.minimumMm, (session, mm) => session.set_pencil_minimum(mm)),
    [setWidth, pencilBar.minimumMm],
  );

  const setPencilMaximum = useCallback(
    (text: string) =>
      setWidth(text, pencilBar.maximumMm, (session, mm) => session.set_pencil_maximum(mm)),
    [setWidth, pencilBar.maximumMm],
  );

  const setPencilCurve = useCallback(
    (curve: PencilCurve) => {
      getSession()?.set_pencil_curve(curve);
      sync();
    },
    [getSession, sync],
  );

  return { pencilBar, setPencilMinimum, setPencilMaximum, setPencilCurve, input, sync };
}
