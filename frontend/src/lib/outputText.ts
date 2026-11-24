/**
 * The small texts and rules of the output group, the play bar and the export flow
 * (`specs/0029-laser-job-preview-and-output` UX notes 5 to 8, 11): everything the session does not
 * already send as text.
 */
import type { Check } from "./outputView.ts";

/** Words a screen reader hears before a check line (UX notes 6.3, sr-only). */
export function checkPrefix(check: Check): string {
  return check.severity === "block" ? "Blocks: " : "Warning: ";
}

/** The accessible name of the Export button. */
export const EXPORT_NAME = "Export job file";

/** The Start pick's value while the stored Start is damaged (criterion 4 e; Proposal). */
export const START_DAMAGED = "Damaged";

/** The title of the error dialog of a failed write. */
export const EXPORT_FAILED_TITLE = "Can't export job file";

/** The notice of `export_job_done` (`"<ms>:<text>"`, the text's lines after a newline); `null` for
 * an empty answer (an unknown token). */
export function parseExportNotice(answer: string): { text: string; ms: number } | null {
  const at = answer.indexOf(":");
  if (at <= 0) return null;
  const ms = Number(answer.slice(0, at));
  const text = answer.slice(at + 1);
  if (!Number.isFinite(ms) || text === "") return null;
  return { text, ms };
}

/** The play readout ("6 min 6 s of 9 min 51 s", time only) in runs of text; the " of " between the
 * times is muted (UX notes 5). */
export function readoutParts(readout: string): { text: string; muted: boolean }[] {
  return readout
    .split(/( of )/)
    .filter((part) => part !== "")
    .map((part) => ({ text: part, muted: part === " of " }));
}

/** The bar is as wide as the page frame between these widths, px (UX notes 5). */
export const BAR_WIDTH_RANGE = { min: 360, max: 640 } as const;
/** The slider's least width, px: 120 in a bar of its usual width; 48 when the canvas leaves less
 * than the bar's 360 px floor (a 760 px window), so the readout keeps its one full line (UX notes 5). */
export function sliderMinWidth(barWidth: number): number {
  return barWidth < BAR_WIDTH_RANGE.min ? 48 : 120;
}
/** The bar's gap under the frame, and from the canvas bottom while it sticks, px (UX notes 5, P8). */
export const BAR_GAP = 10;
export const BAR_STICK = 12;

/** Where the bar goes in a canvas `canvasWidth` x `canvasHeight` px: left-aligned with the frame,
 * kept inside the canvas and right of `leftInset` (the tool columns, `--rail-right` + 12), 10 px
 * under the frame, and stuck 12 px above the canvas bottom whenever the frame's bottom is out of
 * view or too low for the gap; `barHeight` is the bar's measured height. */
export function barPlace(
  frame: { left: number; bottom: number; width: number },
  canvas: { width: number; height: number },
  barHeight: number,
  leftInset = 8,
): { left: number; top: number; width: number } {
  const room = Math.max(0, canvas.width - leftInset - 8);
  const width = Math.min(BAR_WIDTH_RANGE.max, Math.max(BAR_WIDTH_RANGE.min, frame.width), room);
  const left = Math.max(leftInset, Math.min(frame.left, canvas.width - width - 8));
  const stuck = canvas.height - BAR_STICK - barHeight;
  const outOfView = frame.bottom < 0 || frame.bottom > canvas.height;
  const top = outOfView ? stuck : Math.min(frame.bottom + BAR_GAP, stuck);
  return { left, top: Math.max(0, top), width };
}

/** One comma-separated number of a numbers line, with the word "typed" after a typed one. */
export interface NumberPart {
  text: string;
  typed: boolean;
}

/** The word the core text already puts after a typed number; the view draws it once, in ink. */
const TYPED_WORD = " typed";
const TYPED_ORDER = ["power", "speed", "passes"] as const;

/** A numbers line ("90 %, 6,000 mm/min, 1 ×, 0.25 mm") as its numbers; the first three are power,
 * speed and passes, and a typed one is followed by "typed" in ink (UX notes 6.1). */
export function numbersParts(text: string, typed: readonly string[]): NumberPart[] {
  return text.split(", ").map((part, index) => ({
    text: part.endsWith(TYPED_WORD) ? part.slice(0, -TYPED_WORD.length) : part,
    typed: index < TYPED_ORDER.length && typed.includes(TYPED_ORDER[index]),
  }));
}

/** A numbers line as one text, each typed number followed by "typed": the title of a line the
 * ellipsis cuts (`specs/0123` UX review U7). */
export function numbersTitle(text: string, typed: readonly string[], prefix = ""): string {
  const parts = numbersParts(text, typed).map((part) => `${part.text}${part.typed ? " typed" : ""}`);
  return `${prefix}${parts.join(", ")}`;
}

/** The name of the profile at the front of the machine line ("Diode 10 W"), and the rest. */
export function machineLineParts(line: string): { name: string; rest: string } {
  const at = line.indexOf(" · ");
  return at < 0 ? { name: line, rest: "" } : { name: line.slice(0, at), rest: line.slice(at) };
}
