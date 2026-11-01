// Export SVG… (`specs/0024-svg-import-export` criteria 4, 131, 147, 148; `ux-notes.md` "SVG
// export: UX notes"): the four switches remembered per user, and the flow from the button to the
// file. The SVG and every sentence come from the session; this module only moves them.

/** The four switches of the File tab. */
export interface SvgExportOptions {
  textAsPaths: boolean;
  strokesAsPaths: boolean;
  selectionOnly: boolean;
  activePageOnly: boolean;
}

/** All off: the first time and whenever the stored value cannot be read. */
export const DEFAULT_SVG_EXPORT_OPTIONS: SvgExportOptions = {
  textAsPaths: false,
  strokesAsPaths: false,
  selectionOnly: false,
  activePageOnly: false,
};

/** Where the switches are kept, per user, never in a project (criterion 147). */
export const SVG_EXPORT_OPTIONS_KEY = "curvyo.svgExportOptions";

/** The stored switches, or all off when storage is unreadable or holds something else. */
export function loadSvgExportOptions(storage: Pick<Storage, "getItem"> | null): SvgExportOptions {
  try {
    const text = storage?.getItem(SVG_EXPORT_OPTIONS_KEY);
    if (!text) {
      return { ...DEFAULT_SVG_EXPORT_OPTIONS };
    }
    const value = JSON.parse(text) as Record<string, unknown>;
    const flag = (name: keyof SvgExportOptions) => value[name] === true;
    return {
      textAsPaths: flag("textAsPaths"),
      strokesAsPaths: flag("strokesAsPaths"),
      selectionOnly: flag("selectionOnly"),
      activePageOnly: flag("activePageOnly"),
    };
  } catch {
    return { ...DEFAULT_SVG_EXPORT_OPTIONS };
  }
}

/** Keeps the switches; a storage that refuses is ignored (the defaults come back next time). */
export function saveSvgExportOptions(
  storage: Pick<Storage, "setItem"> | null,
  options: SvgExportOptions,
): void {
  try {
    storage?.setItem(SVG_EXPORT_OPTIONS_KEY, JSON.stringify(options));
  } catch {
    // Unwritable storage: the switches hold for this page only.
  }
}

/** What the session's `export_svg` answered. */
export type SvgExportAnswer =
  | { kind: "svg"; objects: number; kept: number; text: string }
  | { kind: "nothing-selected" };

/** Reads `"svg:<objects>:<kept>:<text>"` or `"nothing-selected"`. */
export function parseSvgExportAnswer(answer: string): SvgExportAnswer {
  if (!answer.startsWith("svg:")) {
    return { kind: "nothing-selected" };
  }
  const first = answer.indexOf(":", 4);
  const second = answer.indexOf(":", first + 1);
  return {
    kind: "svg",
    objects: Number(answer.slice(4, first)),
    kept: Number(answer.slice(first + 1, second)),
    text: answer.slice(second + 1),
  };
}

/** Reads the session's `"<ms>:<sentence>"` notice. */
export function parseTimedNotice(notice: string): { ms: number; text: string } {
  const colon = notice.indexOf(":");
  return { ms: Number(notice.slice(0, colon)), text: notice.slice(colon + 1) };
}
