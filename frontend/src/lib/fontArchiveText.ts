// The notice of an Add font that took zip archives (`specs/0083-font-info-and-zip` criterion 13,
// UX notes §4): the archive's name when it was added alone, else summed counts; at most 70
// characters per part, no emoji, no exclamation. Without archives the `0075` notice stands. And
// the font folder name rule the browser shares with the desktop.
import { type AddResult, addNotice, shortName } from "./fontLibraryText.ts";

const MAX_PART = 70;

function counted(count: number, one: string, many: string): string {
  return count === 1 ? `1 ${one}` : `${count} ${many}`;
}

function fonts(count: number): string {
  return counted(count, "font", "fonts");
}

function documents(count: number): string {
  return counted(count, "document", "documents");
}

function unsafePart(count: number): string | null {
  if (count === 0) {
    return null;
  }
  return count === 1 ? "1 file was skipped as unsafe." : `${count} files were skipped as unsafe.`;
}

/** "Pack.zip could not be read." or "Pack.zip and 2 more could not be read." */
function refusedPart(names: string[], one: string, many: string): string | null {
  if (names.length === 0) {
    return null;
  }
  const first = shortName(names[0]);
  return names.length === 1 ? `${first} ${one}` : `${first} and ${names.length - 1} more ${many}`;
}

/** The notice of an Add with archives; the `0075` notice when there were none. */
export function addNoticeWithArchives(
  result: AddResult,
  group: string | null,
): { text: string; kind: "success" | "refusal" } {
  const archives = result.archives ?? [];
  if (archives.length === 0) {
    return addNotice(result, group);
  }
  const loose = result.added.length + result.present.length + result.notFonts.length;
  const unsafe = archives.reduce((sum, a) => sum + a.unsafeEntries, 0);
  if (archives.length === 1 && loose === 0) {
    const archive = archives[0];
    const name = shortName(archive.name);
    let main: string;
    if (archive.refused === "unreadable") {
      main = `${name} could not be read. Nothing was added.`;
    } else if (archive.refused === "over_limit") {
      main = `${name} is over the size limit. Nothing was added.`;
    } else if (archive.refused === "not_written") {
      main = `${name} could not be added.`;
    } else if (archive.added > 0) {
      main =
        archive.documents > 0
          ? `${name}: ${fonts(archive.added)} added, ${documents(archive.documents)} kept.`
          : `${name}: ${fonts(archive.added)} added.`;
    } else if (archive.present > 0) {
      main = `${name}: ${fonts(archive.present)} ${archive.present === 1 ? "was" : "were"} already there.`;
    } else {
      main = `${name} holds no font. Nothing was added.`;
    }
    const parts = [main, unsafePart(unsafe)].filter((p): p is string => p !== null);
    return {
      text: parts.map((p) => p.slice(0, MAX_PART)).join(" "),
      kind: archive.added > 0 ? "success" : "refusal",
    };
  }
  const added = result.added.length + archives.reduce((sum, a) => sum + a.added, 0);
  const present = result.present.length + archives.reduce((sum, a) => sum + a.present, 0);
  const kept = archives.reduce((sum, a) => sum + a.documents, 0);
  const unreadable = archives.filter((a) => a.refused === "unreadable").map((a) => a.name);
  const over = archives.filter((a) => a.refused === "over_limit").map((a) => a.name);
  const notWritten = archives.filter((a) => a.refused === "not_written").map((a) => a.name);
  const empty = archives.filter((a) => a.refused === null && a.added === 0 && a.present === 0).length;
  const parts: (string | null)[] = [
    added > 0 ? `${fonts(added)} added.` : null,
    kept > 0 ? `${documents(kept)} kept.` : null,
    present > 0 ? `${added > 0 ? present : fonts(present)} ${present === 1 ? "was" : "were"} already there.` : null,
    result.notFonts.length === 1 ? `${shortName(result.notFonts[0])} is not a font and was not added.` : null,
    result.notFonts.length > 1 ? `${result.notFonts.length} files are not fonts and were not added.` : null,
    refusedPart(unreadable, "could not be read.", "could not be read."),
    refusedPart(over, "is over the size limit.", "are over the size limit."),
    refusedPart(notWritten, "could not be added.", "could not be added."),
    empty > 0 ? (empty === 1 ? "1 archive holds no font." : `${empty} archives hold no font.`) : null,
    unsafePart(unsafe),
  ];
  return {
    text: parts
      .filter((p): p is string => p !== null)
      .map((p) => p.slice(0, MAX_PART))
      .join(" "),
    kind: added > 0 ? "success" : "refusal",
  };
}

/** The first free folder name for `name` among `taken`, compared by Unicode lower case: `name`,
 * else `<name>-2`, `-3`, ... (the twin of `curvyo-text-core`'s `free_folder_name`, criterion 9). */
export function freeFolderName(name: string, taken: string[]): string {
  const isTaken = (candidate: string) => taken.some((t) => t.toLowerCase() === candidate.toLowerCase());
  if (!isTaken(name)) {
    return name;
  }
  for (let n = 2; ; n += 1) {
    if (!isTaken(`${name}-${n}`)) {
      return `${name}-${n}`;
    }
  }
}
