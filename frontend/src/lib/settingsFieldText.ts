/**
 * Typed numbers of the settings fields (`specs/0028-material-test-library` `ux-notes.md` 8.3,
 * 10.3): the text becomes a number for the session, which holds the ranges and answers with the
 * chip text (empty when taken). Pure functions.
 */

/** The number a field's text names; `NaN` for text that is none (the session answers with its chip). */
export function typedNumber(text: string): number {
  const trimmed = text.trim().replace(",", ".");
  if (trimmed === "") return Number.NaN;
  return Number(trimmed);
}

/** The outcome string `EntryField` takes for a session answer: the chip is its own message. */
export function outcomeOf(chip: string): string {
  return chip === "" ? "committed" : `invalid:${chip}`;
}

/** `EntryField`'s message table where the code is the message itself (the session's chip text). */
export const CHIP_IS_MESSAGE: Readonly<Record<string, string>> = new Proxy(
  {},
  { get: (_target, key) => (typeof key === "string" ? key : undefined) },
);

/** The step of Up and Down in a field; Shift steps by 10 (ux-notes 12). */
export function stepOf(value: number | null, key: string, shift: boolean): number | null {
  if (key !== "ArrowUp" && key !== "ArrowDown") return null;
  const base = value ?? 0;
  const amount = shift ? 10 : 1;
  return key === "ArrowUp" ? base + amount : base - amount;
}

/** The step of Up and Down (Shift: 10) on typed text, as new text; `null` when the key is no step or the text no number (ux-notes 12). */
export function stepText(text: string, key: string, shift: boolean): string | null {
  if (key !== "ArrowUp" && key !== "ArrowDown") return null;
  const value = Number(text.trim().replace(",", "."));
  if (text.trim() === "" || !Number.isFinite(value)) return null;
  const next = value + (key === "ArrowUp" ? 1 : -1) * (shift ? 10 : 1);
  return String(Math.round(next * 10) / 10);
}
