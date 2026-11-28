// The class strings and sizes of the shared looks (`docs/ui-elements.md` §0 to §11): for the
// components in this folder and for the few elements styled without them (a Radix trigger, a
// list's own row button, an armed action).

/** The three button looks of `docs/ui-elements.md` §1 to §3. */
export type ButtonVariant = "primary" | "secondary" | "destructive";
/** Panel buttons have 12 px side padding, bar buttons 8 px; both are 28 px high (36 coarse). */
export type ButtonSize = "panel" | "bar";

/** Size, radius, focus and disabled rules shared by every button look. */
export const BUTTON_BASE =
  "inline-flex h-7 shrink-0 items-center justify-center gap-1 rounded-[5px] border text-sm whitespace-nowrap outline-none select-none focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)] focus-visible:ring-offset-1 focus-visible:ring-offset-[var(--toolbar-bg)] pointer-coarse:h-9 aria-disabled:cursor-default aria-disabled:opacity-40 disabled:opacity-40 [&_svg]:pointer-events-none [&_svg]:shrink-0";

/** The look of each variant, without size. */
export const BUTTON_LOOK: Record<ButtonVariant, string> = {
  primary:
    "border-[var(--toolbar-icon-active-bg)] bg-[var(--toolbar-icon-active-bg)] text-[var(--toolbar-icon-active-fg)] not-aria-disabled:hover:brightness-110",
  secondary:
    "border-[var(--field-border)] bg-transparent text-[var(--toolbar-icon)] not-aria-disabled:hover:bg-[var(--editor-accent-hover)]",
  destructive:
    "border-[var(--destructive)] bg-[var(--background)] text-[var(--destructive)] not-aria-disabled:hover:bg-[var(--destructive)] not-aria-disabled:hover:text-white focus-visible:bg-[var(--destructive)] focus-visible:text-white",
};

const PADDING: Record<ButtonSize, string> = { panel: "px-3", bar: "px-2" };

/** The full class string of a button look: for the few places that style a foreign element. */
export function buttonClass(variant: ButtonVariant, size: ButtonSize = "panel", extra = "") {
  return `${BUTTON_BASE} ${BUTTON_LOOK[variant]} ${PADDING[size]} ${extra}`;
}

/** `plain`: bars' icon groups and list-row actions; `outlined`: panel header actions. */
export type IconButtonLook = "plain" | "outlined";
/** 24 in a list row, 28 in bars and panel headers, 40 in the rail. */
export type IconButtonSize = 24 | 28 | 40;

const SIZE: Record<IconButtonSize, string> = {
  24: "size-6 pointer-coarse:size-9",
  28: "size-7 pointer-coarse:size-9",
  40: "size-10",
};

const LOOK: Record<IconButtonLook, string> = {
  plain: "border-transparent bg-transparent",
  outlined: "border-[var(--field-border)] bg-transparent",
};

/** The class string of an icon button, for an element that is not an `IconButton` (a Radix
 * trigger rendered `asChild`). */
export function iconButtonClass(look: IconButtonLook = "plain", size: IconButtonSize = 28, extra = "") {
  return `inline-flex shrink-0 items-center justify-center rounded-[5px] border text-[var(--toolbar-icon)] outline-none not-aria-disabled:hover:bg-[var(--editor-accent-hover)] focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)] focus-visible:ring-offset-1 focus-visible:ring-offset-[var(--toolbar-bg)] aria-disabled:cursor-default aria-disabled:opacity-40 disabled:opacity-40 ${SIZE[size]} ${LOOK[look]} ${extra}`;
}

/** The text link look, for a `<button>` that opens a page through the host (`openLicence`). */
export const TEXT_LINK_CLASS =
  "inline-flex items-center gap-1 rounded-[2px] text-[var(--editor-accent)] outline-none hover:underline focus-visible:underline focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)]";


/** Where a toggle button sits: a full-width panel row, a labelled bar button, or icon-only. */
export type ToggleLook = "panel" | "bar" | "icon";

/** The pressed state of every toggle (`docs/ui-elements.md` §0 "States", Pressed): the one "on"
 * colour, keyed on `aria-pressed`, which no tooltip trigger rewrites. */
