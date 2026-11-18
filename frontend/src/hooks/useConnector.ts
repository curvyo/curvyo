// The state of the Connector group in the Machine block (`specs/0064-machine-connector`, UX notes
// §4, §7 to §9): the stored pairing, the link line, the last status, the held job and the commands.
// The host holds the pin and the token; this hook only sees fingerprints, answers and statuses.
import { useCallback, useEffect, useRef, useState } from "react";

import { linkStore, useLinkEntry } from "@/hooks/useLinkEntry";
import {
  type CommandAnswer,
  type HeldJob,
  type Line,
  type LinkError,
  type PairAnswer,
  type SendAnswer,
  type StatusMessage,
  FORGET_FAILED_TEXT,
  UNKNOWN_JOB_TEXT,
  UNREADABLE_TEXT,
  clearsRefusal,
  errorLine,
  pairLine,
  refusalText,
  testLine,
} from "@/lib/connectorText";
import { invoke } from "@/platform/host";

/** What the host knows of a stored pairing. */
type PairingState =
  | { kind: "none" }
  | { kind: "paired"; connectorName: string; fingerprint: string }
  | { kind: "noStore" }
  | { kind: "failed"; message: string };

/** The buttons that run a command (the busy one shows a spinner). */
export type Busy = "test" | "pair" | "send" | "start" | "resume" | "pause" | "stop" | null;

export interface ConnectorModel {
  paired: { connectorName: string; fingerprint: string } | null;
  /** This computer has no credential store: pairing cannot work, so the Pair row is not shown. */
  noStore: boolean;
  keyChanged: { stored: string; presented: string } | null;
  line: Line | null;
  /** The text of the hidden alert: a failure that answers the maker's own press, or a changed key. */
  alert: string;
  status: StatusMessage | null;
  held: HeldJob | null;
  sendingName: string | null;
  refusal: Line | null;
  busy: Busy;
  test: () => void;
  pair: (code: string) => Promise<boolean>;
  forget: () => void;
  send: () => void;
  command: (kind: "start" | "resume" | "pause" | "stop") => void;
}

const LOST: Line = { text: "Connection lost", failure: true };

/** The Connector group's model for one saved profile; `enabled` is false when the group is not
 * shown for it (browser, built-in profile, no address). The caller keys the component that uses it
 * on the profile and address, so a new pick starts from empty state. The pairing, the status and
 * the held job live in the page's one link store, which the Output group reads too (`0125` S14). */
export function useConnector(profileId: string | null, address: string, enabled: boolean): ConnectorModel {
  const [line, setLine] = useState<Line | null>(null);
  const [alert, setAlert] = useState("");
  const [refusal, setRefusal] = useState<Line | null>(null);
  const [busy, setBusy] = useState<Busy>(null);
  const generation = useRef(0);
  const live = enabled && profileId !== null && address !== "";
  const entry = useLinkEntry(profileId, address, live, false);
  const lastStatus = useRef<StatusMessage | null>(null);

  const fail = useCallback(
    (error: LinkError, own: boolean) => {
      if (profileId !== null) linkStore.linkFailure(profileId, error);
      const shown = errorLine(error);
      setLine(shown);
      setAlert(own || error.kind === "keyChanged" ? shown.text : "");
    },
    [profileId],
  );

  // A refusal ends when the job moves into another state that has a line count.
  useEffect(() => {
    const next = entry.status;
    if (next !== null && clearsRefusal(lastStatus.current, next)) setRefusal(null);
    lastStatus.current = next;
  }, [entry.status]);

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

  const guard = <T,>(kind: Exclude<Busy, null>, run: () => Promise<T>) => {
    if (busy !== null || !live) return Promise.resolve(undefined);
    setBusy(kind);
    return run().finally(() => setBusy(null));
  };

  const test = () => {
    void guard("test", async () => {
      setLine({ text: "Testing", failure: false });
      setRefusal(null);
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
        setRefusal(null);
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

  const send = () => {
    void guard("send", async () => {
      setRefusal(null);
      setLine(null);
      const id = profileId as string;
      const answer = await invoke<SendAnswer>("connector_send_job", { profileId, address });
      linkStore.patch(id, { sendingName: null });
      if (answer.kind === "ready") {
        linkStore.patch(id, { held: { name: answer.name, lines: answer.lines, bytes: answer.bytes, jobId: null } });
        await read(false, false);
      } else if (answer.kind === "rejected") {
        linkStore.patch(id, { held: { name: answer.name, lines: 0, bytes: 0, jobId: null } });
        setRefusal({ text: refusalText(answer.reason, entry.status?.alarm ?? null), failure: true });
      } else if (answer.kind === "unreadable") {
        // The counts line names the chosen file, as for a refused job (UX notes §8).
        linkStore.patch(id, { held: { name: answer.name, lines: 0, bytes: 0, jobId: null } });
        setRefusal({ text: UNREADABLE_TEXT, failure: true });
      } else if (answer.kind === "failed") {
        fail(answer.error, true);
      }
    });
  };

  const command = (kind: "start" | "resume" | "pause" | "stop") => {
    void guard(kind, async () => {
      setRefusal(null);
      const answer = await invoke<CommandAnswer>("connector_command", {
        profileId,
        address,
        command: kind,
      });
      if (answer.kind === "failed") {
        fail(answer.error, true);
      } else if (answer.kind === "unknownJob") {
        setRefusal({ text: UNKNOWN_JOB_TEXT, failure: true });
      } else {
        // The connector's status says what changed; a fresh read also covers a lost connection.
        await read(false, false);
      }
    });
  };

  return {
    paired: entry.paired,
    noStore: entry.noStore,
    keyChanged: entry.keyChanged,
    line: entry.lost ? LOST : line,
    alert,
    status: entry.status,
    held: entry.held,
    sendingName: entry.sendingName,
    refusal,
    busy,
    test,
    pair,
    forget,
    send,
    command,
  };
}
