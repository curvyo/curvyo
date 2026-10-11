// Whether the Properties panel's body is hidden, kept for the OS user in the webview's own storage
// (`specs/0051-sidebar-and-no-menu/` criterion 10): no file, no host command, not in the project.
// Every access is guarded: storage that cannot be read or written means the body is shown, and
// nothing else is affected.

/** The storage key; the dev server and a release build keep separate values (accepted). */
export const PANEL_COLLAPSED_KEY = "curvyo.panel-collapsed";

/** The part of `Storage` this module uses, so a test can hand in a fake. */
export interface PanelStorage {
  getItem(key: string): string | null;
  setItem(key: string, value: string): void;
}

/** The webview's `localStorage`, or `undefined` when even reaching it throws (a private window,
 * blocked site data). */
function defaultStorage(): PanelStorage | undefined {
  try {
    return window.localStorage;
  } catch {
    return undefined;
  }
}

/** Whether the body was hidden when the maker last changed it. A first start, an unknown value and
 * storage that throws all mean shown (`false`). */
export function readPanelCollapsed(storage: PanelStorage | undefined = defaultStorage()): boolean {
  try {
    return storage?.getItem(PANEL_COLLAPSED_KEY) === "true";
  } catch {
    return false;
  }
}

/** Remembers whether the body is hidden. A failing write is ignored. */
export function writePanelCollapsed(
  collapsed: boolean,
  storage: PanelStorage | undefined = defaultStorage(),
): void {
  try {
    storage?.setItem(PANEL_COLLAPSED_KEY, collapsed ? "true" : "false");
  } catch {
    /* The choice is not kept; the next start shows the body. */
  }
}
