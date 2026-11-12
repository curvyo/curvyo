import { useRef, type KeyboardEvent } from "react";

const SEGMENT =
  "h-7 rounded border border-[color-mix(in_srgb,var(--toolbar-icon)_60%,transparent)] text-sm text-[var(--toolbar-icon)] hover:bg-[var(--editor-accent-hover)] aria-checked:bg-[var(--toolbar-icon-active-bg)] aria-checked:text-[var(--toolbar-icon-active-fg)] focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)]";

interface SegmentGroupProps<T extends string> {
  /** The radiogroup's accessible name ("Kind"). */
  name: string;
  options: readonly (readonly [T, string])[];
  value: T;
  /** Each segment's width in px (78 for Kind, 112 for Add to). */
  width: number;
  onChange: (value: T) => void;
  onReturnFocus: () => void;
}

/**
 * A row of text segments as a radiogroup (`specs/0028` `ux-notes.md` 6.2, 9): one Tab stop on the
 * checked one, arrows move and choose, a pointer choice returns the focus to the canvas.
 */
export function SegmentGroup<T extends string>({ name, options, value, width, onChange, onReturnFocus }: SegmentGroupProps<T>) {
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
    <div role="radiogroup" aria-label={name} className="flex gap-1">
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
          style={{ width }}
          className={SEGMENT}
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
  );
}