export const PRESSED =
  "aria-pressed:border-[var(--toolbar-icon-active-bg)] aria-pressed:bg-[var(--toolbar-icon-active-bg)] aria-pressed:text-[var(--toolbar-icon-active-fg)]";

const TOGGLE_LOOK: Record<ToggleLook, string> = {
  panel: `${BUTTON_BASE} w-full justify-start border-[var(--field-border)] px-2 text-[var(--toolbar-icon)]`,
  bar: `${BUTTON_BASE} border-[var(--field-border)] px-2 text-[var(--toolbar-icon)]`,
  icon: `${BUTTON_BASE} size-7 border-transparent px-0 text-[var(--toolbar-icon)] pointer-coarse:size-9`,
};

/** The class string of a toggle button (`docs/ui-elements.md` §6): off is the secondary button
 * (or the plain icon button), on is "Pressed"; armed fills it `--destructive`. */
export function toggleClass(look: ToggleLook, armed = false, extra = "") {
  const state = armed
    ? "border-[var(--destructive)] bg-[var(--destructive)] text-white"
    : `bg-transparent not-aria-pressed:not-aria-disabled:hover:bg-[var(--editor-accent-hover)] ${PRESSED}`;
  return `${TOGGLE_LOOK[look]} ${state} ${extra}`;
}

/** The tick box of a list row (`docs/ui-elements.md` §8): 16 px, 1 px border, 3 px radius,
 * filled `--toolbar-icon-active-bg` when ticked or partly ticked. */
export function checkboxClass(checked: boolean | "mixed", extra = "") {
  const on = checked !== false;
  return `flex size-4 shrink-0 items-center justify-center rounded-[3px] border text-[var(--toolbar-icon-active-fg)] outline-none focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)] ${
    on
      ? "border-[var(--toolbar-icon-active-bg)] bg-[var(--toolbar-icon-active-bg)]"
      : "border-[var(--toolbar-icon)] bg-white"
  } ${extra}`;
}

/** A segmented control's strip (`docs/ui-elements.md` §7): one 1 px bordered strip, the items
 * 1 px apart (the strip's ground shows between them), in a row, a column or a grid. */
export const SEGMENT_STRIP =
  "gap-px overflow-hidden rounded-[5px] border border-[var(--field-border)] bg-[var(--field-border)]";

/** One item of a segmented control: `role="radio"`, the chosen one "Pressed". */
export const SEGMENT_ITEM =
  "flex h-7 min-w-0 items-center justify-center gap-1 bg-[var(--panel-bg)] px-2 text-sm text-[var(--toolbar-icon)] outline-none hover:bg-[color-mix(in_srgb,var(--editor-accent)_20%,var(--panel-bg))] focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-[var(--editor-accent)] aria-checked:bg-[var(--toolbar-icon-active-bg)] aria-checked:text-[var(--toolbar-icon-active-fg)] pointer-coarse:h-9";

/** The one field look (`docs/ui-elements.md` §10, §11): 28 px, white ground, 1 px
 * `--field-border`, 5 px radius, 14 px; focus `--editor-accent` border plus 1 px inset; invalid
 * the `--field-invalid` border plus 1 px inset (a 2 px look). Padding is the caller's. */
export function fieldClass({
  invalid = false,
  extra = "",
  size = "text-sm",
  height = "h-7",
  placeholder = "placeholder:text-[var(--field-placeholder)]",
}: {
  invalid?: boolean;
  extra?: string;
  /** `text-sm`; `text-xs` for a code in `font-mono`. */
  size?: string;
  /** `h-7`; `h-6` inside a 28 px row or a compact chip. */
  height?: string;
  placeholder?: string;
} = {}) {
  const state = invalid
    ? "border-[var(--field-invalid)] shadow-[inset_0_0_0_1px_var(--field-invalid)]"
    : "border-[var(--field-border)] focus:border-[var(--editor-accent)] focus:shadow-[inset_0_0_0_1px_var(--editor-accent)]";
  return `${height} min-w-0 rounded-[5px] border bg-white ${size} text-[var(--toolbar-icon)] outline-none ${placeholder} ${state} ${extra}`;
}

/** The rename field (`docs/ui-elements.md` §11): replaces a row's name in place, the row's height
 * minus 4 px, 1 px `--editor-accent` border; invalid as every field. Height is the caller's. */
