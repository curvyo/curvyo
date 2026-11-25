// The Frame row of the Machine controls footer (`specs/0118-machine-control` slice 2b, UX notes
// W28 to W38): the button's name, state and cause, the frame pick, and where the head ring stands.
// Pure, so the tests load it without a bundler.
import { type LinkEntry } from "./connectorLink.ts";
import { confirmsFrame, controllerWord, errorLine, waitingText } from "./connectorText.ts";
import { type StatusMessage } from "./connectorText.ts";

/** The two shapes of the frame pick (Frage 311 A). */
export type FrameShape = "rectangle" | "rubber_band";

/** The pick's segments in order, with the words they show. */
export const FRAME_SHAPES: { value: FrameShape; label: string }[] = [
  { value: "rectangle", label: "Rectangle" },
  { value: "rubber_band", label: "Rubber band" },
];

/** What the page says of Frame: whether it has something to frame, and the cause if not. */
export interface PageFrame {
  enabled: boolean;
  cause: string | null;
}

/** What the Frame slot of the row shows. */
export interface FrameSlot {
  enabled: boolean;
  busy: boolean;
  cause: string | null;
  name: "Frame" | "Confirm frame";
}

/** Why Frame is greyed, the first cause that applies (W31): the connector's causes, shared by
 * every button, before the page's. Laser mode and "Not homed" are no causes. */
export function frameCause(entry: LinkEntry, page: PageFrame | null): string | null {
  const status = entry.status;
  if (entry.lost) return "Connection lost";
  if (status === null || status === undefined) {
    if (entry.reading || entry.readError === null) return "Testing";
    return errorLine(entry.readError).text;
  }
  switch (status.state.name) {
    case "Homing":
      return "Homing";
    case "Jogging":
      return "Jogging";
    case "Focusing":
      return "Focus point";
    case "Pulsing":
      return "Pulse";
    case "Running":
    case "Paused":
      return "A job is running";
    case "AwaitingLocalStart":
      return waitingText(status, entry.session);
    case "Receiving":
      return "Sending";
    case "Alarm":
      return "Alarm";
    case "NoController":
      return status.detail ?? "No controller";
    default:
      break;
  }
  const word = controllerWord(status);
  if (word !== null) return `Controller state: ${word}`;
  if (page === null) return "Nothing to frame";
  return page.enabled ? null : (page.cause ?? "Nothing to frame");
}

/** The Frame slot: "Confirm frame" for an owner app while a member's frame waits (not armed,
 * W36); busy from the press until the answer and while the connector reports Framing (W29). */
export function frameSlot(entry: LinkEntry, page: PageFrame | null): FrameSlot {
  const status = entry.status;
  if (confirmsFrame(status, entry.session) && !entry.lost) {
    return { enabled: entry.busy === null, busy: false, cause: null, name: "Confirm frame" };
  }
  if (entry.busy === "frame" || status?.state.name === "Framing") {
    return { enabled: false, busy: true, cause: "Framing", name: "Frame" };
  }
  const cause = frameCause(entry, page);
  return { enabled: cause === null && entry.busy === null, busy: false, cause, name: "Frame" };
}

/** Where the head ring stands for `status`: the controller's work position, or for Head position
 * the offset from `first`, the position when this app's frame began; `null` while not framing. */
export function headPlace(
  status: StatusMessage | null,
  relative: boolean,
  first: [number, number] | null,
): { x: number; y: number } | null {
  const at = status?.position ?? null;
  if (status?.state.name !== "Framing" || at === null) return null;
  if (!relative) return { x: at[0], y: at[1] };
  const from = first ?? [at[0], at[1]];
  return { x: at[0] - from[0], y: at[1] - from[1] };
}

/** The next pick value for an arrow key in the radiogroup: no wrap (W37). */
export function nextShape(current: FrameShape, key: string): FrameShape {
  const at = FRAME_SHAPES.findIndex((shape) => shape.value === current);
  const to = key === "ArrowRight" || key === "ArrowDown" ? at + 1 : key === "ArrowLeft" || key === "ArrowUp" ? at - 1 : at;
  return FRAME_SHAPES[Math.min(FRAME_SHAPES.length - 1, Math.max(0, to))].value;
}
