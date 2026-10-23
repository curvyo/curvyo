// The font preview's background preparation (`specs/0077-font-preview` criteria 17 and 23,
// customer report 2026-10-23: tiles took long and showed a stand-in first): which listed families
// to prepare and in which order, run one at a time while the page is idle.

/** The families prepared ahead at most: the visible ones and the next screens, not a whole
 * system library of 2,000 files. */
export const PRERENDER_MAX = 96;

/** The listed families' indices to prepare, in order: those in view (`first` to `last`), then
 * those after them, then those before, at most `limit`; none for an empty list. */
export function prerenderOrder(count: number, first: number, last: number, limit = PRERENDER_MAX): number[] {
  if (count <= 0 || limit <= 0) {
    return [];
  }
  const start = Math.min(Math.max(0, first), count - 1);
  const end = Math.min(Math.max(start, last), count - 1);
  const order: number[] = [];
  for (let i = start; i <= end && order.length < limit; i += 1) {
    order.push(i);
  }
  for (let i = end + 1; i < count && order.length < limit; i += 1) {
    order.push(i);
  }
  for (let i = start - 1; i >= 0 && order.length < limit; i -= 1) {
    order.push(i);
  }
  return order;
}

/** Runs queued jobs one after the other, each when the page is idle; a new queue replaces the
 * jobs not started yet. */
export interface Prerenderer {
  schedule: (jobs: (() => Promise<void>)[]) => void;
  /** Jobs not started yet. */
  pending: () => number;
}

export function createPrerenderer(idle: (run: () => void) => void): Prerenderer {
  let queue: (() => Promise<void>)[] = [];
  let running = false;
  const next = () => {
    const job = queue.shift();
    if (!job) {
      running = false;
      return;
    }
    void job()
      .catch(() => {})
      .finally(() => idle(next));
  };
  return {
    schedule: (jobs) => {
      queue = [...jobs];
      if (!running && queue.length > 0) {
        running = true;
        idle(next);
      }
    },
    pending: () => queue.length,
  };
}
