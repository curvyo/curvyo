// The Fonts tab's search row (`specs/0074-text-objects` UX notes §4, `specs/0075-font-library`
// UX notes §4): the search field and the Add font… button with its busy state and notice.
import { FilePlus, LayoutGrid, LoaderCircle, Search, X } from "lucide-react";

import { TabNotice } from "@/components/panel/FontTabNotice";
import { Tooltip } from "@/components/ui/tooltip";
import type { FontNotice } from "@/hooks/useFontLibrary";
import { addingText } from "@/lib/fontLibraryText";
import { PREVIEW_BUTTON_ID } from "@/lib/fontPreviewStore";

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
      <div className="relative h-7 min-w-0 flex-1">
        <Search size={16} aria-hidden className="pointer-events-none absolute top-1.5 left-2 text-[var(--panel-muted-fg)]" />
        <input
          ref={searchRef}
          type="text"
          value={search}
          aria-label="Search fonts"
          placeholder="Search fonts"
          onChange={(event) => onSearch(event.target.value)}
          onKeyDown={onKeyDown}
          className="h-7 w-full rounded-[5px] bg-white pr-7 pl-8 text-sm outline-none ring-1 ring-border placeholder:text-[var(--field-placeholder)] focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)]"
        />
        {search ? (
          <button
            type="button"
            aria-label="Clear search"
            onClick={() => {
              onSearch("");
              searchRef.current?.focus();
            }}
            className="absolute top-0.5 right-0.5 flex size-6 items-center justify-center rounded-[5px] text-[var(--toolbar-icon)] hover:bg-[var(--editor-accent-hover)]"
          >
            <X size={14} aria-hidden />
          </button>
        ) : null}
      </div>
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
          <LayoutGrid size={16} strokeWidth={1.5} aria-hidden />
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
            {busy !== null ? <LoaderCircle size={16} aria-hidden className="motion-safe:animate-spin" /> : <FilePlus size={16} aria-hidden />}
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
