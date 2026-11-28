import type { ComponentProps, Ref } from "react";

import { fieldClass } from "@/components/ui/looks";

type TextFieldProps = Omit<ComponentProps<"input">, "ref"> & {
  invalid?: boolean;
  /** Codes and keys: `font-mono` 12 px. */
  mono?: boolean;
  inputRef?: Ref<HTMLInputElement>;
};

/**
 * The one text field look (`docs/ui-elements.md` §11): 28 px, white ground, 1 px
 * `--field-border`, 5 px radius, 6 px padding, 14 px; focus and invalid as the number field. The
 * caller keeps its commit rules.
 */
export function TextField({ invalid = false, mono = false, inputRef, className = "", ...input }: TextFieldProps) {
  return (
    <input
      ref={inputRef}
      type="text"
      aria-invalid={invalid || undefined}
      {...input}
      className={fieldClass({ invalid, extra: `px-1.5 ${mono ? "font-mono" : ""} ${className}`, size: mono ? "text-xs" : "text-sm" })}
    />
  );
}
