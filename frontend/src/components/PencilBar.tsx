import { useRef, useSyncExternalStore } from "react";

import { TraceField } from "@/components/TraceField";
import { ToggleGroup } from "@/components/ui/toggle-group";
import type { EditorSession } from "@/hooks/useEditorSession";
import { SMOOTHING_PARAM } from "@/lib/penModes";
import { openPenProbe } from "@/lib/penProbeStore";
import {
  MAX_RANGE,
  MIN_RANGE,
  PENCIL_DEFAULTS,
  type PressureCurveName,
  pencilSettings,
  setPencilSettings,
  subscribePencilSettings,
  widthsDimmed,
  withMax,
  withMin,
} from "@/lib/pencilSettings";
import type { TraceParam } from "@/lib/traceText";
import { Button } from "@/components/ui/button";
import { Activity, Gauge } from "lucide-react";
import { ToggleButton } from "@/components/ui/toggle-button";
import { BarGroup } from "@/components/TextBar";
import { useRowStarts } from "@/components/SelectToolbar";
import { BAR_ROW_CLASS } from "@/lib/barLayout";
import { Tooltip } from "@/components/ui/tooltip";

/** The two width fields: mm, two decimals, arrows 0.1 (Shift 1, Ctrl 0.01). */
function widthParam(label: string, [min, max]: readonly [number, number], fallback: number): TraceParam {
  return {
    name: "tolerance",
    label,
    kind: "length",
    unit: "mm",
    min,
    max,
    default: fallback,
    step: 0.1,
    big_step: 1,
    fine_step: 0.01,
    group: "pencil",
    advanced: false,
    choices: [],
  };
}

const MIN_PARAM = widthParam("Min", MIN_RANGE, PENCIL_DEFAULTS.min);
const MAX_PARAM = widthParam("Max", MAX_RANGE, PENCIL_DEFAULTS.max);
const CURVES: readonly { value: PressureCurveName; label: string; tooltip: string; icon: string; width: number }[] = [
  { value: "soft", label: "Soft", tooltip: "Soft: light pressure already widens the line", icon: "Soft", width: 52 },
  { value: "linear", label: "Linear", tooltip: "Linear: width in proportion to pressure", icon: "Linear", width: 52 },
  { value: "firm", label: "Firm", tooltip: "Firm: it takes firm pressure to widen the line", icon: "Firm", width: 52 },
];
const DIMMED_REASON = "Needs pen pressure or Simulate pressure.";

/** One group of the bar; a divider sits between groups (UX notes §2). */
/** One group of the bar: the shared bar group (divider only between two groups on one row). */
function Group({ children, divider = true }: { children: React.ReactNode; divider?: boolean }) {
  return (
    <BarGroup divider={divider} className="flex h-9 items-center gap-1">
      {children}
    </BarGroup>
  );
}

/** The Pencil bar (`specs/0158-pen-display` slice 1, UX notes §2, design-system row "Pencil bar"):
 * Smoothing, Simulate pressure, Min and Max, Curve, and the "No pen pressure" state. The values
 * live for the app run (`lib/pencilSettings`); the session takes them at the next stroke. */
export function PencilBar({ editor }: { editor: EditorSession }) {
  const settings = useSyncExternalStore(subscribePencilSettings, pencilSettings);
  const rowRef = useRef<HTMLDivElement>(null);
  useRowStarts(rowRef);
  const dimmed = widthsDimmed(settings);
  const back = () => editor.containerRef.current?.focus();
  const dim = (child: React.ReactNode) => (
    <Tooltip side="bottom" content={dimmed ? DIMMED_REASON : undefined}>
      <div aria-disabled={dimmed || undefined} aria-description={dimmed ? DIMMED_REASON : undefined} className={dimmed ? "opacity-40" : undefined}>
        {child}
      </div>
    </Tooltip>
  );
  return (
    <div
      ref={rowRef}
      role="toolbar"
      aria-label="Pencil options"
      className={`pointer-events-auto max-w-full shrink rounded-[8px] px-2 text-sm ${BAR_ROW_CLASS}`}
      style={{ background: "var(--toolbar-bg)", boxShadow: "var(--panel-elevation-shadow)" }}
    >
      <Group divider={false}>
        <TraceField
          param={SMOOTHING_PARAM}
          name="Pencil smoothing"
          value={settings.smoothing}
          onValue={(value) => setPencilSettings({ ...pencilSettings(), smoothing: value })}
          onEndDrag={() => undefined}
          onCreate={() => undefined}
          onReturnFocus={back}
        />
      </Group>
      <Group>
        <ToggleButton icon={Gauge} look="bar"
          label="Simulate pressure"
          tooltip="Simulate pressure: a slower stroke draws wider, for a mouse or a pen without pressure"
          pressed={settings.simulate}
          onPressedChange={(on) => setPencilSettings({ ...pencilSettings(), simulate: on })}
        />
      </Group>
      <Group>
        {dim(
          <TraceField
            param={MIN_PARAM}
            name="Minimum width"
            width={112}
            value={settings.min}
            onValue={(value) => dimmed || setPencilSettings(withMin(pencilSettings(), value))}
            onEndDrag={() => undefined}
            onCreate={() => undefined}
            onReturnFocus={back}
          />,
        )}
        {dim(
          <TraceField
            param={MAX_PARAM}
            name="Maximum width"
            width={112}
            value={settings.max}
            onValue={(value) => dimmed || setPencilSettings(withMax(pencilSettings(), value))}
            onEndDrag={() => undefined}
            onCreate={() => undefined}
            onReturnFocus={back}
          />,
        )}
      </Group>
      <Group>
        {dim(
          <ToggleGroup
            label="Pressure curve"
            options={CURVES}
            value={settings.curve}
            onChange={(curve) => dimmed || setPencilSettings({ ...pencilSettings(), curve })}
            itemWidth={52}
            itemHeight={28}
            bar
            tooltipSide="bottom"
            onReturnFocus={back}
          />,
        )}
      </Group>
      {settings.penSeen && settings.noPressure ? (
        <Group>
          <span role="status" className="text-sm whitespace-nowrap text-[var(--panel-muted-fg)]">
            No pen pressure
          </span>
          <Button size="bar" icon={Activity}
            onClick={openPenProbe} className="ml-2"
          >
            Pen probe…
          </Button>
        </Group>
      ) : null}
    </div>
  );
}
