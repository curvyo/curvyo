// The library browser's filters and sort (`specs/0152-library-browser` criteria 11 to 17, UX
// notes section 5): strips, a switch and check lists, never dropdowns or popovers; the same
// sections in the facet column and in the Filters band. Each strip and list is one Tab stop. The
// Items view ends with the search keywords' credit line (`specs/0153-library-keywords` UX K8).
import { Check } from "lucide-react";
import { useRef, useState } from "react";

import { PanelSwitch } from "@/components/ui/switch";
import type { BrowserSort, ColourFilter } from "@/lib/libraryBrowserState";
import { type FacetRow, facetSelect } from "@/lib/libraryBrowserQuery";
import { openLicence } from "@/lib/onlineApi";
import { isTauri } from "@/platform/host";

const CUSTOMIZABLE_TIP = "Icons that take the colour you give the object, and templates tagged customizable";
const COLOUR_TIP = "Templates and your own pages have no colour data";
const NO_COLOUR = "No colour data in the fetched items";

/** What the facets show: the Items answer's, or the Sets answer's (no Sort, no Customizable). */
interface FacetData {
  sources: FacetRow[];
  categories: FacetRow[];
  has_categories: boolean;
  has_colour: boolean;
  has_customizable?: boolean;
}

interface FacetsProps {
  answer: FacetData;
  /** The Sets view's facets: Sources (the groups), Colour, Category (H10). */
  sets: boolean;
  searching: boolean;
  sort: BrowserSort;
  onSort: (sort: BrowserSort) => void;
  sources: string[];
  onSources: (keys: string[]) => void;
  colour: ColourFilter;
  onColour: (colour: ColourFilter) => void;
  customizable: boolean;
  onCustomizable: (on: boolean) => void;
  categories: string[];
  onCategories: (keys: string[]) => void;
  /** Clear all is offered (a filter or the search is active). */
  canClear: boolean;
  onClearAll: () => void;
  /** Licences of the search keywords (Items view only, K8 and K9). */
  onKeywordLicences?: () => void;
}

/** The projects whose keywords the search knows (spec `0153` criterion 10a). */
const KEYWORD_PROJECTS: [string, string][] = [
  ["Lucide", "https://lucide.dev"],
  ["Tabler Icons", "https://tabler.io/icons"],
  ["Material Design Icons", "https://pictogrammers.com/library/mdi/"],
];

/** A project's page in the system browser (the browser build: a new tab). */
function openProject(url: string): void {
  if (isTauri) {
    void openLicence(url).catch(() => undefined);
  } else {
    window.open(url, "_blank", "noopener,noreferrer");
  }
}

/** "Search keywords: Lucide, Tabler Icons, Material Design Icons · Licences" (UX notes K8). */
function KeywordCredit({ onLicences }: { onLicences: () => void }) {
  return (
    <p className="mt-4 text-xs text-[var(--panel-muted-fg)]">
      Search keywords:{" "}
      {KEYWORD_PROJECTS.map(([name, url], n) => (
        <span key={name}>
          <button type="button" className="underline" onClick={() => openProject(url)}>
            {name}
          </button>
          {n < KEYWORD_PROJECTS.length - 1 ? ", " : null}
        </span>
      ))}
      {" · "}
      <button type="button" className="text-[var(--editor-accent)] hover:underline" onClick={onLicences}>
        Licences
      </button>
    </p>
  );
}

function Heading({ children }: { children: string }) {
  return <p className="mt-4 flex h-6 items-center text-xs font-semibold text-[var(--panel-muted-fg)]">{children}</p>;
}

interface StripCell<T extends string> {
  value: T;
  label: string;
  title: string;
  disabled?: boolean;
  description?: string;
}

