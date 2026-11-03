// The View card's Pan and Zoom of a manufacturing workbench (`specs/0093` UX notes §3): which one
// is pressed. The canvas reads it on a press: Pan drags the view, Zoom zooms in (Alt: out).
export type ViewTool = "none" | "pan" | "zoom";

let current: ViewTool = "none";
const listeners = new Set<() => void>();

/** The pressed view tool. */
export function viewTool(): ViewTool {
  return current;
}

/** Presses `tool`, or releases it when it already is pressed. */
export function toggleViewTool(tool: Exclude<ViewTool, "none">): void {
  setViewTool(current === tool ? "none" : tool);
}

/** Sets the pressed view tool. */
export function setViewTool(tool: ViewTool): void {
  if (tool !== current) {
    current = tool;
    for (const listener of listeners) listener();
  }
}

/** For `useSyncExternalStore`. */
export function subscribeViewTool(listener: () => void): () => void {
  listeners.add(listener);
  return () => listeners.delete(listener);
}
