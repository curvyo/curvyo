// The words of the Text tool and the Fonts tab (`specs/0074-text-objects`, UX notes §4 and §6):
// notices, the Font button's name, the Fonts tab's blocks and its search. Pure functions, so the
// tests read them without a session.

/** "Nothing was changed." closes every refusal (design system, "Action notice"). */
const NOTHING_CHANGED = "Nothing was changed.";

/** A face name longer than this is cut with an ellipsis, so the refusal stays under 70
 * characters (UX notes §4). */
const MAX_FACE_IN_NOTICE = 32;

/** The notice when edit mode ended and its text could not be written (a peer removed it and a
 * new text could not be made either). */
export const TEXT_WRITE_FAILED_NOTICE = "The text could not be written. Nothing was changed.";

/** Whether the host fonts are listed now: on entering the Text tool only, not on every sync of it
 * (criteria 19, 20; the Fonts tab lists them when it is shown). */
export function listsOnToolChange(previous: string | null, next: string): boolean {
  return next === "text" && previous !== "text";
}

/** The canvas notice of criterion 31. */
export const TEXT_LIMIT_NOTICE = "A text holds at most 10000 characters.";

/** `face` cut to [`MAX_FACE_IN_NOTICE`] characters with an ellipsis. */
export function shortFace(face: string): string {
  return face.length > MAX_FACE_IN_NOTICE ? `${face.slice(0, MAX_FACE_IN_NOTICE - 1)}…` : face;
}

/** The refusal of a command on texts that do not convert (criteria 24, 29): one missing face is
 * named, several texts are counted; a face still loading says so. `null` for any other kind. */
export function textRefusalText(result: { kind: string; count?: number; of?: number; face?: string }): string | null {
  const count = result.count || result.of || 0;
  if (result.kind === "single_line_texts") {
    return singleLineRefusalText(count);
  }
  if (result.kind === "fonts_loading") {
    return `Fonts are still loading. ${NOTHING_CHANGED}`;
  }
  if (result.kind !== "missing_fonts") {
    return null;
  }
  if (result.face) {
    return `Font ${shortFace(result.face)} is missing. ${NOTHING_CHANGED}`;
  }
  return count === 1
    ? `1 text uses a missing font. ${NOTHING_CHANGED}`
    : `${count} texts use missing fonts. ${NOTHING_CHANGED}`;
}

/** The refusal of a command on areas for single-line texts (`specs/0076-single-line-fonts`
 * criterion 19). */
export function singleLineRefusalText(count: number): string {
  return count <= 1
    ? `Single-line text has no area. ${NOTHING_CHANGED}`
    : `${count} single-line texts have no area. ${NOTHING_CHANGED}`;
}

/** Whether a refusal outlines the texts it blames on the canvas. */
export function textRefusalOutlines(kind: string): boolean {
  return kind === "missing_fonts" || kind === "fonts_loading" || kind === "single_line_texts";
}

/** The notice under the Size field after a clamped value (criterion 12), or `null`. */
export function sizeNotice(outcome: string): string | null {
  if (outcome === "max") {
    return "Size set to 2000 mm, the largest size.";
  }
  if (outcome === "min") {
    return "Size set to 0.1 mm, the smallest size.";
  }
  return null;
}

/** The Size field's value: up to two decimals, trailing zeros trimmed ("20", "12.5", "0.25");
 * "" for "Mixed". */
export function sizeText(mm: number | null): string {
  if (mm === null || !Number.isFinite(mm)) {
    return "";
  }
  return String(Math.round(mm * 100) / 100);
}

/** A typed size with an optional unit (`0052`: mm, cm, in, pt, px at 96 per inch), in mm, or
 * `null` for text that is not a number. */
export function parseSize(text: string): number | null {
  const match = /^\s*(-?\d+(?:[.,]\d+)?|-?[.,]\d+)\s*(mm|cm|in|pt|px)?\s*$/i.exec(text);
  if (!match) {
    return null;
  }
  const value = Number(match[1].replace(",", "."));
  const unit = (match[2] ?? "mm").toLowerCase();
  const factor: Record<string, number> = { mm: 1, cm: 10, in: 25.4, pt: 25.4 / 72, px: 25.4 / 96 };
  return value * factor[unit];
}

/** The step of an arrow key in the Size field (UX notes §3): 1 mm, Shift 10 mm, Ctrl 0.1 mm. */
export function sizeStep(shift: boolean, ctrl: boolean): number {
  return ctrl ? 0.1 : shift ? 10 : 1;
}

/** One listed face, as the session lists it. */
export interface FontFace {
  index: number;
  block: "curvyo" | "included" | "system";
  family: string;
  style: string;
  id: string;
  /** The face's index in its file (a collection holds several). */
  faceIndex: number;
  loaded: boolean;
  /** A single-line face: its row draws the name in its own centre lines (`0076` criterion 6). */
  singleLine?: boolean;
}

