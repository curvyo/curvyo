import { type KeyboardEvent, useRef } from "react";

const OPTION_CLASS =
  "h-7 rounded border border-[color-mix(in_srgb,var(--toolbar-icon)_60%,transparent)] text-sm hover:bg-[var(--editor-accent-hover)] aria-checked:bg-[var(--toolbar-icon-active-bg)] aria-checked:text-[var(--toolbar-icon-active-fg)] focus-visible:outline-2 focus-visible:outline-[var(--editor-accent)]";

/**
 * A panel radio group in the target-toggle look (`specs/0090-grid` UX notes §2 and §3): one Tab
 * stop on the checked option, arrows move and choose, a pointer choice returns focus to the canvas.
 */
export function PanelRadioGroup<T extends string>({
  legend,
  options,
  value,
  onChange,
  onReturnFocus,
  stacked = false,
}: {
  legend: string;
  options: readonly (readonly [T, string])[];
  value: T;
  onChange: (value: T) => void;
  onReturnFocus: () => void;
  stacked?: boolean;
}) {
  const refs = useRef<(HTMLButtonElement | null)[]>([]);
  const onKeyDown = (event: KeyboardEvent<HTMLButtonElement>, index: number) => {
    const step = { ArrowRight: 1, ArrowDown: 1, ArrowLeft: -1, ArrowUp: -1 }[event.key];
    if (step === undefined) return;
    event.preventDefault();
    const next = (index + step + options.length) % options.length;
    onChange(options[next][0]);
    refs.current[next]?.focus();
  };
  return (
    <div className="min-w-0">
      <div className="mb-2 text-sm" aria-hidden>{legend}</div>
      <div role="radiogroup" aria-label={legend} className={stacked ? "flex flex-col gap-1" : "grid grid-cols-2 gap-1"}>
        {options.map(([key, label], index) => (
          <button
            key={key}
            ref={(element) => {
              refs.current[index] = element;
            }}
            type="button"
            role="radio"
            aria-checked={value === key}
            tabIndex={value === key ? 0 : -1}
            className={OPTION_CLASS}
            onKeyDown={(event) => onKeyDown(event, index)}
            onClick={(event) => {
              onChange(key);
              if (event.detail > 0) onReturnFocus();
            }}
          >
            {label}
          </button>
        ))}
      </div>
    </div>
  );
}
