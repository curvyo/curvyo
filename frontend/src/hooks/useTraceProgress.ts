/** Times the progress pill of a trace job: shown after a delay, then held. */
import { useCallback, useEffect, useRef, useState } from "react";

import type { WasmSession } from "@/lib/editorSession";
import { PROGRESS_AFTER_MS, progressHoldLeft } from "@/lib/traceText";

/** The progress in the finish button and the old outline at 50 % after this long (UX notes
 * 21.2), held at least `PROGRESS_HOLD_MS` once shown (customer 2026-10-30). */
const BUSY_MS = PROGRESS_AFTER_MS;
/** The paint after the commit that shows the progress: added to the hold so it is on the screen
 * for the whole `PROGRESS_HOLD_MS`. */
const PAINT_MS = 45;

export function useTraceProgress(getSession: () => WasmSession | null, onChange: () => void) {
  const [busy, setBusy] = useState(false);
  /** The running job's whole percent, `null` before the first report or without one. */
  const [percent, setPercent] = useState<number | null>(null);
  const timers = useRef<number[]>([]);
  /** When the progress showed in the finish button, while it shows; its hold timer. */
  const shownAt = useRef<number | null>(null);
  const holdTimer = useRef<number | null>(null);

  // The hold counts from when the progress is on the screen, not from when it was asked for.
  useEffect(() => {
    if (busy && holdTimer.current === null) {
      shownAt.current = performance.now();
    }
  }, [busy]);

  /** Ends a job's timers; a progress already shown stays until it has shown for
   * `PROGRESS_HOLD_MS`, so a job just over 400 ms does not flash percent and x. */
  const clearTimers = useCallback(() => {
    for (const timer of timers.current) {
      window.clearTimeout(timer);
    }
    timers.current = [];
    const hide = () => {
      holdTimer.current = null;
      shownAt.current = null;
      setBusy(false);
      setPercent(null);
    };
    const left = progressHoldLeft(shownAt.current, performance.now());
    if (holdTimer.current !== null) {
      window.clearTimeout(holdTimer.current);
      holdTimer.current = null;
    }
    if (left > 0) {
      holdTimer.current = window.setTimeout(hide, left + PAINT_MS);
    } else {
      hide();
    }
  }, []);

  /** The timers of a job; `known` (over 4 Mpx at full size) shows the progress pill at once
   * (criterion 52). After 400 ms the pill shows and the old outline is drawn at 50 % (criterion 10). */
  const startTimers = useCallback(
    (known = false) => {
      for (const timer of timers.current) {
        window.clearTimeout(timer);
      }
      timers.current = [];
      const busyNow = () => {
        if (holdTimer.current !== null) {
          window.clearTimeout(holdTimer.current);
          holdTimer.current = null;
        }
        shownAt.current ??= performance.now();
        setBusy(true);
        getSession()?.set_trace_preview_stale(true);
        onChange();
      };
      // A progress still held from the job before goes on showing for this one.
      if (known || shownAt.current !== null) {
        busyNow();
        return;
      }
      timers.current.push(window.setTimeout(busyNow, BUSY_MS));
    },
    [getSession, onChange],
  );

  return { busy, percent, setPercent, startTimers, clearTimers };
}
