// The one source of the connector's state in the page (`specs/0125-send-to-machine` S14; architect
// addendum B5; `specs/0118` M11): per machine profile the pairing, the last status, the job this
// app sent, a send that is running and its outcome, and the one press that waits for its answer.
// The Machine block, the Output group and the Machine controls footer read the same entry, so
// they always show equal values; the first reader starts the host's watcher, the last one ends it.
// The host holds the pin and the token; this module only sees fingerprints, answers and statuses.
// No React and no `@/` import, so the tests load it without a bundler.
import {
  type ControllerInfo,
  type Line,
  type LinkError,
  type Session,
  type StatusMessage,
  type TestAnswer,
  errorLine,
  jobIsActive,
  refusalText,
} from "./connectorText.ts";
import { notStartedText } from "./sendText.ts";

/** The one press of a profile's connector controls that waits for its answer; the others of the
 * Machine block and the Machine controls footer are greyed meanwhile (`specs/0118` M11). */
export type Busy = "test" | "pair" | "send" | "start" | "resume" | "pause" | "stop" | "unlock" | "home";

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
  /** The last test reached a connector in maintenance mode, which takes no pairing. */
  maintenance: boolean;
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
  /** A page send that asked for a start runs, from the press to the host's answer (`0134`). */
  startStep: boolean;
  /** Why the page's job was accepted but not started; shown only while that job is `Ready`
   * (`0134` SS6, architect S3). */
  notStarted: { jobId: string; text: string } | null;
  /** The page's job whose start the connector took, while the status still reads it `Ready`: its
   * Ready line is not said in between (`0134` UX P8). */
  startedJob: string | null;
  /** The press that waits for its answer, in either place. */
  busy: Busy | null;
  /** A status read runs. */
  reading: boolean;
  /** Why the last status read failed; `null` after a read that connected. */
  readError: LinkError | null;
  /** This app's role and name and the owner label, from the last connection test or status read
   * (`specs/0128` A26); not stored. */
  session: Session | null;
  /** A member's waiting request that someone else discarded: its job's name, until the next
   * state change, send or test (`specs/0128` UX notes V16). */
  discarded: string | null;
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

/** What the start step of a page send did (`StartView` of the host, `0134`). */
export type StartAnswer = { kind: "started" } | { kind: "notStarted"; reason: string } | { kind: "noAnswer" };

/** The answer of `connector_send_page_job`. */
export type PageSendAnswer =
  | { kind: "resolved"; jobId: string; resolution: Resolution; start?: StartAnswer }
  | { kind: "resolvedElsewhere"; start?: StartAnswer }
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
  maintenance: false,
  keyChanged: null,
  status: null,
  controller: null,
  lost: false,
  held: null,
  pageJobId: null,
  sendingName: null,
  pageSend: null,
  sendFailure: null,
  startStep: false,
  notStarted: null,
  startedJob: null,
  busy: null,
  reading: false,
  readError: null,
  session: null,
  discarded: null,
};

/** The job's name when `next` shows that a member's own waiting request was discarded by someone
 * else (`specs/0128` UX notes V16): it waited, now nothing is held, and this app did not press
 * Stop; `null` otherwise. */
