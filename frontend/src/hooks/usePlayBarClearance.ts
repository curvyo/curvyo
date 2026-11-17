// How far the play bar's top stands above the canvas bottom while it shows, so the canvas notice
// slot sits 8 px above the bar instead of under it (`specs/0029-laser-job-preview-and-output` UX
// review U2).
import { useEffect, useSyncExternalStore } from "react";

/** The gap between the canvas notice and the play bar, px. */
export const NOTICE_GAP = 8;

let clearance: number | null = null;
const listeners = new Set<() => void>();

function subscribe(listener: () => void): () => void {
  listeners.add(listener);
  return () => listeners.delete(listener);
}

function publish(next: number | null) {
  if (next === clearance) return;
  clearance = next;
  for (const listener of listeners) listener();
}

/** The play bar publishes `px`, its top's distance from the canvas bottom, while it is mounted. */
export function usePublishPlayBarClearance(px: number) {
  useEffect(() => publish(Math.max(0, Math.round(px))), [px]);
  useEffect(() => () => publish(null), []);
}

/** The canvas notice's distance from the canvas bottom, px: `fallback` while no play bar shows,
 * else 8 px above the bar. */
export function useNoticeBottom(fallback: number): number {
  const bar = useSyncExternalStore(subscribe, () => clearance);
  return bar === null ? fallback : Math.max(fallback, bar + NOTICE_GAP);
}
