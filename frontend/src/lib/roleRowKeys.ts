// The keys and the window of the Layers block's role controls (`specs/0027-manufacturing-roles`
// UX notes §7, §8): one Tab stop with roving focus over a grid of rows × {Cut, Engrave, Ignore};
// Left and Right move inside a row and never press, Up and Down move to the same button of the
// next row, Home and End to the first or last row, Page Up and Page Down by 5 rows, no wrap.

/** The roles in button order, as stored. */
export const ROLES = ["cut", "engrave", "ignore"] as const;
export type RoleName = (typeof ROLES)[number];

/** How many rows Page Up and Page Down move. */
export const PAGE_ROWS = 5;

/** What a key on a role button asks for. */
export type RoleKeyAction =
  | { kind: "focus"; row: number; column: number }
  | { kind: "press" }
  | { kind: "clear" }
  | { kind: "leave" };

/** The action of `key` on the button `column` of row `row` of `rows`, or null for a key the
 * button keeps (Tab, the tool letters). */
export function roleKey(key: string, row: number, column: number, rows: number): RoleKeyAction | null {
  if (rows <= 0) {
    return null;
  }
  const at = (r: number, c: number): RoleKeyAction => ({
    kind: "focus",
    row: Math.max(0, Math.min(rows - 1, r)),
    column: Math.max(0, Math.min(ROLES.length - 1, c)),
  });
  switch (key) {
    case "ArrowLeft":
      return at(row, column - 1);
    case "ArrowRight":
      return at(row, column + 1);
    case "ArrowUp":
      return at(row - 1, column);
    case "ArrowDown":
      return at(row + 1, column);
    case "Home":
      return at(0, column);
    case "End":
      return at(rows - 1, column);
    case "PageUp":
      return at(row - PAGE_ROWS, column);
    case "PageDown":
      return at(row + PAGE_ROWS, column);
    case " ":
    case "Enter":
      return { kind: "press" };
    case "Delete":
    case "Backspace":
      return { kind: "clear" };
    case "Escape":
      return { kind: "leave" };
    default:
      return null;
  }
}

/** The role a press of `pressed` writes on a row whose role is `current`: pressing the pressed
 * button clears it (spec criterion 2). */
export function roleAfterPress(current: RoleName | null, pressed: RoleName): RoleName | null {
  return current === pressed ? null : pressed;
}

/** The button that is the controls' one Tab stop: the one that last had focus while its row
 * exists, else the pressed button of the first row, else its first button. */
export function roleTabStop(
  keys: string[],
  roles: (RoleName | null)[],
  last: { key: string; column: number } | null,
): { row: number; column: number } | null {
  if (keys.length === 0) {
    return null;
  }
  const at = last === null ? -1 : keys.indexOf(last.key);
  if (at >= 0 && last !== null) {
    return { row: at, column: last.column };
  }
  const pressed = roles[0] === null || roles[0] === undefined ? -1 : ROLES.indexOf(roles[0]);
  return { row: 0, column: Math.max(0, pressed) };
}

/** The height of a row in px: two lines and one 16 px caption line (plus its 4 px gap) per
 * caption, 8 px apart (UX notes §3; 72 px rows with a coarse pointer). */
export function roleRowHeight(captions: number, coarse: boolean): number {
  return (coarse ? 72 : 64) + 16 * captions;
}

/** The rows in the DOM: at most `max` rows around the part of the list at `top`..`top + height`
 * (px from the list's top), the focused row always kept. `before` and `after` are the spacer
 * heights. */
export function roleRowWindow(
  heights: number[],
  top: number,
  height: number,
  focused: number,
  max = 60,
): { first: number; last: number; before: number; after: number; extra: number | null } {
  const total = heights.reduce((sum, h) => sum + h, 0);
  if (heights.length <= max) {
    return { first: 0, last: heights.length - 1, before: 0, after: 0, extra: null };
  }
  let y = 0;
  let first = 0;
  while (first < heights.length - 1 && y + heights[first] <= top) {
    y += heights[first];
    first += 1;
  }
  let last = first;
  let end = y + heights[first];
  while (last < heights.length - 1 && end < top + height && last - first + 1 < max) {
    last += 1;
    end += heights[last];
  }
  const before = y;
  const after = total - end;
  const extra = focused >= 0 && focused < heights.length && (focused < first || focused > last) ? focused : null;
  return { first, last, before, after, extra };
}
