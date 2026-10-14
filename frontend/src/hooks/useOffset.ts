import { useCallback, useMemo } from "react";

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

/** Where the Offset entry chip is placed from, in canvas CSS pixels (`ux-rework.md` 2.4). */
export interface OffsetChipAnchor {
  center: { x: number; y: number };
  box: { left: number; top: number; right: number; bottom: number };
}

/** The Offset tool in the frontend (`specs/0038-path-offset`): the view, the remembered values the
 * entry chip sets, the chip's Enter and the notice after a release. */
export interface OffsetApi {
  view: OffsetView;
  setDistance: (mm: number) => boolean;
  setDirection: (direction: OffsetDirectionName) => void;
  setJoin: (join: OffsetJoinName) => void;
  setResult: (result: OffsetResultName) => void;
  /** Enter in the chip: writes, and returns the refusal sentence the chip shows, or `null` once
   * it was written (the notice says what happened). */
  apply: () => string | null;
  /** Closes the chip without writing. */
  closeChip: () => void;
  /** The chip's anchor now, or `null`. */
  readAnchor: () => OffsetChipAnchor | null;
  /** After a pointer release: shows the notice of what the release did, if anything. */
  showReleaseOutcome: () => void;
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
    handles: raw.phase !== "none",
    dragging: raw.phase === "dragging",
    distanceMm: raw.distance_mm,
    both: raw.both,
    change: raw.result === "change",
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
    dragJoin: raw.drag_join as OffsetJoinName,
    chipOpen: raw.phase === "chip",
    result: raw.result as OffsetResultName,
    minMm: raw.min_mm,
    maxMm: raw.max_mm,
  };
  raw.free();
  return view;
}

function readOutcome(raw: ReturnType<WasmSession["apply_offset"]>): OffsetOutcome {
  const outcome: OffsetOutcome = {
    kind: raw.kind,
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
 * The Offset tool's view and the entry chip's commands. Every decision (what is written, the
 * preview, why a release or Enter is refused) is the session's; this hook reads it after every sync and turns outcomes
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

  const apply = useCallback((): string | null => {
    const session = getSession();
    if (!session) {
      return null;
    }
    const outcome = readOutcome(session.apply_offset());
    onChange();
    const success = offsetSuccessText(outcome);
    if (success !== null) {
      showNotice(success, 3000, "canvas", "status");
    }
    return offsetRefusalText(outcome);
  }, [getSession, onChange, showNotice]);

  const readAnchor = useCallback((): OffsetChipAnchor | null => {
    const raw = getSession()?.offset_chip_anchor();
    if (!raw) {
      return null;
    }
    const anchor = {
      center: { x: raw.center_x, y: raw.center_y },
      box: { left: raw.left, top: raw.top, right: raw.right, bottom: raw.bottom },
    };
    raw.free();
    return anchor;
  }, [getSession]);

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
    closeChip: () => call((session) => session.close_offset_chip()),
    readAnchor,
    showReleaseOutcome,
  };
}
