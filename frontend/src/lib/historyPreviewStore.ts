// The preview the History tab started, shared with the canvas overlay and the tool rail
// (`specs/0042-history-branches` criteria 17 and 19). The session holds the preview; the tab
// writes what it last read here, so the parts outside the panel follow without a prop chain.

import { useSyncExternalStore } from "react";

import { NO_PREVIEW, type PreviewState } from "@/lib/historyPreview";

let current: PreviewState = NO_PREVIEW;
const listeners = new Set<() => void>();

/** Sets the preview the overlay shows. */
export function setPreviewState(next: PreviewState): void {
  if (JSON.stringify(next) === JSON.stringify(current)) {
    return;
  }
  current = next;
  for (const listener of listeners) {
    listener();
  }
}

function subscribe(listener: () => void): () => void {
  listeners.add(listener);
  return () => listeners.delete(listener);
}

/** The preview the overlay shows. */
export function usePreviewState(): PreviewState {
  return useSyncExternalStore(subscribe, () => current);
}
