// The main window's pen window button state (`specs/0158-pen-display` slice 2a criterion 1, UX
// 9.1) and its "Show pen view" switch (2b criterion 6): the label is the state. The link hook
// publishes it; the File tab's Pen display section and the frame overlay read it.
import { useSyncExternalStore } from "react";

/** Where the pen window stands, seen from the main window. */
export type PenWindowState = "closed" | "opening" | "open";

/** The button's label in each state. */
export const PEN_WINDOW_LABELS: Record<PenWindowState, string> = {
  closed: "Open pen window",
  opening: "Opening pen window...",
  open: "Show pen window",
};

/** The notice when the pen window does not come up (criterion 1). */
export const PEN_WINDOW_FAILED = "The pen window could not be opened.";

let state: PenWindowState = "closed";
let press: () => void = () => undefined;
/** The "Show pen view" switch (2b criterion 6): on each time the pen window opens. */
let viewShown = true;
const listeners = new Set<() => void>();

function changed(): void {
  for (const listener of listeners) listener();
}

/** Publishes the state; the hook that owns the link calls it. */
export function setPenWindowState(next: PenWindowState): void {
  if (next === state) return;
  if (next === "open") viewShown = true;
  state = next;
  changed();
}

/** Whether the main window draws the pen window's frame: open and the switch on. */
export function penViewFrameShown(): boolean {
  return state === "open" && viewShown;
}

/** The "Show pen view" switch. */
export function penViewShown(): boolean {
  return viewShown;
}

/** Sets the "Show pen view" switch. */
export function setPenViewShown(on: boolean): void {
  if (on === viewShown) return;
  viewShown = on;
  changed();
}

/** The current state. */
export function penWindowState(): PenWindowState {
  return state;
}

/** What a press of the button does; the hook that owns the link sets it. */
export function setPenWindowPress(handler: () => void): void {
  press = handler;
}

/** A press of the button. */
export function pressPenWindow(): void {
  press();
}

function subscribe(listener: () => void): () => void {
  listeners.add(listener);
  return () => {
    listeners.delete(listener);
  };
}

/** The state, re-rendering on change. */
export function usePenWindowState(): PenWindowState {
  return useSyncExternalStore(subscribe, penWindowState);
}

/** The "Show pen view" switch, re-rendering on change. */
export function usePenViewShown(): boolean {
  return useSyncExternalStore(subscribe, penViewShown);
}
