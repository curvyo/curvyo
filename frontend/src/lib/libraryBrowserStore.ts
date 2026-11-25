// The library browser's page state (`specs/0152-library-browser` criteria 1 to 3 and 9, ADR 0086
// decisions 5 and 6): open or closed (one kind of the one side pane), the list in the page's
// `WasmLibraryBrowser`, asked again whenever the Library tab asks for its rows, and the session's
// choices that are not kept per user (Source, Category, the second level, the current tile).
import { useSyncExternalStore } from "react";

import { browserItems } from "@/lib/libraryApi";
import { claimPane, registerPaneCloser, releasePane } from "@/lib/sidePaneStore";
import type { ColourFilter } from "@/lib/libraryBrowserState";
import { WasmLibraryBrowser } from "@/wasm-bindings/curvyo_editor_wasm.js";

/** The DOM id of the Library tab's Browse button, where the focus goes when the browser closes
 * with the focus inside (UX notes 2). */
export const BROWSE_BUTTON_ID = "library-browse-button";

/** What lives for the session only (UX G7). */
export interface BrowserSession {
  sources: string[];
  categories: string[];
  /** The item whose pages are shown. */
  level: number | null;
  /** The current tile's item, per level (the top level under `-1`). */
  current: Map<number, number>;
  /** The grid's scroll position, kept while the Page preview stands in (UX U19). */
  scrollTop: number;
  /** The hosting guide stands in for the view (criterion 32). */
  guide: boolean;
  /** Sources is a set, catalogue or folder opened from the Sets view (criterion 27). */
  narrowed: boolean;
  /** The Sets view's own search and facets (H10). */
  sets: SetsChoice;
  /** The Sets view's current card. */
  setsCurrent: number | null;
}

/** The Sets view's choices, as `SetsState` reads them. */
export interface SetsChoice {
  text: string;
  sources: string[];
  categories: string[];
  colour: ColourFilter;
}

interface State {
  open: boolean;
  /** Opened by Enter or Space: the search field takes the focus (UX notes 2). */
  focusSearch: boolean;
  /** Back from the Page preview: the current tile takes the focus (UX U19). */
  focusTile: boolean;
  /** Bumped when the list changes. */
  version: number;
  /** Whether an Iconify source is added (the Search online button, criterion 21). */
  hasIconify: boolean;
  session: BrowserSession;
  /** Bumped when the history of places changes (the Back button's name). */
  history: number;
  /** A place to show, taken off the history by Back (JSON `Place`); applied by the pane. */
  pendingPlace: string | null;
  /** The hosting guide was asked for while the pane was closed. */
  pendingGuide: boolean;
}

let state: State = {
  open: false,
  focusSearch: false,
  focusTile: false,
  version: 0,
  hasIconify: false,
  session: {
    sources: [],
    categories: [],
    level: null,
    current: new Map(),
    scrollTop: 0,
    guide: false,
    narrowed: false,
    sets: { text: "", sources: [], categories: [], colour: "all" },
    setsCurrent: null,
  },
  history: 0,
  pendingPlace: null,
  pendingGuide: false,
};
const listeners = new Set<() => void>();
let browser: WasmLibraryBrowser | null = null;
let revision = "";
let held = "";
let asking = false;
let askAgain = false;

function set(next: Partial<State>): void {
  state = { ...state, ...next };
  for (const listener of listeners) {
    listener();
  }
}

function subscribe(listener: () => void): () => void {
  listeners.add(listener);
  return () => listeners.delete(listener);
}

/** The browser's state, live. */
export function useBrowserStore(): State {
  return useSyncExternalStore(subscribe, () => state);
}

/** Whether the browser is open, live (the Browse button's pressed state). */
export function useBrowserOpen(): boolean {
  return useSyncExternalStore(subscribe, () => state.open);
}

/** The page's browser; made on first use (the wasm module is loaded by then). */
export function browserInstance(): WasmLibraryBrowser {
  browser ??= new WasmLibraryBrowser();
  return browser;
}

