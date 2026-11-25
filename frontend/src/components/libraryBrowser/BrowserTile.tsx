// One tile and one card of the library browser (`specs/0152-library-browser` criteria 5, 7, 26
// and 28, UX notes sections 0.2 and 6.2): the preview, under Size 96 the preview alone, else the
// name and the meta line with the licence chip first; a card of the Sets view with its state slot
// (no button: a double-click or Enter opens it).
import { Check, CircleAlert, Folder, Globe, GitBranch, LoaderCircle, WifiOff } from "lucide-react";
import { memo, useEffect, useState } from "react";

import { ItemPreview } from "@/components/libraryBrowser/ItemPreview";
import { CARD_HEIGHT_PX, CARD_WIDTH_PX, TEXT_FROM_SIZE } from "@/lib/libraryBrowserLayout";
import type { CardWords, TileWords } from "@/lib/libraryBrowserQuery";
import { heldIconTile, iconTile } from "@/lib/iconTiles";

interface TileProps {
  words: TileWords;
  size: number;
  current: boolean;
  /** Why it matched the search, when not by its name (current tile only, `0153` UX K7). */
  why: string;
  focused: boolean;
  style: React.CSSProperties;
  onPress: (index: number) => void;
  onLoad: (index: number) => void;
  onShowPages: (index: number) => void;
  onHover: (index: number | null) => void;
  onDragStart: (event: React.PointerEvent<HTMLDivElement>, words: TileWords) => void;
}

export const BrowserTile = memo(function BrowserTile({
  words,
  size,
  current,
  why,
  focused,
  style,
  onPress,
  onLoad,
  onShowPages,
  onHover,
  onDragStart,
}: TileProps) {
  const lines = size >= TEXT_FROM_SIZE;
  return (
    <div
      role="option"
      id={`library-tile-${words.index}`}
      data-tile={words.index}
      aria-selected={current}
      aria-label={words.label}
      aria-description={why ? `${why}. ${words.description}` : words.description}
      title={words.tooltip}
      tabIndex={focused ? 0 : -1}
      className={`absolute flex flex-col rounded-[8px] border border-[var(--tile-edge)] p-2 outline-none select-none focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)] focus-visible:ring-inset ${
        current ? "bg-[var(--value-fill)]" : "bg-[var(--tile-bg)] hover:bg-[var(--editor-accent-hover)]"
      }`}
      style={style}
      onPointerEnter={() => onHover(words.index)}
      onPointerLeave={() => onHover(null)}
      onClick={() => onPress(words.index)}
      onDoubleClick={() => onLoad(words.index)}
      onPointerDown={(event) => onDragStart(event, words)}
    >
      {current ? <span aria-hidden className="absolute top-2 bottom-2 left-0 w-[3px] rounded-r bg-[var(--editor-accent)]" /> : null}
      <ItemPreview words={words} size={size} onShowPages={onShowPages} />
      {lines ? (
        <>
          <span className="mt-1.5 h-4 truncate text-xs leading-4 text-[var(--toolbar-icon)]">{words.name}</span>
          <span className="mt-0.5 flex h-4 min-w-0 items-center gap-1">
            {words.chip ? (
              <span className="shrink-0 rounded-[4px] border border-[color-mix(in_srgb,var(--toolbar-icon)_40%,transparent)] px-1 text-[11px] leading-[14px] text-[var(--toolbar-icon)]">
                {words.chip}
              </span>
            ) : null}
            <span className="min-w-0 truncate text-[11px] text-[var(--panel-muted-fg)]">
              {words.refusal ? "Cannot be loaded" : words.meta}
            </span>
          </span>
        </>
      ) : null}
    </div>
  );
});

interface CardProps {
  words: CardWords;
  busy: boolean;
  current: boolean;
  focused: boolean;
  onSelect: (index: number) => void;
  onOpen: (words: CardWords) => void;
  onHover: (index: number | null) => void;
}

/** A sample icon of a set card: a visible card asks its samples in one request per set, as a
 * visible set row of the tab does (ADR 0086 decision 7). */
