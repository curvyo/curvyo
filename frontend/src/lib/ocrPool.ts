/**
 * How one picture's text lines are shared over a small pool of workers and put back together
 * (`specs/0091-trace-text-recognition` round 2, lead 2026-11-02): the lines are independent, the
 * main trace worker detects them in order, the pool reads and fits them, and the result keeps
 * that order.
 * No React and no workers, so node tests can check it.
 */

/** The most workers that read lines next to the trace worker (Proposal, lead). */
export const MAX_POOL = 4;

/** The pool's size for a machine with `cores` logical processors: one fewer than the cores, at
 * most [`MAX_POOL`], at least one. */
export function poolSize(cores: number | undefined): number {
  const n = Math.floor((cores ?? 2) - 1);
  return Math.max(1, Math.min(MAX_POOL, Number.isFinite(n) ? n : 1));
}

/** The line indices each of `workers` takes: every line once, the costliest first to the worker
 * with the least work so far (ties go to the lower worker and the lower line), each share in line
 * order. */
export function shareLines(costs: readonly number[], workers: number): number[][] {
  const count = Math.max(1, workers);
  const shares: number[][] = Array.from({ length: count }, () => []);
  const load = new Array<number>(count).fill(0);
  const order = costs.map((c, i) => [c, i] as const).sort((a, b) => b[0] - a[0] || a[1] - b[1]);
  for (const [cost, line] of order) {
    let best = 0;
    for (let k = 1; k < count; k++) {
      if (load[k] < load[best]) {
        best = k;
      }
    }
    shares[best].push(line);
    load[best] += cost;
  }
  return shares.filter((s) => s.length > 0).map((s) => s.sort((a, b) => a - b));
}

/** The reads of `shares` (each worker's lines in its share's order, `null` where nothing was
 * read) back in line order, the empty ones left out. */
export function mergeReads<T>(shares: readonly number[][], reads: readonly (readonly (T | null)[])[]): T[] {
  const total = shares.reduce((n, s) => n + s.length, 0);
  const byLine: (T | null)[] = new Array<T | null>(total).fill(null);
  shares.forEach((share, k) => {
    if (reads[k]?.length !== share.length) {
      throw new Error("a pool worker answered for other lines");
    }
    share.forEach((line, j) => {
      byLine[line] = reads[k][j];
    });
  });
  return byLine.filter((r): r is T => r !== null);
}

/** The reading cost of a line `width` px wide: the recogniser pads it to 1024 px at least. */
export function readingCost(width: number): number {
  return Math.max(width, 1024);
}

/** One percent over the pool's lines: `done` of `total` lines read, after the detection's share
 * (the reading of one picture's text as the trace worker reported it before, 20 % detection). */
export function poolPercent(done: number, total: number): number {
  return Math.min(100, 20 + Math.floor((80 * done) / Math.max(1, total)));
}
