import { useCallback, useMemo, useRef } from "react";

import type { WasmSession } from "@/lib/editorSession";
import {
  OFFSET_IDLE,
  type OffsetDirectionName,
  type OffsetJoinName,
  type OffsetOutcome,
  type OffsetResultName,
  type OffsetView,
  offsetRefusalText,
  offsetSuccessText,
} from "@/lib/offsetText";

/** The Offset tool in the frontend (`specs/0038-path-offset`): the bar's view and values, Apply,
 * and the notice after a release. */
export interface OffsetApi {
  view: OffsetView;
  setDistance: (mm: number) => boolean;
  setDirection: (direction: OffsetDirectionName) => void;
  setJoin: (join: OffsetJoinName) => void;
  setResult: (result: OffsetResultName) => void;
  apply: () => void;
  /** After a pointer release: shows the notice of what the release did, if anything. */
  showReleaseOutcome: () => void;
  /** The Distance field, which a double-click on a handle focuses (criterion 13). */
  distanceRef: React.RefObject<HTMLInputElement | null>;
}

const REFUSAL_MS = 8000;

type ShowNotice = (
  text: string,
  ms?: number,
  source?: "canvas" | "panel",
  kind?: "status" | "alert",
) => void;

/** Reads the session's view of the Offset tool. */
export function readOffsetView(session: WasmSession | null): OffsetView {
  const raw = session?.offset_view();
  if (!raw) {
    return OFFSET_IDLE;
  }
  const view: OffsetView = {
    operands: raw.operands,
    hasGroup: raw.has_group,
    handles: raw.handles,
    dragging: raw.dragging,
    distanceMm: raw.distance_mm,
    both: raw.both,
    change: raw.change,
    stop: raw.stop,
    vanished: raw.vanished,
    of: raw.of,
    large: raw.large,
    refusal: raw.refusal,
    refusalCount: raw.refusal_count,
    refusalOf: raw.refusal_of,
    valuesDistanceMm: raw.values_distance_mm,
    direction: raw.direction as OffsetDirectionName,
    join: raw.join as OffsetJoinName,
    result: raw.result as OffsetResultName,
    minMm: raw.min_mm,
    maxMm: raw.max_mm,
  };
  raw.free();
  return view;
}

function readOutcome(raw: ReturnType<WasmSession["apply_offset"]>): OffsetOutcome {
  const outcome: OffsetOutcome = {
    added: raw.added,
    changed: raw.changed,
    count: raw.count,
    of: raw.of,
    vanished: raw.vanished,
    distanceMm: raw.distance_mm,
    direction: raw.direction,
  };
  raw.free();
  return outcome;
}

/**
 * The Offset tool's bar state and commands. Every decision (what applies, the preview, why a
 * release is refused) is the session's; this hook reads it after every sync and turns outcomes
 * into the canvas notice.
 */
export function useOffset(
  getSession: () => WasmSession | null,
  onChange: () => void,
  syncRevision: number,
  showNotice: ShowNotice,
): OffsetApi {
  const view = useMemo(
    () => readOffsetView(getSession()),
    // The session is read again after every sync (`syncRevision`).
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [getSession, syncRevision],
  );
  const distanceRef = useRef<HTMLInputElement | null>(null);

  const announce = useCallback(
    (outcome: OffsetOutcome) => {
      const success = offsetSuccessText(outcome);
      const refusal = offsetRefusalText(outcome);
      if (success !== null) {
        showNotice(success, 3000, "canvas", "status");
      } else if (refusal !== null) {
        showNotice(refusal, REFUSAL_MS, "canvas", "alert");
      }
    },
    [showNotice],
  );

  const call = useCallback(
    (run: (session: WasmSession) => void) => {
      const session = getSession();
      if (session) {
        run(session);
        onChange();
      }
    },
    [getSession, onChange],
  );

  const setDistance = useCallback(
    (mm: number) => {
      const taken = getSession()?.set_offset_distance(mm) ?? false;
      onChange();
      return taken;
    },
    [getSession, onChange],
  );

  const apply = useCallback(() => {
    const session = getSession();
    if (!session) {
      return;
    }
    const outcome = readOutcome(session.apply_offset());
    onChange();
    announce(outcome);
  }, [getSession, onChange, announce]);

  const showReleaseOutcome = useCallback(() => {
    const session = getSession();
    if (!session) {
      return;
    }
    const outcome = readOutcome(session.take_offset_outcome());
    if (outcome.kind !== "ignored") {
      announce(outcome);
    }
  }, [getSession, announce]);

  return {
    view,
    setDistance,
    setDirection: (direction) => call((session) => session.set_offset_direction(direction)),
    setJoin: (join) => call((session) => session.set_offset_join(join)),
    setResult: (result) => call((session) => session.set_offset_result(result)),
    apply,
    showReleaseOutcome,
    distanceRef,
  };
}
