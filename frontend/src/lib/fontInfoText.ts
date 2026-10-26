// The words of the font preview's Info mode (`specs/0083-font-info-and-zip` criteria 2 to 5,
// UX notes §2, §3 and §7): the format, the embedding permission from `fsType`, file sizes, the
// block, the included fonts' licence names and the empty and error states. Pure, so the tests
// read them without a session; the engine gives the raw fields (`adrs.md` decision 6).

/** The raw fields the engine reads from a face (`PreviewFacesGlue::font_info`). */
export interface FontInfoFields {
  format: { kind: "truetype" | "cff" | "cff2" | "collection" | "svg"; face?: number; faces?: number };
  full_name: string | null;
  version: string | null;
  designer: string | null;
  vendor: string | null;
  copyright: string | null;
  trademark: string | null;
  licence: string | null;
  licence_url: string | null;
  fs_type: number | null;
  glyphs: number;
  characters: number;
}

/** A value the font does not state (criterion 2). */
export const NOT_STATED = "Not stated";

/** Criterion 2, Format. */
export function formatText(format: FontInfoFields["format"]): string {
  switch (format.kind) {
    case "truetype":
      return "TrueType (TTF)";
    case "cff":
      return "OpenType CFF (OTF)";
    case "cff2":
      return "OpenType CFF2";
    case "collection":
      return `TrueType collection (TTC), face ${format.face ?? 1} of ${format.faces ?? 1}`;
    default:
      return "SVG font, single-line";
  }
}

/** Criterion 2, Embedding (R-FONT-008): the usage bits in words, the least restrictive one when
 * several are set (as the OpenType rules read them), then "no subsetting" (bit 8) and "bitmap
 * only" (bit 9). Information only: it never blocks anything. */
export function embeddingText(fsType: number | null, singleLine: boolean): string {
  if (singleLine) {
    return "Not stated (SVG font)";
  }
  if (fsType === null) {
    return NOT_STATED;
  }
  let usage = "Installable";
  if (fsType & 0x0008) {
    usage = "Editable";
  } else if (fsType & 0x0004) {
    usage = "Preview and print";
  } else if (fsType & 0x0002) {
    usage = "Restricted: may not be embedded";
  }
  const extras = [fsType & 0x0100 ? "no subsetting" : null, fsType & 0x0200 ? "bitmap only" : null].filter(
    (part): part is string => part !== null,
  );
  return [usage, ...extras].join(", ");
}

/** A file size as the File row shows it: "812 bytes", "412 KB", "1.2 MB" (1 KB = 1024 bytes). */
export function sizeText(bytes: number): string {
  if (bytes < 1024) {
    return bytes === 1 ? "1 byte" : `${bytes} bytes`;
  }
  if (bytes < 1024 * 1024) {
    return `${Math.round(bytes / 1024)} KB`;
  }
  const mb = bytes / (1024 * 1024);
  return `${mb < 10 ? mb.toFixed(1) : Math.round(mb)} MB`;
}

/** The File row's first line: "Lobster-Regular.ttf · 412 KB". */
export function fileLine(file: string, size: number | null): string {
  return size === null ? file : `${file} · ${sizeText(size)}`;
}

/** The File row's block line (criterion 2). */
export function blockText(block: "curvyo" | "included" | "system", desktop: boolean): string {
  if (block === "included") {
    return "Included";
  }
  if (block === "system") {
    return "This computer";
  }
  return desktop ? "Curvyo fonts" : "Browser storage";
}

/** The subject line of Info mode (criterion 1): "Great Vibes Regular · Included". */
export function infoSubjectLine(face: string, block: string): string {
  return face ? `${face} · ${block}` : "";
}

/** An included font's licence id from its `font.toml` in words (criterion 2, Licence). */
export function licenceName(id: string): string {
  switch (id) {
    case "OFL-1.1":
      return "SIL Open Font License 1.1";
    case "Apache-2.0":
      return "Apache License 2.0";
    case "Hershey":
      return "Hershey notice";
    default:
      return id;
  }
}

/** Criterion 2, Glyphs and Characters. */
export function countText(count: number, noun: "glyph" | "character"): string {
  return `${count.toLocaleString("en-US")} ${noun}${count === 1 ? "" : "s"}`;
}

/** The missing state (criterion 5). */
export function missingInfoText(family: string): string {
  return `${family} is missing.`;
}

/** A face whose bytes cannot be read (UX notes §7). */
export function unreadableInfoText(face: string): string {
  return `${face} could not be read.`;
}

/** The last line of a text document cut at 1 MiB (criterion 4). */
export const SHOWN_FIRST_MIB = "Shown: the first 1 MiB.";
/** An empty text document (UX notes §7). */
export const EMPTY_DOCUMENT = "This file is empty.";

/** The notice when a document cannot be read or opened (UX notes §3). */
export function documentFailedText(name: string, pdf: boolean): string {
  return pdf ? `${name} could not be opened.` : `${name} could not be read.`;
}

/** Whether a document is a PDF (opened in the viewer) rather than a text shown in place. */
export function isPdfName(name: string): boolean {
  return name.toLowerCase().endsWith(".pdf");
}

/** Documents A to Z ignoring case (criterion 4). */
export function sortedDocuments<T extends { name: string }>(documents: T[]): T[] {
  return [...documents].sort((a, b) => a.name.toLowerCase().localeCompare(b.name.toLowerCase()));
}

/** The documents a Remove of `family` takes with it (criterion 14): those of every font folder
 * whose listed fonts are all the family's. `faces` is the Curvyo block's listing. */
export function documentsGoing(
  family: string,
  faces: { family: string; folder?: string | null }[],
  folders: { name: string; documents: unknown[] }[],
): number {
  let going = 0;
  for (const folder of folders) {
    const inFolder = faces.filter((face) => face.folder === folder.name);
    if (inFolder.length > 0 && inFolder.some((f) => f.family === family) && inFolder.every((f) => f.family === family)) {
      going += folder.documents.length;
    }
  }
  return going;
}
