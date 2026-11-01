// The words and small rules of the project in the page (`specs/0051-sidebar-and-no-menu/`
// criteria 16, 19, 25, 30): the file name of a path, the status line, the sentence of the
// unsaved-changes check, the notice after a save and the window title. Pure, so
// `tests/projectText.test.ts` covers it.

/** What a replacing action does, in the words of the check's sentence. */
export type ReplacingKind = "new" | "open" | "close" | "quit" | "handover";

/** The file name of a path, either separator, without the folders. */
export function baseName(path: string): string {
  // A trailing separator is not a name of its own: "dir/" is "dir".
  const parts = path.split(/[\\/]/).filter((part) => part !== "");
  return parts[parts.length - 1] ?? path;
}

/** The name the project block shows. */
export function projectName(fileName: string | null): string {
  return fileName === null || fileName === "" ? "Untitled" : fileName;
}

/** The status line under the name: "Unsaved changes" while dirty, otherwise whether the project
 * has a saved file to compare with. */
export function projectStatus(fileName: string | null, dirty: boolean): string {
  if (dirty) {
    return "Unsaved changes";
  }
  return fileName === null ? "Not saved yet" : "No changes since the last save";
}

const ENDS: Record<ReplacingKind, string> = {
  new: "start a new project",
  open: "open another project",
  handover: "open another project",
  close: "close it",
  quit: "quit",
};

/** The sentence of the inline check: what is at stake, what each answer does and that discarding
 * cannot be taken back. */
export function checkSentence(fileName: string | null, kind: ReplacingKind): string {
  const subject =
    fileName === null ? "This project has" : `${shortName(fileName, SENTENCE_NAME_MAX)} has`;
  return `${subject} unsaved changes. Save them before you ${ENDS[kind]}? Discarded changes cannot be brought back.`;
}

/** The longest name the check's sentence shows whole: the sentence keeps within the four lines
 * reserved for it. */
const SENTENCE_NAME_MAX = 40;

/** `name`, cut in the middle with an ellipsis when it is longer than `max`; the end
 * (the extension) stays. */
function shortName(name: string, max: number): string {
  if (name.length <= max) {
    return name;
  }
  const tail = name.slice(-12);
  return `${name.slice(0, max - tail.length - 1)}…${tail}`;
}

/** The window title, and the page title in the browser: "Curvyo", "<file> — Curvyo", and a
 * leading asterisk while the project has unsaved changes. The same rule as `window_title` in
 * `curvyo-app`. */
export function projectTitle(fileName: string | null, dirty: boolean): string {
  const marker = dirty ? "*" : "";
  return fileName === null || fileName === "" ? `${marker}Curvyo` : `${marker}${fileName} — Curvyo`;
}
