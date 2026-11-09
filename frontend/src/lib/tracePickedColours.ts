/** The keyboard rule of the picked colours row beside the pipette (`specs/0089` UX notes §25.4). */

/** What a key on swatch `index` of `count` does: move the roving stop, remove the swatch, or
 * nothing. Enter and Space press the button, whose click removes it, so they are not handled
 * here (they would remove twice). */
export function pickedKey(key: string, index: number, count: number): { move: number } | { remove: number } | null {
  switch (key) {
    case "ArrowRight":
      return { move: Math.min(count - 1, index + 1) };
    case "ArrowLeft":
      return { move: Math.max(0, index - 1) };
    case "Home":
      return { move: 0 };
    case "End":
      return { move: count - 1 };
    case "Delete":
    case "Backspace":
      return { remove: index };
    default:
      return null;
  }
}

/** Where focus goes after a key removed swatch `index` of `count`: the swatch now at the same
 * place, else the previous one, else `null` for the pipette. */
export function focusAfterRemoval(index: number, count: number): number | null {
  const left = count - 1;
  return left === 0 ? null : Math.min(index, left - 1);
}
