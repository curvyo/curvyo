// The library browser's detail bar (`specs/0152-library-browser` criteria 5, 7 and 29, Frage 374
// A, UX notes sections 0.2, 0.5 and 8): the pane's last row with the hovered, else the current
// item or card, its name, place, licence link, author and facts; licence and credit are readable
// before anything is loaded. No button loads (H4): only Show pages and Pick elements…, text
// buttons in the wide layout, icon buttons in the narrow one; a card has none.
import { Files, Folder, GitBranch, Globe, MousePointerClick } from "lucide-react";

import { ItemPreview } from "@/components/libraryBrowser/ItemPreview";
import { Tooltip } from "@/components/ui/tooltip";
import { heldIconTile } from "@/lib/iconTiles";
import { DETAIL_NARROW_PX, DETAIL_WIDE_PX } from "@/lib/libraryBrowserLayout";
import type { CardWords, TileWords } from "@/lib/libraryBrowserQuery";

type Action = TileWords["actions"][number];

const ACTIONS: Record<Action, { name: string; Glyph: typeof Files }> = {
  show_pages: { name: "Show pages", Glyph: Files },
  pick: { name: "Pick elements…", Glyph: MousePointerClick },
};

const BUTTON =
  "h-7 shrink-0 truncate rounded-[5px] text-sm text-[var(--toolbar-icon)] outline-none ring-1 ring-[color-mix(in_srgb,var(--toolbar-icon)_60%,transparent)] hover:bg-[var(--editor-accent-hover)] focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)]";

interface DetailBarProps {
  words: TileWords | null;
  card: CardWords | null;
  /** Whether `words` is the current item (its buttons are drawn, UX U16). */
  isCurrent: boolean;
  wide: boolean;
  onAction: (action: Action, words: TileWords) => void;
  onLicence: (url: string) => void;
}

/** A card's picture in the bar: its samples at 32 px, else its glyph. */
function CardPicture({ card, box }: { card: CardWords; box: number }) {
  const Glyph = card.glyph === "folder" ? Folder : card.glyph === "git" ? GitBranch : Globe;
  const samples = card.samples
    .map((name) => heldIconTile(card.key, card.prefix, name)?.url)
    .filter((url): url is string => Boolean(url));
  return (
    <div className="flex items-center justify-center gap-1" style={{ width: box - 2, height: box - 2 }}>
      {samples.length > 0 ? (
        samples.slice(0, 2).map((url) => <img key={url} src={url} alt="" width={32} height={32} draggable={false} />)
      ) : (
        <Glyph size={32} strokeWidth={1.5} aria-hidden className="text-[var(--toolbar-icon)]" />
      )}
    </div>
  );
}

export function BrowserDetailBar({ words, card, isCurrent, wide, onAction, onLicence }: DetailBarProps) {
  const box = wide ? 96 : 72;
  const actions = words && isCurrent ? words.actions : [];
  const shown = words ?? card;
  const licence = words ? words.licence : (card?.chip ?? "");
  const licenceUrl = words ? words.licence_url : (card?.licence_url ?? "");
  const author = shown?.author ?? "";
  const facts = shown?.facts ?? "";
  return (
    <div
      className="flex shrink-0 items-center gap-3 border-t border-[color-mix(in_srgb,var(--toolbar-icon)_25%,transparent)] bg-[var(--panel-bg)] p-2"
      style={{ height: wide ? DETAIL_WIDE_PX : DETAIL_NARROW_PX }}
    >
      {shown ? (
        <>
          <div
            className="shrink-0 overflow-hidden rounded-[8px] border border-[var(--tile-edge)] bg-[var(--tile-bg)]"
            title={wide ? undefined : facts}
          >
            {words ? (
              <ItemPreview key={words.index} words={words} size={box - 2} />
            ) : card ? (
              <CardPicture card={card} box={box} />
            ) : null}
          </div>
          {/* The buttons come before the licence link in the Tab order; `order-last` draws them at
              the right. The column is reserved so the text does not jump on hover (UX notes 0.5). */}
          <div className="order-last flex shrink-0 flex-col gap-1" style={{ width: wide ? 112 : 28 }}>
            {actions.map((action) =>
              wide ? (
                <button key={action} type="button" className={`${BUTTON} px-3`} onClick={() => words && onAction(action, words)}>
                  {ACTIONS[action].name}
                </button>
              ) : (
                <Tooltip key={action} side="top" content={ACTIONS[action].name}>
                  <button
                    type="button"
                    aria-label={ACTIONS[action].name}
                    className={`${BUTTON} flex w-7 items-center justify-center`}
                    onClick={() => words && onAction(action, words)}
                  >
                    {(() => {
                      const Glyph = ACTIONS[action].Glyph;
                      return <Glyph size={16} strokeWidth={1.5} aria-hidden />;
                    })()}
                  </button>
                </Tooltip>
              ),
            )}
          </div>
          <div className="flex min-w-0 flex-1 flex-col self-start pt-1 select-text">
            <p className={`${wide ? "line-clamp-2" : "truncate"} text-sm break-words text-[var(--toolbar-icon)]`}>{shown.name}</p>
            <p className="truncate text-xs text-[var(--panel-muted-fg)]">{shown.place}</p>
            {licence || author ? (
              <p className="truncate text-xs text-[var(--toolbar-icon)]">
                {licence ? (
                  licenceUrl ? (
                    <button type="button" className="underline" onClick={() => onLicence(licenceUrl)}>
                      {licence}
                    </button>
                  ) : (
                    <span>{licence}</span>
                  )
                ) : null}
                {licence && author ? " · " : null}
                {author}
              </p>
            ) : null}
            {wide && facts ? <p className="truncate text-xs text-[var(--panel-muted-fg)]">{facts}</p> : null}
          </div>
        </>
      ) : (
        <p className="text-sm text-[var(--panel-muted-fg)]">Nothing selected</p>
      )}
    </div>
  );
}
