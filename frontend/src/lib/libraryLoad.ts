// Loading from the library into the open document (`specs/0137`, `specs/0138`, `specs/0152`
// criterion 7): Import page, Load template and an icon import, the same paths for the Library
// tab's rows and the library browser's tiles. The wasm copies; the page shows what it answers.
import type { WasmSession } from "@/lib/editorSession";
import { type SlotUnitSource, fillSlots as fillSlotsOf, putIntoSlot, slotTargets } from "@/lib/intoSlot";
import type { SlotDrag } from "@/lib/libraryDrag";
import { type ImportFrom, importFromLibrary, loadTemplateInto } from "@/lib/libraryImport";
import { previewNow, readerFor } from "@/lib/libraryStore";
import { loadOnlineTemplate, onlineIconImport, onlineTemplateOrigin } from "@/lib/onlineApi";
import { WasmLibrarySource, library_import_failed_text } from "@/wasm-bindings/curvyo_editor_wasm.js";

/** What a load needs of the editor. */
export interface LoadContext {
  getSession: () => WasmSession | null;
  sync: () => void;
  showNotice: (text: string, ms?: number, source?: "canvas" | "panel", kind?: "status" | "alert") => void;
}

/** A point on the canvas in CSS px, or null for the centre of the visible drawing. */
export type DropAt = { x: number; y: number } | null;

/** Whether a source key is an online source's. */
export const isOnlineSource = (key: string) => key.startsWith("online:");

/** One page to import: of a local document or of a downloaded template. */
export interface PageToImport {
  source: string;
  path: string[];
  pageId: string;
  pageName: string;
  /** The document's name (a local document) for Came from. */
  documentName: string;
  /** The source's name (a template's catalogue) for Came from. */
  sourceName: string;
}

/** Import page (`0137` criterion 4): the page as a new page, or dropped at `drop`. */
export async function importPage(ctx: LoadContext, page: PageToImport, drop: DropAt): Promise<boolean> {
  const session = ctx.getSession();
  if (!session || !page.pageId) {
    return false;
  }
  const open = previewNow();
  const same = open.page && open.page.source === page.source && open.page.path.join("/") === page.path.join("/");
  try {
    const reader = same && open.reader ? open.reader : await readerFor(page.source, page.path);
    // A page of a downloaded template carries where it came from and under which licence.
    const origin = isOnlineSource(page.source) ? await onlineTemplateOrigin(page.source, page.path.join("/")) : null;
    const answer = importFromLibrary(
      session,
      reader,
      origin
        ? {
            documentName: page.sourceName,
            pageName: `${origin.name} › ${page.pageName}`,
            pageId: page.pageId,
            online: origin.origin,
            withoutFonts: origin.without_fonts,
          }
        : { documentName: page.documentName, pageName: page.pageName, pageId: page.pageId },
      true,
      [],
      drop,
    );
    if (reader !== open.reader) {
      reader.free();
    }
    ctx.sync();
    if (answer.error) {
      ctx.showNotice(library_import_failed_text(page.pageName, answer.error), 8000, "canvas", "alert");
      return false;
    }
    if (answer.notice) {
      ctx.showNotice(answer.notice, 5000, "canvas", "status");
    }
    return true;
  } catch (error) {
    ctx.showNotice(library_import_failed_text(page.pageName, String(error)), 8000, "canvas", "alert");
    return false;
  }
}

/** Load template (`0138` criterion 3): every page of it, one step; its name when loaded. */
export async function loadTemplate(
  ctx: LoadContext,
  source: string,
  path: string[],
  sourceName: string,
): Promise<string | null> {
  const session = ctx.getSession();
  if (!session) {
    return null;
  }
  const answer = await loadOnlineTemplate(source, path.join("/"));
  if (answer.kind === "refused") {
    ctx.showNotice(answer.text, 8000, "canvas", "alert");
    return null;
  }
  let reader: WasmLibrarySource | null = null;
  try {
    reader = await readerFor(source, path);
    const result = loadTemplateInto(session, reader, {
      documentName: sourceName,
      pageName: answer.name,
      pageId: "",
      online: answer.origin,
      withoutFonts: answer.without_fonts,
    });
    ctx.sync();
    if (result.error) {
      ctx.showNotice(library_import_failed_text(answer.name, result.error), 8000, "canvas", "alert");
      return null;
    }
    // No notice after a success (design system); the caller's polite region says it.
    if (result.notice) {
      ctx.showNotice(result.notice, 5000, "canvas", "status");
    }
    return answer.name;
  } catch (error) {
    ctx.showNotice(library_import_failed_text(answer.name, String(error)), 8000, "canvas", "alert");
    return null;
  } finally {
    reader?.free();
  }
}

