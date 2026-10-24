// The font preview's view state kept per user (`specs/0077-font-preview` criterion 6, `adrs.md`
// decision 4): the sample last typed, the size, split or full and the dragged split (criterion 27),
// and the glyph table's mode, search and Recent list (`specs/0082-glyph-table` criteria 3, 16 and
// 18, `adrs.md` decision 5), as JSON in the webview's own storage under one key. Guarded as `panelCollapsed.ts`: unreadable or out-of-range values mean
// the defaults, a failing write is ignored.

/** The storage key, the same in the desktop and the browser build. */
export const FONT_PREVIEW_KEY = "curvyo.font-preview";

/** The sample of a first start (criterion 8, Proposal). */
export const DEFAULT_SAMPLE = "Anna & Jonas 2026";
/** The sample field's limit (criterion 7). */
export const MAX_SAMPLE_CHARS = 200;
/** The Size slider's range and default, cap height in px (criterion 12). */
export const SIZE_RANGE = { min: 16, max: 160, step: 1 } as const;
export const DEFAULT_SIZE = 32;

/** The pane's modes (`specs/0082-glyph-table` criterion 1). */
export type PreviewMode = "fonts" | "glyphs";
/** The glyph search field's limit (`0082` criterion 16). */
export const MAX_GLYPH_SEARCH_CHARS = 64;
/** The Recent list's length (`0082` criterion 18). */
export const MAX_RECENT = 24;

/** Whether `value` is one Unicode scalar that is not a control character (general category Cc). */
export function isOneCharacter(value: unknown): value is string {
  if (typeof value !== "string") {
    return false;
  }
  const chars = [...value];
  return chars.length === 1 && !/^\p{Cc}$/u.test(value) && !/^[\uD800-\uDFFF]$/.test(value);
}

/** The Recent list after `character` was inserted: it goes first, once, at most 24 kept. */
export function withRecent(recent: readonly string[], character: string): string[] {
  return [character, ...recent.filter((c) => c !== character)].slice(0, MAX_RECENT);
}

/** What is kept between starts. */
export interface PreviewPrefs {
  /** The sample last typed into the field. */
  sample: string;
  /** The cap height in px. */
  size: number;
  /** Full view chosen (not the forced full view of a narrow window). */
  full: boolean;
  /** The split pane's share of the canvas area once the maker dragged the divider; absent means
   * the default (criterion 27). */
  split?: number;
  /** The mode last shown; absent means Fonts (`0082` criterion 3). */
  mode?: PreviewMode;
  /** The glyph search field's text; absent means empty (`0082` criterion 16). */
  glyphSearch?: string;
  /** The characters last inserted from the glyph table, most recent first; absent means none
   * (`0082` criterion 18). */
  recent?: string[];
}

export const DEFAULT_PREFS: PreviewPrefs = { sample: DEFAULT_SAMPLE, size: DEFAULT_SIZE, full: false };

/** The part of `Storage` this module uses, so a test can hand in a fake. */
export interface PreviewStorage {
  getItem(key: string): string | null;
  setItem(key: string, value: string): void;
}

function defaultStorage(): PreviewStorage | undefined {
  try {
    return window.localStorage;
  } catch {
    return undefined;
  }
}

/** Whether `size` is a whole number of px in the slider's range. */
function sizeInRange(size: unknown): size is number {
  return typeof size === "number" && Number.isInteger(size) && size >= SIZE_RANGE.min && size <= SIZE_RANGE.max;
}

/** The kept state; each unreadable or out-of-range value falls back to its default alone. */
export function readPreviewPrefs(storage: PreviewStorage | undefined = defaultStorage()): PreviewPrefs {
  let raw: unknown = null;
  try {
    raw = JSON.parse(storage?.getItem(FONT_PREVIEW_KEY) ?? "null");
  } catch {
    raw = null;
  }
  if (typeof raw !== "object" || raw === null) {
    return { ...DEFAULT_PREFS };
  }
  const value = raw as Record<string, unknown>;
  const split = value.split;
  const { mode, glyphSearch, recent } = value;
  const kept = Array.isArray(recent) ? [...new Set(recent.filter(isOneCharacter))].slice(0, MAX_RECENT) : [];
  return {
    sample:
      typeof value.sample === "string" && [...value.sample].length <= MAX_SAMPLE_CHARS
        ? value.sample
        : DEFAULT_SAMPLE,
    size: sizeInRange(value.size) ? value.size : DEFAULT_SIZE,
    full: typeof value.full === "boolean" ? value.full : false,
    ...(typeof split === "number" && split > 0 && split < 1 ? { split } : {}),
    ...(mode === "fonts" || mode === "glyphs" ? { mode } : {}),
    ...(typeof glyphSearch === "string" && [...glyphSearch].length <= MAX_GLYPH_SEARCH_CHARS ? { glyphSearch } : {}),
    ...(kept.length > 0 ? { recent: kept } : {}),
  };
}

/** Keeps the state. A failing write is ignored: the next start shows the defaults. */
export function writePreviewPrefs(prefs: PreviewPrefs, storage: PreviewStorage | undefined = defaultStorage()): void {
  try {
    storage?.setItem(FONT_PREVIEW_KEY, JSON.stringify(prefs));
  } catch {
    /* Not kept. */
  }
}

/** The size a slider position or a typed value gives: whole px, held to the range. */
export function clampSize(size: number): number {
  if (!Number.isFinite(size)) {
    return DEFAULT_SIZE;
  }
  return Math.min(SIZE_RANGE.max, Math.max(SIZE_RANGE.min, Math.round(size)));
}
