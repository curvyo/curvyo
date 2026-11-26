// The library browser's Sets view (`specs/0152-library-browser` criteria 26 to 28 and 32, UX notes
// 0.2): one card per set, catalogue, folder or unfetched source, in groups by source with sticky
// headers; a press selects, a double-click or Enter opens, Space selects; arrows move by card and
// by row; at the end Host your own library….
import { Lock } from "lucide-react";
import { forwardRef, useImperativeHandle, useLayoutEffect, useMemo, useRef, useState } from "react";

import { BrowserCard } from "@/components/libraryBrowser/BrowserTile";
import { CARD_WIDTH_PX, GRID_GAP_PX, GRID_PAD_PX, columnsFor } from "@/lib/libraryBrowserLayout";
import { type CardWords, type SetsAnswer, cardWords } from "@/lib/libraryBrowserQuery";

export interface SetsHandle {
  /** Focuses the first card (Enter in the search field, H9). */
  focusFirst: () => void;
  /** Focuses card `index` if shown, else the current, else the first. */
  focusCard: (index: number | null) => void;
  /** The first visible card (UX U39). */
  anchor: () => number | null;
  /** Scrolls card `index` into view. */
  reveal: (index: number) => void;
}

interface SetsGridProps {
  answer: SetsAnswer;
  version: number;
  current: number | null;
  busy: ReadonlySet<number>;
  onSelect: (index: number) => void;
  onOpen: (words: CardWords) => void;
  onHover: (index: number | null) => void;
  onType: (text: string) => void;
  /** Host your own library… at the end (desktop build only); null when not offered. */
  onGuide: (() => void) | null;
  onClearAll: (() => void) | null;
}

const BUTTON =
  "h-7 rounded-[5px] px-3 text-sm text-[var(--toolbar-icon)] ring-1 ring-[color-mix(in_srgb,var(--toolbar-icon)_60%,transparent)] hover:bg-[var(--editor-accent-hover)]";

