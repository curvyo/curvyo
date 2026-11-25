// One import from the library into the open document (`specs/0137` criteria 4 to 6): the wasm
// copies, the page shows what it answers. The record's time is the page's clock (core has none).
import type { OnlineOrigin } from "@/lib/onlineApi";
import type { WasmLibrarySource, WasmSession } from "@/wasm-bindings/curvyo_editor_wasm.js";

/** What an import answered. */
export interface ImportAnswer {
  page?: string | null;
  notice?: string | null;
  error?: string;
}

/** Where the copy comes from. An online item adds its origin (`specs/0138`): the Came from keys,
 * the fonts left out (criterion 9) and, for an icon, the step's own name. */
export interface ImportFrom {
  documentName: string;
  pageName: string;
  pageId: string;
  online?: OnlineOrigin;
  withoutFonts?: boolean;
  step?: "icon";
}

function recordOf(from: ImportFrom): string {
  return JSON.stringify({
    document: from.documentName,
    page: from.pageName,
    page_id: from.pageId,
    at: Date.now(),
    online: from.online,
    without_fonts: from.withoutFonts ?? false,
    step: from.step,
  });
}

/** Loads every page of `reader` as new pages in one step "Load template" (`0138` criterion 3). */
export function loadTemplateInto(session: WasmSession, reader: WasmLibrarySource, from: ImportFrom): ImportAnswer {
  try {
    return JSON.parse(session.load_online_template(reader, recordOf(from))) as ImportAnswer;
  } catch {
    return { error: "the import did not answer" };
  }
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
  const record = recordOf(from);
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
