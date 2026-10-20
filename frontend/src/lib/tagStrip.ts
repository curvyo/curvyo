// The cells of the tag strip and their keys (`specs/0075-font-library` criteria 1 and 5 to 8;
// strip editing and the bug report of 2026-10-20): All, Favourites and the tags, only those that
// exist; F2 renames and Delete deletes an own tag, the others ignore both.

/** A tag as the strip needs it. */
export interface StripTag {
  id: string;
  name: string;
  builtin: boolean;
  members: Set<string>;
}

/** One strip cell; `own` cells carry Rename and Delete. */
export interface StripCell {
  id: string;
  name: string;
  own: boolean;
  members: number;
}

/** The cells: All, Favourites, then the tags. A deleted tag has no cell, even when it was
 * chosen: the library has chosen All by then (criterion 6). */
export function stripCells(tags: StripTag[]): StripCell[] {
  return [
    { id: "all", name: "All", own: false, members: 0 },
    { id: "favourites", name: "Favourites", own: false, members: 0 },
    ...tags.map((tag) => ({ id: tag.id, name: tag.name, own: !tag.builtin, members: tag.members.size })),
  ];
}

/** What a key on a focused strip cell does: "rename", "delete", or `null` (the strip's). */
export function stripKeyAction(key: string, own: boolean): "rename" | "delete" | null {
  if (!own) {
    return null;
  }
  if (key === "F2") {
    return "rename";
  }
  return key === "Delete" ? "delete" : null;
}
