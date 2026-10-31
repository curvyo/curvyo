// The words of Recognise text (`specs/0088-trace-recognition` criteria 3, 4, 6, 7 and 10; UX
// notes §2 to §5): the bar's labels, the Faces readout, the tolerance field and its stored
// preference, and the result sentences built from the session's outcome. The session decides;
// this module only words its codes and counts.
import type { TraceParam } from "./traceText.ts";

/** The Select bar button, the bar's state label, the toolbar name and the History row. */
export const RECOGNISE_TEXT = "Recognise text";
/** The finish button. */
export const RECOGNISE = "Recognise";
/** The progress bar's name while a run is in flight. */
export const RECOGNISING = "Recognising";
/** The bar with nothing selected. */
export const RECOGNISE_EMPTY = "No paths selected.";
/** The bar with only images, texts, clones or open paths selected. */
export const RECOGNISE_NOT_PATHS = "No closed paths selected.";
/** No line was found at all. */
export const NO_TEXT_LINES = "No text lines found.";
/** The Faces readout's tooltip: the fixed order of the face sources (criterion 10). */
export const FACES_TOOLTIP = "Document, favourites, installed, included, system fonts";

/** The per-user preference of the tolerance (UX notes §6). */
export const TOLERANCE_KEY = "curvyo.recognise-tolerance";

/** The tolerance field (criterion 6; UX notes §2): mm, 0.05 to 2.00, step 0.05, two decimals. */
export const TOLERANCE_PARAM: TraceParam = {
  name: "tolerance",
  label: "Tolerance",
  kind: "length",
  unit: "mm",
  min: 0.05,
  max: 2,
  default: 0.3,
  step: 0.05,
  big_step: 0.5,
  fine_step: 0.01,
  group: "recognise",
  advanced: false,
  choices: [],
};

/** The stored tolerance, or the default when none or an unreadable one is stored. */
export function readTolerance(storage: Pick<Storage, "getItem"> | null): number {
  try {
    const value = Number.parseFloat(storage?.getItem(TOLERANCE_KEY) ?? "");
    if (Number.isFinite(value)) {
      return Math.min(TOLERANCE_PARAM.max, Math.max(TOLERANCE_PARAM.min, value));
    }
  } catch {
    // A blocked storage keeps the default.
  }
  return TOLERANCE_PARAM.default;
}

/** Stores the tolerance; a blocked storage keeps it for this page only. */
export function writeTolerance(storage: Pick<Storage, "setItem"> | null, value: number): void {
  try {
    storage?.setItem(TOLERANCE_KEY, String(value));
  } catch {
    // Nothing to do: the value stays for this page.
  }
}

/** The face sources as the session names them, in search order. */
export type FaceTierCode = "document" | "favourites" | "installed" | "included" | "system";

const TIER_NAMES: Record<FaceTierCode, string> = {
  document: "document fonts",
  favourites: "favourites",
  installed: "installed fonts",
  included: "included fonts",
  system: "system fonts",
};

/** The words of a face source ("included fonts"). */
export function tierName(code: string): string {
  return TIER_NAMES[code as FaceTierCode] ?? code;
}

/** The Faces readout before a run: "5 face sources" (criterion 10). */
export function facesReadout(sources: number): string {
  return `${sources} face ${sources === 1 ? "source" : "sources"}`;
}

/** A recognised line as the session reports it. */
export interface RecognisedLineView {
  string: string;
  face: string;
  tier: string;
  letters: number;
  /** Character and 1-based position. */
  guessed: [string, number][];
  deviation: number;
}

/** One step's outcome (`RecogniseOutcome::to_json`). */
export type RecogniseOutcome =
  | { kind: "ignored" }
  | { kind: "loading" }
  | { kind: "running"; done: number; total: number }
  | {
      kind: "applied";
      lines: RecognisedLineView[];
      left: number;
      skipped: { images: number; open_paths: number; others: number };
    }
  | {
      kind: "nothing";
      no_lines: boolean;
      nearest: { face: string; deviation: number } | null;
      searched: string | null;
    };

