// The font library in the page (`specs/0075-font-library`): the maker's library and the check
// cache as wasm objects that live as long as the page, their files, what is known of each listed
// family, and Add font and Remove (the cuttable check is `useCuttableChecks`). The rules are in Rust
// (`curvyo-ui-core`); this hook only moves text, bytes and results.
import { useCallback, useEffect, useMemo, useRef, useState } from "react";

import { addBrowserFontFiles, type BrowserAddResult, pickBrowserFontFiles } from "@/lib/browserFonts";
import {
  type AddResult,
  LIBRARY_NEWER,
  LIBRARY_READ_FAILED,
  LIBRARY_SAVE_FAILED,
  STORAGE_FULL_NOTICE,
  addNotice,
  keptNotice,
} from "@/lib/fontLibraryText";
import { type FontFamily, type FontsTabData, fontBlocks } from "@/lib/textText";
import { forgetFileBytes, hostListing } from "@/hooks/useText";
import type { RowTag } from "@/lib/fontRows";
import { loadPage, os2Of, page, representative, resultKey } from "@/hooks/fontLibraryPage";
import { useCuttableChecks } from "@/hooks/useCuttableChecks";
import { invoke, isTauri } from "@/platform/host";

/** One tag of the strip. */
export interface FontTagView {
  /** The built-in id, or the own tag's name. */
  id: string;
  name: string;
  builtin: boolean;
  /** The listed family names that carry it. */
  members: Set<string>;
}

/** A notice of the tab: under the strip (library file) or under the Add font button. */
export interface FontNotice {
  id: number;
  anchor: "strip" | "add";
  kind: "success" | "refusal";
  text: string;
  ms: number;
}


export interface FontLibraryApi {
  ready: boolean;
  /** The chosen strip cell: "all", "favourites", a built-in id or an own tag's name. */
  chosen: string;
  /** Every tag in strip order. */
  tags: FontTagView[];
  isFavourite: (family: string) => boolean;
  isMember: (cell: string, family: string) => boolean;
  /** A family's tags in chip order with their sources. */
  tagsOf: (family: string) => RowTag[];
  choose: (id: string) => void;
  toggleFavourite: (family: string) => void;
  /** The add field's Enter: an error code, or `null` when added. */
  addTag: (family: string, name: string) => { error?: string } | null;
  removeTag: (family: string, id: string) => void;
  completions: (family: string, prefix: string) => string[];
  renameTag: (id: string, name: string) => { error?: string; taken?: string };
  deleteTag: (id: string) => void;
  /** The system fonts' check: done of total, while it runs. */
  progress: { done: number; total: number } | null;
  notice: FontNotice | null;
  /** Adding takes over 150 ms: how many files are being added (UX notes §4). */
  busy: number | null;
  /** Add font: the dialog (no argument), dropped paths (desktop) or files (browser). Resolves to
   * the first new family's name, or `null`. */
  addFonts: (dropped?: { paths?: string[]; files?: File[] }) => Promise<string | null>;
  removeFamily: (family: FontFamily) => Promise<void>;
}

