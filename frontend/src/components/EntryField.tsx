import type { ReactNode, Ref } from "react";

interface EntryFieldProps {
  /** The visible label inside the left padding: text, or a button (the r/⌀ switch). */
  label: ReactNode;
  /** The field's width in CSS pixels. */
  width: number;
  /** The left padding that holds the label, in CSS pixels; the size chip's 20 px by default. */
  labelWidth?: number;
  /** The unit symbol or "°" inside the right edge. */
  suffix: string;
  ariaLabel: string;
  invalid: boolean;
  /** The id of the message line that describes a refused value. */
  messageId: string;
  value: string;
  placeholder?: string;
  inputRef?: Ref<HTMLInputElement>;
  onChange: (text: string) => void;
  onKeyDown: (event: React.KeyboardEvent<HTMLInputElement>) => void;
  onBlur?: (event: React.FocusEvent<HTMLInputElement>) => void;
  onFocus?: (event: React.FocusEvent<HTMLInputElement>) => void;
}

/**
 * One numeric field of an entry chip: the label in its left padding, the value right-aligned in
 * tabular figures, the suffix inside the right edge, and the invalid mark
 * (`docs/design-system.md`, "Transform entry chip"). Shared by the move chip and the placement
 * chip (`specs/0052-click-to-create-shapes/adrs.md` decision 1).
 */
export function EntryField({
  label,
  width,
  labelWidth,
  suffix,
  ariaLabel,
  invalid,
  messageId,
  value,
  placeholder,
  inputRef,
  onChange,
  onKeyDown,
  onBlur,
  onFocus,
}: EntryFieldProps) {
  return (
    <div className="relative" style={{ width }}>
      {typeof label === "string" ? (
        <span
          aria-hidden
          className="pointer-events-none absolute top-1/2 left-1.5 -translate-y-1/2 text-xs"
        >
          {label}
        </span>
      ) : (
        label
      )}
      <input
        ref={inputRef}
        type="text"
        inputMode="decimal"
        autoComplete="off"
        spellCheck={false}
        aria-label={ariaLabel}
        aria-invalid={invalid ? true : undefined}
        aria-describedby={invalid ? messageId : undefined}
        value={value}
        placeholder={placeholder}
        onChange={(event) => onChange(event.target.value)}
        onKeyDown={onKeyDown}
        onBlur={onBlur}
        onFocus={onFocus}
        className={`h-7 w-full rounded-[5px] border bg-white pr-8 text-right text-sm tabular-nums outline-none placeholder:text-[var(--panel-muted-fg)] ${
          labelWidth === undefined ? "pl-5" : ""
        } ${
          invalid
            ? "border-[var(--field-invalid)] shadow-[inset_0_0_0_1px_var(--field-invalid)]"
            : "border-[color-mix(in_srgb,var(--toolbar-icon)_60%,transparent)] focus:border-[var(--editor-accent)] focus:shadow-[inset_0_0_0_1px_var(--editor-accent)]"
        }`}
        style={{
          color: "var(--toolbar-icon)",
          ...(labelWidth === undefined ? {} : { paddingLeft: labelWidth }),
        }}
      />
      <span
        aria-hidden
        className="pointer-events-none absolute top-1/2 right-1.5 -translate-y-1/2 text-xs"
        style={{ opacity: 0.7 }}
      >
        {suffix}
      </span>
    </div>
  );
}
