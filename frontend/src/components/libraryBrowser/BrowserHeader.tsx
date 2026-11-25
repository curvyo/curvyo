// The library browser's header (`specs/0152-library-browser` criteria 4, 18, 21, 26 and 30, UX
// notes sections 0.1, 0.3 and 4): the Items | Sets strip first, the search field with Search
// online and Clear, Size (Items only), the subject line with Back at its start, Full view and
// Close; in the narrow layout a second row with the Filters button.
import { ArrowLeft, Globe, Maximize2, Minimize2, Search, SlidersHorizontal, X } from "lucide-react";
import { useRef } from "react";

import { SizeSlider, type SizeScale } from "@/components/fontPreview/SizeSlider";
import { Tooltip } from "@/components/ui/tooltip";
import { BROWSER_SIZE_RANGE, DEFAULT_BROWSER_SIZE, MAX_BROWSER_SEARCH_CHARS } from "@/lib/libraryBrowserState";

const SCALE: SizeScale = { min: BROWSER_SIZE_RANGE.min, max: BROWSER_SIZE_RANGE.max, fallback: DEFAULT_BROWSER_SIZE, step: 8, bigStep: 32 };
const ICON_BUTTON =
  "flex size-7 shrink-0 items-center justify-center rounded-[5px] text-[var(--toolbar-icon)] outline-none hover:bg-[var(--editor-accent-hover)] focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)]";

/** The view strip's value; the guide presses neither cell. */
export type HeaderView = "items" | "sets" | "guide";

interface HeaderProps {
  wide: boolean;
  view: HeaderView;
  onView: (view: "items" | "sets") => void;
  /** The Back button's name ("Back to Sets"); empty while there is nowhere to go back to. */
  backName: string;
  onBack: () => void;
  text: string;
  onText: (text: string) => void;
  searchRef: React.RefObject<HTMLInputElement | null>;
  /** Down or Enter in the field: the first tile takes the focus (UX U18). */
  onFieldDown: () => void;
  /** Enter or the Globe button: the words go to the added Iconify sources (criterion 21). */
  onSearchOnline: (() => void) | null;
  size: number;
  onSize: (size: number) => void;
  subject: string;
  canSplit: boolean;
  full: boolean;
  onFull: () => void;
  onClose: () => void;
  filtersOpen: boolean;
  onFilters: () => void;
  activeFilters: number;
  onReturnFocus: () => void;
}

/** The view strip (UX U31): one Tab stop on the pressed cell, Left and Right move and choose. */
function ViewStrip({ view, onView }: { view: HeaderView; onView: (view: "items" | "sets") => void }) {
  const ref = useRef<HTMLDivElement>(null);
  const cells: ["items" | "sets", string][] = [
    ["items", "Items"],
    ["sets", "Sets"],
  ];
  return (
    <div
      ref={ref}
      role="radiogroup"
      aria-label="Library view"
      className="flex h-7 shrink-0 gap-px overflow-hidden rounded-[5px] ring-1 ring-[color-mix(in_srgb,var(--toolbar-icon)_60%,transparent)]"
      onKeyDown={(event) => {
        const next = event.key === "ArrowRight" ? "sets" : event.key === "ArrowLeft" ? "items" : null;
        if (next && !event.altKey) {
          event.preventDefault();
          onView(next);
          ref.current?.querySelector<HTMLElement>(`[data-view="${next}"]`)?.focus();
        }
      }}
    >
      {cells.map(([value, label]) => (
        <button
          key={value}
          type="button"
          role="radio"
          data-view={value}
          aria-checked={view === value}
          tabIndex={view === value || (view === "guide" && value === "items") ? 0 : -1}
          onClick={(event) => {
            // The pressed cell takes the focus, so the keyboard stays in the pane (criterion 30).
            event.currentTarget.focus();
            onView(value);
          }}
          className={`px-1.5 text-sm outline-none focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)] focus-visible:ring-inset ${
            view === value
              ? "bg-[var(--toolbar-icon-active-bg)] text-[var(--toolbar-icon-active-fg)]"
              : "bg-[var(--panel-bg)] text-[var(--toolbar-icon)] hover:bg-[var(--editor-accent-hover)]"
          }`}
        >
          {label}
        </button>
      ))}
    </div>
  );
}

