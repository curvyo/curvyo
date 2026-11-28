import { Search, X } from "lucide-react";
import { type ComponentProps, type ReactNode, type RefObject, useRef } from "react";

import { IconButton } from "@/components/ui/icon-button";
import { ICON_PX, ICON_STROKE, fieldClass } from "@/components/ui/looks";

type SearchFieldProps = Omit<ComponentProps<"input">, "ref" | "value" | "onChange" | "aria-label"> & {
  "aria-label": string;
  value: string;
  onValue: (text: string) => void;
  /** The `Search` glyph inside the left edge (absent where the same field also holds a sample). */
  glyph?: boolean;
  /** The clear button's name ("Clear search"). */
  clearLabel?: string;
  /** The clear button is no Tab stop (the library browser: Ctrl+Backspace clears). */
  clearUntabbable?: boolean;
  /** A 24 px icon button left of the clear button (Search online). */
  action?: ReactNode;
  inputRef?: RefObject<HTMLInputElement | null>;
  /** Classes of the wrapper (its width). */
  className?: string;
};

/**
 * The search field (`docs/ui-elements.md` §11): the text field with `Search` 16 px inside the left
 * edge and a clear `X` icon button inside the right edge while it holds text; clearing puts the
 * focus back into the field.
 */
export function SearchField({
  value,
  onValue,
  glyph = true,
  clearLabel = "Clear search",
  clearUntabbable = false,
  action,
  inputRef,
  className = "",
  ...input
}: SearchFieldProps) {
  const own = useRef<HTMLInputElement | null>(null);
  const field = inputRef ?? own;
  const padRight = value ? (action ? "pr-14" : "pr-7") : "pr-1.5";
  return (
    <div className={`relative h-7 min-w-0 ${className}`}>
      {glyph ? (
        <Search
          size={ICON_PX}
          strokeWidth={ICON_STROKE}
          absoluteStrokeWidth
          aria-hidden
          className="pointer-events-none absolute top-1.5 left-2 text-[var(--panel-muted-fg)]"
        />
      ) : null}
      <input
        ref={field}
        type="text"
        spellCheck={false}
        value={value}
        onChange={(event) => onValue(event.target.value)}
        {...input}
        className={fieldClass({ extra: `w-full ${glyph ? "pl-7" : "pl-1.5"} ${padRight}` })}
      />
      {value && action ? <div className="absolute top-0.5 right-7 flex">{action}</div> : null}
      {value ? (
        <IconButton
          icon={X}
          size={24}
          aria-label={clearLabel}
          tabIndex={clearUntabbable ? -1 : undefined}
          className="absolute top-0.5 right-0.5 pointer-coarse:size-6"
          onClick={() => {
            onValue("");
            field.current?.focus();
          }}
        />
      ) : null}
    </div>
  );
}
