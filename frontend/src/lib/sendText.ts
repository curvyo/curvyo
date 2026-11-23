// The words of Send to machine (`specs/0125-send-to-machine` S1 to S5; UX notes §3;
// `specs/0134-send-and-start` UX notes sections 3 and 6): the button's name per state, the cause it
// shows while it is disabled, and the "Not started" reason. Pure, so the tests load it without a
// bundler.
import type { SendButton } from "./outputView.ts";

/** The button's name, tooltip and accessible name when a press only sends. */
export const SEND_NAME = "Send to machine";
/** The name when the next press arms and the one after it sends and starts (`0134` SS9). */
export const SEND_START_NAME = "Send to machine and start";
/** The name while armed (`0134` SS13). */
export const SEND_ARMED_NAME = "Press again to send to machine and start";

/** The button's name: armed, a press that starts (also while that press sends and starts), or a
 * press that only sends. */
export function sendName(starts: boolean, armed: boolean): string {
  if (armed) return SEND_ARMED_NAME;
  return starts ? SEND_START_NAME : SEND_NAME;
}

const NOT_STARTED: Record<string, string> = {
  WrongState: "Connector busy",
  UnknownJob: "The connector holds another job",
  ConnectionLost: "Connection lost",
};

/** "Not started: <reason>" from the host's reason code (`0134` UX notes section 6). */
export function notStartedText(reason: string): string {
  return `Not started: ${NOT_STARTED[reason] ?? reason}`;
}

/** The words of a cause; the session's own text for a check block or a format. */
export function sendCauseText(send: SendButton): string | null {
  switch (send.cause) {
    case null:
      return null;
    case "blocked":
    case "format_not_spoken":
      return send.text;
    case "browser":
      return "Needs the desktop app";
    case "not_paired":
      return "Not paired";
    case "no_longer_paired":
      return "No longer paired";
    case "key_changed":
      return "This connector's key changed";
    case "sending":
      return "Sending";
    case "list_full":
      return "List full";
  }
}
