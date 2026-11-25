// Loading from the library into the open document (`specs/0137`, `specs/0138`, `specs/0152`
// criterion 7): Import page, Load template and an icon import, the same paths for the Library
// tab's rows and the library browser's tiles. The wasm copies; the page shows what it answers.
import type { WasmSession } from "@/lib/editorSession";
import { importFromLibrary, loadTemplateInto } from "@/lib/libraryImport";
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
