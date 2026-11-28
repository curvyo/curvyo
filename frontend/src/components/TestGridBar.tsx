import { Hash } from "lucide-react";
import { useId, useRef, type KeyboardEvent, type ReactNode } from "react";

import { EntryField } from "@/components/panel/EntryField";
import { Divider, useRowStarts } from "@/components/SelectToolbar";
import { OperationGlyph } from "@/components/panel/OperationGlyphs";
import { ToggleButton } from "@/components/ui/toggle-button";
import { Tooltip } from "@/components/ui/tooltip";
import { TestGridMaterialMenu } from "@/components/TestGridMaterialMenu";
import type { TestGridToolApi } from "@/hooks/useTestGridTool";
import type { DitherMode, SetKind } from "@/lib/operationSettingsView";
import {
  CELL_STEP,
  PARAM_STEP,
  PARAM_WORD,
  SHARED_WIDTH,
  axisFieldName,
  axisKeys,
  axisStep,
  axisUnit,
  barName,
  fieldText,
  fieldUnit,
  plainNumber,
  sharedFieldName,
  steppedText,
} from "@/lib/testGridBar";
import type { AxisFields, Param, TestGridBarView } from "@/lib/testGridView";
import { CHIP_IS_MESSAGE, outcomeOf } from "@/lib/settingsFieldText";
import { SEGMENT_ITEM, SEGMENT_STRIP } from "@/components/ui/looks";

const KINDS: readonly { kind: SetKind; word: string }[] = [
  { kind: "cut", word: "Cut" },
  { kind: "score", word: "Score" },
  { kind: "engrave", word: "Engrave" },
  { kind: "image", word: "Image" },
];

/** A bar row: its groups wrap whole, 25 px apart with the 1 px divider between them (UX 4.1);
 * a group that starts a wrapped line has none (`useRowStarts`). */
const ROW_CLASS = "flex min-w-0 flex-wrap items-start gap-x-[25px] gap-y-0";

/** The segments' ground: 28 px, 1 px ink at 60 %, rounded 5 px (the preset strip look). */
const GROUP_CLASS = `flex shrink-0 ${SEGMENT_STRIP}`;

/** The Dither's ground: the same look, its two halves side by side or, when the bar row is too
 * narrow for the six, one under the other (two rows of three). */
const HALVES_CLASS = `flex min-w-0 max-w-full flex-wrap ${SEGMENT_STRIP}`;

interface SegmentOption<T extends string> {
  value: T;
  label: string;
  glyph?: SetKind;
  /** A symbol instead of the label: the label is then its tooltip and accessible name. */
  symbol?: ReactNode;
  width: number;
}

