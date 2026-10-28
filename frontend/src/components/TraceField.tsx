import { useRef, useState } from "react";
import type { KeyboardEvent, PointerEvent } from "react";

import { type TraceParam, parseTyped, stepValue, valueText } from "@/lib/traceText";

export interface TraceFieldProps {
  param: TraceParam;
  value: number | null;
  onValue: (value: number, dragging: boolean) => void;
  onEndDrag: () => void;
  onCreate: () => void;
  onReturnFocus: () => void;
}

/** One value field of the bar (criterion 5): the label inside, the `--value-fill` bar, a drag
 * across it sets the value (the reduced copy is traced while it moves), the keys step it, a typed
 * value commits on Enter and the focus returns to the canvas; Ctrl+Enter creates the paths. */
export function TraceField({ param, value, onValue, onEndDrag, onCreate, onReturnFocus }: TraceFieldProps) {
  const [draft, setDraft] = useState<string | null>(null);
  const trackRef = useRef<HTMLDivElement>(null);
  const shown = value ?? param.default;
  const share = value === null ? 0 : (shown - param.min) / (param.max - param.min);
  const fromPointer = (event: PointerEvent<HTMLDivElement>) => {
    const box = trackRef.current?.getBoundingClientRect();
    if (!box || box.width <= 0) {
      return shown;
    }
    const p = Math.min(1, Math.max(0, (event.clientX - box.left) / box.width));
    return param.min + p * (param.max - param.min);
  };
  const keyDown = (event: KeyboardEvent<HTMLInputElement>) => {
    if (event.key === "Escape") {
      event.preventDefault();
      event.stopPropagation();
      setDraft(null);
      onReturnFocus();
      return;
    }
    if (event.key === "Enter") {
      event.preventDefault();
      if (event.ctrlKey || event.metaKey) {
        onCreate();
        return;
      }
      const typed = draft === null ? null : parseTyped(param, draft);
      if (typed !== null) {
        onValue(typed, false);
      }
      setDraft(null);
      onReturnFocus();
      return;
    }
    const stepped = stepValue(param, shown, event);
    if (stepped !== null) {
      event.preventDefault();
      setDraft(null);
      onValue(stepped, false);
    }
  };
  const label = param.unit ? `${param.label} (${param.unit})` : param.label;
  return (
    <div
      ref={trackRef}
      className="relative my-1 flex h-7 w-[168px] items-center overflow-hidden rounded-md text-sm"
      style={{ background: "color-mix(in srgb, var(--toolbar-icon) 8%, transparent)" }}
      onPointerDown={(event) => {
        if (event.target instanceof HTMLInputElement) {
          return;
        }
        event.currentTarget.setPointerCapture(event.pointerId);
        onValue(Number.parseFloat(valueText(param, fromPointer(event))), true);
      }}
      onPointerMove={(event) => {
        if (event.currentTarget.hasPointerCapture(event.pointerId)) {
          onValue(Number.parseFloat(valueText(param, fromPointer(event))), true);
        }
      }}
      onPointerUp={(event) => {
        if (event.currentTarget.hasPointerCapture(event.pointerId)) {
          event.currentTarget.releasePointerCapture(event.pointerId);
          onEndDrag();
        }
      }}
    >
      <span
        aria-hidden
        className="pointer-events-none absolute inset-y-0 left-0"
        style={{ width: `${share * 100}%`, background: "var(--value-fill)" }}
      />
      <span className="relative whitespace-nowrap pl-2 text-[var(--toolbar-icon)]">{param.label}</span>
      <input
        aria-label={label}
        inputMode="decimal"
        value={draft ?? (value === null ? "" : valueText(param, shown))}
        onChange={(event) => setDraft(event.target.value)}
        onBlur={() => setDraft(null)}
        onKeyDown={keyDown}
        className="relative ml-auto w-12 bg-transparent pr-2 text-right tabular-nums text-[var(--toolbar-icon)] outline-none focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)]"
      />
    </div>
  );
}

