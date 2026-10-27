import { Tooltip } from "radix-ui";
import { useRef, useState } from "react";
import type { KeyboardEvent, PointerEvent } from "react";

import { useRowStarts } from "@/components/SelectToolbar";
import { BarGroup } from "@/components/TextBar";
import type { TraceApi } from "@/hooks/useTrace";
import { BAR_ROW_CLASS } from "@/lib/barLayout";
import {
  CREATE_PATHS,
  CREATE_PATHS_TOOLTIP,
  STOP,
  TRACE_BUTTON,
  TRACE_EMPTY,
  TRACE_IMPORT,
  TRACE_NOT_IMAGE,
  TRACE_STATE_LABEL,
  TRACE_TOOLTIP,
  type TraceParam,
  parseTyped,
  specksHint,
  stepValue,
  valueText,
} from "@/lib/traceText";

const BUTTON_CLASS =
  "my-1 h-7 whitespace-nowrap rounded-md px-2 text-sm text-[var(--toolbar-icon)] outline-none hover:bg-[var(--editor-accent-hover)] focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)]";

const LABEL_CLASS = "text-xs text-[var(--toolbar-icon)]";

/** The Select bar's Trace button (criterion 1): last, in a group of its own; a press chooses the
 * Trace tool. */
export function TraceButton({ onTrace, onReturnFocus }: { onTrace: () => void; onReturnFocus: () => void }) {
  return (
    <Tooltip.Provider delayDuration={400}>
      <Tooltip.Root>
        <Tooltip.Trigger asChild>
          <button
            type="button"
            onClick={(event) => {
              onTrace();
              if (event.detail > 0) {
                onReturnFocus();
              }
            }}
            className={BUTTON_CLASS}
          >
            {TRACE_BUTTON}
          </button>
        </Tooltip.Trigger>
        <Tooltip.Portal>
          <Tooltip.Content
            side="bottom"
            sideOffset={6}
            className="z-50 rounded-md px-2 py-1 text-xs"
            style={{ background: "var(--toolbar-bg)", color: "var(--toolbar-icon)", boxShadow: "var(--panel-elevation-shadow)" }}
          >
            {TRACE_TOOLTIP}
          </Tooltip.Content>
        </Tooltip.Portal>
      </Tooltip.Root>
    </Tooltip.Provider>
  );
}

interface TraceFieldProps {
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
function TraceField({ param, value, onValue, onEndDrag, onCreate, onReturnFocus }: TraceFieldProps) {
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

/** A field with its muted readout beside it (the Specks pixel hint, criterion 4). */
function WithHint({ hint, children }: { hint?: string; children: React.ReactNode }) {
  return (
    <div className="flex items-center gap-2">
      {children}
      {hint ? <span className={`whitespace-pre ${LABEL_CLASS} opacity-70`}>{hint}</span> : null}
    </div>
  );
}

interface TraceBarProps {
  trace: TraceApi;
  onImport: () => void;
  onReturnFocus: () => void;
}

/**
 * The Trace bar in the overlay row (`specs/0025-raster-trace` criteria 1 to 5, 10 to 12, 15, 46;
 * UX notes sections 2 and 3): the state label, Threshold, Smooth, Specks, then the result row
 * with the status and Create paths; wraps by whole groups, the first row never moving. With
 * nothing or something else selected it is one row of words.
 */
export function TraceBar({ trace, onImport, onReturnFocus }: TraceBarProps) {
  const rowRef = useRef<HTMLDivElement>(null);
  useRowStarts(rowRef);
  const param = (name: TraceParam["name"]) => trace.params.find((p) => p.name === name)!;
  const pill = "pointer-events-auto flex min-w-0 flex-col rounded-lg px-2 text-sm";
  const pillStyle = { background: "var(--toolbar-bg)", boxShadow: "var(--panel-elevation-shadow)" };
  const label = (
    <span className={`${LABEL_CLASS} py-2 pr-2`} aria-hidden={false}>
      {TRACE_STATE_LABEL}
    </span>
  );
  if (trace.selection !== "images") {
    return (
      <div data-context-bar="" className={pill} style={pillStyle} role="toolbar" aria-label="Trace">
        <div className="flex h-9 items-center gap-3">
          {label}
          <span className="text-[var(--toolbar-icon)]">{trace.selection === "empty" ? TRACE_EMPTY : TRACE_NOT_IMAGE}</span>
          {trace.selection === "empty" ? (
            <button type="button" className={BUTTON_CLASS} onClick={onImport}>
              {TRACE_IMPORT}
            </button>
          ) : null}
        </div>
      </div>
    );
  }
  const field = (name: TraceParam["name"]) => (
    <TraceField
      param={param(name)}
      value={trace.values[name]}
      onValue={(value, dragging) => trace.setValue(name, value, dragging)}
      onEndDrag={trace.endDrag}
      onCreate={trace.create}
      onReturnFocus={onReturnFocus}
    />
  );
  const canCreate = trace.status.canCreate || trace.status.line === "";
  return (
    <div
      data-context-bar=""
      className={pill}
      style={pillStyle}
      role="toolbar"
      aria-label="Trace"
      aria-busy={trace.busy || trace.creating}
    >
      <div ref={rowRef} className={BAR_ROW_CLASS}>
        <BarGroup divider={false}>
          {label}
          {field("threshold")}
        </BarGroup>
        <BarGroup>{field("smooth")}</BarGroup>
        <BarGroup>
          <WithHint hint={trace.pxPerMm > 0 ? specksHint(trace.values.specks, trace.pxPerMm) : undefined}>
            {field("specks")}
          </WithHint>
        </BarGroup>
        <BarGroup className="flex min-h-9 items-center gap-3">
          <div className="flex flex-col py-1 tabular-nums">
            {trace.imagesLabel ? <span className={LABEL_CLASS}>{trace.imagesLabel}</span> : null}
            <span role="status" aria-live="polite" className="text-[var(--toolbar-icon)]">
              {trace.status.line}
            </span>
            {trace.status.muted ? <span className={`${LABEL_CLASS} opacity-70`}>{trace.status.muted}</span> : null}
          </div>
          {trace.stopShown ? (
            <button type="button" className={BUTTON_CLASS} onClick={trace.stop}>
              {STOP}
            </button>
          ) : null}
          <button
            type="button"
            title={CREATE_PATHS_TOOLTIP}
            aria-disabled={!canCreate}
            aria-busy={trace.creating}
            onClick={() => {
              if (canCreate) {
                trace.create();
              }
            }}
            className={`my-1 h-7 w-[112px] whitespace-nowrap rounded-md px-2 text-sm text-white outline-none focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)] focus-visible:ring-offset-1 ${canCreate ? "" : "opacity-40"}`}
            style={{ background: "var(--editor-accent)" }}
          >
            {CREATE_PATHS}
          </button>
        </BarGroup>
      </div>
      {trace.busy || trace.creating ? (
        <div role="progressbar" aria-label={trace.status.line} className="h-0.5 w-full overflow-hidden">
          <div className="h-full w-full" style={{ background: "var(--editor-accent)" }} />
        </div>
      ) : null}
    </div>
  );
}
