// The items of the History list (`specs/0042-history-branches` criteria 8 and 9): step rows, branch
// header rows and the "N more branches" row as the session returns them, the height model of the
// virtualised list with those three heights, and the geometry of the lane gutter. Pure, so
// `tests/historyItems.test.ts` covers it.

import { type HistoryRowData, ROW_HEIGHT_PX } from "./historyRows.ts";

/** The height of a branch header row, px (UX notes, "Branches in a 244 px list"). */
export const HEADER_HEIGHT_PX = 24;

/** The height of the "N more branches" row, px. */
export const MORE_HEIGHT_PX = 28;

/** The x of the main lane, px, and the pitch between lanes. */
export const MAIN_LANE_X = 8;
export const LANE_PITCH_PX = 12;

/** Where a gutter line starts or ends inside its item. */
export type Edge = "top" | "mark" | "bottom";

/** One part of an item's gutter, as compact arrays: a line, a filled disc, an open ring, or the
 * connector of a fork point (from the lane it leaves to the branch's lane). */
export type GutterSegment =
  | ["line", number, Edge, Edge, "main" | "dotted" | "dashed" | "branch"]
  | ["disc", number]
  | ["ring", number]
  | ["fork", number, number];

/** A step row with its gutter. */
export type RowItem = HistoryRowData & { type: "row"; gutter: GutterSegment[] };

/** A branch header row or the "more" row. */
export interface FoldItem {
  type: "header" | "more";
  /** Stable across reads: the branch id, or "more". */
  key: string;
  text: string;
  expanded: boolean;
  gutter: GutterSegment[];
}

/** One item of the list. */
export type HistoryItemData = RowItem | FoldItem;

/** An item whose height is not a step row's 44 px: a header row, the "more" row, or a step row
 * with its objects disclosure open (with its whole height). Sorted by index. */
export type SpecialItem = [number, "header" | "more"] | [number, "row", number];

/** The shape of the list: how many items, how many lanes, and which items are not plain step
 * rows. */
export interface ListShape {
  total: number;
  lanes: number;
  special: SpecialItem[];
}

/** The shape of an empty list. */
export const EMPTY_SHAPE: ListShape = { total: 0, lanes: 0, special: [] };

/** Reads what `history_layout` returns. */
export function parseShape(json: string): ListShape {
  const value = JSON.parse(json) as Partial<ListShape>;
  return {
    total: value.total ?? 0,
    lanes: value.lanes ?? 0,
    special: Array.isArray(value.special) ? value.special : [],
  };
}

/** Reads what `history_items` returns. */
export function parseItems(json: string): HistoryItemData[] {
  const value: unknown = JSON.parse(json);
  return Array.isArray(value) ? (value as HistoryItemData[]) : [];
}

function heightOf(item: SpecialItem): number {
  if (item[1] === "row") {
    return item[2];
  }
  return item[1] === "header" ? HEADER_HEIGHT_PX : MORE_HEIGHT_PX;
}

/** The height of the item at `index`. */
export function itemHeight(index: number, shape: ListShape): number {
  const special = shape.special.find(([at]) => at === index);
  return special ? heightOf(special) : ROW_HEIGHT_PX;
}

/** The shape with the step row at `index` grown to `height` (an open objects disclosure). */
export function withRowHeight(shape: ListShape, index: number, height: number): ListShape {
  const special: SpecialItem[] = [
    ...shape.special.filter(([at]) => at !== index),
    [index, "row", height] as SpecialItem,
  ].sort((a, b) => a[0] - b[0]);
  return { ...shape, special };
}


/** The top of the item at `index`, px from the top of the list. */
export function itemTop(index: number, shape: ListShape): number {
  let shrink = 0;
  for (const item of shape.special) {
    if (item[0] >= index) {
      break;
    }
    shrink += ROW_HEIGHT_PX - heightOf(item);
  }
  return index * ROW_HEIGHT_PX - shrink;
}

/** The height of the whole list. */
export function listHeight(shape: ListShape): number {
  return itemTop(shape.total, shape);
}

/** The item under `y`, px from the top of the list, clamped to the list. */
export function itemAtY(y: number, shape: ListShape): number {
  let shrink = 0;
  for (const item of shape.special) {
    const at = item[0];
    const top = at * ROW_HEIGHT_PX - shrink;
    if (y < top) {
      break;
    }
    const height = heightOf(item);
    if (y < top + height) {
      return at;
    }
    shrink += ROW_HEIGHT_PX - height;
  }
  return Math.max(0, Math.min(shape.total - 1, Math.floor((y + shrink) / ROW_HEIGHT_PX)));
}

/** The items to draw for a scroll position: those in view plus `overscan` on each side. */
export function itemRange(
  scrollTop: number,
  viewportHeight: number,
  shape: ListShape,
  overscan = 5,
): { first: number; last: number } {
  if (shape.total <= 0) {
    return { first: 0, last: -1 };
  }
  const top = Math.max(0, Math.min(scrollTop, listHeight(shape) - viewportHeight));
  return {
    first: Math.max(0, itemAtY(top, shape) - overscan),
    last: Math.min(shape.total - 1, itemAtY(top + viewportHeight, shape) + overscan),
  };
}

/** The gutter's width: 16 px plus 12 px per branch lane. */
export function gutterWidth(lanes: number): number {
  return 2 * MAIN_LANE_X + LANE_PITCH_PX * lanes;
}

/** The x of a lane's line. */
export function laneX(lane: number): number {
  return MAIN_LANE_X + LANE_PITCH_PX * lane;
}

/** The y of an edge in an item of `height`: the mark sits in the middle. */
export function edgeY(edge: Edge, height: number): number {
  return edge === "top" ? 0 : edge === "bottom" ? height : height / 2;
}

/** The path of a fork connector: leaves `fromLane` at the mark to the right, turns up with a 4 px
 * corner and runs up `lane` to the item's top. */
export function forkPath(fromLane: number, lane: number, height: number): string {
  const y = height / 2;
  const x = laneX(lane);
  return `M ${laneX(fromLane)} ${y} H ${x - 4} Q ${x} ${y} ${x} ${y - 4} V 0`;
}

/** The disclosure of a Document-scope row: one 28 px sub-row per touched object, at most 20, then
 * the line "and N more" (`specs/0042-history-branches` criterion 24). */
export const SUB_ROW_HEIGHT_PX = 28;
export const MAX_SUB_ROWS = 20;

/** How tall the open disclosure of `row` is. */
export function objectsHeight(row: HistoryRowData): number {
  const n = row.objects.length;
  return (Math.min(n, MAX_SUB_ROWS) + (n > MAX_SUB_ROWS ? 1 : 0)) * SUB_ROW_HEIGHT_PX;
}
