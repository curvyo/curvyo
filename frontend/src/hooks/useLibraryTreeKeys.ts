// The Library tree's keyboard (`specs/0137-document-library` UX notes section 11): Up, Down,
// Page Up, Page Down, Home and End between rows; Right expands, then reaches the action cell;
// Left goes back, collapses, then to the parent; Enter or Space toggles or opens a page in the
// preview; Ctrl+Enter imports a page; Delete arms Remove; Escape returns to the canvas; a
// printable key goes to the search field.
import type { KeyboardEvent } from "react";

import type { LibraryRow } from "@/lib/libraryApi";

interface TreeKeys {
  rows: LibraryRow[];
  /** The rows in the keyboard order (no heading), with their index in `rows`. */
  focusable: { row: LibraryRow; index: number }[];
  /** The index of a row's parent, -1 for a source. */
  parentOf: (index: number) => number;
  /** How many rows a Page Up or Page Down moves. */
  pageRows: () => number;
  /** A search is typed (expanding is the search's then). */
  searching: boolean;
  focusRow: (index: number) => void;
  toggle: (row: LibraryRow) => void;
  openRow: (row: LibraryRow) => void;
  importPage: (row: LibraryRow) => void;
  remove: (row: LibraryRow) => void;
  typeIntoSearch: (key: string) => void;
  onReturnFocus: () => void;
}

/** The key handler of one row at `index`. */
export function useLibraryTreeKeys(keys: TreeKeys) {
  const { rows, focusable, parentOf, pageRows, searching, focusRow, toggle, openRow } = keys;
  const step = (from: number, delta: number) => {
    const at = focusable.findIndex(({ index }) => index === from);
    const next = focusable[Math.max(0, Math.min(focusable.length - 1, at + delta))];
    if (next) {
      focusRow(next.index);
    }
  };
  return (event: KeyboardEvent<HTMLDivElement>, row: LibraryRow, index: number) => {
    if (event.target !== event.currentTarget) {
      // The action cell (Import page, Remove): Left goes back to the name cell.
      if (event.key === "ArrowLeft") {
        event.preventDefault();
        event.currentTarget.focus();
      } else if (event.key === "Escape") {
        keys.onReturnFocus();
      }
      return;
    }
    const moves: Record<string, number> = {
      ArrowDown: 1,
      ArrowUp: -1,
      PageDown: pageRows(),
      PageUp: -pageRows(),
      Home: -rows.length,
      End: rows.length,
    };
    if (row.kind === "page" && event.key === "Enter" && (event.ctrlKey || event.metaKey)) {
      event.preventDefault();
      keys.importPage(row);
    } else if (event.key === "Enter" || event.key === " ") {
      event.preventDefault();
      if (row.kind === "page") {
        openRow(row);
      } else {
        toggle(row);
      }
    } else if (event.key === "Delete" && row.removable) {
      keys.remove(row);
    } else if (event.key in moves) {
      event.preventDefault();
      step(index, moves[event.key] ?? 0);
    } else if (event.key === "ArrowRight") {
      event.preventDefault();
      if (row.expandable && !row.expanded && !searching) {
        toggle(row);
      } else {
        event.currentTarget.querySelector<HTMLElement>(".library-action")?.focus();
      }
    } else if (event.key === "ArrowLeft") {
      event.preventDefault();
      if (row.expandable && row.expanded && !searching) {
        toggle(row);
      } else if (parentOf(index) >= 0) {
        focusRow(parentOf(index));
      }
    } else if (event.key === "Escape") {
      event.preventDefault();
      event.stopPropagation();
      keys.onReturnFocus();
    } else if (event.key.length === 1 && !event.ctrlKey && !event.metaKey && !event.altKey) {
      event.preventDefault();
      keys.typeIntoSearch(event.key);
    }
  };
}
