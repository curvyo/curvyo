// The state of the Connector group in the Machine block (`specs/0064-machine-connector`, UX notes
// §4, §7 to §9): the stored pairing, the link line, the last status, the held job and the commands.
// The host holds the pin and the token; this hook only sees fingerprints, answers and statuses.
import { useCallback, useEffect, useRef, useState } from "react";

import {
  type CommandAnswer,
  type HeldJob,
  type Line,
  type LinkError,
  type PairAnswer,
  type SendAnswer,
  type StatusMessage,
  type TestAnswer,
  FORGET_FAILED_TEXT,
  UNKNOWN_JOB_TEXT,
  UNREADABLE_TEXT,
  clearsRefusal,
  errorLine,
  jobIsActive,
  pairLine,
  refusalText,
  testLine,
} from "@/lib/connectorText";
import { invoke, listen } from "@/platform/host";

/** What the host knows of a stored pairing. */
type PairingState =
  | { kind: "none" }
  | { kind: "paired"; connectorName: string; fingerprint: string }
  | { kind: "noStore" }
  | { kind: "failed"; message: string };

type WatchPayload =
  | { profileId: string; kind: "status"; status: StatusMessage }
  | { profileId: string; kind: "sending"; name: string }
  | { profileId: string; kind: "lost" }
  | { profileId: string; kind: "failed"; error: LinkError };

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
 * on the profile and address, so a new pick starts from empty state. */
export function useConnector(profileId: string | null, address: string, enabled: boolean): ConnectorModel {
  const [paired, setPaired] = useState<ConnectorModel["paired"]>(null);
  const [noStore, setNoStore] = useState(false);
  const [keyChanged, setKeyChanged] = useState<ConnectorModel["keyChanged"]>(null);
  const [line, setLine] = useState<Line | null>(null);
  const [alert, setAlert] = useState("");
  const [status, setStatus] = useState<StatusMessage | null>(null);
  const [held, setHeld] = useState<HeldJob | null>(null);
  const [sendingName, setSendingName] = useState<string | null>(null);
  const [refusal, setRefusal] = useState<Line | null>(null);
  const [busy, setBusy] = useState<Busy>(null);
  const generation = useRef(0);
  const statusRef = useRef<StatusMessage | null>(null);
  useEffect(() => {
    statusRef.current = status;
  }, [status]);

  const live = enabled && profileId !== null && address !== "";

  const fail = useCallback((error: LinkError, own: boolean) => {
    const shown = errorLine(error);
    if (error.kind === "keyChanged") {
      setKeyChanged({ stored: error.stored, presented: error.presented });
      setPaired(null);
      setStatus(null);
    } else if (error.kind === "noLongerPaired" || error.kind === "notPaired") {
      setPaired(null);
      setStatus(null);
    }
    if (error.kind === "noCredentialStore") setNoStore(true);
    setLine(shown);
    setAlert(own || error.kind === "keyChanged" ? shown.text : "");
  }, []);

  const startWatch = useCallback(() => {
    if (live) {
      void invoke("connector_watch", { profileId, address }).catch(() => undefined);
    }
  }, [live, profileId, address]);

  const read = useCallback(
    async (own: boolean, explain: boolean): Promise<void> => {
      const mine = generation.current;
      const answer = await invoke<TestAnswer>("connector_test", { profileId, address });
      if (mine !== generation.current) return;
      if (answer.kind === "connected") {
        setStatus(answer.status);
        setLine(explain ? testLine(answer) : null);
        setAlert("");
        startWatch();
      } else {
        fail(answer.error, own);
      }
    },
    [profileId, address, fail, startWatch],
  );

  useEffect(() => {
    generation.current += 1;
    const mine = generation.current;
    if (!enabled || profileId === null) {
      return;
    }
    let unlisten: (() => void) | null = null;
    void listen<WatchPayload>("connector-event", (event) => {
      const payload = event.payload;
      if (payload.profileId !== profileId || mine !== generation.current) return;
      if (payload.kind === "sending") {
        setSendingName(payload.name);
      } else if (payload.kind === "status") {
        if (clearsRefusal(statusRef.current, payload.status)) setRefusal(null);
        statusRef.current = payload.status;
        setStatus(payload.status);
        setLine((now) => (now === LOST ? null : now));
        if (payload.status.state.name !== "Receiving") setSendingName(null);
      } else if (payload.kind === "lost") {
        if (statusRef.current !== null && jobIsActive(statusRef.current.state)) setLine(LOST);
      } else if (payload.error.kind === "keyChanged" || payload.error.kind === "noLongerPaired") {
        fail(payload.error, false);
      } else if (statusRef.current !== null && jobIsActive(statusRef.current.state)) {
        setLine(LOST);
      }
    }).then((off) => {
      if (mine === generation.current) unlisten = off;
      else off();
    });
    void (async () => {
      const stored = await invoke<PairingState>("connector_pairing", { profileId }).catch(
        (): PairingState => ({ kind: "failed", message: "The pairing could not be read" }),
      );
      if (mine !== generation.current) return;
      if (stored.kind === "paired") {
        setPaired({ connectorName: stored.connectorName, fingerprint: stored.fingerprint });
        if (address !== "") {
          setLine({ text: "Testing", failure: false });
          await read(false, false);
        }
      } else if (stored.kind === "none") {
        if (address !== "") setLine({ text: "Not paired", failure: false });
      } else {
        if (stored.kind === "noStore") setNoStore(true);
        setLine(
          stored.kind === "noStore"
            ? errorLine({ kind: "noCredentialStore" })
            : { text: stored.message, failure: true },
        );
      }
    })();
    return () => {
      unlisten?.();
      void invoke("connector_unwatch").catch(() => undefined);
    };
    // `read` and `fail` depend on the profile and address only.
  }, [profileId, address, enabled, read, fail]);

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
        setPaired({ connectorName: answer.connectorName, fingerprint: answer.fingerprint });
        setKeyChanged(null);
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
        setPaired(null);
        setStatus(null);
        setHeld(null);
        setRefusal(null);
        setKeyChanged(null);
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
      const answer = await invoke<SendAnswer>("connector_send_job", { profileId, address });
      setSendingName(null);
      if (answer.kind === "ready") {
        setHeld({ name: answer.name, lines: answer.lines, bytes: answer.bytes });
        await read(false, false);
      } else if (answer.kind === "rejected") {
        setHeld({ name: answer.name, lines: 0, bytes: 0 });
        setRefusal({ text: refusalText(answer.reason, statusRef.current?.alarm ?? null), failure: true });
      } else if (answer.kind === "unreadable") {
        // The counts line names the chosen file, as for a refused job (UX notes §8).
        setHeld({ name: answer.name, lines: 0, bytes: 0 });
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

  return { paired, noStore, keyChanged, line, alert, status, held, sendingName, refusal, busy, test, pair, forget, send, command };
}
