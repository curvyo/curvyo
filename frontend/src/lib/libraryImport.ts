// One import from the library into the open document (`specs/0137` criteria 4 to 6): the wasm
// copies, the page shows what it answers. The record's time is the page's clock (core has none).
import type { WasmLibrarySource, WasmSession } from "@/wasm-bindings/curvyo_editor_wasm.js";

/** What an import answered. */
export interface ImportAnswer {
  page?: string | null;
  notice?: string | null;
  error?: string;
}

/** Where the copy comes from. */
export interface ImportFrom {
  documentName: string;
  pageName: string;
  pageId: string;
}

/** Imports the page `from.pageId` (with `whole`) or the picks `roots` (JSON ids) of `reader`,
 * dropped at the canvas point `drop` (CSS px) or centred on the visible canvas. */
export function importFromLibrary(
  session: WasmSession,
  reader: WasmLibrarySource,
  from: ImportFrom,
  whole: boolean,
  roots: string[],
  drop: { x: number; y: number } | null,
): ImportAnswer {
  const record = JSON.stringify({
    document: from.documentName,
    page: from.pageName,
    page_id: from.pageId,
    at: Date.now(),
  });
  const answer = session.import_from_library(
    reader,
    `[${roots.join(",")}]`,
    whole ? from.pageId : undefined,
    record,
    drop?.x ?? 0,
    drop?.y ?? 0,
    drop !== null,
  );
  try {
    return JSON.parse(answer) as ImportAnswer;
  } catch {
    return { error: "the import did not answer" };
  }
}
