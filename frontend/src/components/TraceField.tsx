import { useRef } from "react";

import { ValueField } from "@/components/panel/ValueField";
import { HELP_RATIO } from "@/lib/sliderNames";
import { type TraceParam, parseTyped, stepOnGrid, valueAt, valueText } from "@/lib/traceText";

export interface TraceFieldProps {
  /** The field as the bar shows it (the Threshold in percent). */
  param: TraceParam;
  value: number | null;
  onValue: (value: number, dragging: boolean) => void;
  onEndDrag: () => void;
  onCreate: () => void;
  onReturnFocus: () => void;
}

/** The width of a Trace bar field, px (UX notes 20.1). */
const TRACE_FIELD_WIDTH = 168;

const UNIT_WORDS: Readonly<Record<string, string>> = { "%": "percent", mm: "millimetres" };

/**
 * One value field of the Trace bar (`specs/0025-raster-trace` criteria 5 and 71, UX notes 20.1):
 * an adapter from the schema's `TraceParam` to the app's shared `ValueField`, 168 px, no reset
 * slot. A click or a drag moves the value (the reduced copy is traced while it moves), a
 * double-click, Enter or F2 opens the number entry, the arrows step it; Ctrl+Enter creates the
 * paths, also in the entry; Escape in a drag puts the value back.
 */
export function TraceField({ param, value, onValue, onEndDrag, onCreate, onReturnFocus }: TraceFieldProps) {
  const shown = value ?? param.default;
  const pressing = useRef<{ start: number | null; last: number } | null>(null);
  const span = param.max - param.min;
  const text = value === null ? "" : valueText(param, shown);
  const unitWords = UNIT_WORDS[param.unit] ?? "";
  const set = (next: number, dragging: boolean) => {
    if (next !== (pressing.current?.last ?? value)) {
      if (pressing.current) {
        pressing.current.last = next;
      }
      onValue(next, dragging);
    }
  };
  const release = () => {
    if (pressing.current) {
      pressing.current = null;
      onEndDrag();
    }
  };
  return (
    <div
      className="my-1"
      onPointerDownCapture={(event) => {
        if (event.button === 0) {
          pressing.current = { start: value, last: shown };
        }
      }}
      onKeyDownCapture={(event) => {
        if (event.key === "Enter" && (event.ctrlKey || event.metaKey)) {
          event.preventDefault();
          event.stopPropagation();
          onCreate();
        }
      }}
    >
      <ValueField
        label={param.label}
        name={param.label}
        text={text}
        bar={value === null || span <= 0 ? 0 : (shown - param.min) / span}
        mixed={false}
        resettable={false}
        noResetSlot
        width={TRACE_FIELD_WIDTH}
        help={HELP_RATIO}
        unit={param.unit}
        valueNow={shown}
        valueMin={param.min}
        valueMax={param.max}
        unitWords={unitWords}
        valueText={text === "" ? "" : `${text}${unitWords ? ` ${unitWords}` : ""}`}
        tipValue={text === "" ? "" : `${text}${param.unit ? ` ${param.unit}` : ""}`}
        defaultText={valueText(param, param.default)}
        messages={{}}
        onPreview={(p, grid) => {
          // A press or a drag traces the reduced copy; Home and End are a plain step.
          set(valueAt(param, p, grid), pressing.current !== null);
        }}
        onSettle={release}
        onStep={(steps, grid) => set(stepOnGrid(param, shown, steps, grid), false)}
        onReset={() => undefined}
        onSubmit={(typed) => {
          const next = parseTyped(param, typed);
          if (next === null || next === value) {
            return "unchanged";
          }
          onValue(next, false);
          return "committed";
        }}
        onCancel={() => {
          const start = pressing.current?.start ?? null;
          pressing.current = null;
          if (start !== null) {
            onValue(start, false);
          }
        }}
        onReturnFocus={() => {
          release();
          onReturnFocus();
        }}
      />
    </div>
  );
}
