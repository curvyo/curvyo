// The rows of the Layers tab (`specs/0050-layers-and-objects-panel`): parsing what the session
// returns, the windowing arithmetic of the virtualised list, the keys of the treegrid and where a
// drag would drop. Pure, so `tests/layerRows.test.ts` covers it. The rules and the words are
// Rust's; this only reads them.

/** The fixed height of a row, px: 28 with a fine pointer, 40 with a coarse one (UX notes). */
export const ROW_HEIGHT_PX = 28;
export const COARSE_ROW_HEIGHT_PX = 40;

/** At most this many row elements are in the page (criterion 48). */
export const MAX_ROW_ELEMENTS = 60;

/** The indent per level, px, and the deepest indent drawn (criterion 6). */
export const INDENT_PX = 16;
export const MAX_INDENT_LEVELS = 6;

/** A drag starts past this many px (criterion 29). */
export const DRAG_THRESHOLD_PX = 4;

/** A collapsed container under a drag opens after this many ms (UX notes). */
export const HOVER_EXPAND_MS = 600;

/** A touch press starts a drag after this many ms (UX notes, "Touch"). */
export const LONG_PRESS_MS = 400;

/** One row as the session returns it. */
export interface LayerRowData {
  index: number;
  /** The node's id as `document.json` writes it; sent back as JSON text. */
  id: { peer: string; counter: number };
  /** `aria-level`: 1 at the top level. */
  level: number;
  kind: string;
  name: string;
  named: boolean;
  visible: boolean;
  locked: boolean;
  effectiveVisible: boolean;
  effectiveLocked: boolean;
  /** "Hidden by layer Cut", or empty. */
  hiddenBy: string;
  lockedBy: string;
  /** "60%", or null at 100 %. */
  readout: string | null;
  container: boolean;
  expanded: boolean;
  containsSelection: boolean;
  /** "selected", "active", "entered" or "". */
  mark: string;
  posinset: number;
  setsize: number;
  /** The accessible name. */
  label: string;
  /** The tooltip's second line. */
  stateLine: string;
  /** A clone row's chain line for its tooltip (`0049`); empty otherwise. */
  chainLine?: string;
  /** "Own placement, own style" for a clone with overrides (`0057` criterion 28). */
  overridesLine?: string;
  /** "Hidden" or "Locked" when a press selects nothing. */
  refusal: string;
  /** "select-contents" for a layer row, else "". */
  rowAction: string;
}

/** The Opacity field at the bottom of the tab (criterion 26). */
export interface LayerOpacityData {
  /** "3 objects", "Layer Cut", "Locked", or empty when nothing is targeted. */
  caption: string;
  /** Whether the field is in the tree. */
  field: boolean;
  /** Whether the target is a layer: the caption starts with the Layers glyph. */
  layer: boolean;
  /** The percentage (the dragged one while a drag runs), or null for Mixed. */
  value: number | null;
  /** Where the value lies on the field's scale, 0 to 1 (Rust's mapping). */
  bar: number;
  /** The caption's tooltip ("Set Style to Own to change it." for clones, `0057` criterion 16). */
  tooltip?: string;
}

/** Which buttons of the toolbar apply; the others are dimmed, never hidden. */
export interface LayerToolbarData {
  newLayer: boolean;
  deleteLayer: boolean;
  moveUp: boolean;
  moveDown: boolean;
  duplicate: boolean;
  /** Linked clone (`specs/0049-linked-clone`): the Select bar's rule. */
  linkedClone: boolean;
}

/** The whole tab as the session returns it. */
export interface LayersViewData {
  count: number;
  subject: string;
  opacity: LayerOpacityData;
  activeLayer: { peer: string; counter: number } | null;
  canDeleteLayer: boolean;
  toolbar: LayerToolbarData;
  /** The first selected row, or null (Shift+Ctrl+L scrolls to it). */
  firstSelected: number | null;
  rows: LayerRowData[];
}

/** What a panel action asks the host to show. */
export interface PanelReplyData {
  kind: "" | "hint" | "notice" | "refusal" | "announce";
  text: string;
}

const EMPTY_VIEW: LayersViewData = {
  count: 0,
  subject: "0 objects",
  opacity: { caption: "", field: false, layer: false, value: null, bar: 0 },
  activeLayer: null,
  canDeleteLayer: false,
  toolbar: {
    newLayer: true,
    deleteLayer: false,
    moveUp: false,
    moveDown: false,
    duplicate: false,
    linkedClone: false,
  },
  firstSelected: null,
  rows: [],
};

/** Reads the view JSON; an unreadable text is an empty list. */
export function parseLayersView(json: string): LayersViewData {
  try {
    return { ...EMPTY_VIEW, ...(JSON.parse(json) as Partial<LayersViewData>) };
  } catch {
    return EMPTY_VIEW;
  }
}

/** A line for the canvas notice slot, as `take_layer_notice` returns it. */
export interface LayerNoticeData {
  kind: "alert" | "status";
  text: string;
}

/** Reads the layer notice; the empty string (no notice) and anything without text are `null`, so
 * an empty notice is never shown. */
export function parseLayerNotice(json: string): LayerNoticeData | null {
  if (json === "") {
    return null;
  }
  try {
    const value = JSON.parse(json) as Partial<LayerNoticeData>;
    if (typeof value.text !== "string" || value.text === "") {
      return null;
    }
    return { kind: value.kind === "status" ? "status" : "alert", text: value.text };
  } catch {
    return null;
  }
}

/** Reads a reply JSON; the empty string and anything else are a quiet reply. */
export function parseReply(json: string): PanelReplyData {
  try {
    const value = JSON.parse(json) as Partial<PanelReplyData>;
    return { kind: value.kind ?? "", text: value.text ?? "" };
  } catch {
    return { kind: "", text: "" };
  }
}

