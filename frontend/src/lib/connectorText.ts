// The texts and rules of the connector's controls (`specs/0064-machine-connector` UX notes §3 to
// §9; since `specs/0118` the job buttons live in the Machine controls footer): what each answer of
// the host says, which job buttons apply in which state, and the number formats. Pure, so the
// tests load it without a bundler.

/** The state of the connector's job, as the protocol sends it (`State` in `curvyo-connector-core`). */
export type JobState =
  | {
      name: "NoController" | "Idle" | "Receiving" | "Ready" | "AwaitingLocalStart" | "Homing" | "Running" | "Paused" | "Alarm";
    }
  | { name: "Done"; result: "Completed" | "Stopped" | "Failed" };

/** The `Status` message. */
export interface StatusMessage {
  type: "Status";
  state: JobState;
  job: { job_id: string; line: number; lines: number; elapsed_ms: number } | null;
  position: [number, number, number] | null;
  alarm: string | null;
  /** The sentence that explains the state (`specs/0127`); absent when there is none. */
  detail?: string | null;
}

/** What the controller reported (`ControllerInfo`, `specs/0127` G19). */
export interface ControllerInfo {
  name: string;
  firmware: string;
  port?: string | null;
  baud?: number | null;
  settings?: { laser_mode: boolean | null; s_max: number | null; travel: [number, number] | null } | null;
}

/** Why the connector could not be used (`LinkError` of the host). */
export type LinkError =
  | { kind: "notPaired" }
  | { kind: "noCredentialStore" }
  | { kind: "badAddress" }
  | { kind: "unreachable"; address: string }
  | { kind: "timedOut" }
  | { kind: "keyChanged"; stored: string; presented: string }
  | { kind: "noLongerPaired" }
  | { kind: "protocolMismatch"; connector: number }
  | { kind: "other"; message: string };

export type TestAnswer =
  | {
      kind: "connected";
      controller: string | null;
      controllerInfo: ControllerInfo | null;
      protocol: number;
      version: string;
      connectorName: string;
      status: StatusMessage;
    }
  | { kind: "failed"; error: LinkError };

export type PairAnswer =
  | { kind: "paired"; connectorName: string; fingerprint: string }
  | { kind: "wrongCode"; attemptsLeft: number }
  | { kind: "codeSpent" }
  | { kind: "codeExpired" }
  | { kind: "codeNotValid" }
  | { kind: "failed"; error: LinkError };

export type SendAnswer =
  | { kind: "cancelled" }
  | { kind: "ready"; name: string; lines: number; bytes: number }
  | { kind: "rejected"; name: string; reason: string }
  | { kind: "unreadable"; name: string }
  | { kind: "failed"; error: LinkError };

export type CommandAnswer =
  | { kind: "done" }
  | { kind: "wrongState" }
  | { kind: "unknownJob" }
  | { kind: "failed"; error: LinkError };

/** One line of the row: its words and how it looks. */
export interface Line {
  text: string;
  failure: boolean;
}

const neutral = (text: string): Line => ({ text, failure: false });
const failure = (text: string): Line => ({ text, failure: true });

/** The link line for an error (UX notes §4); `null` when the error has no line of its own. */
export function errorLine(error: LinkError): Line {
  switch (error.kind) {
    case "notPaired":
      return neutral("Not paired");
    case "noCredentialStore":
      return failure("No credential store on this computer");
    case "badAddress":
      return failure("Not a valid address");
    case "unreachable":
      return failure(`Nothing answers at ${error.address}`);
    case "timedOut":
      return failure("Timed out");
    case "keyChanged":
      return failure("This connector's key changed");
    case "noLongerPaired":
      return neutral("No longer paired");
    case "protocolMismatch":
      return failure(`Protocol mismatch: app 0, connector ${error.connector}`);
    case "other":
      return failure(error.message);
  }
}

/** The line after a connection test. */
export function testLine(answer: TestAnswer): Line {
  if (answer.kind === "failed") {
    return errorLine(answer.error);
  }
  const controller = answer.controller ?? "none";
  return neutral(
    `Connected, controller answered: ${controller}, protocol ${answer.protocol}, connector ${answer.version}`,
  );
}

/** The line after a pairing attempt that did not pair; `null` when it paired. */
export function pairLine(answer: PairAnswer): Line | null {
  switch (answer.kind) {
    case "paired":
      return null;
    case "wrongCode":
      return failure(
        `Wrong code, ${answer.attemptsLeft} ${answer.attemptsLeft === 1 ? "attempt" : "attempts"} left`,
      );
    case "codeSpent":
      return failure("Code spent");
    case "codeExpired":
      return failure("Code expired");
    case "codeNotValid":
      return failure("Code not valid");
    case "failed":
      return errorLine(answer.error);
  }
}

/** The chosen job file could not be read; the connector was never asked. */
export const UNREADABLE_TEXT = "Could not read the file";

/** Forgetting the pairing failed: the credential store would not remove the record. */
export const FORGET_FAILED_TEXT = "Could not forget the pairing";

/** Pairing needs a credential store to keep the key and the token in, so without one (or while a
 * pairing exists) the Pair row is not shown. */
