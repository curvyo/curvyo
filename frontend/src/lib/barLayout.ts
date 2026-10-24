// The layout class of a bar row of groups (Select bar, Text bar; `docs/design-system.md`, "Select
// bar layout").

/** Wraps by whole groups, 25px between two groups (12px, the divider, 12px), top-aligned so a
 * taller group does not move its neighbours. */
export const BAR_ROW_CLASS = "flex min-w-0 flex-wrap items-start gap-x-[25px] gap-y-0";

/** The horizontal shift that keeps a notice under a bar control inside the bars' area (right of
 * the tool rail, left of the canvas's right edge; `0084` tester finding c17). `left` and `right`
 * are the notice's edges without a shift, `areaLeft` and `areaRight` the area's; a notice wider
 * than the area keeps its left edge clear of the rail. */
export function noticeShift(left: number, right: number, areaLeft: number, areaRight: number): number {
  return Math.max(areaLeft - left, Math.min(0, areaRight - right));
}
