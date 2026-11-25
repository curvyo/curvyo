// The library browser's view state kept per user (`specs/0152-library-browser` criterion 3, UX
// notes section 2): split or full, its own dragged share, the Size, the sort chosen without text,
// Colour, Customizable, the Filters band and the search text, as JSON in the webview's storage
// under one key. Source and Category selections are not kept (UX G7). Guarded as
// `fontPreviewState.ts`: an unreadable value means its default, a failing write is ignored.

/** The storage key, the same in the desktop and the browser build. */
export const LIBRARY_BROWSER_KEY = "curvyo.library-browser";

/** The Size slider: the tile's preview edge in px (criterion 4). */
export const BROWSER_SIZE_RANGE = { min: 48, max: 256, step: 1 } as const;
export const DEFAULT_BROWSER_SIZE = 96;
/** The search field's limit (`0138` criterion 10). */
export const MAX_BROWSER_SEARCH_CHARS = 100;

export type BrowserSort = "best" | "name" | "source" | "newest";
/** The two views (criterion 26, H1). */
export type BrowserView = "items" | "sets";
export type ColourFilter = "all" | "mono" | "colour";

export interface BrowserPrefs {
  /** Items or Sets, kept per user (H8). */
  view: BrowserView;
  full: boolean;
  /** The dragged split's share of the canvas area; absent means the default. */
  split?: number;
  size: number;
  /** The sort the maker chose last without text (Best match is never kept). */
  sort: Exclude<BrowserSort, "best">;
  colour: ColourFilter;
  customizable: boolean;
  /** The narrow layout's Filters band is open. */
  filtersOpen: boolean;
  text: string;
}

export const DEFAULT_BROWSER_PREFS: BrowserPrefs = {
  view: "items",
  full: false,
  size: DEFAULT_BROWSER_SIZE,
  sort: "name",
  colour: "all",
  customizable: false,
  filtersOpen: false,
  text: "",
};

/** The part of `Storage` this module uses, so a test can hand in a fake. */
export interface BrowserStorage {
  getItem(key: string): string | null;
  setItem(key: string, value: string): void;
}

function defaultStorage(): BrowserStorage | undefined {
  try {
    return window.localStorage;
  } catch {
    return undefined;
  }
}

/** A size held to whole px of the slider's range. */
export function clampBrowserSize(size: number): number {
  if (!Number.isFinite(size)) {
    return DEFAULT_BROWSER_SIZE;
  }
  return Math.min(BROWSER_SIZE_RANGE.max, Math.max(BROWSER_SIZE_RANGE.min, Math.round(size)));
}

/** The kept state; each unreadable or out-of-range value falls back to its default alone. */
export function readBrowserPrefs(storage: BrowserStorage | undefined = defaultStorage()): BrowserPrefs {
  let raw: unknown = null;
  try {
    raw = JSON.parse(storage?.getItem(LIBRARY_BROWSER_KEY) ?? "null");
  } catch {
    raw = null;
  }
  if (typeof raw !== "object" || raw === null) {
    return { ...DEFAULT_BROWSER_PREFS };
  }
  const value = raw as Record<string, unknown>;
  const { split, size, sort, colour, text } = value;
  return {
    view: value.view === "sets" ? "sets" : "items",
    full: typeof value.full === "boolean" ? value.full : false,
    ...(typeof split === "number" && split > 0 && split < 1 ? { split } : {}),
    size:
      typeof size === "number" && Number.isInteger(size) && size >= BROWSER_SIZE_RANGE.min && size <= BROWSER_SIZE_RANGE.max
        ? size
        : DEFAULT_BROWSER_SIZE,
    sort: sort === "name" || sort === "source" || sort === "newest" ? sort : "name",
    colour: colour === "mono" || colour === "colour" ? colour : "all",
    customizable: value.customizable === true,
    filtersOpen: value.filtersOpen === true,
    text: typeof text === "string" && [...text].length <= MAX_BROWSER_SEARCH_CHARS ? text : "",
  };
}

/** Keeps the state. A failing write is ignored: the next start shows the defaults. */
export function writeBrowserPrefs(prefs: BrowserPrefs, storage: BrowserStorage | undefined = defaultStorage()): void {
  try {
    storage?.setItem(LIBRARY_BROWSER_KEY, JSON.stringify(prefs));
  } catch {
    /* Not kept. */
  }
}
