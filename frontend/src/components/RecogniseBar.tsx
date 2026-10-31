import { CircleAlert, X } from "lucide-react";
import { Tooltip } from "radix-ui";
import { useRef } from "react";

import { ICON_BUTTON_CLASS } from "@/components/TextBar";
import { FINISH_CLASS } from "@/components/TraceBar";
import { TraceField } from "@/components/TraceField";
import { Tooltip as AppTooltip } from "@/components/ui/tooltip";
import { shiftStyle, useNoticeShift } from "@/hooks/useNoticeShift";
import type { RecogniseApi, RecogniseBarNotice, RecogniseSlot } from "@/hooks/useRecognise";
import {
  FACES_TOOLTIP,
  RECOGNISE,
  RECOGNISE_EMPTY,
  RECOGNISE_NOT_PATHS,
  RECOGNISE_TEXT,
  RECOGNISING,
  TOLERANCE_PARAM,
} from "@/lib/recogniseText";
import { FAILED, RETRY, STOP, STOPPED, percentText } from "@/lib/traceText";

/** The look of "Linked clone": a 28 px text button. */
const BUTTON_CLASS =
  "my-1 h-7 whitespace-nowrap rounded-md px-2 text-sm text-[var(--toolbar-icon)] outline-none hover:bg-[var(--editor-accent-hover)] focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)] focus-visible:ring-offset-1 focus-visible:ring-offset-[var(--toolbar-bg)]";

const LABEL_CLASS = "text-xs text-[var(--toolbar-icon)]";

const TOOLTIP_STYLE = { background: "var(--toolbar-bg)", color: "var(--toolbar-icon)", boxShadow: "var(--panel-elevation-shadow)" };

/**
 * The Select bar's "Recognise text" button (`specs/0088-trace-recognition` Frage 104 A, UX notes
 * §1): last, in a group of its own, while a closed path is selected; a press chooses the
 * Recognise bar. The notice of a run that wrote text sits 4 px under it.
 */
export function RecogniseTextButton({
  onEnter,
  onReturnFocus,
  notice,
}: {
  onEnter: (byKeyboard: boolean) => void;
  onReturnFocus: () => void;
  notice: RecogniseBarNotice | null;
}) {
  const noticeRef = useRef<HTMLDivElement>(null);
  const shift = useNoticeShift(noticeRef, notice);
  return (
    <div className="relative">
      <Tooltip.Provider delayDuration={400}>
        <Tooltip.Root>
          <Tooltip.Trigger asChild>
            <button
              type="button"
              data-select-command="recognise-text"
              onClick={(event) => {
                // `detail` is 0 for Space and Enter (the focus goes to the bar), 1 or more for a click.
                onEnter(event.detail === 0);
                if (event.detail > 0) {
                  onReturnFocus();
                }
              }}
              className={BUTTON_CLASS}
            >
              {RECOGNISE_TEXT}
            </button>
          </Tooltip.Trigger>
          <Tooltip.Portal>
            <Tooltip.Content side="bottom" sideOffset={6} className="z-50 rounded-md px-2 py-1 text-xs" style={TOOLTIP_STYLE}>
              {RECOGNISE_TEXT}
            </Tooltip.Content>
          </Tooltip.Portal>
        </Tooltip.Root>
      </Tooltip.Provider>
      {/* Takes no focus; the live region is in the tree from the start (row "Action notice"). */}
      <div
        ref={noticeRef}
        role="status"
        data-recognise-notice=""
        className="absolute top-full right-0 mt-1 w-max max-w-[320px] break-words rounded-md px-2 py-1 text-xs"
        style={notice ? { ...TOOLTIP_STYLE, ...shiftStyle(shift) } : { display: "none" }}
      >
        {notice?.text}
      </div>
    </div>
  );
}

/** The finish button (UX notes §3): always blue, "Recognise" at rest; while a run is in flight
 * (after 400 ms) its percentage and an x; "Stopped" or "Failed" with Retry. */
