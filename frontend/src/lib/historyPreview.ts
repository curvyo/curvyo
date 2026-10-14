// Previews from the History list (`specs/0042-history-branches` criteria 14, 17 to 19): what the
// session says a preview shows, the readout and banner words, and a row's preview and apply hint. Pure,
// so `tests/historyPreview.test.ts` covers it. The words need the local clock time, which only the
// browser knows, so they are built here and not in the session.

import { formatRowTime } from "./historyRows.ts";

/** A preview as the session reports it; `kind` is empty without one. */
export interface PreviewState {
  kind: "" | "object" | "document";
  id?: string;
  name?: string;
  author?: "you" | "earlier";
  time?: number;
  step?: string;
  /** Steps that arrived since the preview started. */
  since?: number;
}

/** No preview. */
export const NO_PREVIEW: PreviewState = { kind: "" };

/** Reads what `history_preview` returns. */
export function parsePreview(json: string): PreviewState {
  const value = JSON.parse(json) as PreviewState;
  return value.kind === "object" || value.kind === "document" ? value : NO_PREVIEW;
}

/** The readout of an object preview: "Preview a1b2c3d, Resize, You, 12:03" (criterion 17). */
export function previewReadout(state: PreviewState, now: Date): string {
  const who = state.author === "earlier" ? "Earlier session" : "You";
  return `Preview ${state.id ?? ""}, ${state.name ?? ""}, ${who}, ${formatRowTime(state.time ?? 0, now)}`;
}

/** The banner of a document preview: "Previewing a1b2c3d, 12:03. Editing is paused." (criterion
 * 19). */
export function previewBanner(state: PreviewState, now: Date): string {
  return `Previewing ${state.id ?? ""}, ${formatRowTime(state.time ?? 0, now)}. Editing is paused.`;
}

/** "(3 new steps since)" while other people's steps arrived; empty otherwise. */
export function sinceText(state: PreviewState): string {
  const n = state.since ?? 0;
  if (n <= 0) {
    return "";
  }
  return `(${n} new ${n === 1 ? "step" : "steps"} since)`;
}

/** A row's tooltip with the hint of criterion 14: how to preview and how to apply. */
export function rowHintTooltip(tooltip: string): string {
  return `${tooltip} Click to preview. Double-click or Enter to apply this version.`;
}

/** A row's accessible name with the hint of criterion 14 at its end. */
export function rowHintAriaName(name: string): string {
  return `${name}, Enter applies this version`;
}

/** Whether the panel's Undo, Redo and Wipe are disabled: a document preview pauses editing
 * (criterion 19). */
export function historyButtonsPaused(state: PreviewState): boolean {
  return state.kind === "document";
}
