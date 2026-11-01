import { EntryField } from "@/components/panel/EntryField";
import { PanelRadioGroup } from "@/components/panel/PanelRadioGroup";
import { PanelSwitch } from "@/components/ui/switch";
import { Tooltip } from "@/components/ui/tooltip";
import type { SnappingApi } from "@/hooks/useSnapping";

const TARGETS = [
  ["objects", "Objects", "Snap to the boxes of other objects."],
  ["guides", "Guides", "Snap to guides."],
  ["nodes", "Nodes", "Snap to nodes."],
  ["page", "Page", "Snap to the page's 25 %, 33.3 %, 50 %, 66.6 %, 75 % and 100 % lines."],
] as const;
const MESSAGES = { number: "Enter a non-negative number" };
const GRID_POINTS = [["centre", "Centre"], ["box", "Box corners and edges"], ["nodes", "Nodes"]] as const;
const TOGGLE_CLASS = "h-7 rounded border border-[color-mix(in_srgb,var(--toolbar-icon)_60%,transparent)] text-sm hover:bg-[var(--editor-accent-hover)] aria-pressed:bg-[var(--toolbar-icon-active-bg)] aria-pressed:text-[var(--toolbar-icon-active-fg)] focus-visible:outline-2 focus-visible:outline-[var(--editor-accent)]";

/** Document-owned snapping controls in their own Properties pane. */
export function SnappingSection({ snapping, onReturnFocus }: {
  snapping: SnappingApi;
  onReturnFocus: () => void;
}) {
  const { view } = snapping;
  return <fieldset disabled={view.hasDrag} className="min-w-0 flex flex-col gap-4 text-[var(--toolbar-icon)]">
    <PanelSwitch label="Snapping" checked={view.enabled} onCheckedChange={() => snapping.toggle()} onReturnFocus={onReturnFocus} />
    <fieldset className="min-w-0">
      <legend className="mb-2 text-sm">Snap targets</legend>
      <div className="grid grid-cols-2 gap-1">
        {TARGETS.map(([key, label, tooltip]) => <Tooltip key={key} side="left" content={tooltip}>
          <button type="button" aria-pressed={view[key]} className={TOGGLE_CLASS}
            onClick={(event) => {
              const next = { ...view, [key]: !view[key] };
              snapping.setTargets(next.objects, next.guides, next.nodes, next.page);
              if (event.detail > 0) onReturnFocus();
            }}>{label}</button>
        </Tooltip>)}
        <button type="button" aria-pressed={view.grid} className={TOGGLE_CLASS}
          onClick={(event) => {
            snapping.setGrid(!view.grid);
            if (event.detail > 0) onReturnFocus();
          }}>Grid</button>
      </div>
    </fieldset>
    <PanelRadioGroup legend="Grid snaps with" options={GRID_POINTS} value={view.gridPoint} stacked
      onChange={(point) => snapping.setGridPoint(point)} onReturnFocus={onReturnFocus} />
    <fieldset className="min-w-0">
      <legend className="mb-2 text-sm">Snap axes</legend>
      <div className="grid grid-cols-2 gap-1">
        {([ ["x", "Horizontal (X)"], ["y", "Vertical (Y)"] ] as const).map(([key, label]) =>
          <button key={key} type="button" aria-pressed={view[key]} className="h-7 rounded border border-[color-mix(in_srgb,var(--toolbar-icon)_60%,transparent)] text-sm hover:bg-[var(--editor-accent-hover)] aria-pressed:bg-[var(--toolbar-icon-active-bg)] aria-pressed:text-[var(--toolbar-icon-active-fg)] focus-visible:outline-2 focus-visible:outline-[var(--editor-accent)]"
            onClick={(event) => {
              snapping.setAxes(key === "x" ? !view.x : view.x, key === "y" ? !view.y : view.y);
              if (event.detail > 0) onReturnFocus();
            }}>{label}</button>)}
      </div>
    </fieldset>
    {(["capturePx", "releasePx"] as const).map((key) => <label key={key} className="flex items-center justify-between text-sm">
      <span>{key === "capturePx" ? "Capture" : "Release"}</span>
      <EntryField label={`${key === "capturePx" ? "Capture" : "Release"}, pixels`} shown={String(view[key])} mixed={false} suffix="px" width={112} inputMode="decimal" commitOnBlur messages={MESSAGES} onReturnFocus={onReturnFocus}
        onSubmit={(text) => {
          if (!text.trim()) return "invalid:number";
          const value = Number(text);
          if (!Number.isFinite(value) || value < 0) return "invalid:number";
          const changed = snapping.setZones(key === "capturePx" ? value : view.capturePx, key === "releasePx" ? value : view.releasePx);
          return changed ? "committed" : "unchanged";
        }} />
    </label>)}
  </fieldset>;
}
