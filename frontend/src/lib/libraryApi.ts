// The document library's data (`specs/0137-document-library`, ADR 0084): on the desktop the host's
// commands over the sources on disk, in the browser the same index (`WasmLibraryIndex`, the Rust
// of `curvyo-ui-core`) over the workspaces the browser stores (decision 10). Both answer the same
// rows; nothing here formats a word.
import { WasmLibraryIndex } from "@/wasm-bindings/curvyo_editor_wasm.js";
import { library_page_picture } from "@/wasm-bindings/curvyo_editor_wasm.js";

import { answerBytes } from "@/lib/binaryArgs";
import {
  libraryEntries,
  libraryEntryBytes,
  libraryIndexText,
  librarySources,
  openLibraryPlace,
  writeLibraryIndexText,
} from "@/platform/browserWorkspaces";
import { invoke, isTauri } from "@/platform/host";

/** One icon cell of an Iconify row (`curvyo_ui_core::library_online::IconCell`). */
export interface IconCell {
  prefix: string;
  name: string;
  label: string;
  tooltip: string;
}

/** What an online row adds (`curvyo_ui_core::library_online::OnlineExtra`); every word is Rust's. */
export interface OnlineExtra {
  glyph: string;
  host: string;
  state: string;
  refresh: boolean;
  /** The Lock mark and the token action (`specs/0156` criteria 4, 5, 13). */
  token: { private: boolean; action: string; attention: boolean };
  status: string;
  author: string;
  licence: string;
  licence_url: string;
  size: string;
  font_note: string;
  thumbnail: boolean;
  loadable: boolean;
  cells: IconCell[];
  samples: IconCell[];
}

/** One row of the tree, as `curvyo_ui_core::library_tree::LibraryRow` serialises it. */
export interface LibraryRow {
  key: string;
  kind: "heading" | "source" | "folder" | "document" | "page" | "ready" | "template" | "set" | "icons" | "status";
  online?: OnlineExtra;
  level: number;
  name: string;
  line: string;
  expandable: boolean;
  expanded: boolean;
  unreadable: boolean;
  source: string;
  path: string[];
  page_id?: string;
  chip?: string;
  picture: boolean;
  open: boolean;
  removable: boolean;
  matched?: [number, number];
  /** The accessible name, worded in Rust. */
  label: string;
}

/** A Came from record as the document stores it. */
export interface CameFromRecord {
  document: string;
  page: string;
  page_id: string;
  item?: string;
  at: number;
}

/** What Add folder... did. */
export type AddFolderAnswer =
  | { kind: "added"; key: string }
  | { kind: "cancelled" }
  | { kind: "refused"; text: string };

/** Where Open source opens a record. */
export interface SourcePlace {
  source: string;
  path: string[];
  in_workspace: boolean;
  file: string;
}

let browserIndex: WasmLibraryIndex | null = null;

async function browserScan(): Promise<number> {
  browserIndex ??= new WasmLibraryIndex(await libraryIndexText());
  const index = browserIndex;
  const sources = await librarySources();
  index.set_sources(JSON.stringify(sources));
  for (const source of sources) {
    const entries = await libraryEntries(source.key);
    for (const entry of entries) {
      const path = JSON.stringify(entry.path);
      if (index.needs_read(source.key, path, entry.size, entry.modified)) {
        index.put(source.key, path, entry.modified, await libraryEntryBytes(source.key, entry.path));
      }
    }
    index.keep_present(source.key, JSON.stringify(entries.map((e) => e.path)));
  }
  await writeLibraryIndexText(index.to_text());
  return index.count();
}

/** Reads every source again where it changed; the number of documents. */
export function scanLibrary(): Promise<number> {
  return isTauri ? invoke<number>("library_scan") : browserScan();
}

/** The rows for `query` and the expanded row keys. `dirty` is this window's unsaved state. */
export async function libraryRows(query: string, expanded: readonly string[], dirty: boolean): Promise<LibraryRow[]> {
  const now = Date.now();
  const offset = -new Date().getTimezoneOffset();
  if (isTauri) {
    return invoke<LibraryRow[]>("library_rows", {
      args: { query, expanded, now, offset_minutes: offset },
    });
  }
  if (!browserIndex) {
    return [];
  }
  const place = dirty ? openLibraryPlace() : null;
  // The Online group is the desktop build's (ADR 0085 decision 2, `0138` UX notes 2): the browser's
  // tab has no Online group, so its empty state stays "No documents in this browser.".
  return JSON.parse(
    browserIndex.rows(query, JSON.stringify(expanded), now, offset, place ? JSON.stringify(place) : ""),
  ) as LibraryRow[];
}

/** The library browser's list as JSON and its revision (`specs/0152`, ADR 0086 decision 5): the
 * host answers null while it is the list of `revision`; the browser build has its own documents. */
export async function browserItems(revision: string): Promise<{ revision: string; json: string } | null> {
  if (isTauri) {
    return invoke<{ revision: string; json: string } | null>("library_browser_items", { revision });
  }
  if (!browserIndex) {
    await browserScan();
  }
  return browserIndex ? { revision: "", json: browserIndex.browse_items_json() } : null;
}

/** The PNG of a page picture, empty when it has none. */
export async function pagePicture(source: string, path: string[], page: string): Promise<Uint8Array> {
  if (isTauri) {
    return answerBytes(await invoke<ArrayBuffer>("library_page_picture", { source, path, page }));
  }
  return library_page_picture(await libraryEntryBytes(source, path), page);
}

/** The bytes of a library document for the preview and the import, read-only. */
export async function readLibraryDocument(source: string, path: string[]): Promise<Uint8Array> {
  if (isTauri) {
    return answerBytes(await invoke<ArrayBuffer>("library_read_document", { source, path }));
  }
  return libraryEntryBytes(source, path);
}

/** Add folder... (desktop only). */
export function addLibraryFolder(): Promise<AddFolderAnswer> {
  return invoke<AddFolderAnswer>("library_add_folder");
}

/** Removes an added folder from the list; nothing on disk changes. */
export function removeLibraryFolder(key: string): Promise<void> {
  return invoke<void>("library_remove_folder", { key });
}

/** Where Open source opens `record`, or null when the source is gone. */
export async function findLibrarySource(record: CameFromRecord): Promise<SourcePlace | null> {
  if (isTauri) {
    return invoke<SourcePlace | null>("library_find_source", { record });
  }
  if (!browserIndex) {
    await browserScan();
  }
  const found = browserIndex?.find_source(JSON.stringify(record)) ?? "";
  if (!found) {
    return null;
  }
  const place = JSON.parse(found) as { source: string; path: string[] };
  const open = openLibraryPlace();
  return { ...place, in_workspace: open?.[0] === place.source, file: place.path.join("/") };
}

/** Open source for a desktop file outside the open workspace: the host hands it over like a file
 * the system opens, so the unsaved-changes check runs first. */
export function openLibraryPath(file: string): Promise<void> {
  return invoke<void>("library_open_path", { file });
}
