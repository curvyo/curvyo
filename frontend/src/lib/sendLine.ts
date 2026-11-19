// The length rule of a connector state line (`specs/0125-send-to-machine` UX notes §4, U5): a job
// name is cut with an ellipsis so a line stays within 70 characters. Since `specs/0118` (Frage 262
// A) the one state line of the Operations tab is the Machine controls footer's
// (`src/lib/machineControls.ts`). Pure, so the tests load it without a bundler.

/** The longest line (UX notes §4, U5). */
export const LINE_CHARS = 70;

/** `name` cut at its end with an ellipsis so that `before + name + after` stays within
 * [`LINE_CHARS`]. */
export function withName(before: string, name: string, after = ""): string {
  const room = LINE_CHARS - before.length - after.length;
  const shown = name.length <= room ? name : `${name.slice(0, Math.max(room - 1, 0))}…`;
  return `${before}${shown}${after}`;
}
