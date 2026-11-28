import { Crosshair, File, Grid3x3, Magnet, MoveHorizontal, MoveVertical, Pin, Spline, Square, SeparatorVertical, type LucideIcon } from "lucide-react";
import { EntryField } from "@/components/panel/EntryField";
import { PanelRadioGroup } from "@/components/panel/PanelRadioGroup";
import type { FixturesApi } from "@/hooks/useFixtures";
import type { SnappingApi } from "@/hooks/useSnapping";
import { ToggleButton } from "@/components/ui/toggle-button";
import { SECTION_HEADING, PANEL_CONTROL_PX, PANEL_LABEL, PANEL_ROW } from "@/components/ui/looks";

const TARGETS: readonly (readonly ["objects" | "guides" | "nodes" | "page", string, string, LucideIcon])[] = [
  ["objects", "Objects", "Snap to the boxes of other objects.", Square],
  ["guides", "Guides", "Snap to guides.", SeparatorVertical],
  ["nodes", "Nodes", "Snap to nodes.", Spline],
  ["page", "Page", "Snap to the page's 25 %, 33.3 %, 50 %, 66.6 %, 75 % and 100 % lines.", File],
];
const MESSAGES = { number: "Enter a non-negative number" };
const GRID_POINTS = [["centre", "Centre"], ["box", "Box corners and edges"], ["nodes", "Nodes"]] as const;
const CELL = "w-full";

/** Document-owned snapping controls in their own Properties pane. */
export function SnappingSection({ snapping, fixtures, onReturnFocus }: {
  snapping: SnappingApi;
  /** The Fixtures target shows only on a laser page with the extension on (`specs/0102` 9.2). */
  fixtures?: FixturesApi;
  onReturnFocus: () => void;
}) {
  const { view } = snapping;
  return <fieldset disabled={view.hasDrag} className="min-w-0 flex flex-col gap-4 text-[var(--toolbar-icon)]">
    <ToggleButton icon={Magnet} label="Snapping" pressed={view.enabled} onPressedChange={() => snapping.toggle()} onReturnFocus={onReturnFocus} />
    <fieldset className="min-w-0">
      <legend className={`mb-2 ${SECTION_HEADING}`}>Snap targets</legend>
      <div className="grid grid-cols-2 gap-1">
        {TARGETS.map(([key, label, tooltip, icon]) => (
          <ToggleButton key={key} look="bar" className={CELL} icon={icon} label={label} tooltip={tooltip} pressed={view[key]}
            onReturnFocus={onReturnFocus}
            onPressedChange={() => {
              const next = { ...view, [key]: !view[key] };
              snapping.setTargets(next.objects, next.guides, next.nodes, next.page);
            }} />
        ))}
        <ToggleButton look="bar" className={CELL} icon={Grid3x3} label="Grid" pressed={view.grid} onReturnFocus={onReturnFocus}
          onPressedChange={() => snapping.setGrid(!view.grid)} />
        {/* A job fixture's centres snap to circle and shape centres (`specs/0113` criterion 30). */}
        <ToggleButton look="bar" className={CELL} icon={Crosshair} label="Centres" tooltip="Snap job fixtures to the centres of circles and shapes."
          pressed={view.centres} onReturnFocus={onReturnFocus} onPressedChange={() => snapping.setCentres(!view.centres)} />
        {fixtures?.view ? (
          // One undo step per switch (`specs/0102` 9.2, "Snap to fixtures on" or "off"); an
          // ordinary target toggle like the others (2026-11-16, customer).
          <ToggleButton look="bar" className={CELL} icon={Pin} label="Fixtures" tooltip="Snap to the fixtures of this laser page."
            pressed={fixtures.view.snappingFixtures} onReturnFocus={onReturnFocus}
            onPressedChange={() => fixtures.setSnapping(!fixtures.view?.snappingFixtures)} />
        ) : null}
      </div>
    </fieldset>
    <PanelRadioGroup legend="Grid snaps with" options={GRID_POINTS} value={view.gridPoint} stacked
      onChange={(point) => snapping.setGridPoint(point)} onReturnFocus={onReturnFocus} />
    <fieldset className="min-w-0">
      <legend className={`mb-2 ${SECTION_HEADING}`}>Snap axes</legend>
      <div className="grid grid-cols-2 gap-1">
        {([ ["x", "Horizontal (X)", MoveHorizontal], ["y", "Vertical (Y)", MoveVertical] ] as const).map(([key, label, icon]) =>
          <ToggleButton key={key} look="bar" className={CELL} icon={icon} label={label} pressed={view[key]} onReturnFocus={onReturnFocus}
            onPressedChange={() => snapping.setAxes(key === "x" ? !view.x : view.x, key === "y" ? !view.y : view.y)} />)}
      </div>
    </fieldset>
    {(["capturePx", "releasePx"] as const).map((key) => <label key={key} className={PANEL_ROW}>
      <span className={PANEL_LABEL}>{key === "capturePx" ? "Capture" : "Release"}</span>
      <EntryField label={`${key === "capturePx" ? "Capture" : "Release"}, pixels`} shown={String(view[key])} mixed={false} suffix="px" width={PANEL_CONTROL_PX} inputMode="decimal" commitOnBlur messages={MESSAGES} onReturnFocus={onReturnFocus}
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
