// The library browser's detail bar (`specs/0152-library-browser` criteria 5, 7 and 29, Frage 374
// A, UX notes sections 0.2, 0.5 and 8): the pane's last row with the hovered, else the current
// item or card, its name, place, licence link, author and facts; licence and credit are readable
// before anything is loaded. No button loads (H4): only Show pages and Pick elements…, text
// buttons in the wide layout, icon buttons in the narrow one; a card has none. While searching, an
// item that matched only through a keyword, category or related word says so on line 4
// (`specs/0153-library-keywords` criterion 14, UX notes K4 to K6).
import { Files, Folder, GitBranch, Globe, MousePointerClick } from "lucide-react";

import { ItemPreview } from "@/components/libraryBrowser/ItemPreview";
import { Tooltip } from "@/components/ui/tooltip";
import { heldIconTile } from "@/lib/iconTiles";
import { DETAIL_NARROW_PX, DETAIL_WIDE_PX } from "@/lib/libraryBrowserLayout";
import type { CardWords, TileWords } from "@/lib/libraryBrowserQuery";
import type { WhyWords } from "@/lib/libraryKeywords";
import { Button } from "@/components/ui/button";
import { iconButtonClass } from "@/components/ui/looks";

type Action = TileWords["actions"][number];

const ACTIONS: Record<Action, { name: string; Glyph: typeof Files }> = {
  show_pages: { name: "Show pages", Glyph: Files },
  pick: { name: "Pick elements…", Glyph: MousePointerClick },
};

const BUTTON = iconButtonClass("outlined", 28);

interface DetailBarProps {
  words: TileWords | null;
  card: CardWords | null;
  /** Whether `words` is the current item (its buttons are drawn, UX U16). */
  isCurrent: boolean;
  wide: boolean;
  /** Why the item matched the search, when not by its name. */
  why: WhyWords | null;
  onAction: (action: Action, words: TileWords) => void;
  onLicence: (url: string) => void;
  /** The Into slot… button of a page tile (`specs/0154` criterion 10), last in the column. */
  extra?: React.ReactNode;
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

export function BrowserDetailBar({ words, card, isCurrent, wide, why, onAction, onLicence, extra = null }: DetailBarProps) {
  const box = wide ? 96 : 72;
  const actions = words && isCurrent ? words.actions : [];
  const shown = words ?? card;
  const licence = words ? words.licence : (card?.chip ?? "");
  const licenceUrl = words ? words.licence_url : (card?.licence_url ?? "");
  const author = shown?.author ?? "";
  const facts = shown?.facts ?? "";
  const reason = words ? why : null;
  // The full reasons and facts the line cuts (K5, K6).
  const boxTitle = [reason?.all, wide && !reason ? "" : facts].filter(Boolean).join("\n") || undefined;
  return (
    <div
      className="flex shrink-0 items-center gap-3 border-t border-[color-mix(in_srgb,var(--toolbar-icon)_25%,transparent)] bg-[var(--panel-bg)] p-2"
      style={{ height: wide ? DETAIL_WIDE_PX : DETAIL_NARROW_PX }}
    >
      {shown ? (
        <>
          <Tooltip side="bottom" content={boxTitle}>
            <div
              className="shrink-0 overflow-hidden rounded-[8px] border border-[var(--tile-edge)] bg-[var(--tile-bg)]"
            >
              {words ? (
                <ItemPreview key={words.index} words={words} size={box - 2} />
              ) : card ? (
                <CardPicture card={card} box={box} />
              ) : null}
            </div>
          </Tooltip>
          {/* The buttons come before the licence link in the Tab order; `order-last` draws them at
              the right. The column is reserved so the text does not jump on hover (UX notes 0.5). */}
          <div className="order-last flex shrink-0 flex-col gap-1" style={{ width: wide ? 112 : 28 }}>
            {actions.map((action) =>
              wide ? (
                <Button key={action} icon={ACTIONS[action].Glyph} className="w-full px-2" onClick={() => words && onAction(action, words)}>
                  {ACTIONS[action].name}
                </Button>
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
            {extra}
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
            {wide && (facts || reason) ? (
              <p className="flex min-w-0 text-xs whitespace-nowrap text-[var(--panel-muted-fg)]">
                {reason ? <span className="max-w-full shrink-0 truncate">{reason.line}</span> : null}
                {facts ? <span className="min-w-0 truncate">{reason ? `\u00a0· ${facts}` : facts}</span> : null}
              </p>
            ) : null}
            {!wide && reason ? <p className="truncate text-xs text-[var(--panel-muted-fg)]">{reason.line}</p> : null}
          </div>
        </>
      ) : (
        <p className="text-sm text-[var(--panel-muted-fg)]">Nothing selected</p>
      )}
    </div>
  );
}
