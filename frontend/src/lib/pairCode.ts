// The pairing code field (`specs/0096-own-devices-sync` ux-notes 4.2): what the maker types is
// upper-cased, spaces and dashes are dropped, and it is shown as two groups of four. Whether a
// text is a code (its alphabet, its check) is Rust's; this only shapes the field.

/** The number of characters of a code. */
export const PAIR_CODE_LENGTH = 8;

/** The typed text without spaces and dashes, upper-cased, at most eight characters. */
export function pairCodeCharacters(text: string): string {
  return text
    .replace(/[\s-]/g, "")
    .toUpperCase()
    .slice(0, PAIR_CODE_LENGTH);
}

/** The typed text as the field shows it: `K7Q2-9M4X`, the dash after the fourth character. */
export function formatPairCode(text: string): string {
  const characters = pairCodeCharacters(text);
  return characters.length > 4 ? `${characters.slice(0, 4)}-${characters.slice(4)}` : characters;
}

/** Whether all eight characters are there (Connect is available). */
export function pairCodeComplete(text: string): boolean {
  return pairCodeCharacters(text).length === PAIR_CODE_LENGTH;
}
