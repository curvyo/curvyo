/**
 * The page rules of Ctrl+C and Ctrl+V for images (`specs/0087-image-clipboard-and-drop` criteria
 * 1 to 6, 13, 15): which key press is the copy chord, which element keeps its own clipboard, which
 * clipboard file a paste takes and its name, and the short reason of a failed copy. The ranking by
 * content is the session's (`paste_rank`); it is passed in.
 */

/** Whether `element` keeps Ctrl+C and Ctrl+V for its own text (a field, the text editor's sink). */
export function keepsClipboard(element: Element | EventTarget | null): boolean {
  return (
    element instanceof Element &&
    element.matches('input, textarea, select, [contenteditable=""], [contenteditable="true"]')
  );
}

/** A key press as far as the chord needs it. */
export interface ChordKey {
  key: string;
  ctrlKey: boolean;
  metaKey: boolean;
  shiftKey: boolean;
  altKey: boolean;
  repeat: boolean;
}

/** Whether `event` is Ctrl+C (Cmd+C on macOS) without Shift or Alt, once per press; the typed
 * character decides, so a German or French layout works. */
export function isCopyChord(event: ChordKey, mac: boolean): boolean {
  const primary = mac ? event.metaKey : event.ctrlKey;
  return primary && !event.shiftKey && !event.altKey && !event.repeat && event.key.toLowerCase() === "c";
}

/** A clipboard file read for the paste. */
export interface ClipboardFile {
  name: string;
  type: string;
  bytes: Uint8Array;
}

/** The names browsers give a bitmap on the clipboard that came without a file (Chromium, Firefox
 * and WebKitGTK say "image.png"): the pasted image is named by the session instead. */
const PLACEHOLDER_NAMES = new Set(["image.png", "image.jpeg", "image.jpg", "image.webp", "image"]);

/** The name a pasted file is imported under: empty (the session's "Pasted image") for a
 * placeholder, else the file's own name (a file copied in the file manager, criterion 6). */
export function pastedName(name: string): string {
  return PLACEHOLDER_NAMES.has(name.trim().toLowerCase()) ? "" : name;
}

/** The file a paste takes: the highest `rank` (2 an image by content, 1 an image type the import
 * refuses with its notice, 0 nothing), the first of equals; `null` when every rank is 0 or there
 * is no file (criteria 4, 5, 7). */
export function pickPasted(files: ClipboardFile[], rank: (file: ClipboardFile) => number): ClipboardFile | null {
  let best: ClipboardFile | null = null;
  let bestRank = 0;
  for (const file of files) {
    const r = rank(file);
    if (r > bestRank) {
      best = file;
      bestRank = r;
    }
  }
  return best;
}

/** The short reason of a failed `navigator.clipboard.write`: the error's message, else its name. */
export function copyFailureReason(error: unknown): string {
  if (error instanceof Error || (typeof error === "object" && error !== null && "message" in error)) {
    const { message, name } = error as { message?: unknown; name?: unknown };
    if (typeof message === "string" && message.trim()) {
      return message;
    }
    if (typeof name === "string" && name) {
      return name;
    }
  }
  return String(error) || "unknown error";
}
