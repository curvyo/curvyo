import type { ComponentProps, Ref } from "react";

import { renameFieldClass } from "@/components/ui/looks";

type RenameFieldProps = Omit<ComponentProps<"input">, "ref"> & {
  /** The row's height minus 4 px: 24 in a 28 px row, 20 in a 24 px row. */
  height?: 20 | 24;
  invalid?: boolean;
  inputRef?: Ref<HTMLInputElement>;
};

/**
 * The rename field (`docs/ui-elements.md` §11): replaces a row's name in place, 1 px
 * `--editor-accent` border, 14 px. The row keeps its rules (Enter writes, Escape cancels, blur
 * commits a valid name).
 */
export function RenameField({ height = 24, invalid = false, inputRef, className = "", ...input }: RenameFieldProps) {
  return (
    <input
      ref={inputRef}
      type="text"
      autoComplete="off"
      spellCheck={false}
      aria-invalid={invalid || undefined}
      {...input}
      className={renameFieldClass(invalid, `${height === 24 ? "h-6" : "h-5"} ${className}`)}
    />
  );
}
