import { Check, Minus } from "lucide-react";
import type { ComponentProps } from "react";

import { checkboxClass, SMALL_GLYPH } from "@/components/ui/looks";

type CheckboxProps = Omit<ComponentProps<"button">, "children" | "role"> & {
  /** Ticked, not ticked, or "mixed" for a row whose items are partly ticked. */
  checked: boolean | "mixed";
};

/**
 * The tick of a list row (`docs/ui-elements.md` §8): ticks an item for a later act. A
 * `role="checkbox"` button with `aria-checked` true, false or mixed; Space and Enter press it.
 * An on/off setting is a `ToggleButton`, not this.
 */
export function Checkbox({ checked, className = "", type = "button", ...props }: CheckboxProps) {
  return (
    <button type={type} role="checkbox" aria-checked={checked} className={checkboxClass(checked, className)} {...props}>
      {checked === "mixed" ? (
        <Minus {...SMALL_GLYPH} aria-hidden />
      ) : checked ? (
        <Check {...SMALL_GLYPH} aria-hidden />
      ) : null}
    </button>
  );
}
