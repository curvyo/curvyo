// The one source of the connector's state in the page (`specs/0125-send-to-machine` S14; architect
// addendum B5): per machine profile the pairing, the last status, the job this app sent, a send
// that is running and its outcome. The Connector row and the Output group read the same entry, so
// they always show equal values; the first reader starts the host's watcher, the last one ends it.
// The host holds the pin and the token; this module only sees fingerprints, answers and statuses.
// No React and no `@/` import, so the tests load it without a bundler.
import {
  type ControllerInfo,
  type Line,
  type LinkError,
  type StatusMessage,
  type TestAnswer,
  errorLine,
  jobIsActive,
  refusalText,
} from "./connectorText.ts";

/** What the page knows of the pairing of a profile; `unknown` until it was read. */
export type Link = "unknown" | "paired" | "not_paired" | "no_longer_paired" | "key_changed";

/** A job this app sent in this run; the connector's `Status` carries no name (protocol 0). */
export interface HeldJob {
  name: string;
  lines: number;
  bytes: number;
  /** The connector's id for it, known for a page send at once and for a manual send once the
   * `Ready` status was seen. */
  jobId: string | null;
}

/** A send from a page that has not ended yet. */
interface Send {
  token: number;
  name: string;
  jobId: string | null;
  lines: number;
  bytes: number;
  /** Called once, with the result, when the connector accepted or lost the job. */
  settle: (accepted: boolean) => void;
  /** Called once when an unresolved send has waited `UNRESOLVED_SEND_MS`. */
  timedOut?: () => void;
  /** The send timed out: Send is free again, a late acceptance still commits. */
  late?: boolean;
  timer?: unknown;
}

/** How long a send whose connection dropped and could not be re-read stays "Sending" (S7, S8;
 * Proposal, vetoable at the demo). */
export const UNRESOLVED_SEND_MS = 30_000;

/** The clock the bound runs on; tests pass their own. */
export interface SendTimers {
  set: (run: () => void, ms: number) => unknown;
  clear: (handle: unknown) => void;
}

const REAL_TIMERS: SendTimers = {
  set: (run, ms) => setTimeout(run, ms),
  clear: (handle) => clearTimeout(handle as ReturnType<typeof setTimeout>),
};

/** What the page shows of one profile's connector. */
export interface LinkEntry {
  link: Link;
  paired: { connectorName: string; fingerprint: string } | null;
  noStore: boolean;
  keyChanged: { stored: string; presented: string } | null;
  status: StatusMessage | null;
  /** What the connector's controller reported at its last identify (`specs/0127` G19). */
  controller: ControllerInfo | null;
  /** The connection dropped while a job was in flight; the last values stay. */
  lost: boolean;
  held: HeldJob | null;
  /** The id of the job the last send from a page left at the connector. */
  pageJobId: string | null;
  /** The name of the job being sent, from a page or from the manual "Send job". */
  sendingName: string | null;
  /** A page's send is running: from the press to the connector's answer. */
  pageSend: { token: number; name: string } | null;
  /** How the last page send failed, until the next send or a state change. */
  sendFailure: { text: string; atState: string | null } | null;
}

type Payload =
  | { profileId: string; kind: "status"; status: StatusMessage }
  | { profileId: string; kind: "controller"; controller: ControllerInfo | null }
  | { profileId: string; kind: "sending"; name: string }
  | { profileId: string; kind: "lost" }
  | { profileId: string; kind: "failed"; error: LinkError }
  | { profileId: string; kind: "sendStarted"; jobId: string; lines: number; bytes: number }
  | { profileId: string; kind: "sendResolved"; jobId: string; resolution: Resolution };

/** How a sent job ended its way to the connector (`ResolutionView` of the host). */
export type Resolution = { kind: "accepted" } | { kind: "rejected"; reason: string } | { kind: "notReceived" };

/** The answer of `connector_send_page_job`. */
export type PageSendAnswer =
  | { kind: "resolved"; jobId: string; resolution: Resolution }
  | { kind: "resolvedElsewhere" }
  | { kind: "unresolved"; error: LinkError }
  | { kind: "failed"; error: LinkError };

type PairingState =
  | { kind: "none" }
  | { kind: "paired"; connectorName: string; fingerprint: string }
  | { kind: "noStore" }
  | { kind: "failed"; message: string };

/** The host calls the store needs: Tauri's `invoke` and `listen`, or a stand-in. */
export interface LinkHost {
  invoke: <T>(command: string, args?: Record<string, unknown>, bytes?: Uint8Array) => Promise<T>;
  listen: <T>(event: string, handler: (event: { payload: T }) => void) => Promise<() => void>;
}

const EMPTY: LinkEntry = {
  link: "unknown",
  paired: null,
  noStore: false,
  keyChanged: null,
  status: null,
  controller: null,
  lost: false,
  held: null,
  pageJobId: null,
  sendingName: null,
  pageSend: null,
  sendFailure: null,
};

