import { Switch } from "radix-ui";
import { useId, useRef, useState } from "react";
import type { KeyboardEvent } from "react";

import { TraceField } from "@/components/TraceField";
import {
  OVER_4_MPX,
  type TraceParam,
  type TraceValues,
  nodeLimitShown,
  parseNodeLimit,
} from "@/lib/traceText";

/** The value each switch of the schema sets (Invert, Ignore background, Full-size preview). */
const SWITCH_VALUES: Partial<Record<TraceParam["name"], "invert" | "ignoreBackground" | "fullSizePreview">> = {
  invert: "invert",
  ignore_background: "ignoreBackground",
  full_size_preview: "fullSizePreview",
};

const LABEL_CLASS = "text-xs text-[var(--toolbar-icon)]";
const CELL_CLASS = "flex h-9 min-w-[168px] items-center";

interface ChoicePillProps {
  param: TraceParam;
  value: number;
  onValue: (value: number) => void;
  onReturnFocus: () => void;
}

/** A segmented pill of a choice (Denoise Off | Low | High, Upscale Auto | Off): a radio group,
 * the arrows move the choice, Escape returns the focus to the canvas. */
function ChoicePill({ param, value, onValue, onReturnFocus }: ChoicePillProps) {
  const buttons = useRef<(HTMLButtonElement | null)[]>([]);
  const keyDown = (event: KeyboardEvent<HTMLDivElement>) => {
    if (event.key === "Escape") {
      event.preventDefault();
      event.stopPropagation();
      onReturnFocus();
      return;
    }
    const step = event.key === "ArrowRight" || event.key === "ArrowDown" ? 1 : event.key === "ArrowLeft" || event.key === "ArrowUp" ? -1 : 0;
    if (step !== 0) {
      event.preventDefault();
      const next = Math.min(param.choices.length - 1, Math.max(0, value + step));
      onValue(next);
      buttons.current[next]?.focus();
    }
  };
  return (
    <div className={CELL_CLASS}>
      <span className={`${LABEL_CLASS} w-16 shrink-0`}>{param.label}</span>
      <div role="radiogroup" aria-label={param.label} className="flex h-7 overflow-hidden rounded-md border border-[var(--toolbar-icon)]/40" onKeyDown={keyDown}>
        {param.choices.map((choice, index) => {
          const on = index === value;
          return (
            <button
              key={choice}
              ref={(element) => {
                buttons.current[index] = element;
              }}
              type="button"
              role="radio"
              aria-checked={on}
              tabIndex={on ? 0 : -1}
              onClick={() => onValue(index)}
              className={`px-2 text-sm outline-none focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-[var(--editor-accent)] ${
                on ? "bg-[var(--toolbar-icon-active-bg)] text-[var(--toolbar-icon-active-fg)]" : "text-[var(--toolbar-icon)] hover:bg-[var(--editor-accent-hover)]"
              }`}
            >
              {choice}
            </button>
          );
        })}
      </div>
    </div>
  );
}

interface TraceSwitchProps {
  label: string;
  checked: boolean;
  disabled?: boolean;
  /** A muted state text beside the label ("Over 4 Mpx"). */
  state?: string;
  onChange: (checked: boolean) => void;
}

/** A switch row (UX notes section 12): label left, switch right, the whole row is the target. */
function TraceSwitch({ label, checked, disabled = false, state, onChange }: TraceSwitchProps) {
  const id = useId();
  return (
    <label
      htmlFor={id}
      className={`${CELL_CLASS} cursor-pointer justify-between gap-2 rounded-md px-1 text-sm text-[var(--toolbar-icon)] hover:bg-[var(--editor-accent-hover)] has-[:focus-visible]:ring-2 has-[:focus-visible]:ring-[var(--editor-accent)]`}
    >
      <span className="flex flex-col whitespace-nowrap leading-tight">
        {label}
        {state ? <span className={`${LABEL_CLASS} opacity-70`}>{state}</span> : null}
      </span>
      <Switch.Root
        id={id}
        aria-label={label}
        aria-disabled={disabled}
        checked={checked && !disabled}
        onCheckedChange={(next) => {
          if (!disabled) {
            onChange(next);
          }
        }}
        className={`inline-flex h-[18px] w-8 shrink-0 cursor-pointer items-center rounded-full border-[1.5px] border-[var(--toolbar-icon)] bg-transparent outline-none data-[state=checked]:border-[var(--toolbar-icon-active-bg)] data-[state=checked]:bg-[var(--toolbar-icon-active-bg)] ${disabled ? "opacity-40" : ""}`}
      >
        <Switch.Thumb className="block size-3 rounded-full bg-[var(--toolbar-icon)] data-[state=checked]:translate-x-[15px] data-[state=checked]:bg-[var(--toolbar-icon-active-fg)] data-[state=unchecked]:translate-x-[2px]" />
      </Switch.Root>
    </label>
  );
}