export function pairRowShown(state: { paired: boolean; noStore: boolean }): boolean {
  return !state.paired && !state.noStore;
}

/** The connector holds another job than the one a command named. */
export const UNKNOWN_JOB_TEXT = "The connector holds another job";

const REFUSALS: Record<string, string> = {
  NotAscii: "Not plain ASCII text",
  TooLarge: "Larger than 64 MiB",
  HashMismatch: "Hash mismatch",
  Busy: "Connector busy",
  NoController: "No controller",
  LaserModeOff: "Laser mode is off ($32=0)",
  LaserModeUnknown: "Laser mode unknown ($32 not read)",
};

/** The words of a refused job; the alarm explains a `Busy` (Q19). */
export function refusalText(reason: string, alarm: string | null): string {
  if (reason === "Busy" && alarm !== null) {
    return `Machine in alarm: ${alarm}`;
  }
  return REFUSALS[reason] ?? reason;
}

/** The states in which a job or a homing is in flight: a dropped connection is then "Connection
 * lost" (`specs/0118` M9). */
export function jobIsActive(state: JobState): boolean {
  return ["Receiving", "Ready", "AwaitingLocalStart", "Homing", "Running", "Paused"].includes(state.name);
}

/** Whether a pushed status ends a shown refusal (UX notes §8, "A refusal stays until the next
 * Send, test or state change"): the job moved into another state that has a line count, or the
 * controller went into Alarm or away, whose line and Unlock button a refusal must not hide
 * (`specs/0127` UX notes section 6). A refused send passes through Receiving back to Idle, and
 * that late push must not clear it. */
export function clearsRefusal(last: StatusMessage | null, next: StatusMessage): boolean {
  const changed =
    last === null ||
    last.state.name !== next.state.name ||
    (last.state.name === "Done" && next.state.name === "Done" && last.state.result !== next.state.result);
  const controllerChanged = next.state.name === "Alarm" || next.state.name === "NoController";
  return changed && (hasProgress(next) || controllerChanged);
}

/** Which job buttons apply in a state (UX notes §7); `null` is "no status yet". */
export interface JobButtons {
  send: boolean;
  start: boolean;
  startName: "Start" | "Resume";
  pause: boolean;
  stop: boolean;
}

export function jobButtons(state: JobState | null): JobButtons {
  const none: JobButtons = { send: false, start: false, startName: "Start", pause: false, stop: false };
  if (state === null) {
    return { ...none, send: true };
  }
  switch (state.name) {
    case "Idle":
    case "Done":
      return { ...none, send: true };
    case "Ready":
      return { ...none, start: true, stop: true };
    case "AwaitingLocalStart":
    case "Homing":
      // Stop ends a homing too and never asks (`specs/0118` criterion 5).
      return { ...none, stop: true };
    case "Running":
      return { ...none, pause: true, stop: true };
    case "Paused":
      return { ...none, start: true, startName: "Resume", stop: true };
    default:
      return none;
  }
}

/** Binary units, one decimal under 10 (UX notes §8): "812 B", "4.2 KiB", "120 KiB", "3.5 MiB". */
export function formatSize(bytes: number): string {
  const units = ["B", "KiB", "MiB", "GiB"];
  let value = bytes;
  let unit = 0;
  while (value >= 1024 && unit < units.length - 1) {
    value /= 1024;
    unit += 1;
  }
  if (unit === 0) {
    return `${bytes} B`;
  }
  return `${value < 10 ? value.toFixed(1) : Math.round(value)} ${units[unit]}`;
}

/** "m:ss" under an hour, "h:mm:ss" from an hour. */
export function formatElapsed(ms: number): string {
  const total = Math.floor(ms / 1000);
  const seconds = String(total % 60).padStart(2, "0");
  const minutes = Math.floor(total / 60) % 60;
  const hours = Math.floor(total / 3600);
  return hours > 0
    ? `${hours}:${String(minutes).padStart(2, "0")}:${seconds}`
    : `${minutes}:${seconds}`;
}

/** A thousands comma: "5,000". */
export function formatCount(n: number): string {
  return String(n).replace(/\B(?=(\d{3})+(?!\d))/g, ",");
}

/** What the job block knows beyond the status (the job this app sent). */
export interface HeldJob {
  name: string;
  lines: number;
  bytes: number;
}

