import { Check, Link2, Route, SquareDashed, type LucideIcon, X } from "lucide-react";
import { Tooltip } from "radix-ui";
import { type KeyboardEvent, useEffect, useId, useRef } from "react";

import { ValueField } from "@/components/panel/ValueField";
import { FINISH_CLASS, InfoButton } from "@/components/TraceBar";
import { TOOLTIP_CLASS, Tooltip as AppTooltip } from "@/components/ui/tooltip";
import type { CentreLinesApi } from "@/hooks/useCentreLines";
import type { TraceInfoApi } from "@/hooks/useTraceInfo";
import {
  CENTRE_INFO_PAGE_ID,
  CENTRE_INFO_TITLE,
  CENTRE_JOIN_TIPS_TOOLTIP,
  CENTRE_LINES_LABEL,
  CENTRE_LINES_NAME,
  CENTRE_NOTHING_TO_CONVERT,
  type CentreFieldSpec,
  type CentreLinesOptions,
  centreEscapeCancels,
  centreField,
  centreFieldParse,
  centreFieldPosition,
  centreFieldStep,
  centreFieldText,
  centreFieldValueAt,
  centreLinesDimmedTooltip,
  centrePreviewCounts,
  focusCentreLinesBar,
} from "@/lib/centreLinesText";
import { HELP_RATIO } from "@/lib/sliderNames";
import { Button } from "@/components/ui/button";
import { buttonClass, GLYPH } from "@/components/ui/looks";
import { ToggleButton } from "@/components/ui/toggle-button";
import { ActionNotice } from "@/components/ui/notice";

const BUTTON_CLASS = buttonClass("secondary", "bar", "my-1");

/** The width of a slider of the bar, px (UX notes §3.1). */
const FIELD_WIDTH = 150;
/** A switch of the bar with its label (UX notes §3.1): Join tips, Keep inner areas. Enter on it
 * applies; a pointer press hands the keyboard back to the canvas. */
function CentreSwitch({
  label,
  icon,
  checked,
  onChange,
  onReturnFocus,
}: {
  label: string;
  icon: LucideIcon;
  checked: boolean;
  onChange: (checked: boolean) => void;
  onReturnFocus: () => void;
}) {
  return (
    <ToggleButton
      look="bar"
      className="my-1"
      icon={icon}
      label={label}
      pressed={checked}
      onPressedChange={onChange}
      onReturnFocus={onReturnFocus}
      extra={{ "data-centre-enter": "" }}
    />
  );
}

/** The Offset field is narrower so that Keep inner areas, Offset, Cancel and Apply fit one row at
 * 1440 px. */
const OFFSET_WIDTH = 130;

const UNIT_WORDS: Readonly<Record<string, string>> = { "%": "percent", mm: "millimetres" };

/**
 * One slider of the Centre lines bar (UX notes §3.1): the app's `ValueField` as the Trace bar
 * uses it, 150 px, no reset slot, the signed bar on tracks that cross zero. A click or drag sets
 * the value, the arrows step it, a double-click, Enter or F2 opens the number entry.
 */
function CentreField({
  field,
  value,
  onValue,
  onReturnFocus,
  width = FIELD_WIDTH,
}: {
  width?: number;
  field: CentreFieldSpec;
  value: number;
  onValue: (value: number) => void;
  onReturnFocus: () => void;
}) {
  const pressing = useRef<{ start: number } | null>(null);
  const text = centreFieldText(field, value);
  const position = centreFieldPosition(field, value);
  const zero = field.min < 0 && field.max > 0 ? centreFieldPosition(field, 0) : null;
  const unitWords = UNIT_WORDS[field.unit] ?? "";
  const set = (next: number) => {
    if (next !== value) {
      onValue(next);
    }
  };
  return (
    <div
      className="my-1"
      onPointerDownCapture={(event) => {
        if (event.button === 0) {
          pressing.current = { start: value };
        }
      }}
    >
      <ValueField
        label={field.label}
        name={field.name}
        text={text}
        bar={zero === null ? position : Math.max(position, zero)}
        barFrom={zero === null ? 0 : Math.min(position, zero)}
        zero={zero}
        mixed={false}
        resettable={false}
        noResetSlot
        width={width}
        help={HELP_RATIO}
        unit={field.unit}
        valueNow={value}
        valueMin={field.min}
        valueMax={field.typedMax}
        unitWords={unitWords}
        valueText={`${text}${unitWords ? ` ${unitWords}` : ""}`}
        tipValue={`${text} ${field.unit}`}
        defaultText=""
        messages={{}}
        onPreview={(p, grid) => set(centreFieldValueAt(field, p, grid))}
        onSettle={() => {
          pressing.current = null;
        }}
        onStep={(steps, grid) => set(centreFieldStep(field, value, steps, grid))}
        onReset={() => undefined}
        onSubmit={(typed) => {
          const next = centreFieldParse(field, typed);
          if (next === null) {
            return "invalid:number";
          }
          if (next === value) {
            return "unchanged";
          }
          onValue(next);
          return "committed";
        }}
        onCancel={() => {
          const start = pressing.current?.start;
          pressing.current = null;
          if (start !== undefined) {
            onValue(start);
          }
        }}
        onReturnFocus={() => {
          pressing.current = null;
          onReturnFocus();
        }}
      />
    </div>
  );
}

