// The keys of the Fonts tab's tree (`specs/0074-text-objects` UX notes §4,
// `specs/0075-font-library` UX notes §6 and tags amendment §8): moving over rows, expanding and
// collapsing, picking, Ctrl+D for the star, F2 and Shift+F2 for tags, Delete to arm and Delete or
// Enter to press the armed Remove trash (`specs/0081-inline-delete-confirm` criterion 7), Escape to
// the canvas, and typing into the search.
import { type Row, familyKey } from "./fontRows.ts";
import type { FontFace as ListedFace, FontFamily } from "./textText.ts";

/** What a key in the tree can do. */
export interface FontTreeKeyContext {
  rows: Row[];
  /** The rows that take focus (all but headers). */
  focusable: Row[];
  activeKey: string | null;
  /** How many rows Page Up and Page Down move. */
  pageRows: number;
  focusRow: (key: string) => void;
  toggle: (family: FontFamily, open?: boolean) => void;
  pickFamily: (family: FontFamily, byKeyboard: boolean) => void;
  pickFace: (face: ListedFace) => void;
  star: (family: FontFamily) => void;
  /** A press on the family's Remove trash (arms it, or removes when armed). */
  pressRemove: (family: FontFamily) => void;
  /** Whether the family's Remove trash is armed. */
  isArmed: (family: FontFamily) => boolean;
  /** F2: the add field of the family's tag line. */
  openAddTag: (family: FontFamily) => void;
  /** Shift+F2: the last chip's x on the family's tag line. */
  focusTagLine: (family: FontFamily) => void;
  returnFocus: () => void;
  typeInSearch: (character: string) => void;
}

/** Handles a key pressed in the tree. */
export function onFontTreeKey(event: React.KeyboardEvent<HTMLElement>, ctx: FontTreeKeyContext): void {
  const at = ctx.focusable.findIndex((row) => row.key === ctx.activeKey);
  const row = ctx.focusable[at];
  const move = (to: number) => {
    const target = ctx.focusable[Math.max(0, Math.min(ctx.focusable.length - 1, to))];
    if (target) {
      event.preventDefault();
      ctx.focusRow(target.key);
    }
  };
  const page = ctx.pageRows;
  if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === "d") {
    // Ctrl+D stars the family of the focused row (UX notes §6); the bookmark key is taken.
    event.preventDefault();
    if (row && row.kind !== "missing" && row.kind !== "header") {
      ctx.star(row.family);
    }
    return;
  }
  switch (event.key) {
    case "ArrowDown":
      return move(at + 1);
    case "ArrowUp":
      return move(at - 1);
    case "Home":
      return move(0);
    case "End":
      return move(ctx.focusable.length - 1);
    case "PageDown":
      return move(at + page);
    case "PageUp":
      return move(at - page);
    case "ArrowRight":
      event.preventDefault();
      if (row?.kind === "family") {
        if (row.open) {
          move(at + 1);
        } else {
          ctx.toggle(row.family, true);
        }
      }
      return;
    case "ArrowLeft":
      event.preventDefault();
      if (row?.kind === "face") {
        ctx.focusRow(familyKey(row.family));
      } else if (row?.kind === "family") {
        ctx.toggle(row.family, false);
      }
      return;
    case "Enter":
    case " ":
      event.preventDefault();
      if (event.key === "Enter" && row && (row.kind === "family" || row.kind === "face") && ctx.isArmed(row.family)) {
        ctx.pressRemove(row.family);
      } else if (row?.kind === "family") {
        ctx.pickFamily(row.family, true);
      } else if (row?.kind === "face") {
        ctx.pickFace(row.face);
      }
      return;
    case "F2":
      // F2 opens the add field of the focused family, Shift+F2 enters its tag line (UX tags
      // amendment §8).
      if (row && row.kind !== "missing" && row.kind !== "header") {
        event.preventDefault();
        if (event.shiftKey) {
          ctx.focusTagLine(row.family);
        } else {
          ctx.openAddTag(row.family);
        }
      }
      return;
    case "Delete":
      if ((row?.kind === "family" || row?.kind === "face") && row.family.block === "curvyo") {
        event.preventDefault();
        ctx.pressRemove(row.family);
      }
      return;
    case "Escape":
      event.preventDefault();
      event.stopPropagation();
      ctx.returnFocus();
      return;
    default:
      if (event.key.length === 1 && !event.ctrlKey && !event.metaKey && !event.altKey) {
        event.preventDefault();
        ctx.typeInSearch(event.key);
      }
  }
}
