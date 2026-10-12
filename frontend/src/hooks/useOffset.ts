import { useCallback, useEffect, useMemo, useRef, useState } from "react";

import type { WasmSession } from "@/lib/editorSession";
import { noticeMs } from "@/lib/pathText";
import {
  OFFSET_CLOSED,
  type OffsetCapName,
  type OffsetJoinName,
  type OffsetResult,
  type OffsetState,
  offsetRefusalText,
  offsetSuccessText,
} from "@/lib/offsetText";

/** A notice of the Offset command: success under the Offset button, refusal under Apply. */
export interface OffsetNotice {
  id: number;
  kind: "success" | "refusal";
  text: string;
}

/** The Offset entry in the frontend (`specs/0038-path-offset`). */
export interface OffsetApi {
  state: OffsetState;
  /** The section is open. */
  open: boolean;
  /** Opens the entry for the selection; returns whether it opened. */
  openEntry: () => boolean;
  /** Closes the entry without writing. */
  close: () => void;
  /** Sets the distance; returns whether it was within the typed range. The preview follows on
   * the next frame. */
  setDistance: (mm: number) => boolean;
  setJoin: (join: OffsetJoinName) => void;
  setCap: (cap: OffsetCapName) => void;
  /** Applies the previewed result; shows its notice. */
  apply: () => OffsetResult;
  notice: OffsetNotice | null;
}

const REFUSAL_MS = 8000;
const SUCCESS_MS = 3000;

function readState(session: WasmSession | null): OffsetState {
  const raw = session?.offset_state();
  if (!raw) {
    return OFFSET_CLOSED;
  }
  const state: OffsetState = {
    kind: raw.kind,
    count: raw.count,
    of: raw.of,
    vanished: raw.vanished,
    offered: raw.offered,
    hasOpen: raw.has_open,
    distanceMm: raw.distance_mm,
    join: raw.join as OffsetJoinName,
    cap: raw.cap as OffsetCapName,
    typedLimitMm: raw.typed_limit_mm,
    dragLimitMm: raw.drag_limit_mm,
  };
  raw.free();
  return state;
}

/**
 * The Offset entry's state and commands. Every decision (what is offered, the preview, why Apply
 * is refused) is the session's; this hook reads it after every sync, coalesces value changes to
 * one re-read per animation frame (criterion 2), and keeps the notice of the last Apply.
 */
export function useOffset(
  getSession: () => WasmSession | null,
  onChange: () => void,
  syncRevision: number,
): OffsetApi {
  const state = useMemo(
    () => readState(getSession()),
    // The session is read again after every sync (`syncRevision`).
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [getSession, syncRevision],
  );
  const [notice, setNotice] = useState<OffsetNotice | null>(null);
  const noticeId = useRef(0);
  const frame = useRef<number | null>(null);

  const syncNextFrame = useCallback(() => {
    if (frame.current !== null) {
      return;
    }
    frame.current = requestAnimationFrame(() => {
      frame.current = null;
      onChange();
    });
  }, [onChange]);
  useEffect(
    () => () => {
      if (frame.current !== null) {
        cancelAnimationFrame(frame.current);
      }
    },
    [],
  );

  useEffect(() => {
    if (notice === null) {
      return undefined;
    }
    const ms = notice.kind === "refusal" ? REFUSAL_MS : (noticeMs(notice.text) ?? SUCCESS_MS);
    const timer = window.setTimeout(() => {
      setNotice((current) => (current?.id === notice.id ? null : current));
      if (notice.kind === "refusal") {
        getSession()?.clear_boolean_refusal();
        onChange();
      }
    }, ms);
    return () => window.clearTimeout(timer);
  }, [notice, getSession, onChange]);

  const openEntry = useCallback(() => {
    const opened = getSession()?.open_offset() ?? false;
    setNotice(null);
    onChange();
    return opened;
  }, [getSession, onChange]);

  const close = useCallback(() => {
    getSession()?.close_offset();
    onChange();
  }, [getSession, onChange]);

  const setDistance = useCallback(
    (mm: number) => {
      const taken = getSession()?.set_offset_distance(mm) ?? false;
      syncNextFrame();
      return taken;
    },
    [getSession, syncNextFrame],
  );

  const setJoin = useCallback(
    (join: OffsetJoinName) => {
      getSession()?.set_offset_join(join);
      syncNextFrame();
    },
    [getSession, syncNextFrame],
  );

  const setCap = useCallback(
    (cap: OffsetCapName) => {
      getSession()?.set_offset_cap(cap);
      syncNextFrame();
    },
    [getSession, syncNextFrame],
  );

  const apply = useCallback((): OffsetResult => {
    const session = getSession();
    const distanceMm = readState(session).distanceMm;
    const raw = session?.apply_offset();
    const result: OffsetResult = raw
      ? {
          kind: raw.kind,
          count: raw.count,
          of: raw.of,
          vanished: raw.vanished,
          distanceMm: raw.distance_mm,
        }
      : { kind: "ignored", count: 0, of: 0, vanished: 0, distanceMm: 0 };
    raw?.free();
    onChange();
    const success = offsetSuccessText(result);
    const refusal = offsetRefusalText(result, distanceMm);
    noticeId.current += 1;
    if (success !== null) {
      setNotice({ id: noticeId.current, kind: "success", text: success });
    } else if (refusal !== null) {
      setNotice({ id: noticeId.current, kind: "refusal", text: refusal });
    }
    return result;
  }, [getSession, onChange]);

  return {
    state,
    open: state.kind !== "closed",
    openEntry,
    close,
    setDistance,
    setJoin,
    setCap,
    apply,
    notice,
  };
}
