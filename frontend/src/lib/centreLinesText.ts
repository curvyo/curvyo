/**
 * The words of Find centre lines (`specs/0117-centre-lines` criteria 4, 12, 16 and 18, UX notes
 * §2, §3 and §5): the button's name, tooltips, refusals, the bar's fields, the info page and the
 * notice after Apply. Rust sends codes and counts; the sentences are written here, once.
 */

/** The button's label and menu item. */
export const CENTRE_LINES_LABEL = "Find centre lines…";

/** The enabled tooltip and the bar's name. */
export const CENTRE_LINES_NAME = "Find centre lines";

/** What `WasmSession.apply_centre_lines` or `centre_lines_refusal` reported. */
export interface CentreLinesResult {
  kind: string;
  converted: number;
  kept: number;
  skipped: number;
  unlinked: number;
  nodes: number;
}

/** The refusal sentence for a code, or `null` when the code is no refusal. */
export function centreLinesRefusalText(kind: string, nodes: number): string | null {
  switch (kind) {
    case "text":
      return "Convert text to path first.";
    case "image":
      return "Trace the image first.";
    case "not_filled":
      return "Select filled shapes.";
    case "failed":
      return "Centre lines failed. Nothing changed.";
    case "too_large":
      return `Too large: ${nodes.toLocaleString("en-US")} nodes. The limit is 100,000.`;
    default:
      return null;
  }
}

/** The dimmed button's tooltip: the name and the reason in lower case. */
export function centreLinesDimmedTooltip(kind: string, nodes: number): string {
  const reason = centreLinesRefusalText(kind, nodes) ?? "Select filled shapes.";
  return `${CENTRE_LINES_NAME}: ${reason.charAt(0).toLowerCase()}${reason.slice(1)}`;
}

/** The notice of one Apply or press, or `null` for a clean conversion (UX notes §5). */
export function centreLinesNotice(result: CentreLinesResult, paths?: number): { text: string; alert: boolean } | null {
  const refusal = centreLinesRefusalText(result.kind, result.nodes);
  if (refusal) {
    return { text: refusal, alert: true };
  }
  if (result.kind !== "applied" || (result.kept === 0 && result.skipped === 0 && result.unlinked === 0)) {
    return null;
  }
  const parts = [
    // The Apply tooltip counts paths; the same word here when the preview's count is known.
    paths === undefined ? [result.converted, "converted"] : [paths, paths === 1 ? "path" : "paths"],
    [result.kept, "kept filled"],
    [result.skipped, "skipped"],
  ]
    .filter(([count]) => (count as number) > 0)
    .map(([count, what]) => `${count} ${what}`);
  let text = `Centre lines: ${parts.length > 0 ? parts.join(", ") : "0 converted"}.`;
  if (result.unlinked > 0) {
    text += " Linked clones became ordinary paths.";
  }
  return { text, alert: false };
}

/** The options of the bar, remembered for the session (UX notes §3). */
export interface CentreLinesOptions {
  /** Percent, −50 to 50. */
  overlap: number;
  /** Thick parts, millimetres, 0.5 to 100 (100: no limit). */
  thickParts: number;
  /** Percent, 0 to 100: higher, fewer nodes and width points (amendment 2026-11-16). */
  optimise: number;
  keepInner: boolean;
  /** Millimetres, −5 to 5. */
  offset: number;
}

export const CENTRE_LINES_DEFAULTS: CentreLinesOptions = {
  overlap: 20,
  thickParts: 6,
  optimise: 75,
  keepInner: false,
  offset: 0.3,
};

/** A typed value clamped to its range, or `null` when it is no number. */
export function parseOption(text: string, min: number, max: number): number | null {
  const value = Number(text.replace(",", ".").replace("−", "-").trim());
  if (!Number.isFinite(value) || text.trim() === "") {
    return null;
  }
  return Math.min(max, Math.max(min, value));
}

/** What a field says about a typed text: the value to keep and the status line (`null` when it is fine). */
export interface FieldFeedback {
  value: number | null;
  message: string | null;
}

