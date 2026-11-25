// The Library tree's keyboard (`specs/0137-document-library` UX notes section 11, `0138` section
// 10): Up, Down, Page Up, Page Down, Home and End between rows; Right expands, then walks the
// row's cells; Left walks back, collapses, then goes to the parent; Enter or Space toggles or
// opens a page in the preview; Ctrl+Enter imports a page or loads a template; Delete arms Remove;
// Escape returns to the canvas; a printable key goes to the search field.
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

/** The cells a row's Right and Left reach after its name cell, in their visual order: the licence
 * link, Load or Import page, Refresh, Remove, and the ready entry's Add (`0138` UX notes 10). */
const CELLS = ".library-licence-link, .library-action, .library-online-add";

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
      // An action cell: Right goes to the next cell, Left to the previous one, then the name cell.
      const cells = [...event.currentTarget.querySelectorAll<HTMLElement>(CELLS)];
      const at = cells.indexOf(event.target as HTMLElement);
      if (event.key === "ArrowLeft") {
        event.preventDefault();
        (cells[at - 1] ?? event.currentTarget).focus();
      } else if (event.key === "ArrowRight" && at >= 0) {
        event.preventDefault();
        cells[at + 1]?.focus();
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
    const importable = row.kind === "page" || (row.kind === "template" && row.online?.loadable === true);
    if (importable && event.key === "Enter" && (event.ctrlKey || event.metaKey)) {
      event.preventDefault();
      keys.importPage(row);
    } else if ((event.key === "Enter" || event.key === " ") && (row.kind === "ready" || row.kind === "icons")) {
      // The ready entry's one act is Add; an icon row's cells are its content.
      event.preventDefault();
      if (row.kind === "ready") {
        event.currentTarget.querySelector<HTMLElement>(".library-online-add")?.click();
      } else {
        event.currentTarget.querySelector<HTMLElement>("[data-icon-cell]")?.focus();
      }
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
        event.currentTarget.querySelector<HTMLElement>(row.kind === "icons" ? "[data-icon-cell]" : CELLS)?.focus();
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
