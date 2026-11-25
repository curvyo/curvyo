import { TriangleAlert } from "lucide-react";
import { useId, useRef } from "react";

import { OverrideRow } from "@/components/panel/CloneBlock";
import { ValueField } from "@/components/panel/ValueField";
import type { SketchInstanceApi } from "@/hooks/useSketchInstance";
import {
  dragSpan,
  parseNumber,
  positionOf,
  shownNumber,
  sliderEnd,
  stepValue,
  valueAt,
} from "@/lib/generatedView";
import { PARAMETERS_SEGMENTS } from "@/lib/cloneText";
import {
  asGeneratedRow,
  followsWords,
  NO_SOLUTION_LINE,
  resetName,
  type SketchInstanceRow,
} from "@/lib/sketchInstanceView";

const MESSAGES = { number: "Enter a number" } as const;

/** One value of the sketch: the panel's Value field on the track Rust gives it, muted while it
 * follows the original (`specs/0099-sketch-pages` criterion 24, UX notes 12.2). */
function ValueRow({
  row,
  api,
  cloneName,
  describedBy,
  onReturnFocus,
}: {
  row: SketchInstanceRow;
  api: SketchInstanceApi;
  cloneName: string;
  describedBy: string | undefined;
  onReturnFocus: () => void;
}) {
  // The value of the last drag or key tick, committed once when the gesture ends.
  const pending = useRef<number | null>(null);
  const field = asGeneratedRow(row);
  const decimals = row.unit === "deg" ? 1 : 2;
  const unit = row.unit === "deg" ? "°" : "mm";
  const current = row.value ?? row.sketchValue;
  const own = row.origin === "own" || row.origin === "mixed";
  const { min, limit } = dragSpan(field);
  const bar = positionOf(field, current);
  const text = row.mixed ? "" : shownNumber(current, decimals);
  const commit = () => {
    const value = pending.current;
    pending.current = null;
    if (value !== null) {
      api.setValue(row.key, value);
    }
  };
  const words = row.unit === "deg" ? "degrees" : "millimetres";
  return (
    <div onKeyUp={commit} className="relative w-fit" data-sketch-value={row.key}>
      <ValueField
        label={row.label}
        name={`${cloneName} ${row.label.toLowerCase()}`}
        text={text}
        bar={Math.min(1, Math.max(0, bar))}
        dragBase={bar}
        dragLimit={limit}
        dragMin={min}
        mixed={row.mixed}
        resettable={own}
        resetLabel={resetName(row)}
        muted={!own}
        valueText={`${text} ${words}${followsWords(row)}`}
        tipValue={`${text}${row.unit === "deg" ? unit : ` ${unit}`}${followsWords(row)}`}
        unit={unit}
        valueNow={current}
        unitWords={words}
        valueMin={field.kind.kind === "length" || field.kind.kind === "angle" ? field.kind.min : undefined}
        valueMax={sliderEnd(field)}
        defaultText={`${shownNumber(row.sketchValue, 4)}${row.unit === "deg" ? unit : ` ${unit}`}`}
        messages={MESSAGES}
        describedBy={describedBy}
        onPreview={(p, grid) => {
          const value = valueAt(field, p, grid);
          pending.current = value;
          api.preview(row.key, value);
        }}
        onSettle={commit}
        onStep={(steps, grid) => {
          // The canvas follows the key; the key-up is the one commit (the Generator block's rule).
          const next = stepValue(field, pending.current ?? current, steps, grid);
          pending.current = next;
          api.preview(row.key, next);
        }}
        onReset={() => api.reset(row.key)}
        onCancel={() => {
          pending.current = null;
          api.cancelPreview();
        }}
        onSubmit={(typed) => {
          const number = parseNumber(typed);
          return number === null ? "invalid:number" : api.setValue(row.key, number);
        }}
        onReturnFocus={() => {
          commit();
          onReturnFocus();
        }}
      />
      {api.notice?.key === row.key ? (
        <p
          role="status"
          className="pointer-events-none absolute top-full right-0 z-10 mt-1 w-max max-w-[244px] break-words rounded-md p-2 text-right text-xs"
          style={{
            background: "var(--toolbar-bg)",
            color: "var(--toolbar-icon)",
            boxShadow: "var(--panel-elevation-shadow)",
          }}
        >
          {api.notice.text}
        </p>
      ) : null}
    </div>
  );
}

/**
 * The Parameters override row of the clone block (`specs/0099-sketch-pages` criteria 24 and 26, UX
 * notes 12.2 and 12.4): the Original's / Own pill and one Value field per exposed value, with the
 * state line under the pill while a value set has no solution. In the tree only while the
 * selected clones' sketch exposes a value (Rust sends `null` otherwise).
 */
export function SketchParametersRow({
  api,
  onReturnFocus,
  onPressedAt,
}: {
  api: SketchInstanceApi;
  onReturnFocus: () => void;
  onPressedAt: (button: DOMRect) => void;
}) {
  const { view } = api;
  const lineId = useId();
  if (!view) {
    return null;
  }
  // "Bracket clone width", "Bracket clones width" (UX notes 12.7); the value is the field's own text.
  const cloneName = `${view.sketchName || "Sketch"} ${view.clones === 1 ? "clone" : "clones"}`;
  return (
    <div className="flex flex-col gap-2" data-sketch-parameters>
      <OverrideRow
        label="Parameters"
        segments={PARAMETERS_SEGMENTS}
        state={view.state}
        onSet={(own) => (own ? api.own() : api.originals())}
        onReturnFocus={onReturnFocus}
        onPressedAt={onPressedAt}
      />
      {view.noSolution ? (
        <p
          id={lineId}
          role="status"
          aria-live="polite"
          data-no-solution
          className="flex items-center gap-1 text-xs"
          style={{ color: "var(--toolbar-icon)" }}
        >
          <TriangleAlert aria-hidden size={12} />
          {NO_SOLUTION_LINE}
        </p>
      ) : null}
      {view.rows.map((row) => (
        <ValueRow
          key={row.key}
          row={row}
          api={api}
          cloneName={cloneName}
          describedBy={view.noSolution ? lineId : undefined}
          onReturnFocus={onReturnFocus}
        />
      ))}
    </div>
  );
}