/** A choice strip: one Tab stop, arrows move and choose. */
function Strip<T extends string>({
  label,
  cells,
  value,
  onChange,
  columns,
  title,
}: {
  label: string;
  cells: StripCell<T>[];
  value: T;
  onChange: (value: T) => void;
  /** CSS grid columns: equal cells by number, or a template (cells sized to their words). */
  columns: number | string;
  title?: string;
}) {
  const ref = useRef<HTMLDivElement>(null);
  const usable = cells.filter((c) => !c.disabled);
  return (
    <div
      ref={ref}
      role="radiogroup"
      aria-label={label}
      title={title}
      className="grid gap-px overflow-hidden rounded-[5px] ring-1 ring-[color-mix(in_srgb,var(--toolbar-icon)_40%,transparent)]"
      style={{ gridTemplateColumns: typeof columns === "number" ? `repeat(${columns}, minmax(0, 1fr))` : columns }}
      onKeyDown={(event) => {
        const at = usable.findIndex((c) => c.value === value);
        const by = event.key === "ArrowRight" || event.key === "ArrowDown" ? 1 : event.key === "ArrowLeft" || event.key === "ArrowUp" ? -1 : 0;
        if (by !== 0 && usable.length > 0) {
          event.preventDefault();
          const next = usable[(at + by + usable.length) % usable.length];
          if (next) {
            onChange(next.value);
            ref.current?.querySelector<HTMLElement>(`[data-value="${next.value}"]`)?.focus();
          }
        }
      }}
    >
      {cells.map((cell) => (
        <button
          key={cell.value}
          type="button"
          role="radio"
          data-value={cell.value}
          aria-checked={cell.value === value}
          aria-disabled={cell.disabled || undefined}
          aria-description={cell.description}
          title={cell.description ?? cell.title}
          tabIndex={cell.value === value ? 0 : -1}
          onClick={() => !cell.disabled && onChange(cell.value)}
          className={`h-7 truncate px-2 text-xs outline-none focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)] focus-visible:ring-inset ${
            cell.value === value
              ? "bg-[var(--toolbar-icon-active-bg)] text-[var(--toolbar-icon-active-fg)]"
              : "bg-[var(--panel-bg)] text-[var(--toolbar-icon)] hover:bg-[var(--editor-accent-hover)]"
          } ${cell.disabled ? "opacity-40" : ""}`}
        >
          {cell.label}
        </button>
      ))}
    </div>
  );
}

/** A multi-select list (criterion 31): a press selects only that row, a press on the only
 * selected row clears, Shift+press adds or removes; Space and Shift+Space the same; Up, Down,
 * Home and End move the focus only; Ctrl+A selects every row. One Tab stop. */
function SelectList({
  label,
  rows,
  selected,
  onChange,
  withRows,
}: {
  label: string;
  rows: FacetRow[];
  selected: string[];
  onChange: (keys: string[]) => void;
  /** Keys a row selects with it ("My documents" its folders, UX U27). */
  withRows?: (key: string) => string[];
}) {
  const ref = useRef<HTMLDivElement>(null);
  const [focus, setFocus] = useState(0);
  const press = (key: string, shift: boolean) => onChange(facetSelect(selected, key, withRows?.(key) ?? [], shift));
  const focusRow = (at: number) => {
    const items = [...(ref.current?.querySelectorAll<HTMLElement>("[role=option]") ?? [])];
    const next = Math.max(0, Math.min(items.length - 1, at));
    setFocus(next);
    items[next]?.focus();
  };
  return (
    <div
      ref={ref}
      role="listbox"
      aria-multiselectable="true"
      aria-label={label}
      onKeyDown={(event) => {
        const moves: Record<string, number> = { ArrowDown: focus + 1, ArrowUp: focus - 1, Home: 0, End: rows.length - 1 };
        const next = moves[event.key];
        if (next !== undefined) {
          event.preventDefault();
          focusRow(next);
        } else if (event.key === " ") {
          event.preventDefault();
          const row = rows[focus];
          if (row) {
            press(row.key, event.shiftKey);
          }
        } else if (event.key.toLowerCase() === "a" && (event.ctrlKey || event.metaKey)) {
          // Every row is the same as none (all).
          event.preventDefault();
          onChange([]);
        }
      }}
    >
      {rows.map((row, index) => {
        const on = selected.includes(row.key);
        return (
          <div
            key={row.key}
            role="option"
            aria-selected={on}
            tabIndex={index === focus ? 0 : -1}
            title={row.label}
            onClick={(event) => {
              setFocus(index);
              press(row.key, event.shiftKey);
            }}
            className={`flex h-7 cursor-default items-center gap-1 rounded-[4px] pr-1 outline-none select-none focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)] focus-visible:ring-inset ${
              on ? "bg-[var(--value-fill)]" : "hover:bg-[var(--editor-accent-hover)]"
            }`}
            style={{ paddingLeft: row.indent ? 20 : 4 }}
          >
            <span className="flex w-4 shrink-0 justify-center">{on ? <Check size={14} aria-hidden /> : null}</span>
            <span className={`min-w-0 flex-1 truncate text-sm ${row.count === 0 ? "text-[var(--panel-muted-fg)]" : "text-[var(--toolbar-icon)]"}`}>
              {row.label}
            </span>
            <span className="shrink-0 text-xs tabular-nums text-[var(--panel-muted-fg)]">{row.count.toLocaleString("en-US")}</span>
          </div>
        );
      })}
    </div>
  );
}