function FinishSlot({ slot, onRun, onStop }: { slot: RecogniseSlot; onRun: () => void; onStop: () => void }) {
  if (slot.kind === "none" || (slot.kind === "running" && slot.percent === null)) {
    return (
      <button
        type="button"
        data-recognise-slot="none"
        className={`${FINISH_CLASS} justify-center px-3 outline-none hover:brightness-110 focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)] focus-visible:ring-offset-1 focus-visible:ring-offset-[var(--toolbar-bg)]`}
        onClick={slot.kind === "none" ? onRun : undefined}
      >
        {RECOGNISE}
      </button>
    );
  }
  if (slot.kind === "running") {
    const fill = slot.percent ?? 0;
    return (
      <div data-recognise-slot="running" className={`${FINISH_CLASS} w-[75px]`}>
        <div
          role="progressbar"
          aria-label={RECOGNISING}
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
    <div data-recognise-slot={slot.kind} className={`${FINISH_CLASS} gap-1 pl-2`}>
      <span
        title={failed ? slot.text : undefined}
        className={`flex h-7 items-center gap-1 rounded-md text-xs ${failed ? "" : "opacity-90"}`}
        style={failed ? { boxShadow: "inset 0 0 0 1px var(--field-invalid)" } : undefined}
      >
        {failed ? <CircleAlert size={12} aria-hidden className="shrink-0" /> : null}
        {failed ? FAILED : STOPPED}
      </span>
      {failed ? (
        <span role="alert" className="sr-only">
          {slot.text}
        </span>
      ) : null}
      <button
        type="button"
        className="h-7 rounded-md px-2 text-sm text-white outline-none hover:bg-white/20 focus-visible:ring-2 focus-visible:ring-white"
        onClick={onRun}
      >
        {RETRY}
      </button>
    </div>
  );
}

/**
 * The Recognise bar in the overlay row (`specs/0088-trace-recognition` criteria 4, 6 and 10; UX
 * notes §2 to §4): the state label, the Tolerance field, the Faces readout and the finish button
 * in one flow, the notice row under it. With nothing or no closed path selected it is one row of
 * words.
 */
export function RecogniseBar({ recognise, onReturnFocus }: { recognise: RecogniseApi; onReturnFocus: () => void }) {
  const pill = "pointer-events-auto flex min-w-0 flex-col rounded-lg px-2 text-sm";
  const pillStyle = { background: "var(--toolbar-bg)", boxShadow: "var(--panel-elevation-shadow)" };
  const label = <span className={`${LABEL_CLASS} whitespace-nowrap py-2`}>{RECOGNISE_TEXT}</span>;
  if (recognise.selection !== "paths" && recognise.slot.kind !== "running") {
    return (
      <div data-context-bar="" className={pill} style={pillStyle} role="toolbar" aria-label={RECOGNISE_TEXT}>
        <div className="flex h-9 items-center gap-3">
          {label}
          <span className="text-[var(--toolbar-icon)]">{recognise.selection === "empty" ? RECOGNISE_EMPTY : RECOGNISE_NOT_PATHS}</span>
        </div>
      </div>
    );
  }
  const noticeShown = recognise.notice !== "";
  return (
    <div data-context-bar="" className={pill} style={pillStyle} role="toolbar" aria-label={RECOGNISE_TEXT}>
      <div className="flex min-w-0 flex-wrap items-start gap-x-2 gap-y-0">
        <div className="flex h-9 items-center">{label}</div>
        <div ref={recognise.fieldRef} className="flex h-9 items-center">
          <TraceField
            param={TOLERANCE_PARAM}
            value={recognise.tolerance}
            onValue={(value) => recognise.setTolerance(value)}
            onEndDrag={() => undefined}
            onCreate={recognise.run}
            onReturnFocus={onReturnFocus}
          />
        </div>
        <AppTooltip side="bottom" content={FACES_TOOLTIP}>
          <span data-recognise-faces="" className={`flex h-9 items-center whitespace-nowrap ${LABEL_CLASS} opacity-70`}>
            {recognise.readout}
          </span>
        </AppTooltip>
        <div className="ml-auto flex h-9 items-center">
          <FinishSlot slot={recognise.slot} onRun={recognise.run} onStop={recognise.stop} />
        </div>
        {/* The notice row: "nothing within tolerance" only, always mounted with its live region,
            no height while empty. */}
        <div data-recognise-state="" className={`basis-full ${noticeShown ? "flex min-h-9 items-center" : ""}`}>
          <div role="status" aria-live="polite" className={noticeShown ? `py-1 tabular-nums ${LABEL_CLASS}` : ""}>
            {recognise.notice}
          </div>
        </div>
      </div>
    </div>
  );
}
