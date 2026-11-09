// The watchdog of a share or a switch of "Sync" (customer feedback on PR #281): it
// fails once nothing moved for `STALL_MS`, never because a large upload simply takes long.

/** How long a share or a switch may make no progress before it fails with a retry (ms). */
export const STALL_MS = 30_000;

/** What moved last and when (ms). */
export interface StallWatch {
  key: string;
  at: number;
}

/** A watch that starts now. */
export function startWatch(key: string, now: number): StallWatch {
  return { key, at: now };
}

/** The watch after a look at `key` (anything that names the progress, "uploading:3") at `now`:
 * a new key restarts it; `stalled` once the same key stayed for `STALL_MS` or longer. */
export function lookAt(watch: StallWatch, key: string, now: number): { watch: StallWatch; stalled: boolean } {
  if (watch.key !== key) {
    return { watch: { key, at: now }, stalled: false };
  }
  return { watch, stalled: now - watch.at >= STALL_MS };
}
