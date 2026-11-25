// The search keyword file of the library browser (`specs/0153-library-keywords` criterion 4,
// ADR 0087 decision 5, UX notes K2 and K3): fetched from the app's own assets on the first search
// text, handed to the page's browser once and kept for the session. A failure is quiet; the file is
// asked again the next time the search field goes from empty to text.
import { useSyncExternalStore } from "react";

import { browserInstance } from "@/lib/libraryBrowserStore";

/** The shipped files (`tools/library-keywords/README.md`). */
const KEYWORDS_FILE = "library/keywords.bin.gz";
const LICENCES_FILE = "library/keywords-LICENSES.txt";
/** After this long " · Loading keywords…" joins the subject line (the "Copying..." threshold). */
const SLOW_MS = 400;

/** What the browser shows of the file. */
export interface KeywordsState {
  /** Bumped when the file arrived: the query runs again. */
  version: number;
  /** Still on the way after 400 ms. */
  slow: boolean;
}

let status: "idle" | "loading" | "loaded" | "failed" = "idle";
let snapshot: KeywordsState = { version: 0, slow: false };
const listeners = new Set<() => void>();

function publish(next: Partial<KeywordsState>): void {
  snapshot = { ...snapshot, ...next };
  for (const listener of listeners) {
    listener();
  }
}

function subscribe(listener: () => void): () => void {
  listeners.add(listener);
  return () => listeners.delete(listener);
}

/** The keyword file's state, live. */
export function useKeywords(): KeywordsState {
  return useSyncExternalStore(subscribe, () => snapshot);
}

/** The search field went from empty to text in the Items view: fetch the file unless it is in or
 * on the way. Nothing is shown unless it takes longer than 400 ms. */
export function wantKeywords(): void {
  if (status === "loading" || status === "loaded") {
    return;
  }
  status = "loading";
  const timer = window.setTimeout(() => {
    if (status === "loading") {
      publish({ slow: true });
    }
  }, SLOW_MS);
  void fetch(`${import.meta.env.BASE_URL}${KEYWORDS_FILE}`)
    .then(async (response) => {
      if (!response.ok) {
        throw new Error(`${KEYWORDS_FILE}: ${response.status}`);
      }
      const bytes = new Uint8Array(await response.arrayBuffer());
      status = browserInstance().set_keywords(bytes) ? "loaded" : "failed";
    })
    .catch(() => {
      status = "failed";
    })
    .finally(() => {
      window.clearTimeout(timer);
      publish({ slow: false, version: snapshot.version + (status === "loaded" ? 1 : 0) });
    });
}

/** Why the detail bar's item matched (criterion 14). */
export interface WhyWords {
  /** 'Matches “magnifier” (keyword)', with " and 1 more". */
  line: string;
  /** Every reason, one per line (the preview box's tooltip). */
  all: string;
}

/** Why item `index` matched `text`; null when its name matched, nothing matched or the file is
 * not in. One call for the one item the bar shows, outside the keystroke pass. */
export function whyMatched(index: number | null, text: string): WhyWords | null {
  if (index === null || status !== "loaded" || !text.trim()) {
    return null;
  }
  try {
    const json = browserInstance().why(index, text);
    return json ? (JSON.parse(json) as WhyWords) : null;
  } catch {
    return null;
  }
}

/** The shipped licence texts of the keyword sources (UX notes K9); null when they cannot be read. */
export async function keywordLicences(): Promise<string | null> {
  try {
    const response = await fetch(`${import.meta.env.BASE_URL}${LICENCES_FILE}`);
    return response.ok ? await response.text() : null;
  } catch {
    return null;
  }
}
