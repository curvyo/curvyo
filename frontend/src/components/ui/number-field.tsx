import type { ComponentProps, ReactNode, Ref } from "react";

import { FIELD_LABEL, FIELD_UNIT, fieldClass } from "@/components/ui/looks";
import { fieldLabelWidth } from "@/lib/fieldLabelWidth";

type NumberFieldProps = Omit<ComponentProps<"input">, "className" | "style" | "type" | "ref"> & {
  /** The muted 12 px word inside the left edge ("Start"), or a control placed there (the r/⌀
   * switch); none where the group has a label of its own. */
  label?: ReactNode;
  /** The least left padding, px: a node label's width, or a column of fields kept aligned. */
  labelPx?: number;
  /** The unit inside the right edge ("mm", "°", "%"). */
  unit?: string;
  /** The room kept free for the unit, px (32 for two or more characters, 24 for one). */
  unitPx?: number;
  /** The unit's distance from the right edge, px (6). */
  unitRight?: number;
  /** The unit in the 14 px value size instead of 12 px: the panel's fields, beside `ValueField`. */
  largeUnit?: boolean;
  /** The "limited" tag before the unit (`docs/ui-elements.md` §10). */
  limited?: boolean;
  invalid?: boolean;
  /** Width of the field, px; the parent's width when absent. */
  width?: number;
  /** Values are right-aligned; a hex or a text value left-aligned. */
  align?: "left" | "right";
  /** 13 px value for a nine-character hex in 84 px (the one kept exception). */
  compact?: boolean;
  /** 24 px instead of 28: a field in a compact chip (36 px for a coarse pointer). */
  small?: boolean;
  /** The placeholder is a value worked out from the others (`--field-derived-fg`). */
  derivedPlaceholder?: boolean;
  /** Native number input (arrow keys step through the browser, no steppers drawn): the bar fields
   * whose accepted step behaviour is the browser's. */
  numeric?: boolean;
  /** Greyed (`aria-disabled` in the bars): 40 % on the value, label and unit. */
  dimmed?: boolean;
  /** Extra classes on the input (a hook class such as `style-field`). */
  inputClassName?: string;
  inputRef?: Ref<HTMLInputElement>;
  /** Overlays that hang from the field: the message chip. */
  children?: ReactNode;
};

/**
 * The one number field look (`docs/ui-elements.md` §10): 28 px, white, 1 px `--field-border`,
 * the label inside the left edge, the value right-aligned 14 px tabular, the unit inside the right
 * edge, the "limited" tag before it. A text input (no native steppers); the caller keeps the
 * commit rules (Enter, Escape, steps), so each field's accepted behaviour stays its own.
 */
export function NumberField({
  label,
  labelPx,
  unit,
  unitPx,
  unitRight = 6,
  largeUnit = false,
  limited = false,
  invalid = false,
  width,
  align = "right",
  compact = false,
  dimmed = false,
  small = false,
  derivedPlaceholder = false,
  numeric = false,
  inputClassName = "",
  inputRef,
  children,
  ...input
}: NumberFieldProps) {
  // The label, 8 px of air and the value never touch (0159 W19): the padding follows the label's
  // measured width.
  const measured = typeof label === "string" ? Math.ceil(6 + fieldLabelWidth(label) + 8) : 0;
  const padLeft = label === undefined ? 6 : Math.max(labelPx ?? 0, measured);
  const unitRoom = unit ? (unitPx ?? (unit.length > 1 ? 32 : 24)) : 6;
  const padRight = unitRoom + (limited ? 46 : 0);
  const fade = dimmed ? "opacity-40" : "";
  return (
    <span className="relative block" style={width === undefined ? undefined : { width }}>
      {typeof label === "string" ? (
        <span aria-hidden className={`${FIELD_LABEL} ${fade}`}>
          {label}
        </span>
      ) : (
        label
      )}
      <input
        ref={inputRef}
        type={numeric ? "number" : "text"}
        autoComplete="off"
        spellCheck={false}
        inputMode="decimal"
        aria-invalid={invalid || undefined}
        {...input}
        className={fieldClass({
          invalid,
          extra: `w-full tabular-nums ${align === "right" ? "text-right" : "text-left"} ${fade} ${inputClassName}`,
          size: compact ? "text-[13px]" : "text-sm",
          height: small ? "h-6 pointer-coarse:h-9" : "h-7",
          placeholder: derivedPlaceholder ? "placeholder:text-[var(--field-derived-fg)]" : undefined,
        })}
        style={{ paddingLeft: padLeft, paddingRight: padRight }}
      />
      {limited ? (
        <span aria-hidden className={`${FIELD_UNIT} text-xs ${fade}`} style={{ right: unitRoom }}>
          limited
        </span>
      ) : null}
      {unit ? (
        <span
          aria-hidden
          // The degree sign is a raised glyph a third of the size of "mm": it gets the size that
          // matches "mm" visually (`0066` UX notes revision 2 review, 5c).
          className={`${FIELD_UNIT} ${unit === "°" ? "text-base leading-none" : largeUnit ? "text-sm" : "text-xs"} ${fade}`}
          style={{ right: unitRight }}
        >
          {unit}
        </span>
      ) : null}
      {children}
    </span>
  );
}