function Sample({ words, name }: { words: CardWords; name: string }) {
  const [url, setUrl] = useState(() => heldIconTile(words.key, words.prefix, name)?.url ?? null);
  useEffect(() => {
    let live = true;
    const timer = window.setTimeout(() => {
      void iconTile(words.key, words.prefix, name).then((tile) => live && tile && setUrl(tile.url));
    }, 100);
    return () => {
      live = false;
      window.clearTimeout(timer);
    };
  }, [words.key, words.prefix, name]);
  return url ? (
    <img src={url} alt="" width={24} height={24} draggable={false} />
  ) : (
    <span aria-hidden className="block h-1.5 w-4 rounded-[2px] bg-[var(--tile-skeleton)]" />
  );
}

export function BrowserCard({ words, busy, current, focused, onSelect, onOpen, onHover }: CardProps) {
  // The ready entry is the tab's not-added row: `Globe` at 55 % (UX notes 7.1).
  const Glyph = words.glyph === "folder" ? Folder : words.glyph === "git" ? GitBranch : Globe;
  return (
    <div
      role="option"
      aria-selected={current}
      aria-label={words.label}
      aria-busy={busy || undefined}
      data-card={words.index}
      tabIndex={focused ? 0 : -1}
      title={words.name}
      className={`relative flex shrink-0 flex-col rounded-[8px] border border-[var(--tile-edge)] p-3 outline-none select-none focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)] focus-visible:ring-inset ${
        current ? "bg-[var(--value-fill)]" : "bg-[var(--tile-bg)] hover:bg-[var(--editor-accent-hover)]"
      }`}
      style={{ width: CARD_WIDTH_PX, height: CARD_HEIGHT_PX }}
      onClick={() => onSelect(words.index)}
      onDoubleClick={() => onOpen(words)}
      onPointerEnter={() => onHover(words.index)}
      onPointerLeave={() => onHover(null)}
    >
      {current ? <span aria-hidden className="absolute top-2 bottom-2 left-0 w-[3px] rounded-r bg-[var(--editor-accent)]" /> : null}
      {words.kind === "set" ? (
        <span className="flex h-6 items-center gap-2">
          {words.samples.map((name) => (
            <Sample key={name} words={words} name={name} />
          ))}
        </span>
      ) : (
        <Glyph size={16} strokeWidth={1.5} aria-hidden className={words.kind === "ready" ? "opacity-55" : ""} />
      )}
      <span className={`mt-1 truncate text-sm ${words.kind === "ready" ? "text-[var(--panel-muted-fg)]" : "text-[var(--toolbar-icon)]"}`}>
        {words.name}
      </span>
      {words.failure ? (
        <span className="flex items-center gap-1 truncate text-xs text-[var(--panel-muted-fg)]">
          {words.offline ? (
            <WifiOff size={14} aria-hidden />
          ) : (
            <CircleAlert size={14} aria-hidden className="text-[var(--destructive)]" />
          )}
          {words.failure}
        </span>
      ) : (
        <span className="truncate text-xs text-[var(--panel-muted-fg)]">{words.line}</span>
      )}
      <span className="mt-auto flex items-center gap-1">
        {words.chip ? (
          <span className="shrink-0 rounded-[4px] border border-[color-mix(in_srgb,var(--toolbar-icon)_40%,transparent)] px-1 text-[11px] leading-[14px] text-[var(--toolbar-icon)]">
            {words.chip}
          </span>
        ) : null}
        <span className="min-w-0 flex-1 truncate text-[11px] text-[var(--panel-muted-fg)]">{words.author}</span>
        {/* The state slot (UX U36): a spinner while loading, else "Loaded" or nothing. */}
        {busy ? (
          <LoaderCircle size={14} className="shrink-0 animate-spin" aria-hidden />
        ) : words.state ? (
          <span className="flex shrink-0 items-center gap-0.5 text-xs text-[var(--panel-muted-fg)]">
            <Check size={14} aria-hidden />
            {words.state}
          </span>
        ) : null}
      </span>
    </div>
  );
}