/** What the Fonts tab shows (criteria 15 to 19, 29). */
export interface FontsTabData {
  faces: FontFace[];
  skipped: string[];
  systemRead: boolean;
  current: {
    family: string;
    style: string;
    missing: boolean;
    index: number | null;
    /** The face a missing one is drawn in (`0076` criterion 22). */
    substitute?: string;
  } | null;
  mixed: boolean;
  next: boolean;
}

export const EMPTY_FONTS: FontsTabData = {
  faces: [],
  skipped: [],
  systemRead: false,
  current: null,
  mixed: false,
  next: true,
};

/** One family of a block: its faces, the Regular (or first) face first for a pick. */
export interface FontFamily {
  block: FontFace["block"];
  family: string;
  faces: FontFace[];
}

/** A block of the Fonts tab. */
export interface FontBlockRows {
  block: FontFace["block"];
  title: string;
  families: FontFamily[];
  /** Families before the search. */
  total: number;
}

const BLOCK_TITLES: Record<FontFace["block"], string> = {
  curvyo: "Curvyo fonts",
  included: "Included",
  system: "This computer",
};

const STYLE_ORDER = ["regular", "italic", "bold", "bold italic"];

function styleRank(style: string): number {
  const at = STYLE_ORDER.indexOf(style.toLowerCase());
  return at < 0 ? STYLE_ORDER.length : at;
}

/** The blocks of the Fonts tab in their order (criterion 16), each family A to Z
 * case-insensitive, faces Regular first, filtered by a case-insensitive substring of the family
 * name (criterion 17) and by `inGroup`, the chosen strip cell (`0075` criterion 2). A block without a
 * family is left out, except "This computer" on the desktop before the system list arrives (it
 * shows "Reading…") and while `keepSystem` (its fonts are being checked). */
export function fontBlocks(
  data: FontsTabData,
  search: string,
  desktop: boolean,
  inGroup?: (family: FontFamily) => boolean,
  keepSystem = false,
): FontBlockRows[] {
  const needle = search.trim().toLowerCase();
  const blocks: FontBlockRows[] = [];
  for (const block of ["curvyo", "included", "system"] as const) {
    const families = new Map<string, FontFace[]>();
    for (const face of data.faces) {
      if (face.block !== block) {
        continue;
      }
      const list = families.get(face.family) ?? [];
      list.push(face);
      families.set(face.family, list);
    }
    const all = [...families.entries()]
      .map(([family, faces]) => ({
        block,
        family,
        faces: faces.sort((a, b) => styleRank(a.style) - styleRank(b.style) || a.style.localeCompare(b.style)),
      }))
      .sort((a, b) => a.family.toLowerCase().localeCompare(b.family.toLowerCase()));
    const shown = all.filter(
      (f) => (!needle || f.family.toLowerCase().includes(needle)) && (!inGroup || inGroup(f)),
    );
    const reading = block === "system" && desktop && ((!data.systemRead && !needle) || keepSystem);
    if (shown.length > 0 || reading) {
      blocks.push({ block, title: BLOCK_TITLES[block], families: shown, total: all.length });
    }
  }
  return blocks;
}

/** The count at the right of a block header: "12", "3 of 120" with a search, and the skipped
 * files of the Curvyo folder ("12 · 2 skipped"). */
export function blockCount(rows: FontBlockRows, searching: boolean, skipped: number): string {
  const count = searching ? `${rows.families.length} of ${rows.total}` : String(rows.total);
  return skipped > 0 && rows.block === "curvyo" ? `${count} · ${skipped} skipped` : count;
}

/** The family row's accessible name: "Great Vibes, included, 1 face". */
export function familyName(family: FontFamily): string {
  const faces = family.faces.length === 1 ? "1 face" : `${family.faces.length} faces`;
  const block = BLOCK_TITLES[family.block].toLowerCase();
  return `${family.family}, ${block}, ${faces}`;
}

/** The Fonts tab's subject line (UX notes §4): the current face, "Mixed", or the face of the
 * next text. */
export function fontsSubject(data: FontsTabData): string {
  if (data.mixed) {
    return "Mixed";
  }
  if (!data.current) {
    return "";
  }
  const face = `${data.current.family} ${data.current.style}`;
  return data.next ? `Next text: ${face}` : face;
}

/** The event the Fonts tab listens for to focus the current face row. */
export const FOCUS_CURRENT_FONT_EVENT = "curvyo-focus-current-font";

/** Asks the Fonts tab to focus the current face row (the Font button, Shift+Ctrl+T), once it is
 * shown. */
export function focusCurrentFont(): void {
  window.requestAnimationFrame(() =>
    window.requestAnimationFrame(() => window.dispatchEvent(new Event(FOCUS_CURRENT_FONT_EVENT))),
  );
}
