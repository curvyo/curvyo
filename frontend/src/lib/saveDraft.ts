/**
 * The draft of the Save as material form (`specs/0028-material-test-library` `ux-notes.md` 9,
 * `docs/design-system.md` "Drafts"): it survives a tab change and is dropped by a mark change, a
 * Rebuild, another selected row, Cancel, Escape and a save. One draft at a time, in memory.
 */
import type { SaveDraft } from "./testGridResult.ts";

let kept: { group: string; draft: SaveDraft } | null = null;

/** The draft kept for the grid with this key, `null` for none. */
export function keptDraft(group: string): SaveDraft | null {
  return kept !== null && kept.group === group ? kept.draft : null;
}

/** Keeps `draft` for the grid with this key. */
export function keepDraft(group: string, draft: SaveDraft): void {
  kept = { group, draft };
}

/** Drops the draft. */
export function dropSaveDraft(): void {
  kept = null;
}
