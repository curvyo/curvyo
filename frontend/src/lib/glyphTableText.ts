// The glyph table's words (`specs/0082-glyph-table` criteria 4, 11, 16, 19 and 20, UX notes P7,
// §3, §5 and §6): the subject line, the empty-grid texts, a cell's accessible name, the detail
// bar's meta line and an empty cell's label.

/** "U+00E4": at least four hex digits, upper case. */
export function codePointText(cp: number): string {
  return `U+${cp.toString(16).toUpperCase().padStart(4, "0")}`;
}

/** The character of a code point. */
export function charOf(cp: number): string {
  return String.fromCodePoint(cp);
}

function count(n: number): string {
  return n.toLocaleString("en-US");
}

/** The header's subject line in Glyphs mode: "Great Vibes Regular · 412 characters", with a
 * search "· 3 of 412 characters", for a missing face "Lobster Regular · missing". */
export function glyphSubjectLine(face: string, total: number, shown: number, searching: boolean, missing: boolean): string {
  if (!face) {
    return "";
  }
  if (missing) {
    return `${face} · missing`;
  }
  const noun = total === 1 ? "character" : "characters";
  return searching ? `${face} · ${count(shown)} of ${count(total)} ${noun}` : `${face} · ${count(total)} ${noun}`;
}

/** The grid's text for a face that cannot be read (criterion 11). */
export function missingFaceText(family: string): string {
  return `${family} is missing. Its characters cannot be shown.`;
}

/** The grid's text for a face that lists nothing (UX notes P7). */
export function noCharactersText(face: string): string {
  return `${face} has no characters to insert.`;
}

/** The grid's text for a search without hits (criterion 16). */
export function noMatchText(family: string, query: string): string {
  return `No character in ${family} matches "${query.trim()}".`;
}

/** Shown while only a name search could match and the names are still loading (UX notes §6). */
export const NAMES_LOADING_TEXT = "Loading character names…";

/** A cell's accessible name (criterion 20): "Latin small letter a with diaeresis, U+00E4", or
 * the code point alone without a name. */
export function cellName(name: string | null, cp: number): string {
  if (!name) {
    return codePointText(cp);
  }
  const lower = name.toLowerCase();
  return `${lower.charAt(0).toUpperCase()}${lower.slice(1)}, ${codePointText(cp)}`;
}

/** The detail bar's meta line: "U+00E4 · adieresis · Latin-1 Supplement" (UX notes §5). */
export function detailMeta(cp: number, glyphName: string | null, block: string): string {
  return [codePointText(cp), glyphName, block].filter(Boolean).join(" · ");
}

/** The label of a cell whose glyph draws nothing (UX notes §3): the Unicode abbreviation ("SP",
 * "NBSP", "ZWJ"), else the code point in hex ("2002"). */
export function emptyCellLabel(abbreviation: string | null, cp: number): string {
  return abbreviation ?? cp.toString(16).toUpperCase().padStart(4, "0");
}

/** The polite announcement of an insert (UX notes §4). */
export function insertedText(character: string): string {
  return `Inserted ${character}`;
}

/** Whether a key on a cell goes to the search field (UX notes §4, P5): one printable character
 * without Ctrl, Alt or Meta, or Backspace. */
export function typesIntoSearch(key: string, ctrl: boolean, alt: boolean, meta: boolean): boolean {
  return key === "Backspace" || ([...key].length === 1 && !ctrl && !alt && !meta);
}

/** Whether a query can only match by character names (no single character, no code point form):
 * while the names load, such a search shows the loading text instead of "No character". */
export function needsNames(query: string): boolean {
  const q = query.trim();
  return [...q].length > 1 && !/^(?:[uU]\+[0-9a-fA-F]{1,6}|[0-9a-fA-F]{4,6})$/.test(q);
}