/** An icon import (`0138` criterion 3): a path 24 mm wide, one step "Import icon". */
export async function importIcon(
  ctx: LoadContext,
  source: string,
  prefix: string,
  name: string,
  sourceName: string,
  at: DropAt,
): Promise<boolean> {
  const session = ctx.getSession();
  if (!session) {
    return false;
  }
  const icon = await onlineIconImport(source, prefix, name);
  if (!icon) {
    ctx.showNotice(library_import_failed_text(name, "the icon could not be read"), 8000, "canvas", "alert");
    return false;
  }
  let reader: WasmLibrarySource | null = null;
  try {
    reader = WasmLibrarySource.open_svg(icon.svg, BigInt(Math.floor(Math.random() * 2 ** 48) + 1));
    const roots = JSON.parse(reader.page_objects("first")) as { id: unknown }[];
    const answer = importFromLibrary(
      session,
      reader,
      {
        documentName: sourceName,
        pageName: `${icon.set} › ${icon.name}`,
        pageId: "first",
        online: icon.origin,
        step: "icon",
      },
      false,
      roots.map((r) => JSON.stringify(r.id)),
      at,
    );
    ctx.sync();
    if (answer.error) {
      ctx.showNotice(library_import_failed_text(name, answer.error), 8000, "canvas", "alert");
      return false;
    }
    return true;
  } catch (error) {
    ctx.showNotice(library_import_failed_text(name, String(error)), 8000, "canvas", "alert");
    return false;
  } finally {
    reader?.free();
  }
}

/** The reader and Came from of a page to import, for a put into a slot (`specs/0154`); `done`
 * frees a reader that is not the open preview's. */
export async function pageSlotSource(
  page: PageToImport,
): Promise<{ source: SlotUnitSource; done: () => void } | null> {
  if (!page.pageId) {
    return null;
  }
  const open = previewNow();
  const same = open.page && open.page.source === page.source && open.page.path.join("/") === page.path.join("/");
  const reader = same && open.reader ? open.reader : await readerFor(page.source, page.path);
  const origin = isOnlineSource(page.source) ? await onlineTemplateOrigin(page.source, page.path.join("/")) : null;
  const from: ImportFrom = origin
    ? {
        documentName: page.sourceName,
        pageName: `${origin.name} › ${page.pageName}`,
        pageId: page.pageId,
        online: origin.origin,
        withoutFonts: origin.without_fonts,
      }
    : { documentName: page.documentName, pageName: page.pageName, pageId: page.pageId };
  return {
    source: { reader, from, whole: true, roots: [] },
    done: () => {
      if (reader !== open.reader) {
        reader.free();
      }
    },
  };
}

/** The selected target slot of the active laser page, the target of a load gesture (UX V15). */
export function selectedTargetSlot(ctx: LoadContext): string | null {
  const session = ctx.getSession();
  return session ? slotTargets(session, null, false).selected : null;
}

/** A load gesture on a page (Ctrl+Enter, double-click): into the selected target slot when there
 * is one (`specs/0154` criterion 17), else Import page. */
export async function loadPage(ctx: LoadContext, page: PageToImport, realSize = false): Promise<boolean> {
  const slot = selectedTargetSlot(ctx);
  if (!slot) {
    return importPage(ctx, page, null);
  }
  const got = await pageSlotSource(page);
  if (!got) {
    return false;
  }
  return putIntoSlot(ctx, got.source, slot, realSize, false, got.done);
}

/** A load gesture on an icon or a template (double-click, Enter): into the selected target slot
 * when there is one (`specs/0154` UX notes 9, V15); null when there is none, so the caller loads
 * as before. A template of several pages is refused as over a slot. */
export async function intoSelectedSlot(ctx: LoadContext, resolve: () => Promise<ResolvedSlotSource>): Promise<boolean | null> {
  const slot = selectedTargetSlot(ctx);
  if (!slot) {
    return null;
  }
  const got = await resolve();
  if (!got) {
    ctx.showNotice("Nothing was changed: the item could not be read.", 8000, "canvas", "alert");
    return false;
  }
  if (got.source.whole && got.source.reader.page_count() > 1) {
    got.done();
    ctx.showNotice("Only one page goes into a slot.", 8000, "canvas", "alert");
    return false;
  }
  return putIntoSlot(ctx, got.source, slot, false, false, got.done);
}

