import { CornerControl } from "@/components/CornerControl";
import { EntryField } from "@/components/panel/EntryField";
import { PanelRadioGroup } from "@/components/panel/PanelRadioGroup";
import { PanelSwitch } from "@/components/ui/switch";
import { GRID_MESSAGES, type GridApi, type GridField } from "@/hooks/useGrid";

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
