import { useEffect, useRef } from "react";

import { jobTick, StaleMarkTimer, staleMark, type JobTickSession } from "@/lib/jobTicks";

/**
 * Works the active laser page's pending job one share per animation frame after every sync while
 * the views read a stale job (no frame is skipped), and 300 ms after the job became pending lets
 * the views write its times as "…" (UX notes P35); `syncRevision` restarts the frames after a
 * command but never re-arms the mark.
 */
export function useJobTicks(
  getSession: () => JobTickSession | null,
  sync: () => void,
  syncRevision: number,
): void {
  const mark = useRef<StaleMarkTimer | null>(null);
  mark.current ??= new StaleMarkTimer(
    (fire, ms) => window.setTimeout(fire, ms),
    (timer) => window.clearTimeout(timer),
  );
  useEffect(() => {
    const timer = mark.current;
    const session = getSession();
    if (!session?.job_pending()) {
      timer?.reset();
      return;
    }
    let frame = 0;
    const tick = () => {
      const current = getSession();
      if (current && jobTick(current, sync)) frame = requestAnimationFrame(tick);
    };
    frame = requestAnimationFrame(tick);
    timer?.arm(() => {
      const current = getSession();
      if (current) staleMark(current, sync);
    });
    return () => cancelAnimationFrame(frame);
  }, [getSession, sync, syncRevision]);
  useEffect(() => () => mark.current?.reset(), []);
}
