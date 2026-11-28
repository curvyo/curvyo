import type { ReactNode, Ref } from "react";

import { NumberField } from "@/components/ui/number-field";

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
  /** Shows a value that cannot be typed over, such as a measured Reference dimension. */
  readOnly?: boolean;
  inputRef?: Ref<HTMLInputElement>;
  onChange: (text: string) => void;
  onKeyDown: (event: React.KeyboardEvent<HTMLInputElement>) => void;
  onBlur?: (event: React.FocusEvent<HTMLInputElement>) => void;
  onFocus?: (event: React.FocusEvent<HTMLInputElement>) => void;
}

/**
 * One numeric field of an entry chip: the label in its left padding, the value right-aligned in
 * tabular figures, the suffix inside the right edge, and the invalid mark
 * (`docs/ui-elements.md` §10, `NumberField`; `docs/design-system.md`, "Transform entry chip"). Shared by the move chip and the placement
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
  readOnly,
  inputRef,
  onChange,
  onKeyDown,
  onBlur,
  onFocus,
}: EntryFieldProps) {
  return (
    <NumberField
      label={label}
      labelPx={labelWidth ?? 20}
      width={width}
      unit={suffix}
      aria-label={ariaLabel}
      invalid={invalid}
      aria-describedby={invalid ? messageId : undefined}
      value={value}
      placeholder={placeholder}
      readOnly={readOnly}
      inputRef={inputRef}
      onChange={(event) => onChange(event.target.value)}
      onKeyDown={onKeyDown}
      onBlur={onBlur}
      onFocus={onFocus}
    />
  );
}