/** Asks the host for the list; it answers only when it changed (ADR 0086 decision 5). */
export async function refreshItems(): Promise<void> {
  if (asking) {
    askAgain = true;
    return;
  }
  asking = true;
  try {
    do {
      askAgain = false;
      const answer = await browserItems(revision);
      if (answer && answer.json !== held && browserInstance().set_items(answer.json)) {
        revision = answer.revision;
        held = answer.json;
        set({
          version: state.version + 1,
          hasIconify: held.includes('"glyph":"iconify"') || held.includes('"key":"set:'),
        });
      }
    } while (askAgain);
  } catch {
    /* The list stays as it was. */
  } finally {
    asking = false;
  }
}

const fetchedListeners = new Set<() => void>();

/** Calls `listener` whenever the browser fetched or added something, so the Library tab asks for
 * its rows again (one data, two views); returns the unsubscribe. */
export function onBrowserFetched(listener: () => void): () => void {
  fetchedListeners.add(listener);
  return () => fetchedListeners.delete(listener);
}

/** The browser added a source, loaded a list or set, searched or loaded a template: both views
 * ask again. */
export function browserFetched(): void {
  void refreshItems();
  for (const listener of fetchedListeners) {
    listener();
  }
}

/** The Library tab asked for its rows, or the browser fetched: the list may have changed. */
export function notifyLibraryChanged(): void {
  if (state.open) {
    void refreshItems();
  }
}

/** Opens the browser; `focusSearch` when a key opened it. Nothing is fetched (criterion 9). */
export function openBrowser(focusSearch = false): void {
  claimPane("library-browser");
  set({ open: true, focusSearch });
  void refreshItems();
}

/** The maker left the place JSON; `byOpening` when something was opened from it (criterion 30). */
export function pushPlace(place: string, byOpening: boolean): void {
  browserInstance().push_place(place, byOpening);
  set({ history: state.history + 1 });
}

/** The Back button's name ("Back to Sets"); empty while there is no place to go back to. */
export function backName(): string {
  return browserInstance().back_name();
}

/** Whether Escape goes Back (the newest place was left by opening something), else closes. */
export function escapeGoesBack(): boolean {
  return browserInstance().escape_goes_back();
}

/** Back (criterion 30): the newest place left comes back; the pane shows it (also from the Page
 * preview of Pick elements…). False when there is none. */
export function goBack(): boolean {
  const place = browserInstance().back();
  if (!place) {
    return false;
  }
  claimPane("library-browser");
  set({ open: true, focusSearch: false, focusTile: true, pendingPlace: place, history: state.history + 1 });
  return true;
}

/** The pane applied the place Back brought. */
export function placeApplied(): void {
  if (state.pendingPlace !== null) {
    set({ pendingPlace: null });
  }
}

/** Host your own library… (criterion 32): the guide as a place of the browser, opened if closed. */
export function openGuide(): void {
  claimPane("library-browser");
  set({ open: true, pendingGuide: true });
  void refreshItems();
}

/** The pane took the guide it was asked for. */
export function guideOpened(): void {
  if (state.pendingGuide) {
    set({ pendingGuide: false });
  }
}

/** Back from the Page preview to the browser as it was, the focus on its current tile. */
export function returnToBrowser(): void {
  if (goBack()) {
    return;
  }
  claimPane("library-browser");
  set({ open: true, focusSearch: false, focusTile: true });
  void refreshItems();
}

/** The current tile took the focus it was asked to. */
export function tileFocused(): void {
  if (state.focusTile) {
    set({ focusTile: false });
  }
}

/** Closes the browser; its session choices stay for the next opening. */
export function closeBrowser(): void {
  if (state.open) {
    set({ open: false, focusSearch: false, focusTile: false });
  }
  releasePane("library-browser");
}

/** The Browse button. */
export function toggleBrowser(byKeyboard: boolean): void {
  if (state.open) {
    closeBrowser();
  } else {
    openBrowser(byKeyboard);
  }
}

/** The search field took the focus it was asked to. */
export function searchFocused(): void {
  if (state.focusSearch) {
    set({ focusSearch: false });
  }
}

/** Changes the session's choices. */
export function setBrowserSession(change: Partial<BrowserSession>): void {
  set({ session: { ...state.session, ...change } });
}

registerPaneCloser("library-browser", closeBrowser);
