// The words of the font preview (`specs/0077-font-preview` criteria 12, 18, 22 and 26, UX notes
// §2 to §4): a tile's accessible name and description, the missing-glyph tooltip, the Size
// tooltip and the header's subject line. Pure, so the tests read them.

/** The most missing characters a tooltip names (criterion 18). */
const MAX_MISSING_SHOWN = 10;

/** What a tile's name says beyond the family. */
export interface TileFacts {
  favourite: boolean;
  cuttable: boolean;
  singleLine: boolean;
  current: boolean;
}

/** The tile's accessible name: "Lobster, favourite, Cuttable, current", parts only when true
 * (criterion 22). */
export function tileName(family: string, facts: TileFacts): string {
  return [
    family,
    facts.favourite ? "favourite" : null,
    facts.cuttable ? "Cuttable" : null,
    facts.singleLine ? "Single line" : null,
    facts.current ? "current" : null,
  ]
    .filter(Boolean)
    .join(", ");
}

/** "No glyph for: ä, ß", at most 10 characters then "…"; empty without missing characters. */
export function missingText(missing: string[]): string {
  if (missing.length === 0) {
    return "";
  }
  const shown = missing.slice(0, MAX_MISSING_SHOWN).join(", ");
  return `No glyph for: ${shown}${missing.length > MAX_MISSING_SHOWN ? ", …" : ""}`;
}

/** The tile's accessible description: what it shows, then the missing characters (P4):
 * "Anna & Jonas 2026. No glyph for: ä". */
export function tileDescription(shown: string, missing: string[]): string {
  const gap = missingText(missing);
  return gap ? `${shown}. ${gap}` : shown;
}

/** The Size slider's tooltip, name and value only (P3): "Size: 32 px". */
export function sizeTip(size: number): string {
  return `Size: ${size} px`;
}

/** A count with thousands separators, "1,240". */
function count(n: number): string {
  return n.toLocaleString("en-US");
}

/** The header's subject line in the Fonts tab's words (criterion 26): "All · 1,240",
 * "Cuttable · 3 of 42" with a search. */
export function subjectLine(cell: string, shown: number, total: number, searching: boolean): string {
  return searching ? `${cell} · ${count(shown)} of ${count(total)}` : `${cell} · ${count(shown)}`;
}
