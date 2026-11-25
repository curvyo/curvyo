// The search keyword licences (`specs/0153-library-keywords` criterion 10, UX notes K9): a place
// of the browser in the hosting guide's look, Back "Back to all items", the shipped
// `keywords-LICENSES.txt` as preformatted text; it works offline in the desktop build.
import { useEffect, useRef, useState } from "react";

import { keywordLicences } from "@/lib/libraryKeywords";

const TITLE = "Search keyword licences";

export function KeywordLicencesView() {
  const [text, setText] = useState<string | null | undefined>(undefined);
  const regionRef = useRef<HTMLDivElement>(null);
  useEffect(() => {
    let live = true;
    void keywordLicences().then((loaded) => live && setText(loaded));
    // The link that opened it is gone: the place takes the keyboard (Escape and Alt+Left go Back).
    regionRef.current?.focus({ preventScroll: true });
    return () => {
      live = false;
    };
  }, []);
  return (
    <div
      ref={regionRef}
      role="region"
      aria-label={TITLE}
      tabIndex={0}
      className="min-h-0 flex-1 overflow-y-auto bg-[var(--panel-bg)] p-4 text-sm text-[var(--toolbar-icon)] outline-none [overscroll-behavior:contain] focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)] focus-visible:ring-inset"
    >
      <div className="flex max-w-[640px] flex-col gap-3 select-text">
        <h2 className="text-sm font-semibold">{TITLE}</h2>
        {text === null ? (
          <p className="text-sm text-[var(--panel-muted-fg)]">Cannot read the licence file.</p>
        ) : (
          <pre className="text-xs whitespace-pre-wrap">{text ?? ""}</pre>
        )}
      </div>
    </div>
  );
}
