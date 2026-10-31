import { Binary, CircleAlert, Image, Info, Layers, Spline, X } from "lucide-react";
import { Tooltip } from "radix-ui";
import { useId, useRef } from "react";
import type { KeyboardEvent } from "react";

import { useRowStarts } from "@/components/SelectToolbar";
import { ExpandButton, ICON_BUTTON_CLASS } from "@/components/TextBar";
import { TraceAdvanced } from "@/components/TraceAdvanced";
import { TraceField } from "@/components/TraceField";
import { PickColoursGroup, RemoveBackgroundGroup } from "@/components/TraceColourGroup";
import { TracePresetMenu } from "@/components/TracePresetMenu";
import { Tooltip as AppTooltip } from "@/components/ui/tooltip";
import type { TraceApi } from "@/hooks/useTrace";
import type { TraceInfoApi } from "@/hooks/useTraceInfo";
import { analysisLine } from "@/lib/traceAnalysis";
import {
  TRACE_FINISH,
  TRACE_METHODS,
  PICK_COLOURS,
  REMOVE_BACKGROUND,
  colourTolerance,
  FAILED,
  PREVIEW_OUT_OF_DATE,
  RETRY,
  STOPPED,
  TRACING,
  type TraceSlot,
  percentText,
  TRACE_BAR_DESCRIPTION,
  TRACE_EXPAND_LABELS,
  TRACE_INFO_PAGE_ID,
  TRACE_INFO_TITLE,
  changedAdvancedText,
  lumaToPercent,
  percentToLuma,
  thresholdPercentParam,
  TRACE_VIEWS,
  type TraceViewMode,
  STOP,
  TRACE_BUTTON,
  TRACE_EMPTY,
  TRACE_IMPORT,
  TRACE_NOT_IMAGE,
  TRACE_STATE_LABEL,
  TRACE_TOOLTIP,
  type TraceParam,
  specksHint,
} from "@/lib/traceText";

const BUTTON_CLASS =
  "my-1 h-7 whitespace-nowrap rounded-md px-2 text-sm text-[var(--toolbar-icon)] outline-none hover:bg-[var(--editor-accent-hover)] focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)]";

const LABEL_CLASS = "text-xs text-[var(--toolbar-icon)]";

/** The bar's one flow (UX notes 21.2): wraps by whole items, 8 px between them, top-aligned. */
const TRACE_ROW_CLASS = "flex min-w-0 flex-wrap items-start gap-x-2 gap-y-0";

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

const VIEW_ICONS: Record<TraceViewMode, typeof Image> = { overlay: Layers, mask: Binary, trace: Spline, image: Image };

/** The View pill (criterion 51, UX notes section 13; `specs/0089` criterion 21): four icon
 * segments, Overlay, Mask, Trace, Image, one Tab stop, the arrows move and select; the pressed
 * segment is the view on the canvas, Image while `\` is held. */
