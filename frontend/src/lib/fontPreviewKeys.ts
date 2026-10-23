// The font preview's key routing (`specs/0077-font-preview` criteria 5 and 22): the tile an arrow,
// Home, End, Page Up or Page Down moves to, what types into the sample, and when Escape outside
// the preview closes it.

/** The index the key moves the focus to in a grid of `count` tiles in `columns` columns with
 * `pageRows` rows on screen, or `null` for a key that does not move. */
export function gridMove(key: string, index: number, count: number, columns: number, pageRows: number): number | null {
  if (count <= 0) {
    return null;
  }
  const cols = Math.max(1, columns);
  const page = cols * Math.max(1, pageRows);
  const by: Record<string, number> = {
    ArrowRight: index + 1,
    ArrowLeft: index - 1,
    ArrowDown: index + cols,
    ArrowUp: index - cols,
    PageDown: index + page,
    PageUp: index - page,
    Home: 0,
    End: count - 1,
  };
  const next = by[key];
  if (next === undefined) {
    return null;
  }
  // Up and Down stay in the column at the grid's ends; the others stop at the first or last tile.
  if ((key === "ArrowUp" || key === "ArrowDown") && (next < 0 || next >= count)) {
    return key === "ArrowDown" && Math.floor(index / cols) < Math.floor((count - 1) / cols) ? count - 1 : index;
  }
  return Math.min(count - 1, Math.max(0, next));
}

/** Whether a key press types into the sample field: one printable character without Ctrl, Alt
 * or Meta. */
export function typesIntoSample(key: string, ctrl: boolean, alt: boolean, meta: boolean): boolean {
  return [...key].length === 1 && !ctrl && !alt && !meta;
}

/** What a key on a focused tile does to its tag line, as on a list row (criterion 14,
 * `specs/0075-font-library` UX tags amendment §8): Shift+F2 enters the line, F2 opens Add tag. */
export function tileTagKey(key: string, shift: boolean): "enter-line" | "add" | null {
  if (key !== "F2") {
    return null;
  }
  return shift ? "enter-line" : "add";
}

/** The canvas's `key_down` outcome when Escape found nothing of the canvas's own to cancel. */
const ESCAPE_NOTHING = "escape-none";

/** Whether the canvas keeps the page default for a key outcome: Escape with nothing to cancel
 * stays unhandled, so it reaches the preview's window listener (criterion 5). */
export function canvasLeavesKey(outcome: string): boolean {
  return outcome === "ignored" || outcome === ESCAPE_NOTHING;
}

/** Whether a key that bubbled to the window closes the open preview (criterion 5, customer
 * decision 2026-10-23): Escape that nothing before it handled (no drag, tool state, text edit,
 * group or selection to cancel, no control's own Escape). */
export function escapeClosesPreview(key: string, defaultPrevented: boolean, open: boolean): boolean {
  return open && key === "Escape" && !defaultPrevented;
}
