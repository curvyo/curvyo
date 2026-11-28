import { Link, TriangleAlert, Unlink } from "lucide-react";
import { SMALL_GLYPH } from "@/components/ui/looks";
import { useId, useRef, useState } from "react";

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
  linkMarkTitle,
  linkWords,
  NO_SOLUTION_LINE,
  resetName,
  type SketchInstanceRow,
} from "@/lib/sketchInstanceView";
import { Tooltip } from "@/components/ui/tooltip";

const MESSAGES = { number: "Enter a number" } as const;

/** The link mark after a field's label (UX notes 12.9.2): `Link` and the group letter muted for a
 * coupled value, `Unlink` at full strength for one this clone holds alone; none otherwise. */
function LinkMark({ row }: { row: SketchInstanceRow }) {
  if (row.group === null || (row.link !== "linked" && row.link !== "unlinked")) {
    return null;
  }
  const unlinked = row.link === "unlinked";
  const Glyph = unlinked ? Unlink : Link;
  return (
    <Tooltip side="left" content={linkMarkTitle(row)}>
      <span
        aria-hidden
        data-link-mark={row.link}
        className="pointer-events-auto inline-flex shrink-0 items-center gap-0.5 text-[11px] font-medium tabular-nums"
        style={{ color: unlinked ? "var(--toolbar-icon)" : "var(--panel-muted-fg)" }}
      >
        <Glyph size={12} strokeWidth={1.5} />
        {row.group}
      </span>
    </Tooltip>
  );
}

/** One value of the sketch: the panel's Value field on the track Rust gives it, muted while it
 * follows the original (`specs/0099-sketch-pages` criterion 24, UX notes 12.2). */
function ValueRow({
  row,
  api,
  cloneName,
  describedBy,
  onReturnFocus,
  following,
  onGesture,
}: {
  row: SketchInstanceRow;
  api: SketchInstanceApi;
  cloneName: string;
  describedBy: string | undefined;
  onReturnFocus: () => void;
  /** A coupled gesture on a partner moves this field: its border follows (UX notes 12.9.3). */
  following: boolean;
  /** A gesture on this field starts (coupled or only this) or ends (`null`). */
  onGesture: (coupled: boolean | null) => void;
}) {
  // The value of the last drag or key tick, committed once when the gesture ends, and whether
  // Shift was held at the gesture's start on a linked value (only this, criterion 34).
  const pending = useRef<number | null>(null);
  const onlyThis = useRef(false);
  const grouped = row.group !== null;
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
      api.setValue(row.key, value, onlyThis.current);
    }
    onlyThis.current = false;
    onGesture(null);
  };
  /** Latches "only this" at the first tick of a gesture: a later Shift changes nothing. */
  const latch = (shift: boolean | undefined) => {
    if (pending.current === null) {
      onlyThis.current = grouped && shift === true;
      onGesture(!onlyThis.current);
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
        valueText={`${text} ${words}${followsWords(row)}${linkWords(row)}`}
        tipValue={`${text}${row.unit === "deg" ? unit : ` ${unit}`}${followsWords(row)}`}
        unit={unit}
        valueNow={current}
        unitWords={words}
        valueMin={field.kind.kind === "length" || field.kind.kind === "angle" ? field.kind.min : undefined}
        valueMax={sliderEnd(field)}
        defaultText={`${shownNumber(row.sketchValue, 4)}${row.unit === "deg" ? unit : ` ${unit}`}`}
        messages={MESSAGES}
        describedBy={describedBy}
        shiftOnlyThis={grouped}
        following={following}
        pageSteps
        labelMark={<LinkMark row={row} />}
        onPreview={(p, grid, shift) => {
          latch(shift);
          const value = valueAt(field, p, grid);
          pending.current = value;
          api.preview(row.key, value, onlyThis.current);
        }}
        onSettle={commit}
        onStep={(steps, grid, shift) => {
          // The canvas follows the key; the key-up is the one commit (the Generator block's rule).
          latch(shift);
          const next = stepValue(field, pending.current ?? current, steps, grid);
          pending.current = next;
          api.preview(row.key, next, onlyThis.current);
        }}
        onReset={() => api.reset(row.key)}
        onCancel={() => {
          pending.current = null;
          onlyThis.current = false;
          onGesture(null);
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
          className="pointer-events-none absolute top-full right-0 z-10 mt-1 w-max max-w-[244px] break-words rounded-[8px] p-2 text-right text-xs"
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
  // The field whose coupled gesture is in flight: its partners' borders follow.
  const [leading, setLeading] = useState<SketchInstanceRow | null>(null);
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
          <TriangleAlert {...SMALL_GLYPH} aria-hidden />
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
          following={
            leading !== null &&
            leading.key !== row.key &&
            leading.group !== null &&
            leading.group === row.group &&
            leading.link === "linked" &&
            row.link === "linked"
          }
          onGesture={(coupled) => setLeading(coupled ? row : null)}
        />
      ))}
    </div>
  );
}
