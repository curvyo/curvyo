// The fixed words of the Guide button and the Template button (`specs/0046-document-shapes`
// criteria 9, 10, 19 and 23, UX notes). Whether the button acts and what it writes is the
// session's; these are the lines of the UI.

/** The Guide button: greyed, not pressed, or pressed. */
export type GuideState = "disabled" | "off" | "pressed";

/** The accessible name and the tooltip of the Guide button: the name only. */
export const GUIDE_NAME = "Guide";

/** The label and the tooltip of the File tab's Template button. */
export const TEMPLATE_NAME = "Template";

/** The canvas notice after a template came in. */
export const TEMPLATE_LOADED_NOTICE = "Template loaded.";

/** The session's `guide_state` string; anything unknown reads as greyed. */
export function readGuideState(state: string): GuideState {
  return state === "off" || state === "pressed" ? state : "disabled";
}

/** What a template load's outcome string shows: the canvas notice, or the error dialog's text. */
export function templateOutcome(outcome: string): { notice: string } | { error: string } {
  if (outcome.startsWith("loaded:")) {
    return { notice: TEMPLATE_LOADED_NOTICE };
  }
  const message = outcome.startsWith("error:") ? outcome.slice("error:".length) : "";
  return { error: message || "This file couldn't be loaded as a template." };
}
