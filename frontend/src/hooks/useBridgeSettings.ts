import { useSyncExternalStore } from "react";

import {
  type BridgeSettings,
  type BridgeStorage,
  readBridgeSettings,
  writeBridgeSettings,
} from "@/lib/bridgeText";

/** The webview's `localStorage`, or `undefined` when even reaching it throws. */
function storage(): BridgeStorage | undefined {
  try {
    return window.localStorage;
  } catch {
    return undefined;
  }
}

let current: BridgeSettings | null = null;
const listeners = new Set<() => void>();

/** The remembered Bridge values (app preference, not in the document; `adrs.md` decision 6). */
export function bridgeSettings(): BridgeSettings {
  current ??= readBridgeSettings(storage());
  return current;
}

/** Changes the Bridge values, remembers them and tells every reader. */
export function setBridgeSettings(change: Partial<BridgeSettings>): void {
  current = { ...bridgeSettings(), ...change };
  writeBridgeSettings(storage(), current);
  for (const listener of listeners) {
    listener();
  }
}

function subscribe(listener: () => void): () => void {
  listeners.add(listener);
  return () => listeners.delete(listener);
}

/** The Bridge values, shared by the Select bar's fields and both Bridge buttons. */
export function useBridgeSettings(): BridgeSettings {
  return useSyncExternalStore(subscribe, bridgeSettings, bridgeSettings);
}