function ViewPill({ mode, onMode }: { mode: TraceViewMode; onMode: (mode: TraceViewMode) => void }) {
  const buttons = useRef<(HTMLButtonElement | null)[]>([]);
  const at = Math.max(0, TRACE_VIEWS.findIndex((v) => v.mode === mode));
  const keyDown = (event: KeyboardEvent<HTMLDivElement>) => {
    const step = event.key === "ArrowRight" || event.key === "ArrowDown" ? 1 : event.key === "ArrowLeft" || event.key === "ArrowUp" ? -1 : 0;
    if (step !== 0) {
      event.preventDefault();
      const next = (at + step + TRACE_VIEWS.length) % TRACE_VIEWS.length;
      onMode(TRACE_VIEWS[next].mode);
      buttons.current[next]?.focus();
    }
  };
  return (
    <Tooltip.Provider delayDuration={400}>
      <div role="radiogroup" aria-label="View" className="my-1 flex h-7 overflow-hidden rounded-md outline outline-1 -outline-offset-1 outline-[var(--toolbar-icon)]/40" onKeyDown={keyDown}>
        {TRACE_VIEWS.map((view, index) => {
          const Icon = VIEW_ICONS[view.mode];
          const on = index === at;
          return (
            <Tooltip.Root key={view.mode}>
              <Tooltip.Trigger asChild>
                <button
                  ref={(element) => {
                    buttons.current[index] = element;
                  }}
                  type="button"
                  role="radio"
                  aria-checked={on}
                  aria-label={view.label}
                  tabIndex={on ? 0 : -1}
                  onClick={() => onMode(view.mode)}
                  className={`flex h-full w-7 items-center justify-center outline-none focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-[var(--editor-accent)] ${
                    on ? "bg-[var(--toolbar-icon-active-bg)] text-[var(--toolbar-icon-active-fg)]" : "text-[var(--toolbar-icon)] hover:bg-[var(--editor-accent-hover)]"
                  }`}
                >
                  <Icon size={16} aria-hidden />
                </button>
              </Tooltip.Trigger>
              <Tooltip.Portal>
                <Tooltip.Content
                  side="bottom"
                  sideOffset={6}
                  className="z-50 rounded-md px-2 py-1 text-xs"
                  style={{ background: "var(--toolbar-bg)", color: "var(--toolbar-icon)", boxShadow: "var(--panel-elevation-shadow)" }}
                >
                  {view.tooltip}
                </Tooltip.Content>
              </Tooltip.Portal>
            </Tooltip.Root>
          );
        })}
      </div>
    </Tooltip.Provider>
  );
}

/** The finish button's blue ground and shape (customer 2026-10-30): the same in every state,
 * 75 px at rest and while running; Stopped and Failed widen it for their Retry. */
export const FINISH_CLASS =
  "relative my-1 flex h-7 min-w-[75px] shrink-0 items-center overflow-hidden rounded-md bg-[var(--editor-accent)] text-sm text-white";

/** The finish button at the far right of the bar (criteria 10, 53, 54; `specs/0089` Frage 84,
 * customer 2026-10-30): always blue, "Trace" at rest (the same as Enter); while a job runs (shown
 * after 400 ms, held at least 300 ms) its percentage and an x inside the same button; "Stopped"
 * or "Failed" with Retry, the reason as tooltip and hidden alert. */
function ProgressSlot({
  slot,
  onStop,
  onRetry,
  onCreate,
  blocked,
}: {
  slot: TraceSlot;
  onStop: () => void;
  onRetry: () => void;
  onCreate: () => void;
  /** Why Trace cannot run, or empty (`ux-notes.md` §22.2). */
  blocked: string;
}) {
  const reasonId = useId();
  if (slot.kind === "none") {
    const button = (
      <button
        type="button"
        data-trace-slot="none"
        aria-disabled={blocked !== "" || undefined}
        aria-describedby={blocked !== "" ? reasonId : undefined}
        className={`${FINISH_CLASS} justify-center px-3 ${blocked !== "" ? "cursor-default opacity-50" : ""} outline-none hover:brightness-110 focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)] focus-visible:ring-offset-1 focus-visible:ring-offset-[var(--toolbar-bg)]`}
        onClick={() => {
          if (blocked === "") {
            onCreate();
          }
        }}
      >
        {TRACE_FINISH}
      </button>
    );
    return blocked === "" ? (
      button
    ) : (
      <>
        <AppTooltip side="bottom" content={blocked}>
          {button}
        </AppTooltip>
        <span id={reasonId} hidden>
          {blocked}
        </span>
      </>
    );
  }
  if (slot.kind === "running") {
    const fill = slot.percent ?? 0;
    return (
      <div data-trace-slot="running" className={`${FINISH_CLASS} w-[75px]`}>
        <div
          role="progressbar"
          aria-label={TRACING}
          aria-valuemin={0}
          aria-valuemax={100}
          aria-valuenow={slot.percent ?? undefined}
          className="flex h-full min-w-0 flex-1 items-center"
        >
          <span aria-hidden className="absolute inset-y-0 left-0" style={{ width: `${fill}%`, background: "color-mix(in srgb, white 25%, transparent)" }} />
          <span className="relative pl-2 text-xs tabular-nums">{percentText(slot.percent)}</span>
        </div>
        <AppTooltip side="bottom" content={STOP}>
          <button type="button" aria-label={STOP} onClick={onStop} className={`${ICON_BUTTON_CLASS} relative text-white hover:bg-white/20`}>
            <X size={16} strokeWidth={1.5} aria-hidden />
          </button>
        </AppTooltip>
      </div>
    );
  }
  const failed = slot.kind === "failed";
  return (
    <div data-trace-slot={slot.kind} className={`${FINISH_CLASS} gap-1 pl-2`}>
      <AppTooltip side="bottom" content={failed ? slot.text : PREVIEW_OUT_OF_DATE}>
        <span
          className={`flex h-7 items-center gap-1 rounded-md text-xs ${failed ? "" : "opacity-90"}`}
          style={failed ? { boxShadow: "inset 0 0 0 1px var(--field-invalid)" } : undefined}
        >
          {failed ? <CircleAlert size={12} aria-hidden className="shrink-0" /> : null}
          {failed ? FAILED : STOPPED}
        </span>
      </AppTooltip>
      {failed ? (
        <span role="alert" className="sr-only">
          {slot.text}
        </span>
      ) : null}
      <button type="button" className="h-7 rounded-md px-2 text-sm text-white outline-none hover:bg-white/20 focus-visible:ring-2 focus-visible:ring-white" onClick={onRetry}>
        {RETRY}
      </button>
    </div>
  );
}

