import { RadioGroup } from "radix-ui";
import type { KeyboardEvent, ReactNode } from "react";

import { Tooltip } from "@/components/ui/tooltip";

export interface StripCell {
  id: string;
  /** The visible content: the name, or a glyph and a name. */
  content: ReactNode;
  /** The accessible name ("Paper A4"). */
  label: string;
  /** The tooltip: lines separated by a newline; the second is muted. None when empty. */
  tooltip: ReactNode;
}

/** How the arrows, Home and End move in a strip of fixed rows (`specs/0123` UX notes 2.5): every
 * arrow moves, without wrap, and Home and End move and select too. */
export interface FixedRows {
  /** Ids after which a new row starts: fixed rows that no platform font changes. */
  breakAfter: readonly string[];
}

interface PresetStripProps {
  /** The id of the group heading that names the strip; or `label` when there is no heading. */
  labelledBy?: string;
  label?: string;
  /** The id of a line that describes the strip (a "Damaged" line), if any. */
  describedBy?: string;
  cells: readonly StripCell[];
  /** The pressed cell, or none. */
  value: string | null;
  /** A press on a cell, also on the pressed one (Rust decides what it writes);
   * `keyboard` is true for an arrow key, which moves and presses. */
  onPress: (id: string, keyboard: boolean) => void;
  /** After a mouse press the keyboard goes back to the canvas. */
  onReturnFocus: () => void;
  /** The strip's width class: the panel column by default, `w-full` inside a form. */
  width?: string;
  /** Fixed rows with their own keys (the Dither strips); without it the strip is the Document
   * section's: one row wrapping by width, Left and Right with wrap. */
  fixedRows?: FixedRows;
  /** The side of the tooltip: "left" by default, "top" where a neighbour cell would be covered. */
  tooltipSide?: "left" | "top";
}

/** The cells cut after each id of `breakAfter`. */
function rowsOf(cells: readonly StripCell[], breakAfter: readonly string[]): StripCell[][] {
  const rows: StripCell[][] = [[]];
  for (const cell of cells) {
    rows[rows.length - 1].push(cell);
    if (breakAfter.includes(cell.id)) rows.push([]);
  }
  return rows.filter((row) => row.length > 0);
}

const BACK = ["ArrowLeft", "ArrowUp"];
const ON = ["ArrowRight", "ArrowDown"];

/** The arrows, Home and End move and select in a strip of fixed rows, without wrap. Captured on
 * a wrapper before Radix's roving focus takes the key: Radix moves the focus a tick later and
 * selects only when the arrow is still held then, so a short press would only move. */
function fixedRowsKey(event: KeyboardEvent<HTMLElement>) {
  const { key } = event;
  if (key !== "Home" && key !== "End" && !BACK.includes(key) && !ON.includes(key)) return;
  const items = Array.from(
    event.currentTarget.querySelectorAll<HTMLElement>('[role="radio"]:not([disabled])'),
  );
  const at = items.findIndex((item) => item === document.activeElement);
  if (at < 0) return;
  event.preventDefault();
  event.stopPropagation();
  const to =
    key === "Home"
      ? 0
      : key === "End"
        ? items.length - 1
        : Math.min(items.length - 1, Math.max(0, at + (BACK.includes(key) ? -1 : 1)));
  // Already there (an arrow at the end, Home on the first, End on the last): no second press.
  if (to === at) return;
  const target = items[to];
  target.focus();
  target.click();
}

const BORDER = "color-mix(in srgb, var(--toolbar-icon) 60%, transparent)";

const CELL =
  "flex h-7 min-w-8 flex-[1_1_auto] items-center justify-center gap-1.5 bg-[var(--panel-bg)] px-1.5 text-sm text-[var(--toolbar-icon)] outline-none hover:[background-image:linear-gradient(var(--editor-accent-hover),var(--editor-accent-hover))] focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-[var(--editor-accent)] aria-checked:![background-image:none] aria-checked:!bg-[var(--toolbar-icon-active-bg)] aria-checked:!text-[var(--toolbar-icon-active-fg)]";

/**
 * A strip of choices for the Document section (`specs/0030-document-size-
 * presets` UX notes): the look of the panel's `ToggleGroup` (one 1 px border,
 * 28 px high, 1 px between cells) with cells as wide as their names need, at
 * least 32 px, wrapping to a further row when they do not fit. Radix's
 * `RadioGroup` gives one Tab stop per group and arrows that move and press; a
 * press on the pressed cell is forwarded too, because Radix reports no change
 * for it. With `fixedRows` (design system "Choice strip", `0123` Dither) the cells stand in fixed
 * rows, every arrow moves without wrap, and Home and End move and select.
 */
export function PresetStrip({
  labelledBy,
  label,
  describedBy,
  cells,
  value,
  onPress,
  onReturnFocus,
  width = "w-[244px]",
  fixedRows,
  tooltipSide = "left",
}: PresetStripProps) {
  // In fixed rows a cell shrinks with its row instead of wrapping to a row of its own.
  const cellClass = fixedRows ? `${CELL} min-w-0 overflow-hidden whitespace-nowrap` : CELL;
  const cell = (one: StripCell) => (
    <Tooltip key={one.id} side={tooltipSide} content={one.tooltip}>
      <RadioGroup.Item
        value={one.id}
        aria-label={one.label}
        onClick={(event) => {
          // `detail` is 0 for keyboard activation, 1 or more for a click.
          onPress(one.id, event.detail === 0);
          if (event.detail > 0) {
            onReturnFocus();
          }
        }}
        className={cellClass}
      >
        {one.content}
      </RadioGroup.Item>
    </Tooltip>
  );
  const shared = {
    "aria-labelledby": labelledBy,
    "aria-label": labelledBy ? undefined : label,
    "aria-describedby": describedBy,
    value: value ?? "",
    // The press itself is the click handler below (it also covers the pressed
    // cell and the arrow keys, which Radix turns into a click).
    onValueChange: () => undefined,
    style: { borderColor: BORDER, background: BORDER },
  };
  if (!fixedRows) {
    return (
      <RadioGroup.Root
        {...shared}
        orientation="horizontal"
        className={`flex ${width} flex-wrap gap-px overflow-hidden rounded-[5px] border`}
      >
        {cells.map(cell)}
      </RadioGroup.Root>
    );
  }
  return (
    <div className="contents" onKeyDownCapture={fixedRowsKey}>
      <RadioGroup.Root
        {...shared}
        // No orientation: Left and Up, Right and Down all move; no wrap.
        loop={false}
        className={`flex ${width} flex-col gap-px overflow-hidden rounded-[5px] border`}
      >
        {rowsOf(cells, fixedRows.breakAfter).map((row) => (
          // A fixed row never wraps (`0123` Dither in a narrow operation row: Grey stays in its row).
          <div key={row[0].id} className="flex flex-wrap gap-px" style={{ flexWrap: "nowrap" }}>
            {row.map(cell)}
          </div>
        ))}
      </RadioGroup.Root>
    </div>
  );
}
