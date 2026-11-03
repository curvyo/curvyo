// The keys of a manufacturing tab's fixture list (`specs/0093-laser-workbench-shell` UX notes §4):
// one Tab stop, Up and Down move between rows, Space toggles the row's eye, Delete removes the
// fixture at once (a reversible action never asks, `docs/design-system.md`).

/** What a key on a fixture row asks for. */
export type FixtureRowAction =
  | { kind: "focus"; index: number }
  | { kind: "toggle_visible" }
  | { kind: "remove" };

/** The action a key on row `index` of `count` rows asks for, or null for a key the row keeps. */
export function fixtureRowKey(key: string, index: number, count: number): FixtureRowAction | null {
  if (count <= 0) {
    return null;
  }
  const focus = (to: number): FixtureRowAction => ({ kind: "focus", index: Math.max(0, Math.min(count - 1, to)) });
  switch (key) {
    case "ArrowDown":
      return focus(index + 1);
    case "ArrowUp":
      return focus(index - 1);
    case "Home":
      return focus(0);
    case "End":
      return focus(count - 1);
    case " ":
      return { kind: "toggle_visible" };
    case "Delete":
      return { kind: "remove" };
    default:
      return null;
  }
}

/** The row index that takes the focus after row `index` of `count` rows is removed, or null when
 * the list is then empty. */
export function focusAfterRemove(index: number, count: number): number | null {
  const left = count - 1;
  if (left <= 0) {
    return null;
  }
  return Math.min(index, left - 1);
}

/** The index of the row that is the list's one Tab stop: the active row while it exists, else the
 * first row; -1 for an empty list. */
export function tabStopRow(ids: string[], active: string | null): number {
  if (ids.length === 0) {
    return -1;
  }
  const at = active === null ? -1 : ids.indexOf(active);
  return at >= 0 ? at : 0;
}
