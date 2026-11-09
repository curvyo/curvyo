import { CircleAlert, Info, SquareDashedMousePointer, X } from "lucide-react";
import { Tooltip } from "radix-ui";
import { useEffect, useId, useRef, useState } from "react";
import type { KeyboardEvent } from "react";

import { useRowStarts } from "@/components/SelectToolbar";
import { ExpandButton, ICON_BUTTON_CLASS } from "@/components/TextBar";
import { TraceAdvanced } from "@/components/TraceAdvanced";
import { TraceParts } from "@/components/TraceParts";
import { joinTooltip, partsCount, partsLabel } from "@/lib/traceParts";
import { TraceField } from "@/components/TraceField";
import { PickColoursGroup, RemoveBackgroundGroup } from "@/components/TraceColourGroup";
import { TracePresetMenu } from "@/components/TracePresetMenu";
import { Tooltip as AppTooltip } from "@/components/ui/tooltip";
import type { TraceApi } from "@/hooks/useTrace";
import type { TraceInfoApi } from "@/hooks/useTraceInfo";
import { ANALYSIS_FAILED } from "@/lib/traceAnalysis";
import {
  CLEAR_MASK,
  maskPickStatus,
  PICK_MASK_PATHS,
  TRACE_MIXED,
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
  STOP,
  TRACE_BUTTON,
  TRACE_EMPTY,
  TRACE_IMPORT,
  TRACE_NOT_IMAGE,
  TRACE_STATE_LABEL,
  TRACE_TOOLTIP,
  type TraceParam,
} from "@/lib/traceText";

const BUTTON_CLASS =
  "my-1 h-7 whitespace-nowrap rounded-md px-2 text-sm text-[var(--toolbar-icon)] outline-none hover:bg-[var(--editor-accent-hover)] focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)]";

const LABEL_CLASS = "text-xs text-[var(--toolbar-icon)]";

/** Ignore below is wider than the other value fields, px: "99.99 mm" keeps a 10 px gap to its
 * label (`ux-notes.md` §24.7, lead 2026-11-06). */
const SPECKS_FIELD_WIDTH = 171;

/** The bar's one flow (UX notes 21.2): wraps by whole items, 8 px between them, top-aligned. */
const TRACE_ROW_CLASS = "flex min-w-0 flex-wrap items-start gap-x-2 gap-y-0";

/** A full-width row under the controls whose text never widens the bar (customer 2026-10-31): its
 * intrinsic width is zero, so the bar is as wide as its controls and the row wraps inside it. */
const FOLLOWS_WIDTH_CLASS = "[contain:inline-size]";

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

/** The Inside / Outside segments of the Mask group (`specs/0094` UX notes §1) in the segmented
 * look of the bar's pills; arrows move and select, no wrap. Selectable with no mask path
 * (`specs/0089` UX notes §24.4). */
function MaskPill({ outside, onOutside }: { outside: boolean; onOutside: (outside: boolean) => void }) {
  const buttons = useRef<(HTMLButtonElement | null)[]>([]);
  const keyDown = (event: KeyboardEvent<HTMLDivElement>) => {
    const step = event.key === "ArrowRight" || event.key === "ArrowDown" ? 1 : event.key === "ArrowLeft" || event.key === "ArrowUp" ? -1 : 0;
    const next = Math.min(1, Math.max(0, (outside ? 1 : 0) + step));
    if (step !== 0) {
      event.preventDefault();
      onOutside(next === 1);
      buttons.current[next]?.focus();
    }
  };
  return (
    <div role="radiogroup" aria-label="Mask" className="my-1 flex h-7 overflow-hidden rounded-md outline outline-1 -outline-offset-1 outline-[var(--toolbar-icon)]/40" onKeyDown={keyDown}>
      {(["Inside", "Outside"] as const).map((label, index) => {
        const on = (index === 1) === outside;
        return (
          <button
            key={label}
            ref={(element) => {
              buttons.current[index] = element;
            }}
            type="button"
            role="radio"
            aria-checked={on}
            tabIndex={on ? 0 : -1}
            onClick={() => onOutside(index === 1)}
            className={`flex h-full w-16 items-center justify-center text-sm outline-none focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-[var(--editor-accent)] ${
              on ? "bg-[var(--toolbar-icon-active-bg)] text-[var(--toolbar-icon-active-fg)]" : "text-[var(--toolbar-icon)] hover:bg-[var(--editor-accent-hover)]"
            }`}
          >
            {label}
          </button>
        );
      })}
    </div>
  );
}

