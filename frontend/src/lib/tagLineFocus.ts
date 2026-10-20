// Where the focus goes around a tag line (`specs/0075-font-library` UX tags amendment §4, §5
// and §8; the UX re-review's focus fixes): pure decisions, so the tests read them without a
// page.

/** Where the focus returns when the add field closes: the Add tag button after a key, or when
 * nothing usable had the focus; else that element. */
export function addFieldReturn<T extends { isConnected: boolean }>(
  byKeyboard: boolean,
  before: T | null,
): T | "add-tag" {
  return byKeyboard || !before || !before.isConnected ? "add-tag" : before;
}

/** The tag that takes a removed chip's place: the next in the family's full tag order, shown
 * or in "+N"; `null` when the removed one was last (the focus then goes to Add tag). */
export function tagAfterRemove(order: string[], removed: string): string | null {
  const at = order.indexOf(removed);
  return at >= 0 && at + 1 < order.length ? order[at + 1] : null;
}
