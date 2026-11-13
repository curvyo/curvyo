import { Hash } from "lucide-react";
import { useId, useRef, type KeyboardEvent } from "react";

import { EntryField } from "@/components/panel/EntryField";
import { Divider, useRowStarts } from "@/components/SelectToolbar";
import { OperationGlyph } from "@/components/panel/OperationGlyphs";
import { TestGridMaterialMenu } from "@/components/TestGridMaterialMenu";
import type { TestGridToolApi } from "@/hooks/useTestGridTool";
import type { SetKind } from "@/lib/operationSettingsView";
import {
  CELL_STEP,
  PARAM_STEP,
  PARAM_UNIT,
  PARAM_WORD,
  SHARED_WIDTH,
  axisFieldName,
  axisKeys,
  axisStep,
  axisUnit,
  barName,
  fieldText,
  plainNumber,
  steppedText,
} from "@/lib/testGridBar";
import type { AxisFields, Param, TestGridBarView } from "@/lib/testGridView";
import { CHIP_IS_MESSAGE, outcomeOf } from "@/lib/settingsFieldText";

const KINDS: readonly { kind: SetKind; word: string }[] = [
  { kind: "cut", word: "Cut" },
  { kind: "score", word: "Score" },
  { kind: "engrave", word: "Engrave" },
];

/** A bar row: its groups wrap whole, 25 px apart with the 1 px divider between them (UX 4.1);
 * a group that starts a wrapped line has none (`useRowStarts`). */
const ROW_CLASS = "flex min-w-0 flex-wrap items-start gap-x-[25px] gap-y-0";

/** The segments' ground: 28 px, 1 px ink at 60 %, rounded 5 px (the preset strip look). */
const GROUP_CLASS = "flex h-7 shrink-0 overflow-hidden rounded-[5px] outline outline-1 -outline-offset-1 outline-[var(--toolbar-icon)]/60";

interface SegmentOption<T extends string> {
  value: T;
  label: string;
  glyph?: SetKind;
  width: number;
}

/** A radiogroup of segments: arrows move and pick, one Tab stop. */
function Segments<T extends string>({
  label,
  options,
  value,
  onPick,
}: {
  label: string;
  options: readonly SegmentOption<T>[];
  value: T;
  onPick: (value: T) => void;
}) {
  const keys = (event: KeyboardEvent<HTMLDivElement>) => {
    const at = options.findIndex((option) => option.value === value);
    const step = event.key === "ArrowRight" || event.key === "ArrowDown" ? 1 : event.key === "ArrowLeft" || event.key === "ArrowUp" ? -1 : 0;
    if (step === 0) return;
    event.preventDefault();
    const next = options[Math.min(options.length - 1, Math.max(0, at + step))];
    onPick(next.value);
    window.requestAnimationFrame(() => {
      (event.currentTarget.querySelector(`[data-segment="${next.value}"]`) as HTMLElement | null)?.focus();
    });
  };
  return (
    <div role="radiogroup" aria-label={label} className={GROUP_CLASS} onKeyDown={keys}>
      {options.map((option) => {
        const on = option.value === value;
        return (
          <button
            key={option.value}
            type="button"
            role="radio"
            aria-checked={on}
            data-segment={option.value}
            tabIndex={on ? 0 : -1}
            onClick={() => onPick(option.value)}
            style={{ minWidth: option.width }}
            className={`flex shrink-0 items-center justify-center gap-1 px-1.5 text-[13px] text-[var(--toolbar-icon)] outline-none hover:bg-[var(--editor-accent-hover)] focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-[var(--editor-accent)] ${on ? "bg-[var(--value-fill)]" : ""}`}
          >
            {option.glyph ? <OperationGlyph kind={option.glyph} /> : null}
            {option.label}
          </button>
        );
      })}
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
            options={axis.offered.map((param) => ({ value: param, label: PARAM_WORD[param], width: param === "interval" ? 64 : param === "passes" ? 60 : 52 }))}
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
 * Interval, Crosshatch for Engrave) and Cell size; row 2: the Columns and the Rows unit. Plain
 * fields with their units inside, no sliders and no captions in row 1. Row 1 wraps by whole
 * groups; each unit of row 2 is one wrap item. A value the grid did not take stays as typed with
 * its cause; nothing is clamped.
 */
export function TestGridBar({ tool, view, onNewMaterial, onReturnFocus }: TestGridBarProps) {
  const noticeId = useId();
  const shared = (param: Param) => view.shared.find((one) => one.param === param);
  const notice = tool.pressNotice ?? view.fit;
  const pill = "pointer-events-auto flex min-w-0 max-w-full flex-col rounded-lg px-2 text-sm";
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
            {view.id !== null ? (
              <span
                title="Test id"
                className="flex h-9 w-16 shrink-0 items-center font-mono text-[13px] font-semibold tracking-[0.08em] text-[var(--toolbar-icon)]"
              >
                {view.id}
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
                  name={`${PARAM_WORD[param]}, ${PARAM_UNIT[param]}`}
                  prefix={PARAM_WORD[param]}
                  unit={PARAM_UNIT[param]}
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
              <button
                type="button"
                aria-label="Crosshatch"
                aria-pressed={view.crosshatch}
                title="Crosshatch"
                onClick={() => tool.setCrosshatch(!view.crosshatch)}
                className={`flex size-7 shrink-0 items-center justify-center rounded-md outline-none focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)] ${
                  view.crosshatch
                    ? "bg-[var(--toolbar-icon-active-bg)] text-[var(--toolbar-icon-active-fg)]"
                    : "text-[var(--toolbar-icon)] hover:bg-[var(--editor-accent-hover)]"
                }`}
              >
                <Hash size={16} aria-hidden />
              </button>
            ) : null}
          </div>
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
          className="pointer-events-auto w-fit max-w-full rounded-md px-2 py-1 text-xs text-[var(--toolbar-icon)]"
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
