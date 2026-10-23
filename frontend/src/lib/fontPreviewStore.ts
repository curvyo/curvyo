// The font preview's page state (`specs/0077-font-preview` criteria 1, 5, 16 and 26): whether it
// is open, and the Fonts tab's listed families as last published, so the preview keeps them
// while another Properties tab is shown or the panel is collapsed.
import type { RowTag } from "@/lib/fontRows";
import type { FontFamily } from "@/lib/textText";

/** One listed family and its tags (criterion 14): all of them as chips, Cuttable and Single line
 * also in the tile's accessible name (criterion 22). */
export interface PreviewFamily {
  family: FontFamily;
  tags: RowTag[];
  cuttable: boolean;
  singleLine: boolean;
}

/** What the Fonts tab lists now. */
export interface PreviewListing {
  families: PreviewFamily[];
  /** "All · 1,240", "Cuttable · 3 of 42". */
  subject: string;
  /** The Fonts tab's empty text when nothing is listed ("No font in Cuttable."), else `null`. */
  empty: string | null;
  /** The strip cell and the search: a change scrolls the grid to the top. */
  filter: string;
  /** Toggles a family's star as the list does (the row stays until the filter changes). */
  star: (family: FontFamily) => void;
  /** The list's tag edits (`specs/0075-font-library` criterion 10), the row kept as there. */
  addTag: (family: string, name: string) => { error?: string } | null;
  removeTag: (family: string, id: string) => void;
  completions: (family: string, prefix: string) => string[];
}

export interface PreviewState {
  open: boolean;
  /** Opened by Enter or Space: the sample field takes the focus (UX notes §1). */
  focusSample: boolean;
  listing: PreviewListing;
}

const EMPTY_LISTING: PreviewListing = {
  families: [],
  subject: "",
  empty: "Loading fonts…",
  filter: "",
  star: () => {},
  addTag: () => null,
  removeTag: () => {},
  completions: () => [],
};

let state: PreviewState = { open: false, focusSample: false, listing: EMPTY_LISTING };
const listeners = new Set<() => void>();

function set(next: Partial<PreviewState>): void {
  state = { ...state, ...next };
  for (const listener of listeners) {
    listener();
  }
}

export function subscribePreview(listener: () => void): () => void {
  listeners.add(listener);
  return () => listeners.delete(listener);
}

export function previewState(): PreviewState {
  return state;
}

/** Opens or closes the preview; `focusSample` when a key opened it. */
export function setPreviewOpen(open: boolean, focusSample = false): void {
  if (state.open !== open || state.focusSample !== focusSample) {
    set({ open, focusSample: open && focusSample });
  }
}

/** The sample field took the focus it was asked to. */
export function sampleFocused(): void {
  if (state.focusSample) {
    set({ focusSample: false });
  }
}

/** The Fonts tab's listing changed. */
export function publishListing(listing: PreviewListing): void {
  set({ listing });
}

/** The DOM id of the Fonts tab's Preview button, where the focus goes when the preview closes
 * with the focus inside (UX notes §1). */
export const PREVIEW_BUTTON_ID = "font-preview-button";
