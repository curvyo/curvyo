// The tag lines of the families in the Fonts tab or the font preview's tiles
// (`specs/0075-font-library` UX tags amendment §4, §5 and §8, `specs/0077-font-preview` criterion
// 14): which family's line holds the add field, where the focus goes when it closes, entering a
// line with Shift+F2, and removing a tag with the focus moving on to the next chip.
import { useRef, useState } from "react";

import type { FontLibraryApi } from "@/hooks/useFontLibrary";
import { type RowTag, familyKey } from "@/lib/fontRows";
import { addFieldReturn, tagAfterRemove } from "@/lib/tagLineFocus";
import type { FontFamily } from "@/lib/textText";

export function useFontTagLine(
  lib: Pick<FontLibraryApi, "removeTag">,
  focusRow: (key: string) => void,
  /** A maker's press keeps the row in view until the cell or the search changes. */
  keep: (family: string) => void,
  /** The tree or the tile grid: the rows (`data-row-key`) are looked up inside it only. */
  scope: React.RefObject<HTMLElement | null>,
) {
  /** The family key whose tag line holds the add field. */
  const [adding, setAdding] = useState<string | null>(null);
  /** Where the focus goes when the add field closes: the Add tag button after F2, else the
   * element that had it before the press. */
  const addReturnRef = useRef<HTMLElement | "add-tag" | null>(null);

  /** The tag line of the family `family` in the scope. */
  const tagLineOf = (family: FontFamily) =>
    scope.current?.querySelector<HTMLElement>(`[data-row-key="${CSS.escape(familyKey(family))}"] [data-tag-line]`);

  const openAddTag = (family: FontFamily, byKeyboard: boolean) => {
    addReturnRef.current = byKeyboard ? "add-tag" : (document.activeElement as HTMLElement | null);
    if (addReturnRef.current === document.body) {
      addReturnRef.current = "add-tag";
    }
    setAdding(familyKey(family));
  };

  const closeAddTag = (family: FontFamily) => {
    setAdding(null);
    const saved = addReturnRef.current;
    addReturnRef.current = null;
    window.requestAnimationFrame(() => {
      // An element that is gone (or none) falls back to Add tag, so the focus is never lost.
      const back = saved === "add-tag" ? saved : addFieldReturn(false, saved);
      if (back === "add-tag") {
        tagLineOf(family)?.querySelector<HTMLElement>("[data-add-tag]")?.focus();
      } else {
        back.focus();
      }
    });
  };

  const focusTagLine = (family: FontFamily) => {
    focusRow(familyKey(family));
    window.requestAnimationFrame(() => {
      const controls = tagLineOf(family)?.querySelectorAll<HTMLElement>("[data-tag-id]");
      const target = controls && controls.length > 0 ? controls[controls.length - 1] : null;
      (target ?? tagLineOf(family)?.querySelector<HTMLElement>("[data-add-tag]"))?.focus();
    });
  };

  const removeTag = (family: FontFamily, tag: RowTag, byKeyboard: boolean, order: RowTag[]) => {
    keep(family.family);
    lib.removeTag(family.family, tag.id);
    if (byKeyboard) {
      // The focus goes to the tag that takes its place (also one coming out of "+N"), else to
      // Add tag.
      const nextId = tagAfterRemove(
        order.map((t) => t.id),
        tag.id,
      );
      window.requestAnimationFrame(() => {
        const after = tagLineOf(family);
        const next = nextId ? after?.querySelector<HTMLElement>(`[data-tag-id="${CSS.escape(nextId)}"]`) : null;
        (next ?? after?.querySelector<HTMLElement>("[data-add-tag]"))?.focus();
      });
    }
  };

  return { adding, openAddTag, closeAddTag, focusTagLine, removeTag };
}