export function BrowserHeader(props: HeaderProps) {
  const { searchRef } = props;
  const strip = <ViewStrip view={props.view} onView={props.onView} />;
  const searchable = props.view !== "guide";
  const field = (
    <div className={`relative h-7 min-w-0 flex-1 ${props.wide ? "max-w-[480px] min-w-[200px]" : ""}`}>
      <Search size={16} strokeWidth={1.5} aria-hidden className="pointer-events-none absolute top-1.5 left-2 text-[var(--panel-muted-fg)]" />
      <input
        ref={searchRef}
        type="text"
        value={props.text}
        maxLength={MAX_BROWSER_SEARCH_CHARS}
        placeholder="Search library"
        aria-label="Search library"
        spellCheck={false}
        onChange={(event) => props.onText(event.target.value)}
        onKeyDown={(event) => {
          if (event.key === "Backspace" && (event.ctrlKey || event.metaKey) && props.text) {
            // Clear search is no Tab stop: the key clears the field (UX review).
            event.preventDefault();
            props.onText("");
          } else if (event.key === "Enter") {
            event.preventDefault();
            props.onSearchOnline?.();
            props.onFieldDown();
          } else if (event.key === "ArrowDown") {
            event.preventDefault();
            props.onFieldDown();
          }
        }}
        className={`h-7 w-full rounded-[5px] bg-white pl-7 ${props.text && props.onSearchOnline ? "pr-14" : "pr-7"} text-sm text-[var(--toolbar-icon)] outline-none ring-1 ring-border focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)]`}
      />
      {props.text && props.onSearchOnline ? (
        <Tooltip side="bottom" content="Search Iconify sources (Enter)">
          <button
            type="button"
            tabIndex={-1}
            aria-label="Search Iconify sources (Enter)"
            onClick={() => {
              props.onSearchOnline?.();
              searchRef.current?.focus();
            }}
            className="absolute top-0.5 right-7 flex size-6 items-center justify-center rounded-[5px] text-[var(--toolbar-icon)] hover:bg-[var(--editor-accent-hover)]"
          >
            <Globe size={16} strokeWidth={1.5} aria-hidden />
          </button>
        </Tooltip>
      ) : null}
      {props.text ? (
        <button
          type="button"
          tabIndex={-1}
          aria-label="Clear search"
          onClick={() => {
            props.onText("");
            searchRef.current?.focus();
          }}
          className="absolute top-0.5 right-0.5 flex size-6 items-center justify-center rounded-[5px] text-[var(--toolbar-icon)] hover:bg-[var(--editor-accent-hover)]"
        >
          <X size={16} strokeWidth={1.5} aria-hidden />
        </button>
      ) : null}
    </div>
  );
  const size = (
    <div className="flex shrink-0 items-center gap-1">
      <span className="mr-1 text-xs text-[var(--panel-muted-fg)]">Size</span>
      <SizeSlider scale={SCALE} size={props.size} width={props.wide ? 120 : 96} onSize={props.onSize} onReturnFocus={props.onReturnFocus} />
    </div>
  );
  const subject = (
    <div className="flex min-w-0 flex-1 items-center gap-1">
      {props.backName ? (
        <Tooltip side="bottom" content={`${props.backName}\nAlt+Left`}>
          <button
            type="button"
            aria-label={props.backName}
            onClick={props.onBack}
            className="flex size-6 shrink-0 items-center justify-center rounded-[5px] text-[var(--toolbar-icon)] outline-none hover:bg-[var(--editor-accent-hover)] focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)]"
          >
            <ArrowLeft size={16} strokeWidth={1.5} aria-hidden />
          </button>
        </Tooltip>
      ) : null}
      <Tooltip side="bottom" content={props.subject}>
        {/* The one polite region of the subject line: Enter outcomes are announced once (UX notes 4). */}
        <p role="status" className="min-w-0 flex-1 truncate text-xs text-[var(--panel-muted-fg)]">
          {props.subject}
        </p>
      </Tooltip>
    </div>
  );
  const buttons = (
    <div className="flex shrink-0 gap-1">
      {props.canSplit ? (
        <Tooltip side="bottom" content={props.full ? "Split view" : "Full view"}>
          <button type="button" aria-label={props.full ? "Split view" : "Full view"} onClick={props.onFull} className={ICON_BUTTON}>
            {props.full ? <Minimize2 size={16} strokeWidth={1.5} aria-hidden /> : <Maximize2 size={16} strokeWidth={1.5} aria-hidden />}
          </button>
        </Tooltip>
      ) : null}
      <Tooltip side="bottom" content="Close library browser">
        <button type="button" aria-label="Close library browser" onClick={props.onClose} className={ICON_BUTTON}>
          <X size={16} strokeWidth={1.5} aria-hidden />
        </button>
      </Tooltip>
    </div>
  );
  const frame = "shrink-0 border-b border-[color-mix(in_srgb,var(--toolbar-icon)_25%,transparent)] bg-[var(--panel-bg)] p-2";
  if (props.wide) {
    return (
      <div className={frame}>
        <div className="flex h-7 items-center">
          {strip}
          <div className="w-3 shrink-0" />
          {searchable ? field : null}
          {searchable ? <div className="w-4 shrink-0" /> : null}
          {props.view === "items" ? size : null}
          {props.view === "items" ? <div className="w-4 shrink-0" /> : null}
          {subject}
          <div className="w-2 shrink-0" />
          {buttons}
        </div>
      </div>
    );
  }
  return (
    <div className={frame}>
      <div className="flex h-7 items-center gap-1">
        {strip}
        {searchable ? field : <div className="flex-1" />}
        {buttons}
      </div>
      <div className="mt-2 flex h-7 min-w-0 items-center gap-3">
        {searchable ? <button
          type="button"
          aria-expanded={props.filtersOpen}
          aria-controls="library-browser-filters"
          onClick={props.onFilters}
          className={`flex h-7 shrink-0 items-center gap-1 rounded-[5px] px-2 text-sm outline-none ring-1 ring-[color-mix(in_srgb,var(--toolbar-icon)_60%,transparent)] focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)] ${
            props.filtersOpen ? "bg-[var(--toolbar-icon-active-bg)] text-[var(--toolbar-icon-active-fg)]" : "text-[var(--toolbar-icon)] hover:bg-[var(--editor-accent-hover)]"
          }`}
        >
          <SlidersHorizontal size={16} strokeWidth={1.5} aria-hidden />
          Filters
          {props.activeFilters > 0 ? <span className="text-xs tabular-nums opacity-80">{props.activeFilters}</span> : null}
        </button> : null}
        {props.view === "items" ? size : null}
      </div>
      {/* Narrow layout: Back and the subject line get a row of their own under the header, so
          the subject is not cut (UX re-check, Proposal). */}
      <div className="mt-2 flex h-6 min-w-0 items-center">{subject}</div>
    </div>
  );
}