/** The Mask group (`specs/0094` UX notes §1, §11; `specs/0089` UX notes §24.4): one flow item
 * after Preset, always drawn, never dimmed: the Pick mask paths toggle, the Inside / Outside
 * segments and Clear mask, which does nothing without mask paths. A mouse press hands the
 * keyboard back to the canvas, a key keeps it; Escape on the armed toggle disarms and returns to
 * the canvas. The mode is announced politely, never shown as text. */
function MaskGroup({ trace, onReturnFocus }: { trace: TraceApi; onReturnFocus: () => void }) {
  const [said, setSaid] = useState("");
  const armed = trace.maskPick;
  const first = useRef(true);
  useEffect(() => {
    if (first.current) {
      first.current = false;
      return;
    }
    setSaid(maskPickStatus(armed));
  }, [armed]);
  const pressed = armed ? "bg-[var(--toolbar-icon-active-bg)] text-[var(--toolbar-icon-active-fg)]" : "text-[var(--toolbar-icon)] hover:bg-[var(--editor-accent-hover)]";
  return (
    <div data-trace-mask="" className="flex h-9 items-center gap-1">
      <AppTooltip side="bottom" content={PICK_MASK_PATHS}>
        <button
          type="button"
          aria-label={PICK_MASK_PATHS}
          aria-pressed={armed}
          data-trace-mask-pick=""
          onClick={(event) => {
            trace.setMaskPick(!armed);
            if (event.detail > 0) {
              onReturnFocus();
            }
          }}
          onKeyDown={(event) => {
            if (event.key === "Escape" && armed) {
              event.preventDefault();
              event.stopPropagation();
              trace.setMaskPick(false);
              onReturnFocus();
            }
          }}
          className={`${ICON_BUTTON_CLASS} ${pressed}`}
        >
          <SquareDashedMousePointer size={16} strokeWidth={1.5} aria-hidden />
        </button>
      </AppTooltip>
      <MaskPill outside={trace.mask.outside} onOutside={trace.setMaskOutside} />
      <AppTooltip side="bottom" content={CLEAR_MASK}>
        <button
          type="button"
          aria-label={CLEAR_MASK}
          data-trace-clear-mask=""
          onClick={(event) => {
            // Nothing to clear: no change, focus stays (`specs/0089` UX notes §24.4).
            if (trace.mask.paths === 0) {
              return;
            }
            trace.clearMask();
            if (event.detail > 0) {
              onReturnFocus();
            }
          }}
          className={`${ICON_BUTTON_CLASS} text-[var(--toolbar-icon)] hover:bg-[var(--editor-accent-hover)]`}
        >
          <X size={16} strokeWidth={1.5} aria-hidden />
        </button>
      </AppTooltip>
      <span role="status" aria-live="polite" className="sr-only">
        {said}
      </span>
    </div>
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
  tooltip = "",
}: {
  slot: TraceSlot;
  onStop: () => void;
  onRetry: () => void;
  onCreate: () => void;
  /** Why Trace cannot run, or empty (`ux-notes.md` §22.2). */
  blocked: string;
  /** The ready button's tooltip, or empty (`specs/0094` UX notes §3: "3 parts joined"). */
  tooltip?: string;
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
    if (blocked === "" && tooltip !== "") {
      return (
        <AppTooltip side="bottom" content={tooltip}>
          {button}
        </AppTooltip>
      );
    }
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
          aria-label={slot.label ?? TRACING}
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
 * one flow of the "i" button, Preset, the Mask group, Method, Threshold or Colour tolerance, the
 * method's pipette group, Smooth, Tolerance, Ignore below, and at the far right the expander and
 * the Trace button, wrapping by whole items; under it the notice row and, when open, the Advanced
 * block that holds everything else (`ux-notes.md` §22, §24). One hidden status region announces
 * the result; nothing shows it.
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
          <span className="text-[var(--toolbar-icon)]">
            {trace.selection === "empty" ? TRACE_EMPTY : trace.selection === "mixed" ? TRACE_MIXED : TRACE_NOT_IMAGE}
          </span>
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
      width={name === "specks" ? SPECKS_FIELD_WIDTH : undefined}
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
        swatches={trace.values.swatches}
        armed={trace.pickMode === "colours"}
        onPipette={() => trace.setPicking(trace.pickMode === "")}
        onRemove={trace.removePicked}
        onReturnFocus={onReturnFocus}
      />
    ) : null;
  const methodLabel = TRACE_METHODS[method]?.label ?? TRACE_METHODS[0].label;
  // `ux-notes.md` §24.5: a result whose fix is in Advanced and a failed analysis join the notice
  // row; nothing picked is only the Trace button's reason (§25.3).
  const reason = trace.status.message;
  const analysisFailed = trace.analysis.kind === "failed";
  const noticeShown =
    trace.status.line !== "" || reason !== "" || analysisFailed || trace.status.muted !== "" || trace.imagesLabel !== "" || trace.palette.sameNote !== "" || trace.maskRefused;
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
      <div ref={rowRef} data-trace-row="" className={TRACE_ROW_CLASS}>
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
        <MaskGroup trace={trace} onReturnFocus={onReturnFocus} />
        {/* With two or more parts the bar's values are the active part's (UX notes §3). */}
        {trace.parts.length > 1 ? (
          <span data-trace-part-label="" className="flex h-9 items-center text-xs whitespace-nowrap text-[var(--panel-muted-fg)] tabular-nums">
            {partsLabel(trace.parts)}
          </span>
        ) : null}
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
        {/* Ignore below alone, no pixel readout (`ux-notes.md` §25.1). */}
        <div className="flex h-9 items-center">{field("specks")}</div>
        <div className="ml-auto flex h-9 items-center gap-2">
          <ExpandButton
            open={trace.advancedOpen}
            onToggle={() => trace.setAdvancedOpen(!trace.advancedOpen)}
            controls={advancedId}
            labels={TRACE_EXPAND_LABELS}
            changed={trace.advancedOpen ? null : [partsCount(trace.parts.length), changedAdvancedText(trace.values, trace.params)].filter((t) => t).join(", ") || null}
            onReturnFocus={onReturnFocus}
          />
          <ProgressSlot slot={trace.slot} onStop={trace.stop} onRetry={trace.retrace} onCreate={trace.create} blocked={trace.traceBlocked} tooltip={joinTooltip(trace.parts.length)} />
        </div>
        {trace.advancedOpen ? (
          // After the expander in the DOM, so Tab reaches the Advanced controls after it
          // (criterion 57, UX notes 21.2); drawn below the notice row, full width. Its own width
          // never widens the bar: the bar is as wide as its controls (customer 2026-10-31).
          <div className={`order-last basis-full ${FOLLOWS_WIDTH_CLASS}`}>
            <TraceAdvanced
              id={advancedId}
              params={trace.params}
              values={trace.values}
              onAnalyse={trace.analyse}
              palette={trace.palette}
              countsStale={trace.countsStale}
              setValue={trace.setValue}
              onEndDrag={trace.endDrag}
              onCreate={trace.create}
              onReturnFocus={onReturnFocus}
              coloursFound={trace.palette.found}
              paletteArmed={trace.pickMode === "colours" && trace.values.method !== PICK_COLOURS}
              onPalettePipette={() => trace.setPicking(trace.pickMode === "")}
              parts={<TraceParts parts={trace.parts} onSelect={trace.selectPart} onAdd={trace.addPart} onRemove={trace.removePart} onReturnFocus={onReturnFocus} />}
            />
          </div>
        ) : null}
        {/* The notice row (`ux-notes.md` §22.2): only what is fixed with a bar control or changes how
            the preview reads, always mounted with its live region, no height while empty; drawn
            under the main rows and above the Advanced block. */}
        <div data-trace-state="" className={`order-[9998] basis-full ${FOLLOWS_WIDTH_CLASS} ${noticeShown ? "flex min-h-9 items-center gap-3" : ""}`}>
          <div role="status" aria-live="polite" className={noticeShown ? "flex flex-col py-1 tabular-nums" : ""}>
            {trace.imagesLabel ? <span className={`block ${LABEL_CLASS}`}>{trace.imagesLabel}</span> : null}
            {trace.status.line ? (
              <span data-trace-line="" className="block text-[var(--toolbar-icon)]">
                {trace.status.line}
              </span>
            ) : null}
            {reason ? (
              <span data-trace-reason="" className="block text-[var(--toolbar-icon)]">
                {reason}
              </span>
            ) : null}
            {analysisFailed ? (
              <span data-trace-analysis-failed="" className="block text-[var(--toolbar-icon)]">
                {ANALYSIS_FAILED}
              </span>
            ) : null}
            {trace.status.muted ? (
              <span data-trace-muted="" className={`block ${LABEL_CLASS} opacity-70`}>
                {trace.status.muted}
              </span>
            ) : null}
            {trace.palette.sameNote ? <span className={`block ${LABEL_CLASS}`}>{trace.palette.sameNote}</span> : null}
            {/* A pick refused (`specs/0094` criterion 38): the criterion 14 string, only while it holds. */}
            {trace.maskRefused ? (
              <span data-trace-mask-refused="" className={`block ${LABEL_CLASS} opacity-70`}>
                {TRACE_MIXED}
              </span>
            ) : null}
          </div>
        </div>
      </div>
      {/* The one hidden status region, always mounted (`ux-notes.md` §24.5). */}
      <div data-trace-hidden-status="" role="status" aria-live="polite" className="sr-only">
        {trace.announcement}
      </div>
    </div>
    </div>
  );
}
