// The fixed Pen layout's rules, pure (`specs/0158-pen-display` slice 2b criteria 1, 4, 5, 7, 8; UX
// 10.1 to 10.4): which controls the rail and the corner hold and what a press does, the switches
// read from the pen page's query and written back, the second column and the touch rule.

/** The pen window's own tool: the Pencil, the Pencil's Eraser mode, Select or Node. */
export type PenTool = "pencil" | "eraser" | "select" | "node";

/** A card of the rail, or the corner. */
export type PenGroup = "Tools" | "Edit" | "View" | "Corner";

/** One control of the layout. */
export interface PenControl {
  id: string;
  label: string;
  group: PenGroup;
  /** A toggle's or radio item's state; `undefined` for a plain button. */
  pressed?: boolean;
  disabled?: boolean;
  press: () => void;
}

/** What the controls show. */
export interface PenLayoutState {
  tool: PenTool;
  undo: string | null;
  redo: string | null;
  canDelete: boolean;
  panning: boolean;
  follow: boolean;
  canvasOnly: boolean;
}

/** What the controls do. */
export interface PenLayoutActions {
  tool: (tool: PenTool) => void;
  undo: () => void;
  redo: () => void;
  delete: () => void;
  zoom: (zoomIn: boolean) => void;
  fit: () => void;
  pan: () => void;
  follow: (on: boolean) => void;
  canvasOnly: (on: boolean) => void;
}

const TOOLS: readonly { tool: PenTool; label: string }[] = [
  { tool: "pencil", label: "Pencil tool (P)" },
  { tool: "eraser", label: "Eraser" },
  { tool: "select", label: "Select tool (S or Esc)" },
  { tool: "node", label: "Node tool (N)" },
];

/** Every control of the layout in its order: Tools, Edit, View, then the corner (criterion 1).
 * A tap on the pressed Eraser returns to the Pencil (UX 10.2). */
export function penControls(state: PenLayoutState, actions: PenLayoutActions): PenControl[] {
  const tools: PenControl[] = TOOLS.map(({ tool, label }) => ({
    id: tool,
    label,
    group: "Tools",
    pressed: state.tool === tool,
    press: () => actions.tool(tool === "eraser" && state.tool === "eraser" ? "pencil" : tool),
  }));
  return [
    ...tools,
    {
      id: "undo",
      label: state.undo ? `Undo: ${state.undo} (Ctrl+Z)` : "Undo (Ctrl+Z)",
      group: "Edit",
      disabled: state.undo === null,
      press: actions.undo,
    },
    {
      id: "redo",
      label: state.redo ? `Redo: ${state.redo} (Ctrl+Shift+Z)` : "Redo (Ctrl+Shift+Z)",
      group: "Edit",
      disabled: state.redo === null,
      press: actions.redo,
    },
    { id: "delete", label: "Delete (Delete)", group: "Edit", disabled: !state.canDelete, press: actions.delete },
    { id: "zoom-in", label: "Zoom in", group: "View", press: () => actions.zoom(true) },
    { id: "zoom-out", label: "Zoom out", group: "View", press: () => actions.zoom(false) },
    { id: "fit", label: "Fit page", group: "View", press: actions.fit },
    { id: "pan", label: "Pan", group: "View", pressed: state.panning, press: actions.pan },
    { id: "follow", label: "Follow view", group: "View", pressed: state.follow, press: () => actions.follow(!state.follow) },
    {
      id: "canvas-only",
      label: state.canvasOnly ? "Show tools" : "Canvas only",
      group: "Corner",
      pressed: state.canvasOnly,
      press: () => actions.canvasOnly(!state.canvasOnly),
    },
  ];
}

/** The pen window's remembered switches (`pen_window.json`, ADR 0096 A4). */
export interface PenFlags {
  followView: boolean;
  canvasOnly: boolean;
}

/** The switches the hub put in the pen page's query (`?window=pen&follow=1&canvas=0`); both off
 * when absent. */
export function readPenFlags(search: string): PenFlags {
  const query = new URLSearchParams(search);
  return { followView: query.get("follow") === "1", canvasOnly: query.get("canvas") === "1" };
}

/** The host call that remembers the switches; the layout's only host call of its own. */
export function savePenFlags(
  flags: PenFlags,
  invoke: (command: string, args: Record<string, unknown>) => Promise<unknown>,
): Promise<unknown> {
  return invoke("pen_window_flags", { followView: flags.followView, canvasOnly: flags.canvasOnly });
}

/** Below this window height (CSS px) the View card moves to a second column (UX 10.1). */
export const ONE_COLUMN_HEIGHT = 640;

/** Whether the View card stands in a second column at this window height. */
export function viewInSecondColumn(height: number): boolean {
  return height < ONE_COLUMN_HEIGHT;
}

/** A target, a card's padding and the gap between cards, px (design system "Pen rail"). */
export const TARGET = 48;
const CARD_PADDING = 4;
const CARD_GAP = 8;
const INSET = 12;

/** The height of a rail card of `count` targets. */
export function cardHeight(count: number): number {
  return count * TARGET + 2 * CARD_PADDING;
}

/** The rail's height with the window's top and bottom insets, for `controls`, in one column or
 * with the View card beside the others. */
export function railHeight(controls: readonly PenControl[], twoColumns: boolean): number {
  const card = (group: PenGroup) => cardHeight(controls.filter((c) => c.group === group).length);
  const first = card("Tools") + CARD_GAP + card("Edit");
  const column = twoColumns ? Math.max(first, card("View")) : first + CARD_GAP + card("View");
  return INSET + column + INSET;
}

/** The rail's width from the window's left edge, inset included: one or two 56 px cards. */
export function railRight(twoColumns: boolean): number {
  return 12 + 56 + (twoColumns ? 8 + 56 : 0);
}

/** A press of a layout control from this pointer type does nothing: touch (UX 10.1). */
export function ignoresPress(pointerType: string): boolean {
  return pointerType === "touch";
}

/** One row as the session gives it (`pen_layers_json`). */
export interface PenLayerRow {
  id: unknown;
  name: string;
  hidden: boolean;
  locked: boolean;
  active: boolean;
}

/** The rows in `json`; none for text that does not read. */
export function readPenLayers(json: string): PenLayerRow[] {
  try {
    const rows: unknown = JSON.parse(json);
    return Array.isArray(rows) ? (rows as PenLayerRow[]) : [];
  } catch {
    return [];
  }
}