/** Apply at the far right (UX notes §3.3, §4): Trace's finish look; while a run or an Apply is
 * slow, its running look with the share done and an x "Stop (Esc)". */
function ApplyButton({ api, onStop }: { api: CentreLinesApi; onStop: () => void }) {
  const reasonId = useId();
  if (api.running || (api.applying && api.slow)) {
    const percent = api.progress === null ? null : Math.round(api.progress * 100);
    return (
      <div data-centre-apply="running" className={`${FINISH_CLASS} w-[75px]`}>
        <div
          role="progressbar"
          aria-label={api.applying ? "Applying" : "Updating preview"}
          aria-valuemin={0}
          aria-valuemax={100}
          aria-valuenow={percent ?? undefined}
          className="flex h-full min-w-0 flex-1 items-center"
        >
          <span aria-hidden className="absolute inset-y-0 left-0" style={{ width: `${percent ?? 0}%`, background: "color-mix(in srgb, white 25%, transparent)" }} />
          <span className="relative pl-2 text-xs tabular-nums">{percent === null ? "…" : `${percent} %`}</span>
        </div>
        <AppTooltip side="bottom" content="Stop (Esc)">
          <button type="button" aria-label="Stop (Esc)" onClick={onStop} className="relative flex size-7 items-center justify-center rounded-[5px] text-white outline-none hover:bg-white/20">
            <X {...GLYPH} aria-hidden />
          </button>
        </AppTooltip>
      </div>
    );
  }
  const nothing = api.preview !== null && api.preview.paths === 0;
  const counts = api.preview ? centrePreviewCounts(api.preview.paths, api.preview.nodes, api.preview.kept) : "";
  const button = (
    <Button variant="primary" size="bar" icon={Check}
      data-centre-apply="ready"
      aria-disabled={nothing || undefined}
      aria-describedby={nothing ? reasonId : undefined}
      onClick={() => {
        if (!nothing) {
          api.apply();
        }
      }}
    >
      Apply
    </Button>
  );
  return (
    <>
      <AppTooltip
        side="bottom"
        content={
          nothing ? (
            CENTRE_NOTHING_TO_CONVERT
          ) : (
            <span className="flex flex-col">
              <span>Apply (Enter)</span>
              {counts ? <span className="text-[var(--panel-muted-fg)]">{counts}</span> : null}
            </span>
          )
        }
      >
        {button}
      </AppTooltip>
      {nothing ? (
        <span id={reasonId} hidden>
          {CENTRE_NOTHING_TO_CONVERT}
        </span>
      ) : null}
    </>
  );
}

/**
 * The options of Find centre lines in the top bar (`specs/0117-centre-lines` criteria 15, 15b and
 * 15c, UX notes §3, §4 and §7; the app has no popups): it replaces the Node bar while open. The
 * "i" button, the sliders Overlap, Thick parts and Optimise, the Keep inner areas switch with its
 * Offset, and at the far right Cancel and Apply; every value change previews in the worker, Enter
 * or Ctrl+Enter applies, Escape cancels. The fields are inert while Apply runs.
 */
