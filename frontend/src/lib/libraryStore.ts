// The page the Library's preview pane shows and its read-only source (`specs/0137` criteria 4, 5,
// UX notes section 6): one at a time, dropped when the pane closes (ADR 0084 §5).
import { useCallback, useEffect, useState, useSyncExternalStore } from "react";

import { WasmLibrarySource } from "@/wasm-bindings/curvyo_editor_wasm.js";

import { readLibraryDocument } from "@/lib/libraryApi";
import { claimPane, registerPaneCloser, releasePane } from "@/lib/sidePaneStore";

/** The page shown in the preview pane. */
export interface PreviewPage {
  source: string;
  path: string[];
  pageId: string;
  pageName: string;
  chip: string;
  /** "Box job › Lid", with " · Saved version" when the row says so. */
  subject: string;
  /** The document's name, for Came from. */
  documentName: string;
  /** Opened by the library browser's Pick elements…: the header offers Back to it (UX U19). */
  back?: boolean;
}

interface State {
  page: PreviewPage | null;
  /** The opened source, `null` while reading; `error` when it cannot be read. */
  reader: WasmLibrarySource | null;
  error: string;
  /** The picked objects, JSON ids. */
  picks: string[];
}

let state: State = { page: null, reader: null, error: "", picks: [] };
const listeners = new Set<() => void>();
let opening = 0;

function set(next: Partial<State>): void {
  state = { ...state, ...next };
  for (const listener of listeners) {
    listener();
  }
}

/** The data attribute that finds a page row again (the pane's Escape returns focus to it). */
export function pageRowKey(source: string, path: readonly string[], pageId: string): string {
  return `${source}\u001f${path.join("/")}\u001f${pageId}`;
}

/** The preview state, live. */
export function usePreview(): State {
  return useSyncExternalStore(
    (listener) => {
      listeners.add(listener);
      return () => listeners.delete(listener);
    },
    () => state,
  );
}

/** The state now, outside React. */
export function previewNow(): State {
  return state;
}

/** Opens `page` in the pane; another page of the same document keeps the reader. */
export async function openPreview(page: PreviewPage): Promise<void> {
  // One side pane at a time (UX notes P15, `specs/0152` criterion 2): the open other kind closes.
  claimPane("library-page");
  const same =
    state.page && state.page.source === page.source && state.page.path.join("/") === page.path.join("/");
  if (same && state.reader) {
    set({ page, picks: [] });
    return;
  }
  state.reader?.free();
  set({ page, reader: null, error: "", picks: [] });
  const ticket = ++opening;
  try {
    const bytes = await readLibraryDocument(page.source, page.path);
    const reader = WasmLibrarySource.open(bytes, BigInt(Math.floor(Math.random() * 2 ** 48) + 1));
    if (ticket !== opening) {
      reader.free();
      return;
    }
    set({ reader });
  } catch (error) {
    if (ticket === opening) {
      set({ error: typeof error === "string" ? error : "Cannot be read: damaged." });
    }
  }
}

/** Closes the pane and drops the source. */
export function closePreview(): void {
  opening += 1;
  state.reader?.free();
  set({ page: null, reader: null, error: "", picks: [] });
  releasePane("library-page");
}

registerPaneCloser("library-page", closePreview);

/** Sets the picks. */
export function setPicks(picks: string[]): void {
  set({ picks });
}

/** The reader of `page`, opened for an import from a row without the pane. */
export async function readerFor(source: string, path: string[]): Promise<WasmLibrarySource> {
  const bytes = await readLibraryDocument(source, path);
  return WasmLibrarySource.open(bytes, BigInt(Math.floor(Math.random() * 2 ** 48) + 1));
}

/** The page Open source activates once its document is open, with the session it left. */
let pendingPage: { id: string; document: string; page: string; left: unknown } | null = null;

/** Asks for page `id` of `document` to become active once a session other than `left` is open. */
export function activateWhenOpen(id: string, document: string, page: string, left: unknown): void {
  pendingPage = { id, document, page, left };
}

/** The page waiting for its document, taken once a new session is open: `{ id }` when it has the
 * page, `{ gone }` with the notice's names when it has not; `null` while nothing new is open. */
export function takePendingPage(
  session: unknown,
  has: (id: string) => boolean,
): { id: string } | { gone: { document: string; page: string } } | null {
  if (!pendingPage || session === pendingPage.left) {
    return null;
  }
  const taken = pendingPage;
  pendingPage = null;
  return has(taken.id) ? { id: taken.id } : { gone: { document: taken.document, page: taken.page } };
}

/** The pane's state for the canvas area: open while a page is shown, its Full view and dragged
 * split for the session. The font preview opening closes it (P15). */
export function useLibraryPane(fontPreviewOpen: boolean): {
  open: boolean;
  full: boolean;
  toggleFull: () => void;
  split: number | undefined;
  setSplit: (split: number | undefined) => void;
} {
  const { page } = usePreview();
  const [full, setFull] = useState(false);
  const [split, setSplit] = useState<number | undefined>(undefined);
  useEffect(() => {
    if (fontPreviewOpen) {
      closePreview();
    }
  }, [fontPreviewOpen]);
  const toggleFull = useCallback(() => setFull((on) => !on), []);
  return { open: page !== null && !fontPreviewOpen, full, toggleFull, split, setSplit };
}
