// The rules of the Machine controls footer of the Operations tab (`specs/0118-machine-control`
// UX notes sections 4, 5, 6 and 8): Home's state and cause, the one state line of the tab, and the
// line that replaces the toolbar while the connector cannot be used. Pure, so the tests load it
// without a bundler.
import { type Busy, type LinkEntry } from "./connectorLink.ts";
import {
  type Line,
  type StatusMessage,
  clearsRefusal,
  controllerWord,
  errorLine,
  PAUSE_PENDING_TEXT,
  hasProgress,
  jobButtons,
  jobLineAlerts,
  jobStateLine,
} from "./connectorText.ts";
import { withName } from "./sendLine.ts";

/** What Home shows: whether a press does something, the busy look, and the tooltip's second line. */
export interface HomeButton {
  enabled: boolean;
  busy: boolean;
  cause: string | null;
}

/** GRBL's states in which the connector refuses Home outside Alarm (`specs/0127` G4, `0118` M3). */
const REFUSING_WORDS = ["Door", "Check", "Sleep"];

/** Home in the footer (UX notes section 4 table): the first cause that applies wins. */
export function homeButton(entry: LinkEntry): HomeButton {
  const off = (cause: string | null): HomeButton => ({ enabled: false, busy: false, cause });
  const status = entry.status;
  if (entry.busy === "home" || status?.state.name === "Homing") {
    return { enabled: false, busy: true, cause: "Homing" };
  }
  switch (status?.state.name) {
    case "Running":
    case "Paused":
      return off("A job is running");
    case "AwaitingLocalStart":
      return off("Waiting for the start at the machine");
    case "Receiving":
      return off("Sending");
    case "NoController":
      return off(status.detail ?? "No controller");
    default:
      break;
  }
  if (entry.lost) return off("Connection lost");
  if (status === null || status === undefined) {
    if (entry.reading || entry.readError === null) return off("Testing");
    return off(errorLine(entry.readError).text);
  }
  const word = controllerWord(status);
  if (status.state.name !== "Alarm" && word !== null && REFUSING_WORDS.includes(word)) {
    return off(`Controller state: ${word}`);
  }
  if (entry.busy !== null) return off(null);
  return { enabled: true, busy: false, cause: null };
}

/** What Pause shows: the same three things as Home. */
export type PauseButton = HomeButton;

/** Whether the connector keeps a Pause pending while the job homes (`specs/0133`). */
export function pausePending(status: StatusMessage | null): boolean {
  return status?.state.name === "Running" && status.pause_pending === true;
}

/** Pause in the toolbar: while pending it stays busy, driven by the status and without a time
 * limit, with "Pause after homing" as its second line (`specs/0133` UX notes H3, H5). */
export function pauseButton(entry: LinkEntry): PauseButton {
  if (pausePending(entry.status)) return { enabled: false, busy: true, cause: PAUSE_PENDING_TEXT };
  const pause = jobButtons(entry.status?.state ?? null).pause;
  return { enabled: pause && entry.busy === null, busy: entry.busy === "pause", cause: null };
}

/** Whether the refusal of the maker's own press ends with `next`: when the job moves into another
 * state with a line count (as built), or, for a refusal that holds only in the state it was given
 * in (`refusedAt`: "No job to pause", `specs/0133` H8), at any other state. */
export function refusalEnds(last: StatusMessage | null, next: StatusMessage, refusedAt: string | null): boolean {
  return clearsRefusal(last, next) || (refusedAt !== null && next.state.name !== refusedAt);
}

/** The progress bar under the state line. */
export interface Bar {
  line: number;
  lines: number;
  failure: boolean;
}

/** The footer's state line: its words, its live role and the bar under it. */
export interface ControlsLine {
  text: string;
  failure: boolean;
  /** `TriangleAlert` and Unlock at the right. */
  alarm: boolean;
  /** Read out as an alert: an alarm, a lost job, a failure of the maker's own press. */
  alerts: boolean;
  bar: Bar | null;
}

/** What only the footer's own presses know (Send job..., a refused command). */
export interface OwnPress {
  /** Send job... runs: the counts line reads the chosen file. */
  sending: boolean;
  /** A refusal or failure of the maker's own press, until the next press or state change. */
  failure: Line | null;
}

export const NO_PRESS: OwnPress = { sending: false, failure: null };

const pressed = (line: Line): ControlsLine => ({ ...line, alarm: false, alerts: line.failure, bar: null });

