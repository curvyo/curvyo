/**
 * What a key does on the play slider (`specs/0029-laser-job-preview-and-output` UX notes 5): Left
 * and Right move 1 % of the run, Page Up and Page Down 10 %, Home and End go to the ends. The
 * session steps by percent and sets a fraction; this only names which.
 */

/** A step of the slider by `percent` of the run, or a jump to `fraction` (0 to 1). */
export type PlayKeyAction = { kind: "step"; percent: number } | { kind: "set"; fraction: number };

/** The action of `key` on the slider; `null` for a key the slider ignores. */
export function playKeyAction(key: string): PlayKeyAction | null {
  switch (key) {
    case "ArrowRight":
    case "ArrowUp":
      return { kind: "step", percent: 1 };
    case "ArrowLeft":
    case "ArrowDown":
      return { kind: "step", percent: -1 };
    case "PageUp":
      return { kind: "step", percent: 10 };
    case "PageDown":
      return { kind: "step", percent: -10 };
    case "Home":
      return { kind: "set", fraction: 0 };
    case "End":
      return { kind: "set", fraction: 1 };
    default:
      return null;
  }
}