export function useFontLibrary(
  data: FontsTabData,
  listHostFonts: () => Promise<void>,
): FontLibraryApi {
  const [ready, setReady] = useState(page.library !== null);
  const [revision, setRevision] = useState(0);
  const [notice, setNotice] = useState<FontNotice | null>(null);
  const [busy, setBusy] = useState<number | null>(null);
  const noticeCounter = useRef(0);
  const bump = useCallback(() => setRevision((r) => r + 1), []);

  const show = useCallback((anchor: FontNotice["anchor"], kind: FontNotice["kind"], text: string, ms?: number) => {
    noticeCounter.current += 1;
    setNotice({ id: noticeCounter.current, anchor, kind, text, ms: ms ?? (kind === "success" ? 3000 : 8000) });
  }, []);

  const showOnce = useCallback(
    (text: string) => {
      if (!page.shown.has(text)) {
        page.shown.add(text);
        show("strip", "refusal", text);
      }
    },
    [show],
  );

  useEffect(() => {
    let live = true;
    void loadPage().then(() => {
      if (!live) {
        return;
      }
      setReady(true);
      if (page.status === "newer") {
        showOnce(LIBRARY_NEWER);
      } else if (page.status === "broken") {
        showOnce(LIBRARY_READ_FAILED);
      }
    });
    return () => {
      live = false;
    };
  }, [showOnce]);

  useEffect(() => {
    if (!notice) {
      return undefined;
    }
    const timer = window.setTimeout(() => setNotice(null), notice.ms);
    return () => window.clearTimeout(timer);
  }, [notice]);

  /** Writes the library after a change (criterion 12): never over a newer file; a damaged one
   * is set aside first. */
  const persist = useCallback(() => {
    bump();
    const library = page.library;
    if (!library || page.status === "newer") {
      return;
    }
    const write = async () => {
      if (page.status === "broken") {
        await invoke("set_font_library_aside");
        page.status = "ok";
      }
      await invoke("write_font_library", { text: library.text() });
    };
    write().catch(() => showOnce(LIBRARY_SAVE_FAILED));
  }, [bump, showOnce]);


  // Every family of every block (unfiltered), for the facts and the view.
  const families = useMemo(() => fontBlocks(data, "", isTauri).flatMap((block) => block.families), [data]);
  const checks = useCuttableChecks(ready, families, data.systemRead, bump);

  // What is known of each family: its class bytes, the shipped tags and its check result.
  useEffect(() => {
    const library = page.library;
    if (!ready || !library) {
      return;
    }
    // Later entries win for a family listed twice: system, then included, then Curvyo.
    const order = { system: 0, included: 1, curvyo: 2 };
    const sorted = [...families].sort((a, b) => order[a.block] - order[b.block]);
    const facts = sorted.map((family) => {
      const face = representative(family);
      const shipped = family.block === "included" ? page.shipped.get(family.family) : undefined;
      const pieces = shipped ? shipped.pieces : page.results.get(resultKey(face));
      return {
        name: family.family,
        os2: os2Of(face),
        shipped: shipped ? [...shipped.tags, ...(shipped.pieces === 1 ? ["cuttable"] : [])] : null,
        pieces: pieces ?? null,
        single_line: family.faces.some((f) => f.singleLine),
      };
    });
    library.set_families(JSON.stringify(facts));
    bump();
  }, [ready, families, bump]);


  const library = page.library;
  // `revision` re-reads the library after every change and result.
  const view = useMemo(() => {
    void revision;
    if (!library) {
      return { chosen: "all", tags: [] as FontTagView[], favourites: new Set<string>(), rows: new Map<string, RowTag[]>() };
    }
    const names = families.map((family) => family.family);
    const raw = JSON.parse(library.view(JSON.stringify(names))) as {
      chosen: string;
      tags: { id: string; name: string; builtin: boolean; members: number[] }[];
      favourites: number[];
      rows: RowTag[][];
    };
    return {
      chosen: raw.chosen,
      tags: raw.tags.map((tag) => ({ ...tag, members: new Set(tag.members.map((at) => names[at])) })),
      favourites: new Set(raw.favourites.map((at) => names[at])),
      rows: new Map(raw.rows.map((tags, at) => [names[at], tags])),
    };
  }, [library, families, revision]);



  const tagById = useMemo(() => new Map(view.tags.map((tag) => [tag.id, tag])), [view]);

  const choose = useCallback(
    (id: string) => {
      if (!page.library?.choose(id)) {
        return;
      }
      if (id === "cuttable") {
        checks.requestSystem();
      }
      persist();
    },
    [persist, checks],
  );

  const addFonts = useCallback(
    async (dropped?: { paths?: string[]; files?: File[] }) => {
      const before = new Set(hostListing("curvyo").map((face) => face.id));
      // The dialog first, so the busy state counts the chosen files and never covers the dialog.
      let source = dropped ?? null;
      if (!source) {
        source = isTauri
          ? { paths: (await invoke<string[] | null>("pick_font_files").catch(() => null)) ?? undefined }
          : { files: (await pickBrowserFontFiles()) ?? undefined };
      }
      const files = source.files;
      const paths = source.paths;
      const count = files?.length ?? paths?.length ?? 0;
      if (count === 0) {
        return null;
      }
      const timer = window.setTimeout(() => setBusy(count), 150);
      let result: AddResult | BrowserAddResult | null = null;
      try {
        result = files
          ? await addBrowserFontFiles(files)
          : await invoke<AddResult>("add_font_files", { paths });
      } catch {
        result = null;
      } finally {
        window.clearTimeout(timer);
        setBusy(null);
      }
      if (!result) {
        return null;
      }
      if ("full" in result && result.full) {
        show("add", "refusal", STORAGE_FULL_NOTICE);
        return null;
      }
      await listHostFonts();
      const fresh = hostListing("curvyo").filter((face) => !before.has(face.id));
      const newFamilies = [...new Set(fresh.map((face) => face.family))];
      // Criterion 23: an own tag or Initials chosen in the strip goes on the added families; the
      // automatic tags never do.
      const chosen = tagById.get(view.chosen);
      const own = chosen && (!chosen.builtin || chosen.id === "initials") ? chosen : null;
      if (own && newFamilies.length > 0) {
        for (const family of newFamilies) {
          page.library?.add_tag(family, own.name);
        }
        persist();
      }
      const words = addNotice(result, own && result.added.length > 0 ? own.name : null);
      show("add", words.kind, words.text, /Not fonts:|Already there:/.test(words.text) ? 8000 : undefined);
      return newFamilies.sort((a, b) => a.toLowerCase().localeCompare(b.toLowerCase()))[0] ?? null;
    },
    [listHostFonts, tagById, view.chosen, persist, show],
  );

  const removeFamily = useCallback(
    async (family: FontFamily) => {
      const faces = family.faces.map((face) => ({ id: face.id, index: face.faceIndex }));
      const report = await invoke<{ removed: string[]; kept: string[] }>("remove_font_files", { faces }).catch(
        () => ({ removed: [], kept: [] as string[] }),
      );
      forgetFileBytes(faces.map((face) => face.id));
      await listHostFonts();
      if (report.kept.length > 0) {
        show("add", "refusal", keptNotice(family.family));
      }
    },
    [listHostFonts, show],
  );

  return {
    ready,
    chosen: view.chosen,
    tags: view.tags,
    isFavourite: (family) => view.favourites.has(family),
    isMember: (cell, family) =>
      cell === "all" ||
      (cell === "favourites" ? view.favourites.has(family) : (tagById.get(cell)?.members.has(family) ?? false)),
    tagsOf: (family) => view.rows.get(family) ?? [],
    choose,
    toggleFavourite: (family) => {
      page.library?.toggle_favourite(family);
      persist();
    },
    addTag: (family, name) => {
      const out = JSON.parse(page.library?.add_tag(family, name) ?? "{}") as { changed?: boolean; error?: string };
      if (out.error) {
        return { error: out.error };
      }
      if (out.changed) {
        persist();
      }
      return null;
    },
    removeTag: (family, id) => {
      if (page.library?.remove_tag(family, id)) {
        persist();
      }
    },
    completions: (family, prefix) => JSON.parse(page.library?.completions(family, prefix) ?? "[]") as string[],
    renameTag: (id, name) => {
      const out = JSON.parse(page.library?.rename_tag(id, name) ?? "{}") as { error?: string; taken?: string };
      if (!out.error) {
        persist();
      }
      return out;
    },
    deleteTag: (id) => {
      if (page.library?.delete_tag(id)) {
        persist();
      }
    },
    progress: checks.progress,
    notice,
    busy,
    addFonts,
    removeFamily,
  };
}