/** The state line of the job block (UX notes §8); `null` when there is no block. */
export function jobStateLine(status: StatusMessage, held: HeldJob | null): Line | null {
  const lines = status.job?.lines ?? held?.lines ?? 0;
  const line = status.job?.line ?? 0;
  switch (status.state.name) {
    case "Receiving":
      return neutral(held ? `Sending ${held.name}` : "Receiving a job");
    case "Ready":
      // A refused homing keeps the job and names its cause until the next state (`0118` §5).
      if (stateFailure(status) !== null) return failure(stateFailure(status) as string);
      return neutral(
        held ? `Ready: ${held.name}, ${formatCount(held.lines)} lines, ${formatSize(held.bytes)}` : "Ready",
      );
    case "AwaitingLocalStart":
      return neutral("Waiting for the start at the machine");
    case "Homing":
      return neutral("Homing");
    case "Running":
      return neutral("Running");
    case "Paused":
      return neutral("Paused");
    case "Done":
      switch (status.state.result) {
        case "Completed":
          return neutral(`Completed, ${formatCount(lines)} lines`);
        case "Stopped":
          return neutral(`Stopped at line ${formatCount(line)}`);
        case "Failed":
          return failure(status.detail ?? `Failed at line ${formatCount(line)}`);
      }
      return null;
    case "Alarm":
      return failure(`Alarm: ${status.alarm ?? ""}`);
    case "NoController":
      return status.detail ? failure(status.detail) : neutral("No controller");
    case "Idle": {
      // A lost job's sentence stays after the reconnect until the next job state (UX notes V7);
      // a refused homing names its cause ("Homing is not enabled (error 5)", `0118` §5).
      const text = stateFailure(status);
      return text === null ? null : failure(text);
    }
    default:
      return null;
  }
}

/** Whether `detail` names a job the connector lost: a safety state, read out as an alert. */
export function isLossText(detail: string | null | undefined): boolean {
  return (
    typeof detail === "string" &&
    (detail.startsWith("Lost the controller") || detail.startsWith("Could not confirm the machine stopped"))
  );
}

/** The sentence of a failure in `Idle` or `Ready`: a lost job or a refused homing; never the
 * controller's own state word ("Controller state: Door"), never the "Stopping" of a stop. */
export function stateFailure(status: StatusMessage): string | null {
  const detail = status.detail ?? null;
  if (detail === null || detail === "" || detail === "Stopping" || controllerWord(status) !== null) return null;
  return detail;
}

/** Whether the job line is an alert (UX notes section 9): an alarm, a lost job, or a refused
 * homing, which answers the maker's press (`specs/0118` UX notes section 5). */
export function jobLineAlerts(status: StatusMessage | null): boolean {
  if (status === null) return false;
  const failed = (status.state.name === "Idle" || status.state.name === "Ready") && stateFailure(status) !== null;
  return status.state.name === "Alarm" || isLossText(status.detail) || failed;
}

/** GRBL's own state word outside the job's states ("Door"), from "Controller state: Door". */
export function controllerWord(status: StatusMessage | null): string | null {
  const prefix = "Controller state: ";
  return status?.detail?.startsWith(prefix) ? status.detail.slice(prefix.length) : null;
}

/** One decimal, never "-0.0" for a value that rounds to zero. */
function tenths(value: number): string {
  const rounded = Math.round(value * 10) / 10;
  return (rounded === 0 ? 0 : rounded).toFixed(1);
}

/** One decimal, the decimal only when needed: "400", "406.4". */
function millimetres(value: number): string {
  const tenths = Math.round(value * 10) / 10;
  return Number.isInteger(tenths) ? String(tenths) : tenths.toFixed(1);
}

/** The rows of the Controller block (`specs/0127` UX notes section 2); a value the controller did
 * not report has no row. */
export function controllerRows(info: ControllerInfo, status: StatusMessage | null): [string, string][] {
  const rows: [string, string][] = [["Controller", `${info.name} ${info.firmware}`]];
  if (info.port) {
    rows.push(["Port", info.baud ? `${info.port}, ${info.baud} baud` : info.port]);
  }
  const word = controllerWord(status);
  if (word !== null) rows.push(["State", word]);
  const settings = info.settings;
  if (settings?.laser_mode != null) rows.push(["Laser mode", settings.laser_mode ? "On" : "Off"]);
  if (settings?.s_max != null) rows.push(["Max power", formatCount(Math.round(settings.s_max))]);
  if (settings?.travel) {
    rows.push(["Travel", `${millimetres(settings.travel[0])} × ${millimetres(settings.travel[1])} mm`]);
  }
  if (status?.position) {
    const [x, y] = status.position;
    rows.push(["Position", `X ${tenths(x)}, Y ${tenths(y)} mm`]);
  }
  return rows;
}

/** Whether the job block shows a progress bar and the counts (UX notes §8 table). */
export function hasProgress(status: StatusMessage): boolean {
  switch (status.state.name) {
    case "AwaitingLocalStart":
    case "Running":
    case "Paused":
    case "Done":
      return true;
    case "Alarm":
      return status.job !== null;
    default:
      return false;
  }
}

/** What is typed in the Code field, cut as typed (UX notes §6): at most 9 characters, upper case. */
export function typedCode(text: string): string {
  return text.toUpperCase().slice(0, 9);
}

/** Whether the field holds 8 characters of the code's alphabet (spaces and hyphens ignored). */
export function codeComplete(text: string): boolean {
  return /^[0-9A-TV-Z]{8}$/.test(text.toUpperCase().replace(/[-\s]/g, ""));
}

/** The fingerprint as two lines of 4 groups (UX notes §5). */
export function fingerprintLines(fingerprint: string): [string, string] {
  const groups = fingerprint.split(" ");
  return [groups.slice(0, 4).join(" "), groups.slice(4).join(" ")];
}
