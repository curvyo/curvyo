// The font preview's page state for a component (`lib/fontPreviewStore.ts`).
import { useSyncExternalStore } from "react";

import { type PreviewState, previewState, subscribePreview } from "@/lib/fontPreviewStore";

export function useFontPreviewState(): PreviewState {
  return useSyncExternalStore(subscribePreview, previewState);
}
