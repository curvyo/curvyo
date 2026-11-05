// The busy line of a row whose keyring act waits for the server (`specs/0097` criterion 29a,
// ADR 0041 decision 11): the act's word, and after 10 s that it goes out once connected.

import { useEffect, useMemo, useState } from "react";

import { membership_word } from "@/wasm-bindings/curvyo_editor_wasm.js";

/** How long an act waits before its row says it goes out when the connection is back. */
export const PENDING_OFFLINE_MS = 10_000;

/** The line for `pending` (a `membership_word` name, or null when nothing waits). */
export function usePendingLine(pending: string | null | undefined): string | null {
  // A new token per pending act: the timer marks only the act it was started for.
  const token = useMemo(() => ({ pending }), [pending]);
  const [lateFor, setLateFor] = useState<object | null>(null);
  useEffect(() => {
    if (!token.pending) {
      return undefined;
    }
    const timer = window.setTimeout(() => setLateFor(token), PENDING_OFFLINE_MS);
    return () => window.clearTimeout(timer);
  }, [token]);
  if (!pending) {
    return null;
  }
  return membership_word(lateFor === token ? "pending-offline" : pending);
}