/** The id as the JSON text the session parses. */
export function idText(id: { peer: string; counter: number } | null): string {
  return id === null ? "" : JSON.stringify(id);
}

/** A stable React key for a row. */
export function rowKey(row: LayerRowData): string {
  return `${row.id.peer}:${row.id.counter}`;
}

/** The indent of a row, px: 16 per level below the top, at most six levels (criterion 6). */
export function indentPx(level: number): number {
  return Math.min(Math.max(level - 1, 0), MAX_INDENT_LEVELS) * INDENT_PX;
}

/** The rows `first` to `last` a scroll position shows, with a few more on each side, never more
 * than [`MAX_ROW_ELEMENTS`] in all. */
export function windowRange(
  scrollTop: number,
  viewportHeight: number,
  total: number,
  rowHeight: number = ROW_HEIGHT_PX,
): { first: number; count: number } {
  if (total <= 0) {
    return { first: 0, count: 0 };
  }
  const shown = Math.ceil(viewportHeight / rowHeight) + 1;
  const overscan = Math.max(0, Math.floor((MAX_ROW_ELEMENTS - 1 - shown) / 2));
  const top = Math.max(0, Math.min(scrollTop, total * rowHeight - viewportHeight));
  const first = Math.max(0, Math.floor(top / rowHeight) - overscan);
  const count = Math.min(total - first, Math.min(MAX_ROW_ELEMENTS - 1, shown + 2 * overscan));
  return { first, count };
}

/** The rows the list reads: the scroll window, and the focused row on its own when it lies
 * outside, so focus is never lost (criterion 48) and the window always follows the scroll. The
 * window used to stretch from the focused row, capped at 60 rows, and with the focus on row 1
 * and the list scrolled far down it rendered nothing on screen (`0049` UX review must-fix 9). */
export function listWindow(
  range: { first: number; count: number },
  focused: number,
  total: number,
): { first: number; count: number; extra: number | null } {
  const inside = focused >= range.first && focused < range.first + range.count;
  return {
    first: range.first,
    count: range.count,
    extra: inside || focused >= total || total === 0 ? null : focused,
  };
}

/** `view` with the focused row read on its own (`extra`, from [`listWindow`]) added. */
export function withExtraRow(view: LayersViewData, extra: LayersViewData | null): LayersViewData {
  if (!extra || extra.rows.length === 0) {
    return view;
  }
  const known = new Set(view.rows.map((row) => row.index));
  const added = extra.rows.filter((row) => !known.has(row.index));
  return added.length === 0 ? view : { ...view, rows: [...view.rows, ...added] };
}

/** The row a key moves the roving focus to, or null for another key (criterion 38). */
export function nextRow(key: string, current: number, total: number, pageRows: number): number | null {
  if (total <= 0) {
    return null;
  }
  const clamp = (n: number) => Math.max(0, Math.min(total - 1, n));
  switch (key) {
    case "ArrowDown":
      return clamp(current + 1);
    case "ArrowUp":
      return clamp(current - 1);
    case "Home":
      return 0;
    case "End":
      return total - 1;
    case "PageDown":
      return clamp(current + pageRows);
    case "PageUp":
      return clamp(current - pageRows);
    default:
      return null;
  }
}

/** The reorder an Alt+key asks for (criterion 32), or null for another key. In the browser build
 * Alt+Home is the browser's home page and is handled before the page sees it, so there the two
 * keys are Shift+Alt+Home and Shift+Alt+End (the UX notes' fallback); the desktop windows take
 * Alt+Home and Alt+End as they are. */
export function reorderFor(key: string, shift = false, browser = false): string | null {
  if (browser && (key === "Home" || key === "End") && !shift) {
    return null;
  }
  switch (key) {
    case "ArrowUp":
      return "forward";
    case "ArrowDown":
      return "backward";
    case "Home":
      return "front";
    case "End":
      return "back";
    case "ArrowLeft":
      return "out";
    case "ArrowRight":
      return "in";
    default:
      return null;
  }
}

/** Where a drag over a row would drop: the upper quarter is above it, the lower quarter below
 * it, the middle of a container row into it; the middle of another row goes to the nearer half
 * (criterion 29). `offset` is the pointer's distance from the row's top. */
export function dropPlace(offset: number, rowHeight: number, container: boolean): "above" | "below" | "into" {
  if (container && offset >= rowHeight / 4 && offset <= (rowHeight * 3) / 4) {
    return "into";
  }
  return offset < rowHeight / 2 ? "above" : "below";
}

/** Rows to scroll by at a drag edge: one row-height zone at the top and the bottom (UX notes). */
export function edgeScroll(pointerY: number, top: number, height: number, rowHeight: number): number {
  if (pointerY < top + rowHeight) {
    return -rowHeight / 2;
  }
  if (pointerY > top + height - rowHeight) {
    return rowHeight / 2;
  }
  return 0;
}

/** The text of the drag badge for `count` rows, or empty for one. */
export function dragBadge(count: number): string {
  return count > 1 ? String(count) : "";
}

/** The button of the toolbar a key moves the roving focus to: Left and Right one place without
 * wrapping, Home and End to the ends; `null` for any other key (UX review of 2026-10-14,
 * must-fix 2). */
export function toolbarStep(key: string, index: number, count: number): number | null {
  if (count <= 0) {
    return null;
  }
  switch (key) {
    case "ArrowLeft":
      return Math.max(0, index - 1);
    case "ArrowRight":
      return Math.min(count - 1, index + 1);
    case "Home":
      return 0;
    case "End":
      return count - 1;
    default:
      return null;
  }
}