export function discardedName(
  last: StatusMessage | null,
  next: StatusMessage,
  session: Session | null,
  ownStop: boolean,
): string | null {
  const waited = last?.state.name === "AwaitingLocalStart" && Boolean(last.requested_by);
  const mine = session?.role === "Member" && last?.requested_by === session.clientName;
  if (!waited || !mine || ownStop || next.state.name !== "Idle") return null;
  return last?.job_name ?? "the job";
}

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
    } else if (error.kind === "memberOnly") {
      // The connector knows better than the login answer did (`specs/0128` UX notes 8.4).
      const session = get(profileId).session;
      if (session) patch(profileId, { session: { ...session, role: "Member" } });
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
    const ours = status.state.name === "Ready" && jobId !== null && jobId === now.notStarted?.jobId;
    const started = status.state.name === "Ready" && jobId !== null && jobId === now.startedJob;
    const discarded = discardedName(now.status, status, now.session, now.busy === "stop");
    patch(profileId, {
      discarded: discarded ?? (changed ? null : now.discarded),
      notStarted: ours ? now.notStarted : null,
      startedJob: started ? now.startedJob : null,
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
        patch(profileId, { sendingName: payload.name, discarded: null });
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
    patch(profileId, { reading: true });
    const answer = await host
      .invoke<TestAnswer>("connector_test", { profileId, address })
      .catch((): TestAnswer => ({ kind: "failed", error: { kind: "other", message: "Connector error" } }));
    patch(profileId, {
      reading: false,
      readError: answer.kind === "connected" || answer.kind === "maintenance" ? null : answer.error,
      maintenance: answer.kind === "maintenance",
    });
    if (answer.kind === "connected") {
      const session: Session | null =
        answer.role && answer.clientName
          ? { role: answer.role, clientName: answer.clientName, ownerLabel: answer.ownerLabel ?? null }
          : get(profileId).session;
      patch(profileId, { controller: answer.controllerInfo ?? null, session, discarded: null });
      setStatus(profileId, answer.status);
      void host.invoke("connector_watch", { profileId, address }).catch(() => undefined);
    } else if (answer.kind === "failed") {
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
    /** Marks `kind` as the press of `profileId` that waits for its answer; `false` (and nothing
     * changes) while another press waits. */
    claim(profileId: string, kind: Busy): boolean {
      if (get(profileId).busy !== null) return false;
      patch(profileId, { busy: kind });
      return true;
    },
    /** The waiting press got its answer. */
    release(profileId: string): void {
      patch(profileId, { busy: null });
    },
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
      job: { token: number; name: string; bytes: Uint8Array; start?: boolean },
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
      const start = job.start === true;
      patch(profileId, {
        pageSend: { token: job.token, name: job.name },
        sendingName: job.name,
        sendFailure: null,
        startStep: start,
        notStarted: null,
        startedJob: null,
      });
      let answer: PageSendAnswer;
      try {
        answer = await host.invoke<PageSendAnswer>(
          "connector_send_page_job",
          { profileId, address, name: job.name, start },
          job.bytes,
        );
      } catch {
        answer = { kind: "failed", error: { kind: "other", message: "Connector error" } };
      }
      const jobId = sends.get(profileId)?.jobId ?? (answer.kind === "resolved" ? answer.jobId : get(profileId).pageJobId);
      const took = answer.kind === "resolved" || answer.kind === "resolvedElsewhere" ? answer.start?.kind === "started" : false;
      const shown = get(profileId).status;
      const stillReady = shown?.state.name === "Ready" && jobId !== null && shown.job?.job_id === jobId;
      const startedJob = start && took && stillReady ? jobId : null;
      patch(profileId, { startStep: false, notStarted: notStartedOf(start, answer, jobId), startedJob });
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

/** Why a page send that asked for a start left its job `Ready` (`0134` SS6), or `null`. A dropped
 * connection after the transfer reads "Connection lost": no start follows it. */
export function notStartedOf(asked: boolean, answer: PageSendAnswer, jobId: string | null): LinkEntry["notStarted"] {
  if (!asked || jobId === null || jobId === "") return null;
  if (answer.kind === "unresolved") return { jobId, text: notStartedText("ConnectionLost") };
  if (answer.kind !== "resolved" && answer.kind !== "resolvedElsewhere") return null;
  return answer.start?.kind === "notStarted" ? { jobId, text: notStartedText(answer.start.reason) } : null;
}

/** "Not started: <reason>" while the page's job it names is still `Ready` (`0134` architect S3). */
export function notStartedLine(entry: LinkEntry): string | null {
  const status = entry.status;
  const shown = entry.notStarted !== null && status?.state.name === "Ready" && status.job?.job_id === entry.notStarted.jobId;
  return shown ? (entry.notStarted?.text ?? null) : null;
}

/** The failure line of a stored send failure, in the look of the link line. */
export function failureLine(entry: LinkEntry): Line | null {
  return entry.sendFailure ? { text: entry.sendFailure.text, failure: true } : null;
}