/** The Node limit field (criterion 43): a number, empty for none, thousands separator on blur. */
function NodeLimitField({ value, onValue, onReturnFocus }: { value: number | null; onValue: (value: number | null) => void; onReturnFocus: () => void }) {
  const [draft, setDraft] = useState<string | null>(null);
  const commit = () => {
    if (draft !== null) {
      onValue(parseNodeLimit(draft));
      setDraft(null);
    }
  };
  return (
    <label className={`${CELL_CLASS} gap-2`}>
      <span className={`${LABEL_CLASS} w-16 shrink-0`}>Node limit</span>
      <input
        inputMode="numeric"
        aria-label="Node limit"
        value={draft ?? nodeLimitShown(value)}
        onChange={(event) => setDraft(event.target.value)}
        onBlur={commit}
        onKeyDown={(event) => {
          if (event.key === "Enter") {
            event.preventDefault();
            commit();
            onReturnFocus();
          } else if (event.key === "Escape") {
            event.preventDefault();
            event.stopPropagation();
            setDraft(null);
            onReturnFocus();
          }
        }}
        className="h-7 w-[88px] rounded-md px-2 text-right text-sm tabular-nums text-[var(--toolbar-icon)] outline-none focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)]"
        style={{ background: "color-mix(in srgb, var(--toolbar-icon) 8%, transparent)" }}
      />
    </label>
  );
}

interface TraceAdvancedProps {
  params: readonly TraceParam[];
  values: TraceValues;
  /** The shown image has more than 4 Mpx: Full-size preview is disabled. */
  overFullSizeLimit: boolean;
  setValue: <K extends keyof TraceValues>(name: K, value: TraceValues[K], dragging?: boolean) => void;
  onEndDrag: () => void;
  onCreate: () => void;
  onReturnFocus: () => void;
}

/**
 * The Advanced block (`specs/0025-raster-trace` criteria 41, 43, 44; UX notes section 12): below
 * the result row inside the bar, after a divider, two columns of 168 px: Denoise and Upscale,
 * Corners and Node limit, Invert and Ignore background, Full-size preview.
 */
export function TraceAdvanced({ params, values, overFullSizeLimit, setValue, onEndDrag, onCreate, onReturnFocus }: TraceAdvancedProps) {
  const param = (name: TraceParam["name"]) => params.find((p) => p.name === name);
  const denoise = param("denoise");
  const upscale = param("upscale");
  const corners = param("corners");
  return (
    <div role="group" aria-label="Advanced" className="mt-1 flex flex-wrap gap-x-2 border-t border-[var(--toolbar-icon)]/25 py-1">
      {denoise ? <ChoicePill param={denoise} value={values.denoise} onValue={(v) => setValue("denoise", v)} onReturnFocus={onReturnFocus} /> : null}
      {upscale ? <ChoicePill param={upscale} value={values.upscale} onValue={(v) => setValue("upscale", v)} onReturnFocus={onReturnFocus} /> : null}
      {corners ? (
        <div className={CELL_CLASS}>
          <TraceField
            param={corners}
            value={values.corners}
            onValue={(v, dragging) => setValue("corners", v, dragging)}
            onEndDrag={onEndDrag}
            onCreate={onCreate}
            onReturnFocus={onReturnFocus}
          />
        </div>
      ) : null}
      {param("node_limit") ? <NodeLimitField value={values.nodeLimit} onValue={(v) => setValue("nodeLimit", v)} onReturnFocus={onReturnFocus} /> : null}
      {params
        .filter((p) => p.advanced && p.kind === "toggle")
        .map((p) => {
          const key = SWITCH_VALUES[p.name];
          if (!key) {
            return null;
          }
          const fullSize = key === "fullSizePreview";
          return (
            <TraceSwitch
              key={p.name}
              label={p.label}
              checked={values[key]}
              disabled={fullSize && overFullSizeLimit}
              state={fullSize && overFullSizeLimit ? OVER_4_MPX : undefined}
              onChange={(v) => setValue(key, v)}
            />
          );
        })}
    </div>
  );
}
