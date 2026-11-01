import { type KeyboardEvent, useRef } from "react";

import { EntryField } from "@/components/panel/EntryField";
import { PanelRadioGroup } from "@/components/panel/PanelRadioGroup";
import { PanelSwitch } from "@/components/ui/switch";
import { GRID_MESSAGES, type GridApi, type GridField, type OriginCorner } from "@/hooks/useGrid";

const MODES = [["adaptive", "Adaptive"], ["fixed", "Fixed"]] as const;
const SCALES = [["linear", "Linear"], ["log", "Logarithmic"]] as const;

const CORNERS = [
  ["top_left", "Top left", "left-0 top-0"],
  ["top_right", "Top right", "right-0 top-0"],
  ["bottom_left", "Bottom left", "bottom-0 left-0"],
  ["bottom_right", "Bottom right", "bottom-0 right-0"],
] as const;

/**
 * The origin corner (`specs/0090-grid` criterion 18): a small page pictogram with one radio button
 * on each corner. One Tab stop on the chosen corner, arrows move and choose in reading order.
 */
function OriginCornerPicker({ value, onChange, onReturnFocus }: {
  value: OriginCorner;
  onChange: (corner: OriginCorner) => void;
  onReturnFocus: () => void;
}) {
  const refs = useRef<(HTMLButtonElement | null)[]>([]);
  const onKeyDown = (event: KeyboardEvent<HTMLButtonElement>, index: number) => {
    const step = { ArrowRight: 1, ArrowDown: 1, ArrowLeft: -1, ArrowUp: -1 }[event.key];
    if (step === undefined) return;
    event.preventDefault();
    const next = (index + step + CORNERS.length) % CORNERS.length;
    onChange(CORNERS[next][0]);
    refs.current[next]?.focus();
  };
  return (
    <div className="flex items-center justify-between gap-2 text-sm">
      <span aria-hidden>Origin</span>
      <div role="radiogroup" aria-label="Origin corner" className="relative h-[72px] w-[56px]">
        <div aria-hidden className="absolute inset-[7px] rounded-[2px] border border-[color-mix(in_srgb,var(--toolbar-icon)_60%,transparent)]" />
        {CORNERS.map(([key, label, place], index) => (
          <button
            key={key}
            ref={(element) => {
              refs.current[index] = element;
            }}
            type="button"
            role="radio"
            aria-label={label}
            aria-checked={value === key}
            tabIndex={value === key ? 0 : -1}
            className={`absolute ${place} flex h-[14px] w-[14px] items-center justify-center rounded-full hover:bg-[var(--editor-accent-hover)] focus-visible:outline-2 focus-visible:outline-[var(--editor-accent)] group`}
            onKeyDown={(event) => onKeyDown(event, index)}
            onClick={(event) => {
              onChange(key);
              if (event.detail > 0) onReturnFocus();
            }}
          >
            <span aria-hidden className="h-2 w-2 rounded-full border border-[var(--toolbar-icon)] group-aria-checked:border-[var(--editor-accent)] group-aria-checked:bg-[var(--editor-accent)]" />
          </button>
        ))}
      </div>
    </div>
  );
}

/**
 * The active page's grid and rulers' origin (`specs/0090-grid` criteria 2, 3 and 18), in its own
 * Properties pane.
 */
export function GridSection({ grid, disabled, onReturnFocus }: {
  grid: GridApi;
  /** A canvas drag is in flight: the controls are inert, as in the Snapping pane. */
  disabled: boolean;
  onReturnFocus: () => void;
}) {
  const { view } = grid;
  const field = (key: GridField, label: string, shown: string, suffix: string | undefined, width: number) => (
    <EntryField
      label={label}
      shown={shown}
      mixed={false}
      suffix={suffix}
      width={width}
      inputMode="decimal"
      commitOnBlur
      messages={GRID_MESSAGES}
      onReturnFocus={onReturnFocus}
      onSubmit={(text) => grid.setField(key, text)}
    />
  );
  return (
    <fieldset disabled={disabled} className="min-w-0 flex flex-col gap-4 text-[var(--toolbar-icon)]">
      <div className="flex min-w-0 items-center justify-between gap-2 text-sm" aria-label={`Page ${view.pageName}`}>
        <span aria-hidden>Page</span>
        <span aria-hidden className="min-w-0 truncate text-[var(--panel-muted-fg)]">{view.pageName}</span>
      </div>
      <OriginCornerPicker value={view.origin} onChange={(corner) => grid.setOrigin(corner)} onReturnFocus={onReturnFocus} />
      <PanelSwitch label="Grid" checked={view.shown} onCheckedChange={(checked) => grid.setShown(checked)} onReturnFocus={onReturnFocus} />
      <PanelRadioGroup legend="Mode" options={MODES} value={view.mode} onChange={(mode) => grid.setMode(mode)} onReturnFocus={onReturnFocus} />
      <PanelRadioGroup legend="Scale" options={SCALES} value={view.scale} onChange={(scale) => grid.setScale(scale)} onReturnFocus={onReturnFocus} />
      <div className="flex flex-col gap-2">
        <label className="flex items-center justify-between text-sm">
          <span>Spacing</span>
          {field("spacing", "Spacing", view.spacing, view.unit, 112)}
        </label>
        <label className="flex items-center justify-between text-sm">
          <span>Subdivision</span>
          {field("subdivision", "Subdivision", view.subdivision, undefined, 112)}
        </label>
        <label className="flex items-center justify-between text-sm">
          <span>Angle</span>
          {field("angle", "Angle", view.angle, "°", 112)}
        </label>
      </div>
    </fieldset>
  );
}