/** A source resolved for a slot, and the call that frees its reader. */
export type ResolvedSlotSource = { source: SlotUnitSource; done: () => void } | null;

/** The icon of a cell as a slot source (`specs/0154` V1): one object, not wrapped. */
export async function iconSlotSource(
  source: string,
  prefix: string,
  name: string,
  sourceName: string,
): Promise<ResolvedSlotSource> {
  const icon = await onlineIconImport(source, prefix, name);
  if (!icon) {
    return null;
  }
  const reader = WasmLibrarySource.open_svg(icon.svg, BigInt(Math.floor(Math.random() * 2 ** 48) + 1));
  const roots = (JSON.parse(reader.page_objects("first")) as { id: unknown }[]).map((r) => JSON.stringify(r.id));
  return {
    source: {
      reader,
      from: { documentName: sourceName, pageName: `${icon.set} › ${icon.name}`, pageId: "first", online: icon.origin },
      whole: false,
      roots,
    },
    done: () => reader.free(),
  };
}

/** A one-page online template as a slot source (`specs/0154` V1): it is downloaded first; a
 * template of several pages is refused by the caller (`page_count`). */
export async function templateSlotSource(source: string, path: string[], sourceName: string): Promise<ResolvedSlotSource> {
  const answer = await loadOnlineTemplate(source, path.join("/"));
  if (answer.kind === "refused") {
    return null;
  }
  const reader = await readerFor(source, path);
  const pageId = reader.first_page_id();
  return {
    source: {
      reader,
      from: {
        documentName: sourceName,
        pageName: answer.name,
        pageId,
        online: answer.origin,
        withoutFonts: answer.without_fonts,
      },
      whole: true,
      roots: [],
    },
    done: () => reader.free(),
  };
}

/** How a source is read for a drag: `eager` reads it as soon as the drag reaches the canvas (a
 * local page, an icon), else only on the release (a template, which downloads); `pages` is a
 * known page count. */
export interface SlotDragRead {
  eager: boolean;
  pages?: number;
}

/** The slot route of a library drag (`specs/0154` criteria 1, 8 and 9): the cue gets the unit's
 * size and outlines once the source is read, the release puts it into the slot, fills the holder,
 * or activates another laser page. */
export function slotDragOf(ctx: LoadContext, resolve: () => Promise<ResolvedSlotSource>, read: SlotDragRead): SlotDrag {
  let used = false;
  let got: ResolvedSlotSource = null;
  let pending: Promise<ResolvedSlotSource> | null = null;
  const start = () => {
    pending ??= resolve().then(
      (value) => {
        got = value;
        return value;
      },
      () => null,
    );
    return pending;
  };
  return {
    getSession: ctx.getSession,
    redraw: ctx.sync,
    source: () => {
      if (read.eager) {
        void start();
      }
      return got?.source ?? null;
    },
    multiPage: () => (read.pages ?? 1) > 1 || (got !== null && got.source.whole && got.source.reader.page_count() > 1),
    onSlot: (cue, alt, ctrl) => {
      used = true;
      void start().then((value) => {
        if (!value) {
          ctx.showNotice("Nothing was changed: the item could not be read.", 8000, "canvas", "alert");
          return;
        }
        if (value.source.whole && value.source.reader.page_count() > 1) {
          ctx.showNotice("Only one page goes into a slot.", 8000, "canvas", "alert");
        } else if (cue.kind === "fill" && cue.holder) {
          fillSlotsOf(ctx, value.source, cue.holder, alt);
        } else if (cue.slot) {
          // The put owns the reader now: a filled slot's Replace still needs it.
          putIntoSlot(ctx, value.source, cue.slot, alt, false, value.done, ctrl && cue.kind === "slot");
          return;
        }
        value.done();
      });
    },
    end: () => {
      if (!used) {
        void pending?.then((value) => value?.done());
      }
    },
  };
}

/** The slot route of a page drag. */
export function pageSlotDrag(ctx: LoadContext, page: PageToImport): SlotDrag {
  return slotDragOf(ctx, () => pageSlotSource(page), { eager: true });
}
