/**
 * What a drag over the canvas carries (`specs/0087-image-clipboard-and-drop` criteria 20, 21, 23;
 * customer Frage 68 A): files (dropped files and image data from a browser, both as file items),
 * a link only (`text/uri-list`, or `text/html` such as an `<img src>`, with no file), or nothing
 * for the canvas (plain text). A link is never fetched: nothing here touches the network.
 */

/** The kind of a drag, read from its types (all a page may read before the drop). */
export type DragKind = "files" | "link" | null;

/** The kind of a drag from its `DataTransfer.types`. With `takesText` (the desktop build, whose
 * host reads local paths a plain text names) plain text counts as a link too. */
export function dragKind(types: readonly string[], takesText = false): DragKind {
  if (types.includes("Files")) {
    return "files";
  }
  if (types.includes("text/uri-list") || types.includes("text/html") || (takesText && types.includes("text/plain"))) {
    return "link";
  }
  return null;
}

/** The part of a `DataTransfer` a drop reads. */
export interface DropTransfer {
  types: readonly string[];
  files: ArrayLike<File>;
  items?: ArrayLike<{ kind: string; getAsFile(): File | null }>;
}

/** What a drop brings: its files (file items first, else the file list), a bare link, or nothing. */
export type DropContent = { kind: "files"; files: File[] } | { kind: "link" } | null;

/** The content of a drop. A drag that said "Files" but brings none is read by its other types. */
export function dropContent(data: DropTransfer): DropContent {
  const items = data.items
    ? Array.from(data.items)
        .filter((item) => item.kind === "file")
        .map((item) => item.getAsFile())
        .filter((file): file is File => file !== null)
    : [];
  const files = items.length > 0 ? items : Array.from(data.files);
  if (files.length > 0) {
    return { kind: "files", files };
  }
  return dragKind(data.types.filter((type) => type !== "Files")) === "link" ? { kind: "link" } : null;
}
