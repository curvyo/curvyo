import { Grid3x3 } from "lucide-react";
import { CornerControl } from "@/components/CornerControl";
import { EntryField } from "@/components/panel/EntryField";
import { PanelRadioGroup } from "@/components/panel/PanelRadioGroup";
import { GRID_MESSAGES, type GridApi, type GridField } from "@/hooks/useGrid";
import { ToggleButton } from "@/components/ui/toggle-button";
import { PANEL_CONTROL_PX, PANEL_LABEL, PANEL_ROW } from "@/components/ui/looks";

const MODES = [["adaptive", "Adaptive"], ["fixed", "Fixed"]] as const;
const SCALES = [["linear", "Linear"], ["log", "Logarithmic"]] as const;

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
      <CornerControl label="Origin" corner={view.origin} onChange={(corner) => grid.setOrigin(corner)} onReturnFocus={onReturnFocus} />
      <ToggleButton icon={Grid3x3} label="Grid" pressed={view.shown} onPressedChange={(checked) => grid.setShown(checked)} onReturnFocus={onReturnFocus} />
      <PanelRadioGroup legend="Mode" options={MODES} value={view.mode} onChange={(mode) => grid.setMode(mode)} onReturnFocus={onReturnFocus} />
      <PanelRadioGroup legend="Scale" options={SCALES} value={view.scale} onChange={(scale) => grid.setScale(scale)} onReturnFocus={onReturnFocus} />
      <div className="flex flex-col gap-2">
        <label className={PANEL_ROW}>
          <span className={PANEL_LABEL}>Spacing</span>
          {field("spacing", "Spacing", view.spacing, view.unit, PANEL_CONTROL_PX)}
        </label>
        <label className={PANEL_ROW}>
          <span className={PANEL_LABEL}>Divisions</span>
          {field("subdivision", "Subdivision", view.subdivision, undefined, PANEL_CONTROL_PX)}
        </label>
        <label className={PANEL_ROW}>
          <span className={PANEL_LABEL}>Angle</span>
          {field("angle", "Angle", view.angle, "°", PANEL_CONTROL_PX)}
        </label>
      </div>
    </fieldset>
  );
}
