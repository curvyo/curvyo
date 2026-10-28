import { Contrast, ImageMinus, type LucideIcon } from "lucide-react";
import { useRef } from "react";
import type { KeyboardEvent } from "react";

import { ICON_BUTTON_CLASS } from "@/components/TextBar";
import { TraceField } from "@/components/TraceField";
import { Tooltip } from "@/components/ui/tooltip";
import type { TraceParam, TraceStatusText, TraceValues } from "@/lib/traceText";

/** The value and icon each toggle of the schema sets (Invert, Ignore background). */
const TOGGLES: Partial<Record<TraceParam["name"], { key: "invert" | "ignoreBackground"; icon: LucideIcon }>> = {
  invert: { key: "invert", icon: Contrast },
  ignore_background: { key: "ignoreBackground", icon: ImageMinus },
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
      <div role="radiogroup" aria-label={param.label} className="flex h-7 overflow-hidden rounded-md outline outline-1 -outline-offset-1 outline-[var(--toolbar-icon)]/40" onKeyDown={keyDown}>
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
              className={`min-w-7 px-2 text-sm outline-none focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-[var(--editor-accent)] ${
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

interface TraceToggleProps {
  label: string;
  icon: LucideIcon;
  pressed: boolean;
  onChange: (pressed: boolean) => void;
  onReturnFocus: () => void;
}

/** An icon toggle (UX notes 19): the look and wiring of the Glyphs button, no label drawn, the
 * full label in the tooltip and as the name; Escape returns the focus to the canvas. */
function TraceToggle({ label, icon: Icon, pressed, onChange, onReturnFocus }: TraceToggleProps) {
  const on = pressed;
  return (
    <Tooltip side="bottom" content={label}>
      <button
        type="button"
        aria-label={label}
        aria-pressed={on}
        onClick={() => onChange(!pressed)}
        onKeyDown={(event) => {
          if (event.key === "Escape") {
            event.preventDefault();
            event.stopPropagation();
            onReturnFocus();
          }
        }}
        className={`${ICON_BUTTON_CLASS} ${
          on ? "bg-[var(--toolbar-icon-active-bg)] text-[var(--toolbar-icon-active-fg)]" : "text-[var(--toolbar-icon)] hover:bg-[var(--editor-accent-hover)]"
        }`}
      >
        <Icon size={16} strokeWidth={1.5} aria-hidden />
      </button>
    </Tooltip>
  );
}

interface TraceAdvancedProps {
  params: readonly TraceParam[];
  values: TraceValues;
  /** The counts line of the last result and its muted line (criterion 11). */
  status: TraceStatusText;
  /** The counts belong to an older result (a running or stopped trace): drawn at 50 %. */
  countsStale: boolean;
  setValue: <K extends keyof TraceValues>(name: K, value: TraceValues[K], dragging?: boolean) => void;
  onEndDrag: () => void;
  onCreate: () => void;
  onReturnFocus: () => void;
  /** The id `aria-controls` of the expander names. */
  id: string;
}

/**
 * The Advanced block (`specs/0025-raster-trace` criteria 11, 41, 44, 71; UX notes 20.4, 20.5):
 * below the main rows inside the bar, after a divider: row 1 the two icon toggles (Invert,
 * Ignore background) and Denoise, then Upscale and Corners, the counts line always last in a
 * row of its own.
 */
export function TraceAdvanced({ params, values, status, countsStale, setValue, onEndDrag, onCreate, onReturnFocus, id }: TraceAdvancedProps) {
  const param = (name: TraceParam["name"]) => params.find((p) => p.name === name);
  const denoise = param("denoise");
  const upscale = param("upscale");
  const corners = param("corners");
  return (
    <div id={id} role="group" aria-label="Advanced" className="mt-1 flex flex-wrap gap-x-2 border-t border-[var(--toolbar-icon)]/25 py-1">
      <div className="flex h-9 items-center gap-1">
        {params
          .filter((p) => p.advanced && p.kind === "toggle")
          .map((p) => {
            const toggle = TOGGLES[p.name];
            return toggle ? (
              <TraceToggle
                key={p.name}
                label={p.label}
                icon={toggle.icon}
                pressed={values[toggle.key]}
                onChange={(v) => setValue(toggle.key, v)}
                onReturnFocus={onReturnFocus}
              />
            ) : null;
          })}
      </div>
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
      {status.counts ? (
        // Not a live region: the counts are not announced (UX notes 20.4).
        <div data-trace-counts="" className={`flex min-h-9 basis-full flex-col justify-center py-1 tabular-nums ${countsStale ? "opacity-50" : ""}`}>
          <span className="text-[var(--toolbar-icon)]">{status.counts}</span>
          {status.countsMuted ? <span className={`${LABEL_CLASS} opacity-70`}>{status.countsMuted}</span> : null}
        </div>
      ) : null}
    </div>
  );
}
