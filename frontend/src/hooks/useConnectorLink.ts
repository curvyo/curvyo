// The link of the Connector group in the Machine block (`specs/0064-machine-connector`, UX notes
// §4 to §6; `specs/0118` M11): the stored pairing, Test connection, Pair, Forget pairing and the
// link line. The job buttons are the Machine controls footer's (`useMachineControls`). The host
// holds the pin and the token; this hook only sees fingerprints, answers and statuses.
import { useCallback, useEffect, useRef, useState } from "react";

import { linkStore, useLinkEntry } from "@/hooks/useLinkEntry";
import { type Busy } from "@/lib/connectorLink";
import {
  type ControllerInfo,
  type Line,
  type LinkError,
  type PairAnswer,
  type Session,
  type StatusMessage,
  FORGET_FAILED_TEXT,
  errorLine,
  pairLine,
  testLine,
} from "@/lib/connectorText";
import { invoke } from "@/platform/host";

/** What the host knows of a stored pairing. */
type PairingState =
  | { kind: "none" }
  | { kind: "paired"; connectorName: string; fingerprint: string }
  | { kind: "noStore" }
  | { kind: "failed"; message: string };

export interface ConnectorLinkModel {
  paired: { connectorName: string; fingerprint: string } | null;
  /** This computer has no credential store: pairing cannot work, so the Pair row is not shown. */
  noStore: boolean;
  /** The last test reached a connector in maintenance mode: no Pair row (`specs/0128` UX notes 8.4). */
  maintenance: boolean;
  keyChanged: { stored: string; presented: string } | null;
  line: Line | null;
  /** The text of the hidden alert: a failure that answers the maker's own press, or a changed key. */
  alert: string;
  status: StatusMessage | null;
  /** What the controller reported at its last identify. */
  controller: ControllerInfo | null;
  /** This app's role and the owner label, from the last login (`specs/0128` A26). */
  session: Session | null;
  /** The press that waits for its answer, here or in the Machine controls footer. */
  busy: Busy | null;
  test: () => void;
  pair: (code: string) => Promise<boolean>;
  forget: () => void;
}

const LOST: Line = { text: "Connection lost", failure: true };

/** The Connector group's link for one saved profile; `enabled` is false when the group is not
 * shown for it (browser, built-in profile, no address). The caller keys the component that uses it
 * on the profile and address, so a new pick starts from empty state. The pairing, the status and
 * the busy press live in the page's one link store, which the Output group and the Machine
 * controls footer read too (`0125` S14, `0118` M11). */
export function useConnectorLink(profileId: string | null, address: string, enabled: boolean): ConnectorLinkModel {
  const [line, setLine] = useState<Line | null>(null);
  const [alert, setAlert] = useState("");
  const generation = useRef(0);
  const live = enabled && profileId !== null && address !== "";
  const entry = useLinkEntry(profileId, address, live, false);

  const fail = useCallback(
    (error: LinkError, own: boolean) => {
      if (profileId !== null) linkStore.linkFailure(profileId, error);
      const shown = errorLine(error);
      setLine(shown);
      setAlert(own || error.kind === "keyChanged" ? shown.text : "");
    },
    [profileId],
  );

  // The watcher learned the pairing is gone or the key changed.
  useEffect(() => {
    if (entry.link === "key_changed" && entry.keyChanged) {
      setLine(errorLine({ kind: "keyChanged", ...entry.keyChanged }));
    } else if (entry.link === "no_longer_paired") {
      setLine(errorLine({ kind: "noLongerPaired" }));
    }
  }, [entry.link, entry.keyChanged]);

  const read = useCallback(
    async (own: boolean, explain: boolean): Promise<void> => {
      if (profileId === null) return;
      const mine = generation.current;
      const answer = await linkStore.read(profileId, address);
      if (mine !== generation.current) return;
      if (answer.kind === "connected") {
        setLine(explain ? testLine(answer) : null);
        setAlert("");
      } else if (answer.kind === "maintenance") {
        setLine(explain ? testLine(answer) : null);
        setAlert("");
      } else {
        fail(answer.error, own);
      }
    },
    [profileId, address, fail],
  );

  useEffect(() => {
    generation.current += 1;
    const mine = generation.current;
    if (!enabled || profileId === null) {
      return;
    }
    void (async () => {
      const stored = await invoke<PairingState>("connector_pairing", { profileId }).catch(
        (): PairingState => ({ kind: "failed", message: "The pairing could not be read" }),
      );
      if (mine !== generation.current) return;
      if (stored.kind === "paired") {
        linkStore.patch(profileId, {
          link: "paired",
          paired: { connectorName: stored.connectorName, fingerprint: stored.fingerprint },
        });
        if (address !== "") {
          setLine({ text: "Testing", failure: false });
          await read(false, false);
        }
      } else if (stored.kind === "none") {
        linkStore.patch(profileId, { link: "not_paired", paired: null });
        if (address !== "") setLine({ text: "Not paired", failure: false });
      } else {
        if (stored.kind === "noStore") linkStore.patch(profileId, { link: "not_paired", noStore: true });
        setLine(
          stored.kind === "noStore"
            ? errorLine({ kind: "noCredentialStore" })
            : { text: stored.message, failure: true },
        );
      }
    })();
    return () => {
      generation.current += 1;
    };
    // `read` and `fail` depend on the profile and address only.
  }, [profileId, address, enabled, read]);

  const guard = <T,>(kind: Busy, run: () => Promise<T>) => {
    if (!live || profileId === null || !linkStore.claim(profileId, kind)) return Promise.resolve(undefined);
    return run().finally(() => linkStore.release(profileId));
  };

  const test = () => {
    void guard("test", async () => {
      setLine({ text: "Testing", failure: false });
      await read(true, true);
    });
  };

  const pair = async (code: string): Promise<boolean> => {
    const result = await guard("pair", async () => {
      setLine({ text: "Pairing", failure: false });
      const answer = await invoke<PairAnswer>("connector_pair", { profileId, address, code });
      const shown = pairLine(answer);
      if (answer.kind === "paired") {
        linkStore.patch(profileId as string, {
          link: "paired",
          paired: { connectorName: answer.connectorName, fingerprint: answer.fingerprint },
          keyChanged: null,
        });
        setLine(null);
        setAlert("");
        await read(false, false);
        return true;
      }
      if (answer.kind === "failed") {
        fail(answer.error, true);
      } else {
        setLine(shown);
        setAlert(shown?.text ?? "");
      }
      return false;
    });
    return result === true;
  };

  const forget = () => {
    void invoke("connector_forget", { profileId }).then(
      () => {
        linkStore.patch(profileId as string, {
          link: "not_paired",
          paired: null,
          status: null,
          held: null,
          pageJobId: null,
          keyChanged: null,
          lost: false,
        });
        setAlert("");
        setLine({ text: "Not paired", failure: false });
      },
      () => {
        // The record is still there: the pairing stays shown, with the reason.
        setLine({ text: FORGET_FAILED_TEXT, failure: true });
        setAlert(FORGET_FAILED_TEXT);
      },
    );
  };

  return {
    paired: entry.paired,
    noStore: entry.noStore,
    maintenance: entry.maintenance,
    keyChanged: entry.keyChanged,
    line: entry.lost ? LOST : line,
    alert,
    status: entry.status,
    controller: entry.controller,
    session: entry.session,
    busy: entry.busy,
    test,
    pair,
    forget,
  };
}
