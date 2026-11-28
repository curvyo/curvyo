import { RadioGroup } from "radix-ui";
import type { ReactNode } from "react";

import { Tooltip } from "@/components/ui/tooltip";

export interface ToggleOption<T extends string> {
  value: T;
  /** The accessible name ("Miter join"). */
  label: string;
  /** The tooltip text; empty for none. */
  tooltip: string;
  icon: ReactNode;
  /** The item's width in px when it differs from the group's `itemWidth` (text items of
   * different lengths: Document 76, Object 56). */
  width?: number;
}

interface ToggleGroupProps<T extends string> {
  /** The accessible name of the group ("Stroke join"). */
  label: string;
  /** A longer description for assistive technology, when the tooltip is short. */
  description?: string;
  options: readonly ToggleOption<T>[];
  /** The pressed item; `null` shows none (mixed, or a value without a button). */
  value: T | null;
  onChange: (value: T) => void;
  disabled?: boolean;
  /** Item width in px: 40 for Join and Cap, 44 for Paint and Fill type. */
  itemWidth: number;
  /** Item height in px: 28, or 32 for Fill type and the Pen bar. */
  itemHeight?: number;
  /** Where the tooltips open: left of the item, or below it for a bar at the top of the canvas
   * (the Pen bar, `specs/0110` UX review U6). */
  tooltipSide?: "left" | "bottom";
  /** After a mouse click, so the letter keys keep working. */
  onReturnFocus: () => void;
  /** Marks the group's tab stop as the panel's first focus target. */
  firstFocus?: boolean;
  /** Lays the items out in this many columns (a strip of four long names in two rows of two,
   * `specs/0144` UX notes §3); one radio group, one Tab stop. */
  columns?: number;
  /** With `columns`: how many items each row holds (`[2, 3, 2]`); an item spans `columns / n` of the
   * grid columns, so rows of different counts fill the same width (the Symbology strip, `specs/0144`
   * UX notes §11.14). */
  rows?: readonly number[];
  /** The tool bars' look (the Polygon/Star switch): one ring around the strip, no lines between
   * the items (the Pen bar, customer 2026-11-11). */
  bar?: boolean;
  /** The items wrap to further rows inside a narrow bar instead of running past it (the
   * Generators bar, 0159 F40). */
  wrap?: boolean;
}

const BORDER = "border-[color-mix(in_srgb,var(--toolbar-icon)_60%,transparent)]";

/**
 * A strip of radio buttons (`docs/design-system.md`, "`ToggleGroup` item") on
 * Radix's `RadioGroup`: one Tab stop per group, the arrow keys move focus and
 * select, Home and End jump and select. The tooltip trigger shares the item's
 * element and replaces its `data-state` ("closed", "delayed-open"), so the
 * pressed look is keyed on `aria-checked`, which nothing else touches. One 1 px
 * bordered strip; the pressed item on
 * `--toolbar-icon-active-bg`, or, when the group is disabled, on
 * `--toolbar-icon` at 30%.
 */
export function ToggleGroup<T extends string>({
  label,
  description,
  options,
  value,
  onChange,
  disabled = false,
  itemWidth,
  itemHeight = 28,
  tooltipSide = "left",
  onReturnFocus,
  firstFocus = false,
  bar = false,
  wrap = false,
  columns,
  rows,
}: ToggleGroupProps<T>) {
  // Position of every item in a `rows` layout: its row, whether it starts the row, its span.
  const places = rows
    ? rows.flatMap((count, row) =>
        Array.from({ length: count }, (_, at) => ({
          row,
          first: at === 0,
          span: (columns ?? count) / count,
        })),
      )
    : [];
  return (
    <RadioGroup.Root
      // With nothing pressed the first item is the Tab stop (Radix would keep the
      // one it remembers): a new group starts over.
      key={value === null ? "none" : "set"}
      aria-label={label}
      aria-description={description}
      orientation="horizontal"
      disabled={disabled}
      value={value ?? ""}
      onValueChange={(next) => onChange(next as T)}
      onKeyDownCapture={(event) => {
        // The arrows select the next or previous item (wrapping), Home and End the
        // first and last: Radix moves focus on them, but selects on an arrow only
        // when the key reaches the document, which a panel that keeps its keys
        // from the canvas prevents (`0124` UX review U10).
        const step = { ArrowRight: 1, ArrowDown: 1, ArrowLeft: -1, ArrowUp: -1 }[event.key];
        if (event.key !== "Home" && event.key !== "End" && step === undefined) {
          return;
        }
        const items = Array.from(
          event.currentTarget.querySelectorAll<HTMLElement>('[role="radio"]:not(:disabled)'),
        );
        const at = items.indexOf(event.target as HTMLElement);
        const target =
          step !== undefined
            ? at < 0
              ? undefined
              : items[(at + step + items.length) % items.length]
            : event.key === "Home"
              ? items[0]
              : items[items.length - 1];
        if (target) {
          event.preventDefault();
          event.stopPropagation();
          target.focus();
          onChange(target.dataset.value as T);
        }
      }}
      style={columns ? { display: "grid", gridTemplateColumns: `repeat(${columns}, ${itemWidth}px)` } : undefined}
      className={
        bar
          ? `inline-flex w-fit shrink-0 overflow-hidden rounded-[5px] ring-1 ring-[var(--field-border)] ${wrap ? "max-w-full flex-wrap" : ""}`
          : `inline-flex w-fit shrink-0 self-start overflow-hidden rounded-[5px] border ${BORDER}`
      }
    >
      {options.map((option, index) => (
        <Tooltip key={option.value} side={tooltipSide} content={option.tooltip}>
          <RadioGroup.Item
            value={option.value}
            data-value={option.value}
            aria-label={option.label}
            // The one Tab stop: the pressed item, or the first when none is.
            data-first-focus={
              firstFocus && (option.value === value || (value === null && index === 0))
                ? ""
                : undefined
            }
            onClick={(event) => {
              // `detail` is 0 for keyboard activation, 1 or more for a click.
              if (event.detail > 0) {
                onReturnFocus();
              }
            }}
            className={`flex items-center justify-center bg-transparent text-[var(--toolbar-icon)] outline-none enabled:hover:bg-[var(--editor-accent-hover)] focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-[var(--editor-accent)] aria-checked:enabled:!bg-[var(--toolbar-icon-active-bg)] aria-checked:enabled:!text-[var(--toolbar-icon-active-fg)] aria-checked:disabled:bg-[color-mix(in_srgb,var(--toolbar-icon)_30%,transparent)] ${
              rows && places[index]
                ? `${places[index].first ? "" : `border-l ${BORDER}`} ${places[index].row > 0 ? `border-t ${BORDER}` : ""}`
                : columns
                ? `${index % columns > 0 ? `border-l ${BORDER}` : ""} ${index >= columns ? `border-t ${BORDER}` : ""}`
                : index > 0 && !bar
                  ? `border-l ${BORDER}`
                  : ""
            }`}
            style={{
              width: option.width ?? itemWidth * (places[index]?.span ?? 1),
              height: itemHeight,
              ...(places[index] ? { gridColumn: `span ${places[index].span}` } : {}),
            }}
          >
            {option.icon}
          </RadioGroup.Item>
        </Tooltip>
      ))}
    </RadioGroup.Root>
  );
}
