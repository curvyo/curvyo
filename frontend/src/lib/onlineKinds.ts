// The row kinds and heights of the Library tab's Online group (`specs/0138-template-library` UX
// notes 4 to 8); no component here, so the rows file only exports components.
import type { LibraryRow } from "@/lib/libraryApi";

/** Whether a row belongs to the Online group's own kinds. */
export function isOnlineKind(kind: LibraryRow["kind"]): boolean {
  return kind === "ready" || kind === "template" || kind === "set" || kind === "icons" || kind === "status";
}

/** The height of a row in CSS px (UX notes 4 to 8). */
export function onlineHeight(row: LibraryRow): number | null {
  switch (row.kind) {
    case "ready":
      return 32;
    case "template":
      return row.online?.font_note ? 68 : 52;
    case "set":
      return 44;
    case "icons":
      return 40;
    case "status":
      return 28;
    default:
      return null;
  }
}