/** A radiogroup of segments: arrows move and pick, one Tab stop. */
function Segments<T extends string>({
  label,
  options,
  value,
  onPick,
  halves = false,
}: {
  label: string;
  options: readonly SegmentOption<T>[];
  value: T;
  onPick: (value: T) => void;
  /** The Dither (`specs/0123` UX notes 5): Home and End move and select too, and the segments
   * stand in two halves that wrap into two rows when the row is too narrow for one. */
  halves?: boolean;
}) {
  const keys = (event: KeyboardEvent<HTMLDivElement>) => {
    const at = options.findIndex((option) => option.value === value);
    const ends = halves && (event.key === "Home" || event.key === "End");
    const step = event.key === "ArrowRight" || event.key === "ArrowDown" ? 1 : event.key === "ArrowLeft" || event.key === "ArrowUp" ? -1 : 0;
    if (step === 0 && !ends) return;
    event.preventDefault();
    const next = ends
      ? options[event.key === "Home" ? 0 : options.length - 1]
      : options[Math.min(options.length - 1, Math.max(0, at + step))];
    onPick(next.value);
    // React clears `currentTarget` after the handler: keep the group for the next frame.
    const group = event.currentTarget;
    window.requestAnimationFrame(() => {
      (group.querySelector(`[data-segment="${next.value}"]`) as HTMLElement | null)?.focus();
    });
  };
  const segment = (option: SegmentOption<T>) => {
    const on = option.value === value;
    const button = (
      <button
        key={option.value}
        type="button"
        role="radio"
        aria-checked={on}
        aria-label={option.symbol ? option.label : undefined}
        data-segment={option.value}
        tabIndex={on ? 0 : -1}
        onClick={() => onPick(option.value)}
        style={{ minWidth: option.width }}
        className={`${SEGMENT_ITEM} shrink-0 px-1.5`}
      >
        {option.glyph ? <OperationGlyph kind={option.glyph} /> : null}
        {option.symbol ?? option.label}
      </button>
    );
    return option.symbol ? (
      <Tooltip key={option.value} side="bottom" content={option.label}>
        {button}
      </Tooltip>
    ) : (
      button
    );
  };
  if (halves) {
    const half = Math.ceil(options.length / 2);
    return (
      <div role="radiogroup" aria-label={label} className={HALVES_CLASS} onKeyDown={keys}>
        <div className="flex h-7">{options.slice(0, half).map(segment)}</div>
        <div className="flex h-7">{options.slice(half).map(segment)}</div>
      </div>
    );
  }
  return (
    <div role="radiogroup" aria-label={label} className={GROUP_CLASS} onKeyDown={keys}>
      {options.map(segment)}
    </div>
  );
}

interface BarFieldProps {
  name: string;
  prefix: string;
  unit: string;
  fieldKey: string;
  value: number;
  width: number;
  step: number;
  view: TestGridBarView;
  tool: TestGridToolApi;
  onReturnFocus: () => void;
}

/** One plain field with its word and unit inside (no slider): Enter or leaving commits, Escape
 * reverts, Up and Down step (Shift: ten times). */
function BarField({ name, prefix, unit, fieldKey, value, width, step, view, tool, onReturnFocus }: BarFieldProps) {
  // Escape reverts: a value the grid did not take is kept by the session (criterion 5), so
  // Escape on it re-commits the grid's own value, which drops the kept text and makes no step.
  const revertKept = (event: KeyboardEvent<HTMLDivElement>) => {
    if (event.key === "Escape" && view.invalid?.field === fieldKey) {
      tool.setField(fieldKey, plainNumber(value));
    }
  };
  return (
    <div className="flex h-9 items-center" onKeyDownCapture={revertKept}>
      <EntryField
        label={name}
        prefix={prefix}
        shown={fieldText(view, fieldKey, value)}
        mixed={false}
        suffix={unit}
        smallSuffix={unit.length > 2}
        width={width}
        gutter={unit.length > 2 ? 52 : undefined}
        commitOnBlur
        inputMode="decimal"
        stepText={(text, key, shift) =>
          key === "ArrowUp" || key === "ArrowDown" ? steppedText(text, step, key === "ArrowUp" ? 1 : -1, shift) : null
        }
        onSubmit={(text) => outcomeOf(tool.setField(fieldKey, text))}
        messages={CHIP_IS_MESSAGE}
        onReturnFocus={onReturnFocus}
      />
    </div>
  );
}

/** One axis unit of row 2: its caption, the parameter switch and Min, Max, Count. It breaks once,
 * after its parameter, when it is wider than the bar (G5). */