/** A field with its muted readout beside it (the Specks pixel hint, criterion 4). */
function WithHint({ hint, children }: { hint?: string; children: React.ReactNode }) {
  return (
    <div className="flex items-center gap-2">
      {children}
      {/* The readout's room is kept while the pixel size is unknown, so the rows do not jump when it appears. */}
      <span className={`w-9 shrink-0 whitespace-pre ${LABEL_CLASS} opacity-70`}>{hint ?? ""}</span>
    </div>
  );
}

/** The "i" button (criterion 70, UX notes 21.1): shows the About Trace tab, or returns to the tab
 * and panel state before it; pressed while that tab is shown. A mouse press hands the keyboard
 * back to the canvas, a key keeps it. */
function InfoButton({ info, onReturnFocus }: { info: TraceInfoApi; onReturnFocus: () => void }) {
  return (
    <AppTooltip side="bottom" content={TRACE_INFO_TITLE}>
      <button
        type="button"
        aria-label={TRACE_INFO_TITLE}
        aria-pressed={info.shown}
        aria-controls={info.shown ? TRACE_INFO_PAGE_ID : undefined}
        onClick={(event) => {
          info.toggle();
          if (event.detail > 0) {
            onReturnFocus();
          }
        }}
        className={`${ICON_BUTTON_CLASS} ${
          info.shown ? "bg-[var(--toolbar-icon-active-bg)] text-[var(--toolbar-icon-active-fg)]" : "text-[var(--toolbar-icon)] hover:bg-[var(--editor-accent-hover)]"
        }`}
      >
        <Info size={16} strokeWidth={1.5} aria-hidden />
      </button>
    </AppTooltip>
  );
}

interface TraceBarProps {
  trace: TraceApi;
  info: TraceInfoApi;
  onImport: () => void;
  onReturnFocus: () => void;
}

/**
 * The Trace bar in the overlay row (`specs/0025-raster-trace` criteria 1 to 5, 10 to 12, 15, 37,
 * 40, 41, 46, 51 to 54, 57, 71; UX notes section 21.2; `specs/0089` criterion 14, Frage 84, 85 B):
 * one flow of the "i" button, Preset, View, Method, Threshold or Colour tolerance, the method's
 * pipette group, Smooth, Tolerance, Ignore below, and at the far right the expander and the
 * Trace button, wrapping by whole items; under it the notice row and, when open, the Advanced
 * block that holds everything else (`ux-notes.md` §22).
 * With nothing or something else selected it is one row of words.
 */
