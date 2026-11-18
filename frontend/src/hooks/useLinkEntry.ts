// The page's one connector link store (`src/lib/connectorLink.ts`) bound to the host, and the hook
// that reads one profile's entry from it (`specs/0125-send-to-machine` S14).
import { useEffect, useSyncExternalStore } from "react";

import { type LinkEntry, createLinkStore } from "@/lib/connectorLink";
import { invoke, listen } from "@/platform/host";

/** The one store: the Connector row and the Output group both read it. */
export const linkStore = createLinkStore({ invoke, listen: (event, handler) => listen(event, handler) });

const NO_ENTRY = linkStore.get("");

/**
 * The entry of `profileId`, `NO_ENTRY` (all empty) without one. While `active`, this reader is
 * counted: the first reader starts the host's watcher and the last ends it; with `bootstrapping`
 * it also reads the pairing and one status when it is the first.
 */
export function useLinkEntry(profileId: string | null, address: string, active: boolean, bootstrapping: boolean): LinkEntry {
  const snapshot = () => (profileId === null ? NO_ENTRY : linkStore.get(profileId));
  const entry = useSyncExternalStore(linkStore.subscribe, snapshot, snapshot);
  useEffect(() => {
    if (!active || profileId === null || address === "") return undefined;
    return linkStore.acquire(profileId, address, bootstrapping);
  }, [profileId, address, active, bootstrapping]);
  return entry;
}