/** The outcome of a step's JSON; anything unreadable is "ignored". */
export function parseOutcome(json: string): RecogniseOutcome {
  try {
    const value = JSON.parse(json) as RecogniseOutcome;
    return typeof value?.kind === "string" ? value : { kind: "ignored" };
  } catch {
    return { kind: "ignored" };
  }
}

const plural = (n: number, one: string, many: string) => `${n} ${n === 1 ? one : many}`;

/** The most guessed letters a notice lists before "and N more" (criterion 7). */
const GUESSED_LISTED = 5;

/** "2 letters guessed: e (5), l (9)"; past five, "and 3 more" (criterion 7). */
export function guessedText(guessed: [string, number][]): string {
  const listed = guessed.slice(0, GUESSED_LISTED).map(([c, at]) => `${c} (${at})`).join(", ");
  const more = guessed.length - GUESSED_LISTED;
  const head = `${plural(guessed.length, "letter", "letters")} guessed: `;
  return more > 0 ? `${head}${listed} and ${more} more` : `${head}${listed}`;
}

/** "1 image and 2 open paths were left as they are." (UX notes §4, as Stroke to path). */
function skippedText(skipped: { images: number; open_paths: number; others: number }): string {
  const parts: string[] = [];
  if (skipped.images > 0) {
    parts.push(plural(skipped.images, "image", "images"));
  }
  if (skipped.open_paths > 0) {
    parts.push(plural(skipped.open_paths, "open path", "open paths"));
  }
  if (skipped.others > 0) {
    parts.push(plural(skipped.others, "other object", "other objects"));
  }
  if (parts.length === 0) {
    return "";
  }
  const total = skipped.images + skipped.open_paths + skipped.others;
  const list = parts.length === 1 ? parts[0] : `${parts.slice(0, -1).join(", ")} and ${parts[parts.length - 1]}`;
  return ` ${list} ${total === 1 ? "was" : "were"} left as ${total === 1 ? "it is" : "they are"}.`;
}

/** The success notice (criteria 3, 7 and 10; UX notes §4). */
export function appliedText(outcome: Extract<RecogniseOutcome, { kind: "applied" }>): string {
  const { lines } = outcome;
  const letters = lines.reduce((sum, line) => sum + line.letters, 0);
  const deviation = Math.max(0, ...lines.map((line) => line.deviation)).toFixed(2);
  const faces: { face: string; tier: string }[] = [];
  for (const line of lines) {
    if (!faces.some((f) => f.face === line.face && f.tier === line.tier)) {
      faces.push({ face: line.face, tier: line.tier });
    }
  }
  const lineWords = lines.length > 1 ? ` in ${lines.length} lines` : "";
  const faceWords =
    faces.length === 1
      ? ` as ${faces[0].face} (${tierName(faces[0].tier)})`
      : `, ${faces.length} faces (${faces.map((f) => `${f.face}, ${tierName(f.tier)}`).join("; ")})`;
  let text = `Recognised ${plural(letters, "letter", "letters")}${lineWords}${faceWords}, largest deviation ${deviation} mm`;
  const guessed = lines.flatMap((line) => line.guessed);
  if (guessed.length > 0) {
    text += `; ${guessedText(guessed)}`;
  }
  if (outcome.left > 0) {
    text += `; ${plural(outcome.left, "outline", "outlines")} left as paths`;
  }
  return `${text}.${skippedText(outcome.skipped)}`;
}

/** The notice row when nothing is within the tolerance (criterion 4; UX notes §4). */
export function nothingText(outcome: Extract<RecogniseOutcome, { kind: "nothing" }>, tolerance: number): string {
  if (outcome.no_lines || !outcome.nearest) {
    return NO_TEXT_LINES;
  }
  return `No text within ${tolerance.toFixed(2)} mm; nearest face ${outcome.nearest.face}, ${outcome.nearest.deviation.toFixed(2)} mm.`;
}

/** The Faces readout after "nothing within tolerance": "Searched: system fonts". */
export function searchedReadout(searched: string | null): string | null {
  return searched ? `Searched: ${tierName(searched)}` : null;
}

/** How long the success notice stays: 8 s past 70 characters, 3 s below (UX notes §4, row
 * "Action notice"). */
export function noticeMs(text: string): number {
  return text.length > 70 ? 8000 : 3000;
}