export function renameFieldClass(invalid = false, extra = "") {
  const state = invalid
    ? "border-[var(--field-invalid)] shadow-[inset_0_0_0_1px_var(--field-invalid)]"
    : "border-[var(--editor-accent)]";
  return `min-w-0 rounded-[5px] border bg-white px-1.5 text-sm text-[var(--toolbar-icon)] outline-none ${state} ${extra}`;
}

/** The muted 12 px word inside a field's left edge ("Start", "Scan angle"). */
export const FIELD_LABEL =
  "pointer-events-none absolute top-1/2 left-1.5 -translate-y-1/2 text-xs whitespace-nowrap text-[var(--panel-muted-fg)]";

/** The unit inside a field's right edge ("mm", "%"); its size (12 px, or 14 px beside a panel
 * value) is the caller's. */
export const FIELD_UNIT =
  "pointer-events-none absolute top-1/2 right-1.5 -translate-y-1/2 whitespace-nowrap text-[var(--panel-muted-fg)]";

/** Glyph sizes of `docs/ui-elements.md` §0 "Icon rule": 16 px in controls, 20 px in the rail,
 * 12 px for disclosure chevrons and the glyph of a 12 px notice line; 1.5 px strokes. */
export const ICON_PX = 16;
export const RAIL_ICON_PX = 20;
export const SMALL_ICON_PX = 12;
export const ICON_STROKE = 1.5;

/** A field whose label and unit sit in its own flex row (the machine settings): the field look
 * on the frame, the focus border when the input inside has focus. */
export function fieldFrameClass(invalid = false, extra = "") {
  const state = invalid
    ? "border-[var(--field-invalid)] shadow-[inset_0_0_0_1px_var(--field-invalid)]"
    : "border-[var(--field-border)] focus-within:border-[var(--editor-accent)] focus-within:shadow-[inset_0_0_0_1px_var(--editor-accent)]";
  return `flex h-7 min-w-0 items-center rounded-[5px] border bg-white text-sm text-[var(--toolbar-icon)] ${state} ${extra}`;
}

/** `fieldFrameClass()` with its usual 6 px padding and 4 px gap. */
export const FIELD_FRAME = fieldFrameClass(false, "gap-1 px-1.5");

/** The label inside a `FIELD_FRAME`: 12 px muted, as `FIELD_LABEL`. */
export const FIELD_FRAME_LABEL = "shrink-0 text-xs whitespace-nowrap text-[var(--panel-muted-fg)]";

/** A full-width text field that shows its invalid look from `aria-invalid` (the Share tab, the
 * Sketch panel). */
export const FIELD_WIDE = fieldClass({
  extra: "w-full px-1.5 aria-invalid:border-[var(--field-invalid)] aria-invalid:shadow-[inset_0_0_0_1px_var(--field-invalid)]",
});

/** The props of a control's Lucide glyph (`docs/ui-elements.md` §0 "Icon rule"): 16 px, 1.5 px
 * absolute stroke. Spread on the icon: `<Plus {...GLYPH} aria-hidden />`. */
export const GLYPH = { size: ICON_PX, strokeWidth: ICON_STROKE, absoluteStrokeWidth: true } as const;
/** A disclosure chevron or the glyph of a 12 px line: 12 px. */
export const SMALL_GLYPH = { size: SMALL_ICON_PX, strokeWidth: ICON_STROKE, absoluteStrokeWidth: true } as const;
/** A rail button's glyph: 20 px. */
export const RAIL_GLYPH = { size: RAIL_ICON_PX, strokeWidth: ICON_STROKE, absoluteStrokeWidth: true } as const;

/** A panel section heading (`docs/ui-elements.md` §17): 12 px semibold `--toolbar-icon`. */
export const SECTION_HEADING = "text-xs font-semibold text-[var(--toolbar-icon)]";

/** A panel row (`docs/ui-elements.md` §17 "Panel row"): 28 px, the 60 px label column, 8 px, the
 * 176 px control column. */
export const PANEL_ROW = "flex h-7 items-center gap-2";
/** The panel's one label column: 60 px, 14 px `--toolbar-icon`. */
export const PANEL_LABEL = "w-[60px] shrink-0 text-sm text-[var(--toolbar-icon)]";
/** The panel's control column, px. */
export const PANEL_CONTROL_PX = 176;
