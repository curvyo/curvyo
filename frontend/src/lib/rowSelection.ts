/**
 * Which operation rows are selected (`specs/0028-material-test-library` `ux-notes.md` 8.2, M10;
 * amends `0101` 4.4): a plain press selects one row, Ctrl toggles a row, Shift extends from the
 * anchor to the pressed row, Shift+Up and Shift+Down extend by one. The keys are always kept in
 * list order. Pure functions over the row keys in list order.
 */

/** The selected rows' keys in list order and the row a range grows from. */
export interface RowSelection {
  keys: string[];
  anchor: string | null;
}

/** Nothing selected. */
export const NO_ROWS: RowSelection = { keys: [], anchor: null };

/** The modifier keys of a press. */
export interface PressMods {
  ctrl: boolean;
  shift: boolean;
}

function inOrder(order: readonly string[], keys: ReadonlySet<string>): string[] {
  return order.filter((key) => keys.has(key));
}

function range(order: readonly string[], from: string, to: string): string[] {
  const a = order.indexOf(from);
  const b = order.indexOf(to);
  if (a < 0 || b < 0) return [to];
  return order.slice(Math.min(a, b), Math.max(a, b) + 1);
}

/** The selection after a press on `key`. */
export function press(
  current: RowSelection,
  order: readonly string[],
  key: string,
  mods: PressMods,
): RowSelection {
  if (mods.shift && current.anchor !== null && order.includes(current.anchor)) {
    return { keys: range(order, current.anchor, key), anchor: current.anchor };
  }
  if (mods.ctrl) {
    const next = new Set(current.keys);
    if (next.has(key)) next.delete(key);
    else next.add(key);
    return { keys: inOrder(order, next), anchor: key };
  }
  return { keys: [key], anchor: key };
}

/** The selection after Shift+Up (`step` -1) or Shift+Down (1) from the focused row `focus`. */
export function extend(
  current: RowSelection,
  order: readonly string[],
  focus: string,
  step: -1 | 1,
): RowSelection {
  const at = order.indexOf(focus);
  const target = order[at + step];
  if (at < 0 || target === undefined) return current;
  const anchor = current.anchor !== null && order.includes(current.anchor) ? current.anchor : focus;
  return { keys: range(order, anchor, target), anchor };
}

/** The selection without the rows that are gone. */
export function prune(current: RowSelection, order: readonly string[]): RowSelection {
  const keys = current.keys.filter((key) => order.includes(key));
  if (keys.length === current.keys.length) return current;
  return { keys, anchor: current.anchor !== null && order.includes(current.anchor) ? current.anchor : (keys[0] ?? null) };
}
