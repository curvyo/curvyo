/** The 28 px (36 px coarse) symbol button of the Machine block and the field frame (`docs/design-system.md`). */
export const ICON_BUTTON =
  "inline-flex h-7 w-7 shrink-0 items-center justify-center rounded-md text-[var(--toolbar-icon)] outline-none hover:bg-[var(--editor-accent-hover)] focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)] pointer-coarse:size-9";

/** The armed symbol button of every other armed press of the block: filled `--destructive`. */
export const ARMED_DESTRUCTIVE_BUTTON = ICON_BUTTON.replace(
  "hover:bg-[var(--editor-accent-hover)]",
  "bg-[var(--destructive)] text-white",
);

export const FIELD =
  "flex h-7 items-center gap-1 rounded-[5px] border border-[var(--field-border)] bg-white px-2 text-sm focus-within:ring-2 focus-within:ring-[var(--editor-accent)]";

/** A 16 px Lucide glyph at 1.5 px absolute stroke (`docs/design-system.md`). */
export const GLYPH = { size: 16, strokeWidth: 1.5, absoluteStrokeWidth: true, "aria-hidden": true } as const;

/** The 12 px glyph before a state line. */
export const SMALL = { size: 12, strokeWidth: 1.5, absoluteStrokeWidth: true, "aria-hidden": true } as const;