function AxisUnit({
  side,
  axis,
  view,
  tool,
  onReturnFocus,
}: {
  side: "columns" | "rows";
  axis: AxisFields;
  view: TestGridBarView;
  tool: TestGridToolApi;
  onReturnFocus: () => void;
}) {
  const keys = axisKeys(side);
  const caption = side === "columns" ? "Columns" : "Rows";
  const field = (part: "Min" | "Max" | "Count", key: string, value: number, width: number) => (
    <BarField
      name={axisFieldName(side, part, axis)}
      prefix={part}
      unit={axisUnit(part, axis)}
      fieldKey={key}
      value={value}
      width={width}
      step={axisStep(part, axis)}
      view={view}
      tool={tool}
      onReturnFocus={onReturnFocus}
    />
  );
  return (
    <div role="group" aria-label={caption} className="group/row relative flex min-w-0 max-w-full items-start">
      {side === "rows" ? <Divider /> : null}
      <span className="flex h-9 w-14 shrink-0 items-center text-xs text-[var(--panel-muted-fg)]">{caption}</span>
      <div className="flex min-w-0 flex-wrap items-center gap-x-2">
        <div className="flex h-9 items-center">
          <Segments
            label={`${caption} parameter`}
            value={axis.param}
            options={axis.offered.map((param) => ({ value: param, label: PARAM_WORD[param], width: param === "interval" || param === "strength" ? 64 : param === "passes" ? 60 : 52 }))}
            onPick={(param) => tool.setAxis(side, param)}
          />
        </div>
        {/* In the smallest window even the broken unit is wider than the bar: Count then goes
            under Min rather than past the pill's edge. */}
        <div className="flex min-w-0 flex-wrap items-center gap-x-1">
          {field("Min", keys.min, axis.min, 128)}
          {field("Max", keys.max, axis.max, 128)}
          {field("Count", keys.count, axis.count, 76)}
        </div>
      </div>
    </div>
  );
}

interface TestGridBarProps {
  tool: TestGridToolApi;
  view: TestGridBarView;
  /** "New material": the Materials tab opens with its form; `grid` is the node id JSON, or empty. */
  onNewMaterial: (grid: string) => void;
  onReturnFocus: () => void;
}

/**
 * The Test grid bar (`specs/0112-test-grid-and-materials` `ux-notes.md` 4): two rows, always both
 * shown. Row 1: Kind, the test id and the material, the shared values (Power, Speed, Passes,
 * Interval, Crosshatch for Engrave, the Dither strip for Image) and Cell size; row 2: the Columns and the Rows unit. Plain
 * fields with their units inside, no sliders and no captions in row 1. Row 1 wraps by whole
 * groups; each unit of row 2 is one wrap item. A value the grid did not take stays as typed with
 * its cause; nothing is clamped.
 */
