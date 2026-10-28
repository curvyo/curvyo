import { useLayoutEffect, useRef, useState } from "react";

import { TRACE_INFO_ENTRIES, TRACE_INFO_INTRO, TRACE_INFO_PAGE_ID, TRACE_INFO_TITLE, infoEntryParts } from "@/lib/traceText";

/**
 * The body of the About Trace tab (`specs/0025-raster-trace` criterion 70, UX notes 21.1): the
 * intro and one entry per value of the bar, terms semibold, texts muted. Not a live region; a Tab
 * stop only when it scrolls; Escape returns the keyboard to the canvas and leaves the page open.
 */
export function TraceInfoPage({ onReturnFocus }: { onReturnFocus: () => void }) {
  const ref = useRef<HTMLElement>(null);
  const [scrolls, setScrolls] = useState(false);
  useLayoutEffect(() => {
    const element = ref.current;
    if (!element) {
      return undefined;
    }
    const measure = () => setScrolls(element.scrollHeight > element.clientHeight + 1);
    measure();
    const observer = new ResizeObserver(measure);
    observer.observe(element);
    return () => observer.disconnect();
  }, []);
  return (
    <section
      ref={ref}
      id={TRACE_INFO_PAGE_ID}
      aria-label={TRACE_INFO_TITLE}
      tabIndex={scrolls ? 0 : undefined}
      onKeyDown={(event) => {
        if (event.key === "Escape") {
          event.preventDefault();
          event.stopPropagation();
          onReturnFocus();
        }
      }}
      className="h-full w-[244px] overflow-y-auto text-sm outline-none focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)]"
    >
      <p className="text-[var(--toolbar-icon)]">{TRACE_INFO_INTRO}</p>
      <dl className="mt-3 flex flex-col gap-3">
        {TRACE_INFO_ENTRIES.map((entry) => {
          const { term, text } = infoEntryParts(entry);
          return (
            <div key={term} className="flex flex-col gap-1">
              <dt className="font-semibold text-[var(--toolbar-icon)]">{term}</dt>
              <dd className="text-[var(--panel-muted-fg)]">{text}</dd>
            </div>
          );
        })}
      </dl>
    </section>
  );
}
