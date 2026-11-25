// The Focus point mode of the jog pad (`specs/0118-machine-control` slice 2d, UX notes W53 to W59):
// why the toggle is greyed, when the mode ends, whether a canvas press goes out, and where the
// target ring stands. Pure, so the tests load it without a bundler.
import { type LinkEntry } from "./connectorLink.ts";
import { type StatusMessage, controllerWord, errorLine, waitingText } from "./connectorText.ts";

/** The name and tooltip of the toggle. */
export const FOCUS_POINT = "Focus point";

/** The pulse a press asks for, as the page's profile copy gives it (`specs/0118` 35 a). */
export interface FocusPulse {
  powerPermille: number;
  lengthMs: number;
}

/** Where a press on the page sends the head (job file mm), or the page's cause (W55). */
export type FocusTarget = { x: number; y: number } | { cause: string };

/** Whether this app's own go to point moves, pulses or waits for Enter at the machine. */
export function ownFocus(status: StatusMessage | null, pressed: boolean): boolean {
  if (!pressed || status === null) return false;
  const name = status.state.name;
  return name === "Focusing" || name === "Pulsing" || (name === "AwaitingLocalStart" && Boolean(status.target));
}

/**
 * Why the toggle is greyed (W55), the first cause that applies: the connector's causes, then "Not
 * homed", then the page's (`page`, "Start is Head position" or "Nothing to place"). `own` is this
 * app's own jog, move, pulse or wait: those are no causes. Laser mode is no cause; the move is
 * laser off and only the pulse is left out.
 */
export function focusCause(entry: LinkEntry, page: string | null, own: { jog: boolean; focus: boolean }): string | null {
  const status = entry.status;
  if (entry.lost) return "Connection lost";
  if (status === null || status === undefined) {
    if (entry.reading || entry.readError === null) return "Testing";
    return errorLine(entry.readError).text;
  }
  switch (status.state.name) {
    case "NoController":
      return status.detail ?? "No controller";
    case "Homing":
      return "Homing";
    case "Jogging":
      if (!own.jog) return "Jogging";
      break;
    case "Framing":
      return "Framing";
    case "Focusing":
      if (!own.focus) return "Focus point";
      break;
    case "Pulsing":
      if (!own.focus) return "Pulse";
      break;
    case "Running":
    case "Paused":
      return "A job is running";
    case "AwaitingLocalStart":
      if (!own.focus) return waitingText(status, entry.session);
      break;
    case "Receiving":
      return "Sending";
    case "Alarm":
      return "Alarm";
    default:
      break;
  }
  const word = controllerWord(status);
  if (word !== null && word !== "Jog") return `Controller state: ${word}`;
  if (status.homed !== true) return "Not homed";
  return page;
}

/** Whether a press on the canvas goes out now: never during this app's own move, pulse or wait
 * (W57, W58), which queues nothing. */
export function pressTakes(status: StatusMessage | null, pressed: boolean): boolean {
  return !ownFocus(status, pressed);
}

/** The target ring's place in job file mm: the connector's clamped target of this app's go to
 * point (W59); `null` while the status carries none or the go to point is another app's. */
export function ringTarget(status: StatusMessage | null, pressed: boolean): [number, number] | null {
  if (!ownFocus(status, pressed) || !status?.target) return null;
  return [status.target[0], status.target[1]];
}

/** A click, not a drag (W57): pointer down and up within the canvas's click tolerance. */
export const CLICK_TOLERANCE_PX = 4;

export function isClick(down: { x: number; y: number }, up: { x: number; y: number }): boolean {
  return Math.hypot(up.x - down.x, up.y - down.y) <= CLICK_TOLERANCE_PX;
}

/** The session's answer for a press, read from its JSON; `null` off a laser page. */
export function targetOf(json: string): FocusTarget | null {
  const value: unknown = JSON.parse(json || "null");
  if (value === null || typeof value !== "object") return null;
  const v = value as { x?: number; y?: number; cause?: string };
  if (typeof v.cause === "string") return { cause: v.cause };
  if (typeof v.x === "number" && typeof v.y === "number") return { x: v.x, y: v.y };
  return null;
}

/** The pulse of the session's JSON (`{power_permille, length_ms}` or `null`). */
export function pulseOf(json: string): FocusPulse | null {
  const value: unknown = JSON.parse(json || "null");
  if (value === null || typeof value !== "object") return null;
  const v = value as { power_permille?: number; length_ms?: number };
  if (typeof v.power_permille !== "number" || typeof v.length_ms !== "number") return null;
  return { powerPermille: v.power_permille, lengthMs: v.length_ms };
}

/** The page's cause of the toggle without a press: the session's answer for any page point. */
export function pageCause(target: FocusTarget | null): string | null {
  if (target === null) return "Nothing to place";
  return "cause" in target ? target.cause : null;
}