export function BrowserFacets(props: FacetsProps) {
  const { answer } = props;
  const localKeys = answer.sources.filter((r) => r.indent).map((r) => r.key);
  return (
    <div className="flex flex-col">
      <div className="flex h-6 items-center justify-between">
        <span className="text-sm text-[var(--toolbar-icon)]">Filters</span>
        <button
          type="button"
          onClick={props.onClearAll}
          className={`h-6 text-xs text-[var(--editor-accent)] ${props.canClear ? "" : "invisible"}`}
          tabIndex={props.canClear ? 0 : -1}
          aria-hidden={!props.canClear}
        >
          Clear all
        </button>
      </div>
      {/* The Sets view has no Sort and no Customizable (UX U37). */}
      {props.sets ? null : <Heading>Sort</Heading>}
      {props.sets ? null : <Strip<BrowserSort>
        label="Sort"
        columns={2}
        value={props.sort}
        onChange={props.onSort}
        cells={[
          {
            value: "best",
            label: "Best match",
            title: "Best match",
            disabled: !props.searching,
            description: props.searching ? undefined : "Only while searching",
          },
          { value: "name", label: "Name", title: "Name A to Z" },
          { value: "source", label: "Source", title: "Source, then name" },
          { value: "newest", label: "Newest", title: "Newest" },
        ]}
      />}
      {answer.sources.length > 0 ? (
        <>
          <Heading>Sources</Heading>
          <SelectList
            label="Sources"
            rows={answer.sources}
            selected={props.sources}
            onChange={props.onSources}
            withRows={(key) => (key === "local" ? localKeys : [])}
          />
        </>
      ) : null}
      <Heading>Colour</Heading>
      <Strip<ColourFilter>
        label="Colour"
        // "Monochrome" keeps its whole word in the 216 px column: All and Colour take their width.
        columns="auto minmax(0, 1fr) auto"
        value={props.colour}
        onChange={props.onColour}
        title={props.sets ? "Folders and catalogues have no colour data" : COLOUR_TIP}
        cells={[
          { value: "all", label: "All", title: "All" },
          {
            value: "mono",
            label: "Monochrome",
            title: "Monochrome",
            disabled: !answer.has_colour,
            description: answer.has_colour ? undefined : NO_COLOUR,
          },
          {
            value: "colour",
            label: "Colour",
            title: "Colour",
            disabled: !answer.has_colour,
            description: answer.has_colour ? undefined : NO_COLOUR,
          },
        ]}
      />
      {props.sets ? null : <div className="mt-4 flex h-7 items-center" title={CUSTOMIZABLE_TIP}>
        <PanelSwitch
          label="Customizable"
          checked={props.customizable}
          onCheckedChange={(on) => answer.has_customizable && props.onCustomizable(on)}
          onReturnFocus={() => {}}
          extra={answer.has_customizable ? { title: CUSTOMIZABLE_TIP } : { "aria-disabled": "true", title: "Nothing fetched is customizable" }}
        />
      </div>}
      <Heading>Category</Heading>
      {answer.has_categories ? (
        <SelectList label="Category" rows={answer.categories} selected={props.categories} onChange={props.onCategories} />
      ) : (
        <p className="text-xs text-[var(--panel-muted-fg)]">
          {props.sets ? "No categories in the sets." : "No categories in the fetched items."}
        </p>
      )}
      {props.sets || !props.onKeywordLicences ? null : <KeywordCredit onLicences={props.onKeywordLicences} />}
    </div>
  );
}
