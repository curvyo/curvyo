/**
 * The numbers line of a run (`specs/0029-laser-job-preview-and-output` UX notes 6.1): its text and
 * the fields the maker typed; the one decoder of both the operation rows and the fixture run lines.
 */

/** A typed field of a numbers line. */
export type TypedField = "power" | "speed" | "passes";
const TYPED: readonly TypedField[] = ["power", "speed", "passes"];

/** One numbers line ("35 %, 6,000 mm/min, 1 ×, 0.25 mm") with its typed fields. */
export interface NumbersLine {
  text: string;
  typed: TypedField[];
}

/** One line from its JSON value; `null` for a broken one. */
export function asNumbersLine(value: unknown): NumbersLine | null {
  if (typeof value !== "object" || value === null || Array.isArray(value)) return null;
  const line = value as { text?: unknown; typed?: unknown };
  if (typeof line.text !== "string" || !Array.isArray(line.typed)) return null;
  const typed: TypedField[] = [];
  for (const item of line.typed) {
    const field = TYPED.find((word) => word === item);
    if (field === undefined) return null;
    typed.push(field);
  }
  return { text: line.text, typed };
}

/** The lines of a list; `null` when it is no list or one line is broken. */
export function asNumbersLines(value: unknown): NumbersLine[] | null {
  if (!Array.isArray(value)) return null;
  const lines: NumbersLine[] = [];
  for (const item of value) {
    const line = asNumbersLine(item);
    if (line === null) return null;
    lines.push(line);
  }
  return lines;
}