/** A typed text checked against its range: clamped with "From a to b mm" naming the field, or "Enter a number". */
export function fieldFeedback(text: string, field: { label: string; min: number; max: number; suffix: string }): FieldFeedback {
  const parsed = parseOption(text, field.min, field.max);
  if (parsed === null) {
    return { value: null, message: `${field.label}: Enter a number` };
  }
  const typed = Number(text.replace(",", ".").replace("−", "-").trim());
  const range = `From ${field.min} to ${field.max}${field.suffix ? ` ${field.suffix}` : ""}`;
  return { value: parsed, message: typed === parsed ? null : `${field.label}: ${range}` };
}


/** One value field of the Centre lines bar (UX notes §3.1). */
export interface CentreFieldSpec {
  key: "overlap" | "thickParts" | "optimise" | "offset";
  label: string;
  /** The accessible name. */
  name: string;
  unit: string;
  /** The track's ends; a typed value may go up to `typedMax`. */
  min: number;
  max: number;
  typedMax: number;
  /** Arrows, Shift+arrows, Ctrl+arrows. */
  step: number;
  bigStep: number;
  fineStep: number;
  /** Decimals shown. */
  digits: number;
  /** A log track `v = min · (max / min)^p` instead of a linear one. */
  log: boolean;
}

/** The four sliders in bar order (UX notes §3.1 table). */
export const CENTRE_FIELDS: readonly CentreFieldSpec[] = [
  { key: "overlap", label: "Overlap", name: "Overlap, percent of the line width", unit: "%", min: -50, max: 50, typedMax: 50, step: 1, bigStep: 10, fineStep: 1, digits: 0, log: false },
  { key: "thickParts", label: "Thick parts", name: "Thick parts, millimetres", unit: "mm", min: 0.5, max: 20, typedMax: 100, step: 0.1, bigStep: 1, fineStep: 0.01, digits: 1, log: true },
  { key: "optimise", label: "Optimise", name: "Optimise, percent", unit: "%", min: 0, max: 100, typedMax: 100, step: 1, bigStep: 10, fineStep: 1, digits: 0, log: false },
  { key: "offset", label: "Offset", name: "Inner area offset, millimetres", unit: "mm", min: -5, max: 5, typedMax: 5, step: 0.1, bigStep: 1, fineStep: 0.01, digits: 2, log: false },
];

/** The field of `key`. */
export function centreField(key: CentreFieldSpec["key"]): CentreFieldSpec {
  // invariant: every key has a field.
  return CENTRE_FIELDS.find((field) => field.key === key) as CentreFieldSpec;
}

/** `value` on the grid of `step`, without float noise. */
function onGrid(value: number, step: number): number {
  return Number((Math.round(value / step) * step).toFixed(6));
}

/** The position 0 to 1 of `value` on the field's track (1 beyond its end). */
export function centreFieldPosition(field: CentreFieldSpec, value: number): number {
  const p = field.log ? Math.log(value / field.min) / Math.log(field.max / field.min) : (value - field.min) / (field.max - field.min);
  return Math.min(1, Math.max(0, p));
}

/** The value at position `p` (0 to 1) of the track, on the grid's step. */
export function centreFieldValueAt(field: CentreFieldSpec, p: number, grid: "normal" | "coarse" | "fine"): number {
  const q = Math.min(1, Math.max(0, p));
  const raw = field.log ? field.min * (field.max / field.min) ** q : field.min + q * (field.max - field.min);
  const step = grid === "fine" ? field.fineStep : grid === "coarse" ? field.bigStep : field.step;
  return Math.min(field.max, Math.max(field.min, onGrid(raw, step)));
}

/** `steps` arrow steps from `value` (Shift ten times, Ctrl a tenth), clamped to the typed range. */
export function centreFieldStep(field: CentreFieldSpec, value: number, steps: number, grid: "normal" | "coarse" | "fine"): number {
  const step = grid === "fine" ? field.fineStep : grid === "coarse" ? field.bigStep : field.step;
  return Math.min(field.typedMax, Math.max(field.min, onGrid(value + steps * step, field.fineStep)));
}