export function CentreLinesBar({ api, info, onReturnFocus }: { api: CentreLinesApi; info: TraceInfoApi; onReturnFocus: () => void }) {
  const set = (patch: Partial<CentreLinesOptions>) => api.setOptions({ ...api.options, ...patch });
  const latest = useRef(api);
  useEffect(() => {
    latest.current = api;
  });
  const stop = () => latest.current.stop();
  // Escape with the focus anywhere else (the body after the bar's button went away, a tool rail
  // button, a canvas child): the bar's and the canvas's handlers stop what they handle, so only
  // the rest reaches the window.
  useEffect(() => {
    const onWindowKey = (event: globalThis.KeyboardEvent) => {
      const target = event.target;
      const entry = target instanceof HTMLElement && (target.tagName === "INPUT" || target.tagName === "TEXTAREA" || target.closest("[role=dialog],[role=menu]") !== null);
      // A field or a dialog elsewhere keeps its own Escape.
      if (event.key !== "Escape" || (entry && event.defaultPrevented)) {
        return;
      }
      event.preventDefault();
      if (latest.current.applying) {
        latest.current.stop();
        return;
      }
      latest.current.cancel();
      onReturnFocus();
    };
    window.addEventListener("keydown", onWindowKey);
    return () => window.removeEventListener("keydown", onWindowKey);
  }, [onReturnFocus]);
  const onKeyDown = (event: KeyboardEvent<HTMLDivElement>) => {
    const escape = event.key === "Escape";
    if (escape ? !centreEscapeCancels(event) : event.defaultPrevented) {
      return;
    }
    if (escape) {
      event.preventDefault();
      event.stopPropagation();
      // During Apply, Escape stops it and the bar stays (UX notes §4); else it cancels.
      if (api.applying) {
        api.stop();
        return;
      }
      api.cancel();
      onReturnFocus();
    } else if (event.key === "Enter") {
      // Ctrl+Enter applies anywhere, also in the number entry; a plain Enter on the i button or
      // the switch applies; on a slider it opens the entry, on Cancel and Apply it presses them.
      const onApplier = event.target instanceof HTMLElement && event.target.closest("[data-centre-enter]") !== null;
      if (event.ctrlKey || event.metaKey || onApplier) {
        event.preventDefault();
        event.stopPropagation();
        api.apply();
      }
    }
  };
  const offset = centreField("offset");
  return (
    <div
      data-context-bar=""
      data-centre-lines-bar=""
      role="toolbar"
      aria-label={CENTRE_LINES_NAME}
      className="pointer-events-auto flex min-w-0 max-w-full flex-wrap items-center gap-x-2 rounded-[8px] px-2 text-sm"
      style={{ background: "var(--toolbar-bg)", boxShadow: "var(--panel-elevation-shadow)" }}
      onKeyDown={onKeyDown}
    >
      {/* The values are inert while Apply runs; Cancel, the x and Escape stay live (UX §4). */}
      <div className="contents" inert={api.applying || undefined}>
      <span className="flex h-9 items-center" data-centre-enter="">
        <InfoButton info={info} onReturnFocus={onReturnFocus} title={CENTRE_INFO_TITLE} pageId={CENTRE_INFO_PAGE_ID} />
      </span>
      {(["overlap", "thickParts", "optimise"] as const).map((key) => (
        <CentreField key={key} field={centreField(key)} value={api.options[key]} onValue={(value) => set({ [key]: value })} onReturnFocus={onReturnFocus} />
      ))}
      <AppTooltip side="bottom" content={CENTRE_JOIN_TIPS_TOOLTIP}>
        <CentreSwitch icon={Link2} label="Join tips" checked={api.options.joinTips} onChange={(joinTips) => set({ joinTips })} onReturnFocus={onReturnFocus} />
      </AppTooltip>
      <CentreSwitch icon={SquareDashed} label="Keep inner areas" checked={api.options.keepInner} onChange={(keepInner) => set({ keepInner })} onReturnFocus={onReturnFocus} />
      {api.options.keepInner ? <CentreField field={offset} width={OFFSET_WIDTH} value={api.options.offset} onValue={(value) => set({ offset: value })} onReturnFocus={onReturnFocus} /> : null}
      </div>
      <div className="ml-auto flex h-9 items-center gap-1">
        <Button size="bar" icon={X}
          onClick={() => {
            api.cancel();
            onReturnFocus();
          }}
        >
          Cancel
        </Button>
        <ApplyButton api={api} onStop={stop} />
      </div>
      <p role="status" className="sr-only">
        {api.announcement}
      </p>
    </div>
  );
}

/**
 * The Find centre lines button of the Node bar (`specs/0117-centre-lines` criterion 1, UX notes
 * §1 and §2): a group of its own after Close path and before Edit width, the `Route` glyph before
 * the label, in the tree whenever the Node tool has a selection, dimmed with the reason in its
 * tooltip when nothing selected is a filled shape; a dimmed press answers with the refusal. The
 * press opens the options bar; by Enter or Space the keyboard goes to Overlap.
 */
export function CentreLinesButton({
  enabled,
  refusal,
  api,
}: {
  enabled: boolean;
  /** The refusal code while dimmed, for the tooltip. */
  refusal: string;
  api: CentreLinesApi;
}) {
  const notice = api.notice;
  return (
    <div className="relative">
      <Tooltip.Provider delayDuration={400}>
        <Tooltip.Root>
          <Tooltip.Trigger asChild>
            <button
              type="button"
              aria-disabled={!enabled}
              data-node-command="centre-lines"
              onClick={() => {
                api.press();
                // A mouse press too: the keys of the bar work without a click into it.
                focusCentreLinesBar();
              }}
              className={BUTTON_CLASS}
            >
              <Route {...GLYPH} aria-hidden />
              {CENTRE_LINES_LABEL}
            </button>
          </Tooltip.Trigger>
          <Tooltip.Portal>
            <Tooltip.Content
              side="bottom"
              sideOffset={6}
              className={TOOLTIP_CLASS}
              style={{
                background: "var(--toolbar-bg)",
                color: "var(--toolbar-icon)",
                boxShadow: "var(--panel-elevation-shadow)",
              }}
            >
              {enabled ? CENTRE_LINES_NAME : centreLinesDimmedTooltip(refusal, 0)}
            </Tooltip.Content>
          </Tooltip.Portal>
        </Tooltip.Root>
      </Tooltip.Provider>
      <ActionNotice
        refusal={notice?.alert ?? false}
        maxWidth="min(320px, 60vw)"
        className="absolute top-full left-0 mt-1"
        style={notice ? undefined : { display: "none" }}
      >
        {notice?.text}
      </ActionNotice>
    </div>
  );
}
