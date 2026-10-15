/**
 * The spacer that keeps the Style tab's scroll offset when Original's closes the open style
 * sections (`specs/0057-clone-overrides`, UX review must-fix 2): the height the panel body lost,
 * measured at the press and after the collapse has been laid out. Never negative; whole pixels.
 */
export function collapseSpacer(heightAtPress: number, heightAfter: number): number {
  if (!Number.isFinite(heightAtPress) || !Number.isFinite(heightAfter)) {
    return 0;
  }
  return Math.max(0, Math.round(heightAtPress - heightAfter));
}
