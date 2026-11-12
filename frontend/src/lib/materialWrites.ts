// The host write list a session hands out after a library step (`specs/0028-material-test-library`
// adrs.md §6), decoded field by field: the frontend never trusts unchecked JSON.

/** One write of a library step, or a preference file's new text. */
export type MaterialWrite =
  | { op: "write"; id: string; text: string }
  | { op: "remove"; id: string }
  | { op: "order"; text: string }
  | { op: "grid_choice"; text: string };

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

/** One write of the session's list, or `null` when a field is missing or of the wrong type. */
export function decodeMaterialWrite(value: unknown): MaterialWrite | null {
  if (!isRecord(value)) {
    return null;
  }
  const { op, id, text } = value;
  switch (op) {
    case "write":
      return typeof id === "string" && typeof text === "string" ? { op, id, text } : null;
    case "remove":
      return typeof id === "string" ? { op, id } : null;
    case "order":
    case "grid_choice":
      return typeof text === "string" ? { op, text } : null;
    default:
      return null;
  }
}

/** The session's write list (`take_material_writes`); an entry that does not decode is left
 * out, a text that is no list is none. */
export function decodeMaterialWrites(text: string): MaterialWrite[] {
  let value: unknown;
  try {
    value = JSON.parse(text);
  } catch {
    return [];
  }
  if (!Array.isArray(value)) {
    return [];
  }
  return value.map(decodeMaterialWrite).filter((write) => write !== null);
}


/** The name a record's JSON text carries, `null` when it is not readable. */
export function recordName(text: string): string | null {
  try {
    const value: unknown = JSON.parse(text);
    if (isRecord(value) && typeof value.name === "string" && value.name !== "") {
      return value.name;
    }
  } catch {
    // An unreadable text has no name.
  }
  return null;
}

/** The name of the material a failed write was about: the new text's name, else the kept record's
 * (`known` holds the record files as the host listed them, `<id>.json`). */
export function failedWriteName(
  write: MaterialWrite,
  known: readonly { name: string; text: string }[],
): string | null {
  if (write.op === "write") {
    const own = recordName(write.text);
    if (own !== null) return own;
  }
  if (write.op === "write" || write.op === "remove") {
    const kept = known.find((record) => record.name === `${write.id}.json`);
    return kept ? recordName(kept.text) : null;
  }
  return null;
}