export function TestGridBar({ tool, view, onNewMaterial, onReturnFocus }: TestGridBarProps) {
  const noticeId = useId();
  const shared = (param: Param) => view.shared.find((one) => one.param === param);
  const notice = tool.pressNotice ?? view.fit;
  const pill = "pointer-events-auto flex min-w-0 max-w-full flex-col rounded-[8px] px-2 text-sm";
  const row1 = useRef<HTMLDivElement>(null);
  const row2 = useRef<HTMLDivElement>(null);
  useRowStarts(row1);
  useRowStarts(row2);
  return (
    <div className="pointer-events-none flex min-w-0 flex-col gap-1">
      <div
        data-context-bar=""
        role="toolbar"
        aria-label={barName(view)}
        aria-describedby={notice ? noticeId : undefined}
        className={pill}
        style={{ background: "var(--toolbar-bg)", boxShadow: "var(--panel-elevation-shadow)" }}
      >
        <div ref={row1} data-testgrid-row="1" className={ROW_CLASS}>
          <div className="group/row relative flex h-9 items-center">
            <Segments<SetKind>
              label="Kind"
              value={view.kind}
              options={KINDS.map((one) => ({ value: one.kind, label: one.word, glyph: one.kind, width: 68 }))}
              onPick={(kind) => tool.setKind(kind)}
            />
          </div>
          <div className="group/row relative flex items-center gap-2">
            <Divider />
            {/* The code says what it is: the muted caption "Test" before it, like the row captions
                (0164 F47). */}
            {view.id !== null ? (
              <span className="flex h-9 shrink-0 items-center gap-1.5 text-[var(--toolbar-icon)]">
                <span className="text-xs text-[var(--panel-muted-fg)]">Test</span>
                <Tooltip side="bottom" content="Test id">
                  <span className="font-mono text-sm font-semibold tracking-[0.08em]">{view.id}</span>
                </Tooltip>
              </span>
            ) : null}
            <TestGridMaterialMenu
              view={view}
              onPick={(id) => tool.pickMaterial(id)}
              onNewMaterial={() => onNewMaterial(view.group === null ? "" : JSON.stringify(view.group))}
            />
          </div>
          <div className="group/row relative flex flex-wrap items-center gap-x-2">
            <Divider />
            {(["power", "speed", "passes", "interval"] as const).map((param) => {
              const one = shared(param);
              if (!one) return null;
              return (
                <BarField
                  key={param}
                  name={sharedFieldName(param)}
                  prefix={PARAM_WORD[param]}
                  unit={fieldUnit(param)}
                  fieldKey={param}
                  value={one.value}
                  width={SHARED_WIDTH[param]}
                  step={PARAM_STEP[param]}
                  view={view}
                  tool={tool}
                  onReturnFocus={onReturnFocus}
                />
              );
            })}
            {view.crosshatch !== null ? (
              <ToggleButton
                look="icon"
                icon={Hash}
                label="Crosshatch"
                tooltip="Crosshatch"
                pressed={view.crosshatch}
                onPressedChange={(on) => tool.setCrosshatch(on)}
              />
            ) : null}
          </div>
          {view.picture !== null ? (
            <div className="group/row relative flex min-w-0 max-w-full items-center">
              <Divider />
              <div className="flex min-h-9 min-w-0 max-w-full items-center py-1">
                <Segments<"ramp" | "selected">
                  label="Picture"
                  value={view.picture.value}
                  options={[
                    { value: "ramp" as const, label: "Ramp", width: 56 },
                    ...(view.picture.selectedOffered ? [{ value: "selected" as const, label: "Selected picture", width: 120 }] : []),
                  ]}
                  onPick={(source) => tool.setPicture(source)}
                />
              </div>
            </div>
          ) : null}
          {view.dither !== null ? (
            <div className="group/row relative flex min-w-0 max-w-full items-center">
              <Divider />
              <div className="flex min-h-9 min-w-0 max-w-full items-center py-1">
                <Segments<DitherMode>
                  halves
                  label="Dither"
                  value={view.dither.value}
                  options={view.dither.modes.map((mode) => ({
                    value: mode.value,
                    label: mode.label,
                    ...(mode.label === "Floyd-Steinberg" ? { symbol: "Floyd" } : {}),
                    width: 56,
                  }))}
                  onPick={(mode) => tool.setDither(mode)}
                />
              </div>
            </div>
          ) : null}
          <div className="group/row relative">
            <Divider />
            <BarField
              name="Cell size, millimetres"
              prefix="Cell size"
              unit="mm"
              fieldKey="cell"
              value={view.cellMm}
              width={120}
              step={CELL_STEP}
              view={view}
              tool={tool}
              onReturnFocus={onReturnFocus}
            />
          </div>
        </div>
        <div ref={row2} data-testgrid-row="2" className={ROW_CLASS}>
          <AxisUnit side="columns" axis={view.columns} view={view} tool={tool} onReturnFocus={onReturnFocus} />
          <AxisUnit side="rows" axis={view.rows} view={view} tool={tool} onReturnFocus={onReturnFocus} />
        </div>
      </div>
      {notice ? (
        <p
          id={noticeId}
          role="alert"
          className="pointer-events-auto w-fit max-w-full rounded-[8px] px-2 py-1 text-xs text-[var(--toolbar-icon)]"
          style={{ background: "var(--toolbar-bg)", boxShadow: "var(--panel-elevation-shadow)" }}
        >
          {notice}
        </p>
      ) : null}
      <div role="status" aria-live="polite" aria-atomic="true" className="sr-only">
        {tool.status}
      </div>
    </div>
  );
}
