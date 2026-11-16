import { ArrowRight } from "lucide-react";
import { type ReactNode, useLayoutEffect, useRef, useState } from "react";

import { CENTRE_INFO_ENTRIES, CENTRE_INFO_INTRO, CENTRE_INFO_PAGE_ID, CENTRE_INFO_TITLE } from "@/lib/centreLinesText";

const INK = "var(--toolbar-icon)";
const LINE = "var(--editor-accent)";

/** One 104 x 40 px frame of a before/after strip. */
function Frame({ children }: { children: ReactNode }) {
  return (
    <svg width={104} height={40} viewBox="0 0 104 40" aria-hidden>
      {children}
    </svg>
  );
}

/** The before and after frames of each entry (UX notes §3.2, Proposal), in bar order. */
const PICTURES: Readonly<Record<string, [ReactNode, ReactNode]>> = {
  Overlap: [
    <path key="b" d="M12 10H92V16H55V34H49V16H12Z" fill={INK} />,
    <g key="a" stroke={LINE} strokeWidth={1} fill="none">
      <path d="M12 13H92" />
      <path d="M52 15V34" />
    </g>,
  ],
  "Thick parts": [
    <g key="b" fill="color-mix(in srgb, var(--toolbar-icon) 25%, transparent)" stroke={INK} strokeWidth={1}>
      <path d="M8 17H96V23H8Z" />
      <ellipse cx={52} cy={20} rx={12} ry={11} />
    </g>,
    <g key="a">
      <ellipse cx={52} cy={20} rx={12} ry={11} fill={INK} />
      <path d="M8 20H40M64 20H96" stroke={LINE} strokeWidth={1.5} fill="none" />
    </g>,
  ],
  Optimise: [
    <g key="b">
      <path d="M8 26C20 6 32 34 44 14S68 30 80 12S92 22 96 20" stroke={LINE} strokeWidth={1} fill="none" />
      {[8, 16, 24, 32, 40, 48, 56, 64, 72, 80, 88, 96].map((x) => (
        <circle key={x} cx={x} cy={20} r={1.5} fill={LINE} />
      ))}
    </g>,
    <g key="a">
      <path d="M8 26C20 6 32 34 44 14S68 30 80 12S92 22 96 20" stroke={LINE} strokeWidth={1} fill="none" />
      {[8, 44, 80, 96].map((x) => (
        <circle key={x} cx={x} cy={x === 44 ? 14 : x === 80 ? 12 : x === 8 ? 26 : 20} r={1.5} fill={LINE} />
      ))}
    </g>,
  ],
  "Keep inner areas": [
    <path key="b" d="M52 4A16 16 0 1 0 52.01 4ZM52 10A10 10 0 1 1 51.99 10Z" fill={INK} fillRule="evenodd" />,
    <g key="a">
      <circle cx={52} cy={20} r={8} fill="#D9D9D9" stroke={INK} strokeWidth={1.5} />
      <circle cx={52} cy={20} r={13} stroke={LINE} strokeWidth={1} fill="none" />
    </g>,
  ],
  Offset: [
    <g key="b">
      <circle cx={52} cy={20} r={6} fill="#D9D9D9" stroke={INK} strokeWidth={1} />
      <circle cx={52} cy={20} r={13} stroke={INK} strokeWidth={6} fill="none" />
    </g>,
    <g key="a">
      <circle cx={52} cy={20} r={12} fill="#D9D9D9" stroke={INK} strokeWidth={1} />
      <circle cx={52} cy={20} r={13} stroke={INK} strokeWidth={6} fill="none" opacity={0.55} />
    </g>,
  ],
};

/**
 * The body of the About centre lines tab (`specs/0117-centre-lines` criterion 15c, UX notes
 * §3.2): the intro and one entry per value of the bar, each with its before and after strip;
 * terms semibold, texts muted. Not a live region; a Tab stop only when it scrolls; Escape returns
 * the keyboard to the canvas and leaves the page open.
 */
export function CentreInfoPage({ onReturnFocus }: { onReturnFocus: () => void }) {
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
      id={CENTRE_INFO_PAGE_ID}
      aria-label={CENTRE_INFO_TITLE}
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
      <p className="text-[var(--toolbar-icon)]">{CENTRE_INFO_INTRO}</p>
      <dl className="mt-3 flex flex-col gap-3">
        {CENTRE_INFO_ENTRIES.map(({ term, text }) => {
          const picture = PICTURES[term];
          return (
            <div key={term} className="flex flex-col gap-1">
              <dt className="font-semibold text-[var(--toolbar-icon)]">{term}</dt>
              <dd className="text-[var(--panel-muted-fg)]">{text}</dd>
              {picture ? (
                <dd aria-hidden className="flex h-12 w-[244px] items-center">
                  <Frame>{picture[0]}</Frame>
                  <span className="flex w-9 justify-center text-[var(--panel-muted-fg)]">
                    <ArrowRight size={16} strokeWidth={1.5} />
                  </span>
                  <Frame>{picture[1]}</Frame>
                </dd>
              ) : null}
            </div>
          );
        })}
      </dl>
    </section>
  );
}