/** The text of a send that ended without an accepted job (S7, S11). */
export function resolutionText(resolution: Resolution, alarm: string | null): string | null {
  switch (resolution.kind) {
    case "accepted":
      return null;
    case "rejected":
      return refusalText(resolution.reason, alarm);
    case "notReceived":
      return "Not received";
  }
}

/** The spelling `set_send_link` takes for an entry. */
export function sendLinkOf(entry: LinkEntry): string {
  return entry.link === "unknown" ? "not_paired" : entry.link;
}

export function createLinkStore(host: LinkHost, timers: SendTimers = REAL_TIMERS) {
  const entries = new Map<string, LinkEntry>();
  const consumers = new Map<string, { count: number; address: string }>();
  const sends = new Map<string, Send>();
  const listeners = new Set<() => void>();
  let unlisten: (() => void) | null = null;
  let listening = false;

  const get = (profileId: string): LinkEntry => entries.get(profileId) ?? EMPTY;
  const emit = () => listeners.forEach((listener) => listener());
  const patch = (profileId: string, change: Partial<LinkEntry>) => {
    entries.set(profileId, { ...get(profileId), ...change });
    emit();
  };

  /** A failure of the link itself: the pairing is gone or the key changed. */
  const linkFailure = (profileId: string, error: LinkError) => {
    if (error.kind === "keyChanged") {
      patch(profileId, {
        link: "key_changed",
        keyChanged: { stored: error.stored, presented: error.presented },
        paired: null,
        status: null,
      });
    } else if (error.kind === "noLongerPaired") {
      patch(profileId, { link: "no_longer_paired", paired: null, status: null });
    } else if (error.kind === "notPaired") {
      patch(profileId, { link: "not_paired", paired: null, status: null });
    } else if (error.kind === "noCredentialStore") {
      patch(profileId, { noStore: true });
    }
  };

  const setStatus = (profileId: string, status: StatusMessage) => {
    const now = get(profileId);
    const jobId = status.job?.job_id ?? null;
    const held =
      now.held && now.held.jobId === null && status.state.name === "Ready" && jobId !== null
        ? { ...now.held, jobId }
        : now.held;
    const changed = now.status?.state.name !== status.state.name;
    patch(profileId, {
      status,
      held,
      lost: false,
      sendFailure: changed && now.sendFailure?.atState !== status.state.name ? null : now.sendFailure,
      sendingName: status.state.name === "Receiving" ? now.sendingName : null,
    });
  };

  /** The page's send ended: commit through its `settle` exactly once. */
  const settle = (profileId: string, jobId: string | null, resolution: Resolution) => {
    const send = sends.get(profileId);
    if (!send || (jobId !== null && send.jobId !== null && send.jobId !== jobId)) return;
    sends.delete(profileId);
    if (send.timer !== undefined) timers.clear(send.timer);
    const entry = get(profileId);
    const text = resolutionText(resolution, entry.status?.alarm ?? null);
    if (text === null) {
      patch(profileId, {
        pageSend: null,
        sendingName: null,
        sendFailure: null,
        held: { name: send.name, lines: send.lines, bytes: send.bytes, jobId: send.jobId ?? jobId },
        pageJobId: send.jobId ?? jobId,
      });
    } else {
      patch(profileId, {
        pageSend: null,
        sendingName: null,
        sendFailure: { text, atState: entry.status?.state.name ?? null },
      });
    }
    send.settle(text === null);
  };

  const onEvent = (payload: Payload) => {
    const { profileId } = payload;
    switch (payload.kind) {
      case "status":
        setStatus(profileId, payload.status);
        break;
      case "controller":
        patch(profileId, { controller: payload.controller });
        break;
      case "sending":
        patch(profileId, { sendingName: payload.name });
        break;
      case "lost":
        if (jobIsActive(get(profileId).status?.state ?? { name: "Idle" })) patch(profileId, { lost: true });
        break;
      case "failed":
        if (payload.error.kind === "keyChanged" || payload.error.kind === "noLongerPaired") {
          linkFailure(profileId, payload.error);
        } else if (jobIsActive(get(profileId).status?.state ?? { name: "Idle" })) {
          patch(profileId, { lost: true });
        }
        break;
      case "sendStarted": {
        const send = sends.get(profileId);
        if (send) {
          send.jobId = payload.jobId;
          send.lines = payload.lines;
          send.bytes = payload.bytes;
        }
        break;
      }
      case "sendResolved":
        settle(profileId, payload.jobId, payload.resolution);
        break;
    }
  };

  const startListening = () => {
    if (listening) return;
    listening = true;
    void host
      .listen<Payload>("connector-event", (event) => onEvent(event.payload))
      .then((off) => {
        if (listening) unlisten = off;
        else off();
      });
  };

  const stopListening = () => {
    if (consumers.size > 0) return;
    listening = false;
    unlisten?.();
    unlisten = null;
  };

  /** Reads the pairing and one status of a profile; starts the host's watcher when it connects. */
  const bootstrap = async (profileId: string, address: string) => {
    const stored = await host
      .invoke<PairingState>("connector_pairing", { profileId })
      .catch((): PairingState => ({ kind: "failed", message: "The pairing could not be read" }));
    if (!consumers.has(profileId)) return;
    if (stored.kind === "paired") {
      patch(profileId, {
        link: "paired",
        paired: { connectorName: stored.connectorName, fingerprint: stored.fingerprint },
      });
      if (address !== "") await read(profileId, address);
    } else if (stored.kind === "none") {
      patch(profileId, { link: "not_paired", paired: null });
    } else if (stored.kind === "noStore") {
      patch(profileId, { link: "not_paired", noStore: true });
    }
  };

  /** A connection test: the status, or why the connector cannot be used. */
  const read = async (profileId: string, address: string): Promise<TestAnswer> => {
    const answer = await host.invoke<TestAnswer>("connector_test", { profileId, address });
    if (answer.kind === "connected") {
      patch(profileId, { controller: answer.controllerInfo ?? null });
      setStatus(profileId, answer.status);
      void host.invoke("connector_watch", { profileId, address }).catch(() => undefined);
    } else {
      linkFailure(profileId, answer.error);
    }
    return answer;
  };

  return {
    get,
    subscribe(listener: () => void): () => void {
      listeners.add(listener);
      return () => void listeners.delete(listener);
    },
    patch,
    setStatus,
    linkFailure,
    read,
    /** Counts a reader of `profileId`; `bootstrapping` readers also read the pairing and a status
     * once. The last release ends the host's watcher. */
    acquire(profileId: string, address: string, bootstrapping: boolean): () => void {
      const now = consumers.get(profileId);
      consumers.set(profileId, { count: (now?.count ?? 0) + 1, address });
      startListening();
      if (bootstrapping && !now) void bootstrap(profileId, address);
      let released = false;
      return () => {
        if (released) return;
        released = true;
        const held = consumers.get(profileId);
        if (!held) return;
        if (held.count > 1) {
          consumers.set(profileId, { ...held, count: held.count - 1 });
          return;
        }
        consumers.delete(profileId);
        void Promise.resolve(host.invoke("connector_unwatch", { profileId })).catch(() => undefined);
        stopListening();
      };
    },
    /** A page's send: `settle` is called once with whether the connector accepted the job. */
    async sendPage(
      profileId: string,
      address: string,
      job: { token: number; name: string; bytes: Uint8Array },
      settleSend: (accepted: boolean) => void,
      timedOut?: () => void,
    ): Promise<void> {
      const previous = sends.get(profileId);
      if (previous?.timer !== undefined) timers.clear(previous.timer);
      sends.set(profileId, {
        token: job.token,
        name: job.name,
        jobId: null,
        lines: 0,
        bytes: job.bytes.length,
        settle: settleSend,
        timedOut,
      });
      patch(profileId, { pageSend: { token: job.token, name: job.name }, sendingName: job.name, sendFailure: null });
      let answer: PageSendAnswer;
      try {
        answer = await host.invoke<PageSendAnswer>(
          "connector_send_page_job",
          { profileId, address, name: job.name },
          job.bytes,
        );
      } catch {
        answer = { kind: "failed", error: { kind: "other", message: "Connector error" } };
      }
      if (answer.kind === "resolved") {
        settle(profileId, answer.jobId || null, answer.resolution);
      } else if (answer.kind === "failed") {
        linkFailure(profileId, answer.error);
        const send = sends.get(profileId);
        if (!send) return;
        sends.delete(profileId);
        if (send.timer !== undefined) timers.clear(send.timer);
        patch(profileId, {
          pageSend: null,
          sendingName: null,
          sendFailure: { text: errorLine(answer.error).text, atState: get(profileId).status?.state.name ?? null },
        });
        send.settle(false);
      }
      // `resolvedElsewhere` and `unresolved`: the watcher's `sendResolved` event ends the send. An
      // unresolved one is bounded: after `UNRESOLVED_SEND_MS` the line reads "Timed out" and Send
      // is free again; a late acceptance of that job id still commits once.
      if (answer.kind === "unresolved") {
        const send = sends.get(profileId);
        if (send && send.token === job.token) {
          send.timer = timers.set(() => {
            if (sends.get(profileId) !== send) return;
            send.late = true;
            send.timer = undefined;
            patch(profileId, {
              pageSend: null,
              sendingName: null,
              sendFailure: { text: "Timed out", atState: get(profileId).status?.state.name ?? null },
            });
            send.timedOut?.();
          }, UNRESOLVED_SEND_MS);
        }
      }
    },
    /** Whether the page has a send that awaits its answer. */
    sending(profileId: string): boolean {
      const send = sends.get(profileId);
      return send !== undefined && !send.late;
    },
  };
}

/** The failure line of a stored send failure, in the look of the link line. */
export function failureLine(entry: LinkEntry): Line | null {
  return entry.sendFailure ? { text: entry.sendFailure.text, failure: true } : null;
}
