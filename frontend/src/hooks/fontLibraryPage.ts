// The font library's state that lives as long as the page (`specs/0075-font-library`): the
// wasm library and check cache, their file status, check results, shipped data, and what the
// listing knows of a family's representative face.
import { wasmReady } from "@/lib/editorSession";
import type { FontFace as ListedFace, FontFamily } from "@/lib/textText";
import { hostListing } from "@/hooks/useText";
import { invoke } from "@/platform/host";
import { WasmFontChecks, WasmFontLibrary } from "@/wasm-bindings/curvyo_editor_wasm.js";

/** The page's library, cache and their state; one per page, kept across tab switches and
 * sessions. */
export const page = {
  library: null as WasmFontLibrary | null,
  checks: null as WasmFontChecks | null,
  /** "ok", "newer" (never written over) or "broken" (set aside at the first change). */
  status: "ok" as string,
  /** Check results by `<host id>#<index>`. */
  results: new Map<string, number>(),
  /** Families of the system block already queued. */
  systemQueued: false,
  /** Notices shown once per session. */
  shown: new Set<string>(),
  shipped: new Map<string, { tags: string[]; pieces: number }>(),
  loading: null as Promise<void> | null,
};

export function loadPage(): Promise<void> {
  page.loading ??= (async () => {
    await wasmReady();
    page.library = new WasmFontLibrary();
    page.checks = new WasmFontChecks();
    const [library, checks, index] = await Promise.all([
      invoke<string | null>("read_font_library").catch(() => null),
      invoke<string | null>("read_font_checks").catch(() => null),
      fetch(`${import.meta.env.BASE_URL}fonts/index.json`)
        .then((r) => r.json() as Promise<{ families?: { family: string; tags: string[]; cuttable_pieces: number }[] }>)
        .catch(() => ({ families: [] })),
    ]);
    if (library !== null) {
      page.status = page.library.load(library);
    }
    if (checks !== null) {
      page.checks.load(checks);
    }
    for (const family of index.families ?? []) {
      page.shipped.set(family.family, { tags: family.tags, pieces: family.cuttable_pieces });
    }
  })();
  return page.loading;
}

/** The representative face of a family: Regular, else the first (`adrs.md` decision 5); the
 * blocks list Regular first. */
export function representative(family: FontFamily): ListedFace {
  return family.faces[0];
}

export function resultKey(face: ListedFace): string {
  return `${face.id}#${face.faceIndex}`;
}

export function stampOf(face: ListedFace): string {
  const listed = hostListing(face.block).find((f) => f.id === face.id && f.index === face.faceIndex);
  return listed?.stamp ?? "";
}

export function os2Of(face: ListedFace): string | null {
  const listed = hostListing(face.block).find((f) => f.id === face.id && f.index === face.faceIndex);
  return listed?.os2 ?? null;
}

/** Which of the family `names` are starred now, read from the library (the font preview's tiles,
 * `specs/0077-font-preview` criterion 14, also while the Fonts tab is not shown). */
export function favouritesAmong(names: string[]): Set<string> {
  const library = page.library;
  if (!library || names.length === 0) {
    return new Set();
  }
  const raw = JSON.parse(library.view(JSON.stringify(names))) as { favourites: number[] };
  return new Set(raw.favourites.map((at) => names[at]));
}
