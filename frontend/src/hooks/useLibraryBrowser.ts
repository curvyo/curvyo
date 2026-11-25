// The library browser's state for the editor (`specs/0152-library-browser` criteria 2 and 3): open
// or closed (the page store), and the view kept per user: split or full and its own dragged share,
// the Size, the sort chosen without text, Colour, Customizable, the Filters band and the search.
import { useCallback, useState } from "react";

import { type BrowserPrefs, readBrowserPrefs, writeBrowserPrefs } from "@/lib/libraryBrowserState";
import { closeBrowser, useBrowserStore } from "@/lib/libraryBrowserStore";

export interface LibraryBrowserApi {
  open: boolean;
  focusSearch: boolean;
  prefs: BrowserPrefs;
  /** Changes and keeps the prefs. */
  keep: (change: Partial<BrowserPrefs>) => void;
  wantFull: boolean;
  toggleFull: () => void;
  split: number | undefined;
  setSplit: (split: number | undefined) => void;
  close: () => void;
}

export function useLibraryBrowser(): LibraryBrowserApi {
  const store = useBrowserStore();
  const [prefs, setPrefs] = useState<BrowserPrefs>(() => readBrowserPrefs());
  const keep = useCallback((change: Partial<BrowserPrefs>) => {
    setPrefs((before) => {
      const next = { ...before, ...change };
      writeBrowserPrefs(next);
      return next;
    });
  }, []);
  return {
    open: store.open,
    focusSearch: store.focusSearch,
    prefs,
    keep,
    wantFull: prefs.full,
    toggleFull: useCallback(() => setPrefs((before) => {
      const next = { ...before, full: !before.full };
      writeBrowserPrefs(next);
      return next;
    }), []),
    split: prefs.split,
    setSplit: useCallback(
      (split: number | undefined) =>
        setPrefs((before) => {
          const { split: _old, ...rest } = before;
          const next: BrowserPrefs = split === undefined ? rest : { ...rest, split };
          writeBrowserPrefs(next);
          return next;
        }),
      [],
    ),
    close: closeBrowser,
  };
}
