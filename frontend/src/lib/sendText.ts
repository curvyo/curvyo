// The words of Send to machine (`specs/0125-send-to-machine` S1 to S5; UX notes §3): the button's
// name and the cause it shows while it is disabled. Pure, so the tests load it without a bundler.
import type { SendButton } from "./outputView.ts";

/** The button's name, tooltip and accessible name. */
export const SEND_NAME = "Send to machine";

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
  }
}