function barOf(status: StatusMessage, line: Line): Bar | null {
  // A Home that waits for the local press carries no job: no "0 / 0 lines" bar.
  if (!hasProgress(status) || status.job === null) return null;
  const alarm = status.state.name === "Alarm";
  return { line: status.job?.line ?? 0, lines: status.job?.lines ?? 0, failure: alarm || line.failure };
}

/** The one state line of the Operations tab (Frage 262 A): the footer's own press first, then a
 * Send to machine press of the Output group, then the connector's job in the `0064` form;
 * `null` when there is nothing to say. */
export function controlsLine(entry: LinkEntry, own: OwnPress = NO_PRESS): ControlsLine | null {
  if (own.sending && entry.sendingName !== null) {
    return pressed({ text: `Sending ${entry.sendingName}`, failure: false });
  }
  if (own.failure !== null) return pressed(own.failure);
  if (entry.sendFailure !== null) return pressed({ text: entry.sendFailure.text, failure: true });
  if (entry.pageSend !== null) return pressed({ text: withName("Sending ", entry.pageSend.name), failure: false });
  const status = entry.status;
  if (status === null) return null;
  // The name and size of this app's job only; a job of another app reads as plain "Ready".
  const id = status.job?.job_id ?? null;
  const held = entry.held !== null && (entry.held.jobId === null || entry.held.jobId === id) ? entry.held : null;
  const line = jobStateLine(status, held);
  if (line === null) return null;
  const bar = barOf(status, line);
  if (entry.lost) return { text: "Connection lost", failure: true, alarm: false, alerts: false, bar };
  return { ...line, alarm: status.state.name === "Alarm", alerts: jobLineAlerts(status), bar };
}

/** The job's name on the counts line (`0064` UX notes section 8): the job being sent, the job the
 * status carries, or the file a refusal of the maker's own press names; none once the connector
 * holds no job (dropped by an alarm, or a homing without a job). */
export function countsName(entry: LinkEntry, own: OwnPress = NO_PRESS): string | null {
  if (own.sending) return entry.sendingName;
  if (entry.pageSend !== null) return entry.pageSend.name;
  const held = entry.held;
  if (held === null) return null;
  const job = entry.status?.job ?? null;
  if (job !== null) return held.jobId === null || held.jobId === job.job_id ? held.name : null;
  return held.jobId === null && own.failure !== null ? held.name : null;
}

/** The footer's one line while the toolbar cannot apply (UX notes section 8); `null` once paired
 * (or before the pairing was read). */
export function footerNotice(entry: LinkEntry, desktop: boolean): Line | null {
  if (!desktop) return { text: "Needs the desktop app", failure: false };
  if (entry.noStore) return { text: "No credential store on this computer", failure: true };
  switch (entry.link) {
    case "key_changed":
      return { text: "This connector's key changed", failure: true };
    case "no_longer_paired":
      return { text: "No longer paired", failure: false };
    case "not_paired":
      return { text: "Not paired", failure: false };
    default:
      return null;
  }
}

/** When a press of `kind` stops being busy: the state it waits for (`specs/0127` UX notes
 * sections 6 and 7), or the time limit; Home's ends with the connector's answer, so Stop stays
 * pressable while homing (`specs/0118` M11). */
export const SETTLES: Record<
  Exclude<Busy, "test" | "pair" | "send">,
  { done: (s: StatusMessage | null) => boolean; ms: number }
> = {
  start: { done: () => true, ms: 0 },
  home: { done: () => true, ms: 0 },
  // A pending pause is the connector's answer: from there the status drives Pause (`specs/0133`).
  resume: { done: (s) => s?.state.name !== "Paused" && !pausePending(s), ms: 3_000 },
  pause: { done: (s) => s?.state.name !== "Running" || pausePending(s), ms: 3_000 },
  stop: {
    done: (s) => !["Running", "Paused", "Homing"].includes(s?.state.name ?? ""),
    ms: 8_000,
  },
  unlock: { done: (s) => s?.state.name !== "Alarm", ms: 5_000 },
};

/** The words the hidden status says once after a homing ended (W7): "Idle" or the Ready line. */
export function afterHoming(before: StatusMessage | null, now: StatusMessage | null, held: LinkEntry["held"]): string | null {
  if (before?.state.name !== "Homing" || now === null || now.state.name === "Homing") return null;
  if (now.state.name === "Idle") return "Idle";
  if (now.state.name === "Ready") return jobStateLine(now, held)?.text ?? "Ready";
  return null;
}
