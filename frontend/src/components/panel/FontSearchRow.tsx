// The Fonts tab's search row (`specs/0074-text-objects` UX notes §4, `specs/0075-font-library`
// UX notes §4): the search field and the Add font… button with its busy state and notice.
import { FilePlus, LayoutGrid, LoaderCircle } from "lucide-react";
import { GLYPH } from "@/components/ui/looks";

import { TabNotice } from "@/components/panel/FontTabNotice";
import { Tooltip } from "@/components/ui/tooltip";
import type { FontNotice } from "@/hooks/useFontLibrary";
import { addingText } from "@/lib/fontLibraryText";
import { PREVIEW_BUTTON_ID } from "@/lib/fontPreviewStore";
import { SearchField } from "@/components/ui/search-field";

interface FontSearchRowProps {
  searchRef: React.RefObject<HTMLInputElement | null>;
  search: string;
  onSearch: (text: string) => void;
  onKeyDown: (event: React.KeyboardEvent<HTMLInputElement>) => void;
  /** The number of files being added, while that takes over 150 ms. */
  busy: number | null;
  /** The font preview is open (`specs/0077-font-preview` criterion 1). */
  previewOpen: boolean;
  /** The Preview button: opens or closes the preview. */
  onPreview: (byKeyboard: boolean) => void;
  /** Files are dragged over the tab: the button shows its pressed look. */
  dropOver: boolean;
  notice: FontNotice | null;
  onAdd: (byKeyboard: boolean) => void;
}

export function FontSearchRow({
  searchRef,
  search,
  onSearch,
  onKeyDown,
  busy,
  previewOpen,
  onPreview,
  dropOver,
  notice,
  onAdd,
}: FontSearchRowProps) {
  return (
    <div className="relative mb-2 flex h-7 shrink-0 gap-1">
      <SearchField
        className="flex-1"
        inputRef={searchRef}
        value={search}
        onValue={onSearch}
        aria-label="Search fonts"
        placeholder="Search fonts"
        onKeyDown={onKeyDown}
      />
      <Tooltip side="left" content="Font preview">
        <button
          id={PREVIEW_BUTTON_ID}
          type="button"
          aria-label="Font preview"
          aria-pressed={previewOpen}
          onClick={(event) => onPreview(event.detail === 0)}
          className={`flex size-7 shrink-0 items-center justify-center rounded-[5px] border border-[color-mix(in_srgb,var(--toolbar-icon)_60%,transparent)] outline-none hover:bg-[var(--editor-accent-hover)] focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)] ${
            previewOpen
              ? "border-transparent bg-[var(--toolbar-icon-active-bg)] text-[var(--toolbar-icon-active-fg)] hover:bg-[var(--toolbar-icon-active-bg)]"
              : "text-[var(--toolbar-icon)]"
          }`}
        >
          <LayoutGrid {...GLYPH} aria-hidden />
        </button>
      </Tooltip>
      <div className="relative">
        <Tooltip side="left" content="Add font…">
          <button
            type="button"
            aria-label="Add font…"
            aria-busy={busy !== null}
            onClick={(event) => {
              if (busy === null) {
                onAdd(event.detail === 0);
              }
            }}
            className={`flex size-7 items-center justify-center rounded-[5px] border border-[color-mix(in_srgb,var(--toolbar-icon)_60%,transparent)] text-[var(--toolbar-icon)] outline-none hover:bg-[var(--editor-accent-hover)] focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)] ${
              dropOver ? "!bg-[var(--toolbar-icon-active-bg)] !text-[var(--toolbar-icon-active-fg)]" : ""
            }`}
          >
            {busy !== null ? <LoaderCircle {...GLYPH} aria-hidden className="motion-safe:animate-spin" /> : <FilePlus {...GLYPH} aria-hidden />}
          </button>
        </Tooltip>
        {busy !== null ? (
          <TabNotice
            notice={{ id: 0, anchor: "add", kind: "success", text: addingText(busy), ms: 0 }}
            anchor="add"
          />
        ) : (
          <TabNotice notice={notice} anchor="add" />
        )}
      </div>
    </div>
  );
}
