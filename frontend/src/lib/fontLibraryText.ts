// The words of the font library in the Fonts tab (`specs/0075-font-library`, UX notes §2 to §5):
// notices, prompts, chip tooltips, empty states and progress. Pure functions, so the tests read
// them without a session.

/** A name in a notice longer than this is cut with an ellipsis (UX notes §4). */
const MAX_NAME_IN_NOTICE = 24;

/** A notice part's limit and a whole notice's limit with file names (design system, "Action
 * notice": 70 characters, 140 with names for 8 s). */
const MAX_PART = 70;
const MAX_WITH_NAMES = 140;

/** `name` cut to `max` characters with an ellipsis. */
export function shortName(name: string, max = MAX_NAME_IN_NOTICE): string {
  return name.length > max ? `${name.slice(0, max - 1)}…` : name;
}

function fonts(count: number): string {
  return count === 1 ? "1 font" : `${count} fonts`;
}

/** What Add font did, by file name. */
export interface AddResult {
  added: string[];
  present: string[];
  notFonts: string[];
}

/** The notice of an Add (criteria 22, 23): what happened, the names of the refused and present
 * files when they fit, success when one font was added, refusal otherwise. */
export function addNotice(result: AddResult, group: string | null): { text: string; kind: "success" | "refusal" } {
  const parts: string[] = [];
  const added = result.added.length;
  if (added > 0) {
    parts.push(group ? `${fonts(added)} added to ${shortName(group)}.` : `${fonts(added)} added.`);
  }
  const present = result.present.length;
  if (present > 0) {
    const verb = present === 1 ? "was" : "were";
    parts.push(added > 0 ? `${present} ${verb} already there.` : `${fonts(present)} ${verb} already there.`);
  }
  const refused = result.notFonts.length;
  if (refused === 1) {
    parts.push(`${shortName(result.notFonts[0])} is not a font and was not added.`);
  } else if (refused > 1) {
    parts.push(`${refused} files are not fonts and were not added.`);
  }
  let text = parts.map((part) => part.slice(0, MAX_PART)).join(" ");
  const lists: [string, string[]][] = [
    ["Not fonts", refused > 1 ? result.notFonts : []],
    ["Already there", result.present],
  ];
  for (const [label, names] of lists) {
    if (names.length === 0) {
      continue;
    }
    const list = nameList(names, MAX_WITH_NAMES - text.length - label.length - 4);
    if (list) {
      text = `${text} ${label}: ${list}.`;
    }
  }
  return { text, kind: added > 0 ? "success" : "refusal" };
}

/** The names joined with commas while they fit in `room` characters, else the first two and
 * "and 3 more"; `null` when not even that fits. */
export function nameList(names: string[], room: number): string | null {
  const all = names.map((name) => shortName(name)).join(", ");
  if (all.length <= room) {
    return all;
  }
  const first = names.slice(0, 2).map((name) => shortName(name)).join(", ");
  const short = `${first} and ${names.length - 2} more`;
  return short.length <= room ? short : null;
}

/** The validation chip of a tag name (criterion 7). */
export function tagNameError(error: string, taken?: string): string {
  switch (error) {
    case "empty":
      return "Enter a name.";
    case "long":
      return "Use at most 24 characters.";
    case "reserved":
      return "All and Favourites are not tags.";
    case "taken":
      return `A tag named ${shortName(taken ?? "")} exists.`;
    default:
      return "Curvyo holds at most 100 tags.";
  }
}

/** The inline confirmation of Delete tag (criterion 8). */
export function deleteTagPrompt(name: string, members: number): string {
  const tag = shortName(name);
  return members === 0 ? `Delete tag ${tag}?` : `Delete tag ${tag}? Its ${fonts(members)} ${members === 1 ? "stays" : "stay"} in Curvyo.`;
}

/** The inline confirmation of Remove (criterion 25). */
export function removePrompt(family: string, texts: number): string {
  const first = `Remove ${shortName(family)} from Curvyo fonts?`;
  if (texts === 0) {
    return first;
  }
  return texts === 1 ? `${first} 1 text here uses it.` : `${first} ${texts} texts here use it.`;
}

/** The notice when a collection file was kept (criterion 25). */
export function keptNotice(family: string): string {
  return `${shortName(family)} is in a file with other fonts and was kept.`;
}

/** The tooltip of a tag chip, by its source code from the library (UX tags amendment §2);
 * `null` for an own tag. */
export function chipTooltip(source: string): string | null {
  if (source === "you") {
    return "Added by you";
  }
  if (source === "shipped") {
    return "Set for the included fonts";
  }
  if (source.startsWith("declared:")) {
    return `Declared by the font: ${source.slice("declared:".length)}`;
  }
  if (source === "detected:single_line") {
    return "Detected: single-line font";
  }
  if (source.startsWith("detected:")) {
    const pieces = Number(source.slice("detected:".length));
    return pieces === 1 ? "Detected: one piece" : `Detected: ${pieces} pieces`;
  }
  return null;
}

/** The family row's accessible name part and tooltip line for its tags; empty without tags. */
export function tagsLine(names: string[]): string {
  return names.length === 0 ? "" : `Tags: ${names.join(", ")}`;
}

/** The overflow chip's accessible name (UX tags amendment §2). */
export function moreTagsName(count: number): string {
  return count === 1 ? "1 more tag, show all" : `${count} more tags, show all`;
}

/** The empty list of a strip cell (criterion 3, UX notes §5). */
export function emptyTagText(group: string, search: string, checking: boolean): string {
  const needle = search.trim();
  if (needle) {
    return `No font in ${group} matches "${needle}".`;
  }
  return checking ? `No font in ${group} yet.` : `No font in ${group}.`;
}

/** The progress of the system fonts' check (UX notes §5). */
export function checkingText(done: number, total: number): string {
  return `checking ${done}/${total}`;
}

/** The Cuttable cell's description while checking. */
export function checkingDescription(done: number, total: number): string {
  return `Checking fonts: ${done} of ${total}`;
}

/** The refusal under the Add font button when the browser's storage is full (criterion 30). */
export const STORAGE_FULL_NOTICE = "The browser has no room for this font. Nothing was added.";
/** The library file's notices (criterion 12, `adrs.md` decision 8). */
export const LIBRARY_SAVE_FAILED = "The font library could not be saved.";
export const LIBRARY_READ_FAILED = "The font library could not be read.";
export const LIBRARY_NEWER = "The font library was made by a newer version of Curvyo.";

/** The busy text while adding (UX notes §4). */
export function addingText(count: number): string {
  return count === 1 ? "Adding 1 font…" : `Adding ${count} fonts…`;
}