export const SetsGrid = forwardRef<SetsHandle, SetsGridProps>(function SetsGrid(props, ref) {
  const { answer, current } = props;
  const scrollRef = useRef<HTMLDivElement>(null);
  const [width, setWidth] = useState(400);
  const [focus, setFocus] = useState(0);
  /** The position to focus once a render holds it. */
  const pendingFocus = useRef<number | null>(null);
  const flat = useMemo(() => answer.groups.flatMap((g) => g.cards), [answer]);
  const words = useMemo(
    () => new Map(cardWords(flat).map((w) => [w.index, w])),
    // `version` re-asks the words after a new list.
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [flat, props.version],
  );
  const cols = columnsFor(width, CARD_WIDTH_PX);
  const focusAt = Math.min(focus, Math.max(0, flat.length - 1));

  useLayoutEffect(() => {
    const el = scrollRef.current;
    if (!el) {
      return undefined;
    }
    const read = () => setWidth(el.clientWidth);
    read();
    const observer = new ResizeObserver(read);
    observer.observe(el);
    return () => observer.disconnect();
  }, []);

  useLayoutEffect(() => {
    // Only the render that holds the asked position (see `BrowserGrid`).
    if (pendingFocus.current === null || pendingFocus.current !== focusAt) {
      return;
    }
    pendingFocus.current = null;
    const el = scrollRef.current?.querySelector<HTMLElement>(`[data-card="${flat[focusAt] ?? -1}"]`);
    el?.focus({ preventScroll: true });
    el?.scrollIntoView({ block: "nearest" });
  });

  const moveTo = (position: number, select = true) => {
    if (flat.length === 0) {
      return;
    }
    const target = Math.max(0, Math.min(flat.length - 1, position));
    pendingFocus.current = target;
    setFocus(target);
    if (select) {
      props.onSelect(flat[target] ?? 0);
    }
  };

  useImperativeHandle(ref, () => ({
    focusFirst: () => moveTo(0),
    focusCard: (index) => {
      const at = index === null ? -1 : flat.indexOf(index);
      const fallback = current === null ? -1 : flat.indexOf(current);
      moveTo(at >= 0 ? at : fallback >= 0 ? fallback : 0, false);
    },
    anchor: () => {
      const el = scrollRef.current;
      if (!el) {
        return null;
      }
      const top = el.getBoundingClientRect().top;
      const first = [...el.querySelectorAll<HTMLElement>("[data-card]")].find(
        (card) => card.getBoundingClientRect().bottom > top,
      );
      return first ? Number(first.dataset.card) : null;
    },
    reveal: (index) => {
      scrollRef.current?.querySelector<HTMLElement>(`[data-card="${index}"]`)?.scrollIntoView({ block: "start" });
    },
  }));

  const onKeyDown = (event: React.KeyboardEvent<HTMLDivElement>) => {
    if (!(event.target as HTMLElement).closest("[data-card]")) {
      return;
    }
    const moves: Record<string, number> = {
      ArrowRight: focusAt + 1,
      ArrowLeft: focusAt - 1,
      ArrowDown: focusAt + cols,
      ArrowUp: focusAt - cols,
      Home: 0,
      End: flat.length - 1,
    };
    const next = event.altKey ? undefined : moves[event.key];
    if (next !== undefined) {
      event.preventDefault();
      moveTo(next);
    } else if (event.key === "Enter") {
      event.preventDefault();
      const card = words.get(flat[focusAt] ?? -1);
      if (card) {
        props.onOpen(card);
      }
    } else if (event.key === " ") {
      event.preventDefault();
      props.onSelect(flat[focusAt] ?? 0);
    } else if (event.key === "Backspace" && !event.altKey && !event.ctrlKey && !event.metaKey) {
      event.preventDefault();
      props.onType("\b");
    } else if (event.key.length === 1 && !event.ctrlKey && !event.metaKey && !event.altKey) {
      event.preventDefault();
      props.onType(event.key);
    }
  };

  let position = 0;
  return (
    <div
      ref={scrollRef}
      className="relative min-h-0 flex-1 overflow-y-auto bg-[var(--canvas-bg)] [overscroll-behavior:contain]"
      style={{ padding: GRID_PAD_PX }}
    >
      <div role="listbox" aria-label="Sets" onKeyDown={onKeyDown}>
        {answer.groups.map((group) => (
          <div
            key={group.key}
            role="group"
            aria-label={group.private ? `${group.heading}, private` : group.heading}
            className="mb-2"
          >
            <p className="sticky top-[-12px] z-10 flex h-7 items-center bg-[var(--canvas-bg)] text-xs font-semibold text-[var(--panel-muted-fg)]">
              {group.private && group.heading.startsWith(group.name) ? (
                <>
                  {group.name}
                  <Lock size={12} strokeWidth={1.5} aria-hidden="true" className="ml-1 shrink-0" />
                  {group.heading.slice(group.name.length)}
                </>
              ) : (
                group.heading
              )}
            </p>
            <div className="flex flex-wrap" style={{ gap: GRID_GAP_PX }}>
              {group.cards.map((index) => {
                const card = words.get(index);
                const at = position;
                position += 1;
                return card ? (
                  <div key={index} onFocus={() => setFocus(at)}>
                    <BrowserCard
                      words={card}
                      busy={props.busy.has(index)}
                      current={current === index}
                      focused={at === focusAt}
                      onSelect={(i) => {
                        setFocus(at);
                        props.onSelect(i);
                      }}
                      onOpen={props.onOpen}
                      onHover={props.onHover}
                    />
                  </div>
                ) : null;
              })}
            </div>
          </div>
        ))}
      </div>
      {answer.empty ? (
        <div className="flex flex-col items-center gap-3 px-4 pt-3 text-center">
          <p className="text-sm text-[var(--panel-muted-fg)]">{answer.empty}</p>
          {props.onClearAll ? (
            <button type="button" onClick={props.onClearAll} className={BUTTON}>
              Clear all
            </button>
          ) : null}
        </div>
      ) : null}
      {props.onGuide ? (
        <button
          type="button"
          onClick={props.onGuide}
          className="mt-4 flex h-7 items-center text-xs text-[var(--editor-accent)] hover:underline"
        >
          Host your own library…
        </button>
      ) : null}
    </div>
  );
});
