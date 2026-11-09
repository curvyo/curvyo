// The file name an embedded font gets when it is installed (`specs/0107-document-fonts`
// criterion 14): "Family-Style" and the extension its first bytes name, so Add font's checks and
// archive handling see an ordinary file.
import type { FontFace } from "@/lib/textText";

/** The extension of font bytes: otf, ttc, svg (a single-line font) or ttf. */
export function fontExtension(bytes: Uint8Array): string {
  const head = String.fromCharCode(...bytes.slice(0, 4));
  if (head === "OTTO") {
    return "otf";
  }
  if (head === "ttcf") {
    return "ttc";
  }
  if (head.startsWith("<")) {
    return "svg";
  }
  return "ttf";
}

/** `Family-Style.ext` without characters a file name cannot hold. */
export function fontFileName(face: Pick<FontFace, "family" | "style">, bytes: Uint8Array): string {
  const clean = (text: string) => text.replace(/[\\/:*?"<>|\u0000-\u001f]/g, "").trim() || "font";
  return `${clean(face.family)}-${clean(face.style)}.${fontExtension(bytes)}`;
}