export function TraceBar({ trace, info, onImport, onReturnFocus }: TraceBarProps) {
  const rowRef = useRef<HTMLDivElement>(null);
  useRowStarts(rowRef);
  const advancedId = useId();
  const descriptionId = useId();
  const param = (name: TraceParam["name"]) => trace.params.find((p) => p.name === name)!;
  const pill = "pointer-events-auto flex min-w-0 flex-col rounded-lg px-2 text-sm";
  const pillStyle = { background: "var(--toolbar-bg)", boxShadow: "var(--panel-elevation-shadow)" };
  const label = (
    <span className={`${LABEL_CLASS} py-2`}>
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
  const field = (name: "smooth" | "tolerance" | "specks") => (
    <TraceField
      param={param(name)}
      value={trace.values[name]}
      onValue={(value, dragging) => trace.setValue(name, value, dragging)}
      onEndDrag={trace.endDrag}
      onCreate={trace.create}
      onReturnFocus={onReturnFocus}
    />
  );
  // The Threshold is shown and typed in percent; the tracer keeps the exact luma until an edit
  // (criterion 9, Terms).
  const threshold = trace.values.threshold;
  const thresholdField = (
    <TraceField
      param={thresholdPercentParam(param("threshold"))}
      value={threshold === null ? null : lumaToPercent(threshold)}
      onValue={(percent, dragging) => trace.setValue("threshold", percentToLuma(percent), dragging)}
      onEndDrag={trace.endDrag}
      onCreate={trace.create}
      onReturnFocus={onReturnFocus}
    />
  );
  // The method's field takes the Threshold's slot (`specs/0089` criterion 14, UX notes §5).
  const method = trace.values.method;
  const methodField =
    method === REMOVE_BACKGROUND || method === PICK_COLOURS ? (
      <TraceField
        param={param("colour_tolerance")}
        value={colourTolerance(trace.values)}
        onValue={(value, dragging) => trace.setColourTolerance(value, dragging)}
        onEndDrag={trace.endDrag}
        onCreate={trace.create}
        onReturnFocus={onReturnFocus}
      />
    ) : (
      thresholdField
    );
  const colourGroup =
    method === REMOVE_BACKGROUND ? (
      <RemoveBackgroundGroup
        background={trace.values.backgroundAuto ? trace.suggestedBackground : trace.values.background}
        auto={trace.values.backgroundAuto}
        armed={trace.pickMode === "background"}
        onAuto={trace.setBackgroundAuto}
        onPipette={() => trace.setPicking(trace.pickMode === "")}
        onReturnFocus={onReturnFocus}
      />
    ) : method === PICK_COLOURS ? (
      <PickColoursGroup
        count={trace.values.swatches.length}
        armed={trace.pickMode === "colours"}
        onPipette={() => trace.setPicking(trace.pickMode === "")}
        onReturnFocus={onReturnFocus}
      />
    ) : null;
  const methodLabel = TRACE_METHODS[method]?.label ?? TRACE_METHODS[0].label;
  const noticeShown = trace.status.line !== "" || trace.status.muted !== "" || trace.imagesLabel !== "" || trace.palette.sameNote !== "";
  return (
    <div className="pointer-events-none flex min-w-0 flex-col gap-2">
    <div
      data-context-bar=""
      className={pill}
      style={pillStyle}
      role="toolbar"
      aria-label="Trace"
      aria-describedby={descriptionId}
    >
      <span id={descriptionId} hidden>
        {TRACE_BAR_DESCRIPTION}
      </span>
      <div ref={rowRef} className={TRACE_ROW_CLASS}>
        {/* No "Trace" label at the start (Frage 84); the "i" stays. */}
        <span className="flex h-9 items-center">
          <InfoButton info={info} onReturnFocus={onReturnFocus} />
        </span>
        <div className="flex h-9 items-center">
          <TracePresetMenu
            presets={trace.presets}
            current={trace.choice.preset}
            edited={trace.choice.edited}
            label={trace.presetLabel}
            onPick={trace.pick}
          />
        </div>
        <ViewPill mode={trace.viewMode} onMode={trace.setViewMode} />
        <div className="flex h-9 items-center">
          <TracePresetMenu
            name="Method"
            widthClass="w-[176px]"
            presets={TRACE_METHODS}
            current={TRACE_METHODS[method]?.key ?? TRACE_METHODS[0].key}
            edited={false}
            label={methodLabel}
            onPick={(choice) => trace.setMethod(TRACE_METHODS.indexOf(choice))}
          />
        </div>
        <div className="flex h-9 items-center">{methodField}</div>
        {colourGroup}
        <div className="flex h-9 items-center">{field("smooth")}</div>
        <div className="flex h-9 items-center">{field("tolerance")}</div>
        <div className="flex h-9 items-center gap-2">
          <WithHint hint={trace.pxPerMm > 0 ? specksHint(trace.values.specks, trace.pxPerMm) : undefined}>
            {field("specks")}
          </WithHint>
        </div>
        <div className="ml-auto flex h-9 items-center gap-2">
          <ExpandButton
            open={trace.advancedOpen}
            onToggle={() => trace.setAdvancedOpen(!trace.advancedOpen)}
            controls={advancedId}
            labels={TRACE_EXPAND_LABELS}
            changed={trace.advancedOpen ? null : changedAdvancedText(trace.values, trace.params)}
            onReturnFocus={onReturnFocus}
          />
          <ProgressSlot slot={trace.slot} onStop={trace.stop} onRetry={trace.retrace} onCreate={trace.create} blocked={trace.traceBlocked} />
        </div>
        {trace.advancedOpen ? (
          // After the expander in the DOM, so Tab reaches the Advanced controls after it
          // (criterion 57, UX notes 21.2); drawn below the notice row, full width.
          <div className="order-last basis-full">
            <TraceAdvanced
              id={advancedId}
              params={trace.params}
              values={trace.values}
              status={trace.advancedStatus}
              analysis={trace.analysis}
              onAnalyse={trace.analyse}
              palette={trace.palette}
              hsvOf={trace.hsvOf}
              rgbOf={trace.rgbOf}
              countsStale={trace.countsStale}
              setValue={trace.setValue}
              onEndDrag={trace.endDrag}
              onCreate={trace.create}
              onReturnFocus={onReturnFocus}
              coloursFound={trace.palette.found}
              paletteArmed={trace.pickMode === "colours" && trace.values.method !== PICK_COLOURS}
              onPalettePipette={() => trace.setPicking(trace.pickMode === "")}
            />
          </div>
        ) : null}
        {/* The notice row (`ux-notes.md` §22.2): only what is fixed with a bar control or changes how
            the preview reads, always mounted with its live region, no height while empty; drawn
            under the main rows and above the Advanced block. */}
        <div data-trace-state="" className={`order-[9998] basis-full ${noticeShown ? "flex min-h-9 items-center gap-3" : ""}`}>
          <div role="status" aria-live="polite" className={noticeShown ? "flex flex-col py-1 tabular-nums" : ""}>
            {trace.imagesLabel ? <span className={`block ${LABEL_CLASS}`}>{trace.imagesLabel}</span> : null}
            {trace.status.line ? (
              <span data-trace-line="" className="block text-[var(--toolbar-icon)]">
                {trace.status.line}
              </span>
            ) : null}
            {trace.status.muted ? (
              <span data-trace-muted="" className={`block ${LABEL_CLASS} opacity-70`}>
                {trace.status.muted}
              </span>
            ) : null}
            {trace.palette.sameNote ? <span className={`block ${LABEL_CLASS}`}>{trace.palette.sameNote}</span> : null}
          </div>
        </div>
      </div>
      {trace.advancedOpen ? null : (
        // With Advanced closed the analysis and status lines are announced here (`ux-notes.md` §22.2).
        <div data-trace-hidden-status="" role="status" aria-live="polite" className="sr-only">
          {[analysisLine(trace.analysis), trace.advancedStatus.text, trace.advancedStatus.muted].filter((t) => t !== "").join(". ")}
        </div>
      )}
    </div>
    </div>
  );
}
