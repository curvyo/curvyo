import { RadioGroup } from "radix-ui";
import type { KeyboardEvent, ReactNode } from "react";

export interface Tile {
  value: string;
  /** The visible name, also the accessible name. */
  label: string;
  icon: ReactNode;
}

interface TileGroupProps {
  /** The accessible name of the group ("Pattern generator"). */
  label: string;
  tiles: readonly Tile[];
  /** The pressed tile; `null` presses none (mixed). */
  value: string | null;
  onChange: (value: string) => void;
  /** Tiles per row. */
  columns: number;
  /** After a pointer press, so the canvas keeps the keys. */
  onReturnFocus: () => void;
}

const BORDER = "border-[color-mix(in_srgb,var(--toolbar-icon)_60%,transparent)]";

/**
 * A two-dimensional radio group of tiles (`specs/0071-pattern-fill` UX notes §4): one Tab stop,
 * Left and Right move and select along the reading order, Up and Down move one row, Home and End
 * select the first and last. Each move selects, as `ToggleGroup` does. The pressed tile has the
 * `ToggleGroup` pressed look.
 */
export function TileGroup({ label, tiles, value, onChange, columns, onReturnFocus }: TileGroupProps) {
  const select = (event: KeyboardEvent<HTMLDivElement>, index: number) => {
    const items = Array.from(event.currentTarget.querySelectorAll<HTMLElement>('[role="radio"]'));
    const target = items[index];
    if (target) {
      event.preventDefault();
      event.stopPropagation();
      target.focus();
      onChange(target.dataset.value ?? "");
    }
  };
  return (
    <RadioGroup.Root
      key={value === null ? "none" : "set"}
      aria-label={label}
      value={value ?? ""}
      loop
      onValueChange={(next) => onChange(next)}
      onKeyDownCapture={(event) => {
        const current = tiles.findIndex((tile) => tile.value === (event.target as HTMLElement).dataset.value);
        const last = tiles.length - 1;
        switch (event.key) {
          case "Home":
            select(event, 0);
            break;
          case "End":
            select(event, last);
            break;
          case "ArrowUp":
            select(event, current - columns < 0 ? current : current - columns);
            break;
          case "ArrowDown":
            select(event, current + columns > last ? current : current + columns);
            break;
          default:
        }
      }}
      className="grid w-fit gap-1"
      style={{ gridTemplateColumns: `repeat(${columns}, 120px)` }}
    >
      {tiles.map((tile) => (
        <RadioGroup.Item
          key={tile.value}
          value={tile.value}
          data-value={tile.value}
          aria-label={tile.label}
          onClick={(event) => {
            if (event.detail > 0) {
              onReturnFocus();
            }
          }}
          className={`flex h-10 items-center gap-1.5 rounded-[5px] border ${BORDER} bg-transparent px-1.5 text-left text-xs leading-tight text-[var(--toolbar-icon)] outline-none hover:bg-[var(--editor-accent-hover)] focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)] focus-visible:ring-offset-1 focus-visible:ring-offset-[var(--toolbar-bg)] aria-checked:!bg-[var(--toolbar-icon-active-bg)] aria-checked:!text-[var(--toolbar-icon-active-fg)]`}
        >
          <span className="shrink-0">{tile.icon}</span>
          <span>{tile.label}</span>
        </RadioGroup.Item>
      ))}
    </RadioGroup.Root>
  );
}
