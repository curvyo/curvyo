// The fixed words of the Guide button and the Template button (`specs/0046-document-shapes`
// criteria 9, 10, 19 and 23, UX notes). Whether the button acts and what it writes is the
// session's; these are the lines of the UI.

/** The Guide button: greyed, not pressed, pressed, or pressed and inert (only linked clones of
 * guides are selected: they show their origin's flag and carry none of their own). */
export type GuideState = "disabled" | "off" | "pressed" | "pressed-inert";

/** The accessible name and the tooltip of the Guide button: the name only. */
export const GUIDE_NAME = "Guide";

/** The label and the tooltip of the File tab's Template button. */
export const TEMPLATE_NAME = "Template";

/** The canvas notice after a template came in. */
export const TEMPLATE_LOADED_NOTICE = "Template loaded.";

/** The session's `guide_state` string; anything unknown reads as greyed. */
export function readGuideState(state: string): GuideState {
  return state === "off" || state === "pressed" || state === "pressed-inert" ? state : "disabled";
}

/** What a template load's outcome string shows: the canvas notice, or the error dialog's text. */
export function templateOutcome(outcome: string): { notice: string } | { error: string } {
  if (outcome.startsWith("loaded:")) {
    return { notice: TEMPLATE_LOADED_NOTICE };
  }
  const message = outcome.startsWith("error:") ? outcome.slice("error:".length) : "";
  return { error: message || "This file couldn't be loaded as a template." };
}

/** The left edge of the canvas area right of the tool rail, CSS pixels, when no bar is laid out. */
const RAIL_FREE_LEFT_PX = 128;

/** The space kept between the bar overlay and what is fitted under it, CSS pixels. */
const BAR_GAP_PX = 12;

/** A rectangle as `getBoundingClientRect` gives it. */
export interface Box {
  left: number;
  top: number;
  bottom: number;
}

/**
 * The part of the canvas the bars leave free (`specs/0046-document-shapes` criterion 13): right
 * of the bar overlay's left edge (the tool rail) and below its bottom edge, in CSS pixels from
 * the canvas's top-left corner. Without a bar, the rail's edge and the top margin.
 */
export function freeCanvasArea(canvas: Box, bar: Box | null): { left: number; top: number } {
  if (!bar) {
    return { left: RAIL_FREE_LEFT_PX, top: 24 };
  }
  return {
    left: Math.max(bar.left - canvas.left, 0),
    top: Math.max(bar.bottom - canvas.top + BAR_GAP_PX, 0),
  };
}
