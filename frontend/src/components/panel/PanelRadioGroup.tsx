import { type KeyboardEvent, useRef } from "react";
import { SEGMENT_ITEM, SEGMENT_STRIP } from "@/components/ui/looks";


/**
 * A panel radio group in the segmented-control look (`docs/ui-elements.md` §7; `specs/0090-grid`
 * UX notes §2 and §3): one bordered strip, one Tab stop on the checked option, arrows move and
 * choose, Home and End jump, a pointer choice returns focus to the canvas.
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
    const jump = event.key === "Home" ? 0 : event.key === "End" ? options.length - 1 : undefined;
    if (step === undefined && jump === undefined) return;
    event.preventDefault();
    const next = jump ?? (index + (step ?? 0) + options.length) % options.length;
    onChange(options[next][0]);
    refs.current[next]?.focus();
  };
  return (
    <div className="min-w-0">
      <div className="mb-2 text-sm" aria-hidden>{legend}</div>
      <div role="radiogroup" aria-label={legend} className={`${stacked ? "flex flex-col" : "grid grid-cols-2"} ${SEGMENT_STRIP}`}>
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
            className={SEGMENT_ITEM}
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
