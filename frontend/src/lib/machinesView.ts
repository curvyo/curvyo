/**
 * The decoders of the Machines tab (`curvyo-editor-wasm/src/session/machines/`, ADR 0098
 * decisions 4 and 5) and of the Machine group of an output page's Document tab. A broken value
 * decodes to `null` and the tab shows nothing rather than a guess.
 */

/** One field of a machine's body, in the order of UX unify 4.3. */
export interface MachineField {
  key: string;
  label: string;
  value: string;
  unit: string;
  kind: "number" | "switch" | "corner" | "text" | "entry";
}

/** One machine row. */
export interface MachineRow {
  id: string;
  name: string;
  line2: string;
  builtin: boolean;
  fields: MachineField[];
}

/** One family's section. */
export interface MachineSection {
  /** The family key as the session sends it (`plotter`, `embroidery`, `cnc` and the laser's). */
  family: string;
  word: string;
  rows: MachineRow[];
  active: boolean;
  notices: string[];
}

/** The inline question after an edit (Frage 434 C). */
export interface MachineFollow {
  id: string;
  sentence: string;
  pages: number;
}

/** The whole Machines tab. */
export interface MachinesView {
  sections: MachineSection[];
  follow: MachineFollow | null;
}

/** The Machine group of an output page: the picker and the copy's state. */
export interface MachinePicker {
  family: MachineSection["family"];
  id: string | null;
  name: string;
  state: "differs" | "missing" | null;
  profiles: { id: string; name: string; detail: string; chosen: boolean }[];
}

type Json = Record<string, unknown>;

function isObject(value: unknown): value is Json {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

const KINDS = ["number", "switch", "corner", "text", "entry"] as const;

function asField(value: unknown): MachineField | null {
  if (!isObject(value)) return null;
  const { key, label, unit, kind } = value;
  if (typeof key !== "string" || typeof label !== "string" || typeof unit !== "string") return null;
  if (typeof value.value !== "string" || !(KINDS as readonly unknown[]).includes(kind)) return null;
  return { key, label, value: value.value, unit, kind: kind as MachineField["kind"] };
}

function asRow(value: unknown): MachineRow | null {
  if (!isObject(value) || !Array.isArray(value.fields)) return null;
  const { id, name, line2, builtin } = value;
  if (typeof id !== "string" || typeof name !== "string" || typeof line2 !== "string" || typeof builtin !== "boolean") {
    return null;
  }
  const fields = value.fields.map(asField);
  if (fields.some((f) => f === null)) return null;
  return { id, name, line2, builtin, fields: fields as MachineField[] };
}

/** The Machines tab from its JSON text; `null` off an output page or for a broken value. */
export function decodeMachinesView(text: string): MachinesView | null {
  let value: unknown;
  try {
    value = JSON.parse(text);
  } catch {
    return null;
  }
  if (!isObject(value) || !Array.isArray(value.sections)) return null;
  const sections: MachineSection[] = [];
  for (const raw of value.sections) {
    if (!isObject(raw) || typeof raw.family !== "string" || !Array.isArray(raw.rows)) return null;
    const rows = raw.rows.map(asRow);
    if (rows.some((r) => r === null) || typeof raw.word !== "string" || typeof raw.active !== "boolean") return null;
    const notices = Array.isArray(raw.notices) ? raw.notices.filter((n): n is string => typeof n === "string") : [];
    sections.push({ family: raw.family, word: raw.word, rows: rows as MachineRow[], active: raw.active, notices });
  }
  const f = value.follow;
  const follow =
    isObject(f) && typeof f.id === "string" && typeof f.sentence === "string" && typeof f.pages === "number"
      ? { id: f.id, sentence: f.sentence, pages: f.pages }
      : null;
  return { sections, follow };
}
