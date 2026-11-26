/**
 * Which typed entry chips end with the Copy check (`specs/0073-parameter-handle-clone`
 * criterion 11): the chips of a parameter handle, whose Enter can write the value to a copy.
 * The size, angle, skew and fixture chips have none.
 */
const PARAMETER_KINDS: ReadonlySet<string> = new Set([
  "curve",
  "arc-angle",
  "corner-radius",
  "inner-ratio",
  "rounded",
]);

/** Whether the chip of `kind` shows the Copy check. */
export function entryHasCopyCheck(kind: string): boolean {
  return PARAMETER_KINDS.has(kind);
}

