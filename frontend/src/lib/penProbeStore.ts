// Whether the Pen probe page is open (`specs/0158-pen-display` slice 0): the File tab's button
// opens it, its Close button and Escape close it. Nothing else is kept.
import { useSyncExternalStore } from "react";

let open = false;
const listeners = new Set<() => void>();

function set(next: boolean): void {
  if (open !== next) {
    open = next;
    listeners.forEach((listener) => listener());
  }
}

export const openPenProbe = (): void => set(true);
export const closePenProbe = (): void => set(false);

export function usePenProbeOpen(): boolean {
  return useSyncExternalStore(
    (listener) => {
      listeners.add(listener);
      return () => listeners.delete(listener);
    },
    () => open,
  );
}