/** How the field shows `value`: "20", "6.0", "0.30" without trailing noise ("6.25" typed). */
export function centreFieldText(field: CentreFieldSpec, value: number): string {
  const shown = value.toFixed(field.digits);
  const typed = value.toFixed(2);
  return Number(typed) !== Number(shown) ? String(Number(typed)) : shown.replace("-", "−");
}

/** A typed value, clamped to the typed range, or `null` for no number. */
export function centreFieldParse(field: CentreFieldSpec, text: string): number | null {
  return parseOption(text.replace(/(mm|%)\s*$/i, ""), field.min, field.typedMax);
}

/** The About centre lines tab (UX notes §3.2). */
export const CENTRE_INFO_TAB = "centre-info";
export const CENTRE_INFO_TITLE = "About centre lines";
export const CENTRE_INFO_PAGE_ID = "centre-info-page";
export const CENTRE_INFO_INTRO =
  "Find centre lines turns filled line art into paths along the middle of each line, with a width that follows the line. Press Enter or Apply to replace the selected shapes.";
/** The terms in bar order with their texts (at most two sentences each). */
export const CENTRE_INFO_ENTRIES: readonly { term: string; text: string }[] = [
  { term: "Overlap", text: "How far a side line reaches into the line it joins, in percent of that line's width. 0\u00a0% touches its edge; a negative value leaves a gap." },
  { term: "Thick parts", text: "A part wider than this stays filled as it is; the rest becomes lines. Raise it to turn broad bands into lines too." },
  { term: "Optimise", text: "Higher gives fewer nodes and a looser fit; lower follows the shape more closely. Corners keep their detail at every setting." },
  { term: "Keep inner areas", text: "Also turns each hole inside a shape into a grey filled shape of its own. Off leaves the holes empty." },
  { term: "Offset", text: "Moves the edge of each inner area inward, in mm. A negative value moves it outward, under the line." },
];

/** The Apply tooltip's second line and the announcement of a preview (UX notes §3.3, §4). */
export function centrePreviewCounts(paths: number, nodes: number, kept: number): string {
  if (paths === 0) {
    return CENTRE_NOTHING_TO_CONVERT;
  }
  const many = (n: number, one: string, more: string) => `${n.toLocaleString("en-US")} ${n === 1 ? one : more}`;
  const parts = [many(paths, "path", "paths"), many(nodes, "node", "nodes")];
  if (kept > 0) {
    parts.push(`${kept.toLocaleString("en-US")} kept filled`);
  }
  return parts.join(", ");
}

/** The dimmed Apply's reason when nothing would be converted. */
export const CENTRE_NOTHING_TO_CONVERT = "No centre lines: every shape stays filled.";

/** The announcement while a preview is computed. */
export const CENTRE_UPDATING = "Updating preview";

/** Shift+F10 or the Menu key: the keyboard's way to the context menu. */
export function isContextMenuKey(event: { key: string; shiftKey: boolean; ctrlKey?: boolean; altKey?: boolean }): boolean {
  return event.key === "ContextMenu" || (event.key === "F10" && event.shiftKey && !event.ctrlKey && !event.altKey);
}

/** Focuses the bar's Overlap field (opened by the keyboard, UX notes §7). */
export function focusCentreLinesBar() {
  window.requestAnimationFrame(() => {
    document.querySelector<HTMLElement>("[data-centre-lines-bar] [role=spinbutton]")?.focus();
  });
}

/**
 * Whether an Escape key press cancels the bar: always, in a number entry too (the entry closes
 * with it). A slider or a tooltip layer may have prevented the default already (a dismissed
 * tooltip must not cost a second press).
 */
export function centreEscapeCancels(event: { key: string; defaultPrevented: boolean; target: EventTarget | { tagName?: string } | null }): boolean {
  return event.key === "Escape";
}
