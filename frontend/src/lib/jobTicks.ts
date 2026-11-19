/** What the job ticks read of the session. */
export interface JobTickSession {
  job_pending(): boolean;
  advance_job(): boolean;
  show_stale_words(): void;
}

/** How long a pending job shows the last numbers before its times read "…" (UX notes P35). */
export const STALE_WORDS_MS = 300;

/**
 * The 300 ms mark of a pending job: its times and lengths now read "…"; `sync` reads the views
 * again. Nothing when the job was built meanwhile.
 */
export function staleMark(session: JobTickSession, sync: () => void): void {
  if (!session.job_pending()) return;
  session.show_stale_words();
  sync();
}

/**
 * One animation frame of a pending job (`specs/0123-raster-engraving` criteria 12, 37 d; ADR 0069
 * §12): one share of the job's dots. `sync` runs once the job is no longer pending (built by
 * this tick or by another read, such as the Toolpath view), so the views read it. Whether another
 * frame is needed.
 */
export function jobTick(session: JobTickSession, sync: () => void): boolean {
  if (session.advance_job() || !session.job_pending()) {
    sync();
    return false;
  }
  return true;
}

/**
 * The 300 ms mark of one pending period (UX notes P35): armed once when a job becomes pending and
 * kept across the syncs meanwhile (a sync, the mark's own included, never re-arms it), so a long
 * pending job syncs once at its mark and not every 300 ms; reset when no job is pending.
 */
export class StaleMarkTimer {
  private timer: number | null = null;
  private readonly setTimer: (fire: () => void, ms: number) => number;
  private readonly clearTimer: (timer: number) => void;

  constructor(
    setTimer: (fire: () => void, ms: number) => number,
    clearTimer: (timer: number) => void,
  ) {
    this.setTimer = setTimer;
    this.clearTimer = clearTimer;
  }

  /** Arms the mark to call `fire` after 300 ms, unless this pending period has one already. */
  arm(fire: () => void): void {
    if (this.timer !== null) return;
    this.timer = this.setTimer(fire, STALE_WORDS_MS);
  }

  /** Ends the pending period: the next pending job gets a mark of its own. */
  reset(): void {
    if (this.timer !== null) this.clearTimer(this.timer);
    this.timer = null;
  }
}
