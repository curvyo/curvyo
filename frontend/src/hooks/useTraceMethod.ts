/** Applies the Trace method menu, the colour tolerance and the pipette to the choice. */
import { useCallback, useEffect, type MutableRefObject } from "react";

import type { WasmSession } from "@/lib/editorSession";
import { pickModeOf, type ViewState } from "@/lib/traceSource";
import {
  BRIGHTNESS,
  MULTICOLOUR,
  PICK_COLOURS,
  REMOVE_BACKGROUND,
  editChoice,
  pickSwatch,
  type TraceChoice,
  type TraceValues,
} from "@/lib/traceText";

export interface MethodContext {
  active: boolean;
  syncRevision: number;
  getSession: () => WasmSession | null;
  onChange: () => void;
  apply: (next: TraceChoice) => void;
  choiceRef: MutableRefObject<TraceChoice>;
  dragging: MutableRefObject<boolean>;
  viewRef: MutableRefObject<ViewState>;
  backgrounds: MutableRefObject<Map<string, string>>;
  /** A pick for the palette (Brightness + Multicolour, `specs/0089` UX notes §7). */
  palettePick: (hex: string) => void;
}

export function useTraceMethod(ctx: MethodContext) {
  const { active, syncRevision, getSession, onChange, apply, choiceRef, dragging, viewRef, backgrounds, palettePick } = ctx;
  const setPicking = useCallback(
    (armed: boolean) => {
      const { method, output } = choiceRef.current.values;
      getSession()?.set_trace_pick(armed ? pickModeOf(method, output) : "");
      onChange();
    },
    [getSession, onChange, choiceRef],
  );

  const setMethod = useCallback(
    (method: number) => {
      dragging.current = false;
      const id = viewRef.current.shown;
      const suggested = id ? backgrounds.current.get(id) : undefined;
      const current = choiceRef.current;
      // Auto is pressed on entering Remove background (UX notes §5).
      const values: TraceValues = {
        ...current.values,
        method,
        ...(method === REMOVE_BACKGROUND ? { backgroundAuto: true, background: suggested ?? current.values.background } : {}),
      };
      // The pipette is armed on entering Pick colours with no colour, disarmed otherwise.
      getSession()?.set_trace_pick(method === PICK_COLOURS && values.swatches.length === 0 ? "colours" : "");
      apply({ ...current, values });
      onChange();
    },
    [apply, getSession, onChange, choiceRef, dragging, viewRef, backgrounds],
  );

  const setColourTolerance = useCallback(
    (value: number, isDragging = false) => {
      dragging.current = isDragging;
      const key = choiceRef.current.values.method === REMOVE_BACKGROUND ? "backgroundTolerance" : "pickTolerance";
      apply(editChoice(choiceRef.current, key, value));
    },
    [apply, choiceRef, dragging],
  );

  const setBackgroundAuto = useCallback(() => {
    dragging.current = false;
    const id = viewRef.current.shown;
    const suggested = id ? (backgrounds.current.get(id) ?? null) : null;
    apply({ ...choiceRef.current, values: { ...choiceRef.current.values, backgroundAuto: true, background: suggested } });
  }, [apply]);

  const removeSwatch = useCallback(
    (hex: string) => {
      dragging.current = false;
      const values = choiceRef.current.values;
      apply({ ...choiceRef.current, values: { ...values, swatches: values.swatches.filter((s) => s !== hex) } });
    },
    [apply, choiceRef, dragging],
  );

  // A pick of the pipette on the canvas (`specs/0089` criteria 16, 17; Frage 89 A): Remove
  // background takes it as its colour and releases Auto; Pick colours replaces its colours with
  // it, or adds it with Shift.
  useEffect(() => {
    const session = getSession();
    if (!active || !session) {
      return;
    }
    const [hex, how] = session.take_trace_pick();
    if (!hex) {
      return;
    }
    dragging.current = false;
    const values = choiceRef.current.values;
    if (values.method === REMOVE_BACKGROUND) {
      apply({ ...choiceRef.current, values: { ...values, backgroundAuto: false, background: hex } });
    } else if (values.method === PICK_COLOURS) {
      const swatches = pickSwatch(values.swatches, hex, how === "add");
      if (swatches.join() !== values.swatches.join()) {
        apply({ ...choiceRef.current, values: { ...values, swatches } });
      }
    } else if (values.method === BRIGHTNESS && values.output === MULTICOLOUR) {
      palettePick(hex);
    }
  }, [syncRevision, active, getSession, apply, choiceRef, dragging, palettePick]);

  return { setPicking, setMethod, setColourTolerance, setBackgroundAuto, removeSwatch };
}
