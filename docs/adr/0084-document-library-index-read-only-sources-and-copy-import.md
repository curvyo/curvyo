# ADR 0084: Document library: page summaries in the container, a per-user index, read-only sources and copy import with a Came from record

**Status:** Proposed 2026-11-24 by the architect, `needs-customer` (document model and file format: the `came_from` record, format 58; new container members `library.json` and `pages/`; two new per-user files). Spec: [`0137`](../../specs/0137-document-library/specification.md) criteria 1 to 13, Fragen 268 B and C, 269 A, 270 A; Frage 341 (DOCLIB-WEB) answered 2026-11-24: C, the browser build lists its own stored documents (decision 10). Builds on [ADR 0004](0004-persistence-and-cross-machine-sync.md) §1 (container), [ADR 0037](0037-shared-workspace-list-rooms-and-keys.md) (workspace on disk), [ADR 0047](0047-document-fonts.md) (fonts are assets), [ADR 0042](0042-page-kinds.md), [ADR 0043](0043-sketch-pages-content-by-ownership.md) and [ADR 0051](0051-page-auto-layout-and-ownership-by-centre.md) (page membership and layout). [ADR 0085](0085-online-template-sources-read-only.md) adds online sources on the same surface.

**In plain words (for the customer).** When you save, Curvyo now also writes a small list of the document's pages and a little picture of each page into the file. The Library reads only those, so browsing hundreds of documents never opens them, never marks anything changed and never touches the files. Curvyo keeps its own list of what it found in its app folder, not in your projects. Taking a page or some objects copies them into your open document, with their pictures and embedded fonts; afterwards nothing links them to the source. Each copy remembers "From <document> › <page>" and when; this note is saved in the file, so files with it need the new app version (format 58). Documents saved before this version show their pages without pictures until they are saved once more. In the browser, the Library lists the documents the browser itself keeps (its workspaces), by the same rules; it never reads files on your disk.

## Context

- A container has one `thumbnail.png` (the active page, `0095`); page names, sizes and kinds exist only inside `document.loro`. A document has no id; a page id is `<peer>-<counter>` (`pages.rs::new_page_id`), random per session and so practically unique across documents.
- Every copy today is inside one Loro document (`Document::duplicate_objects`, with clone, carrier and placement remapping). Nothing copies between two documents.
- Fonts and pictures are assets keyed by SHA-256 in the document's `AssetStore` (ADR 0017, 0047); derived image results too (ADR 0077).
- The host knows its own project path and dirty flag (`curvyo-app/src/state.rs`); it cannot know another process's.
- The browser build keeps documents in one place only: the workspaces it holds in IndexedDB `curvyo-workspaces` (`frontend/src/platform/browserWorkspaces.ts`, `0096` criteria 35 and 38), one key-value store whose entry `<id>|e|<path>` holds a document's whole container bytes and its `modified` time (seconds), answered through the same host command names as the desktop's folders. A file opened from disk is not stored, and a save outside a workspace downloads. Its save path joins no pictures today (`save_project_bytes` in `frontend/src/platform/host.ts`), and its `probe_workspace_document` answers "no thumbnail". IndexedDB has no partial read: a member of a stored container is read by loading the whole entry.

## Options

**Page list and page pictures without opening a document**

| Option | Why it won or lost |
|---|---|
| **A. Two container members written at save: `library.json` (page summaries) and `pages/<page id>.png` (chosen)** | Reading is the zip directory plus small members, as the `0095` probe already does; no Loro load for 500 documents; the pictures match what was saved. Cost: a save renders one picture per page; old files lack the members. |
| B. Load each document read-only in the background and render pictures on expand, cached in the index | No save cost, but the index build loads every Loro snapshot and the tab needs a second renderer and a picture cache with its own invalidation. |
| C. Read `document.json` | A full plain export, often megabytes; ADR 0004 §1 keeps it for debugging and recovery. |

**Copy between two documents**

| Option | Why it won or lost |
|---|---|
| **A. A `Fragment` value extracted from a read-only source document and inserted into the target with an id map (chosen)** | Every register travels unchanged, references are remapped by the same rules as Duplicate; one commit, one undo step; in memory only, so no clipboard format. |
| B. Through SVG export and import (`0024`) | Loses clones, sketches, generated objects, operations, page kinds. |
| C. Import the source's Loro updates | Brings the source's whole history and its ids into the target. |

**Finding the source again (criterion 7)**

| Option | Why it won or lost |
|---|---|
| **A. The source page id in the record, looked up in the index (chosen)** | Survives renames and moves inside and between sources; the file holds no file-system path. |
| B. The source path in the record | Breaks on a move (UX E9); puts the maker's folder names into a file that may be shared. |
| C. A new document id register | A second format change; Save as and workspace copies would share it anyway. |

**Where the index model lives**

| Option | Why it won or lost |
|---|---|
| **A. Records, text, change check and search in `curvyo-ui-core`; folders, scan and member reads in `curvyo-storage-io` (chosen)** | Follows `recent_workspaces.rs` and the `0095` probe; search is Rust, so the `cpu_time` budget test can measure it; no new crate. |
| B. Create `curvyo-library-core` (ADR 0011's slot) now | It would hold a list view model; ADR 0085 creates the crate for the remote catalogue protocols, which is that slot's text. |
| C. TypeScript in the frontend | The 100 ms budget is thread CPU time measured in Rust tests (`CLAUDE.md` §5). |

**The browser build's source (Frage 341 C, answered 2026-11-24)**

| Option | Why it won or lost |
|---|---|
| **A. The workspaces in `curvyo-workspaces`, read by wasm functions over the entry bytes, with the index text in the same store (chosen)** | That store is where the browser keeps documents; the change check uses its `modified` and byte length as the desktop uses size and modification time; the member rules run in Rust once for both builds; no new database, crate or dependency. Cost: an entry is loaded whole to read its members. |
| B. Write the page summaries and pictures into the stored entry beside the bytes at save | Saves the whole-entry load, but a second copy of the members outside the container, written by every path that writes an entry (save, sync, restore). |
| C. No index in the browser, read every document each time the tab is shown | Loads every stored container on each visit. |

**Format of the record**

| Option | Why it won or lost |
|---|---|
| **A. Format 58 (chosen)** | An older build would drop the record silently on Object to path and never show it; no users yet, so a refusal by older builds costs nothing. |
| B. No bump, as the additive keys of `0041` | The note would vanish in an older build without a word. |

## Decision

1. **Container members** (no `format_version` change, like `thumbnail.png`; a build that does not know them never reads them):
   - `library.json`: `{"format": 1, "pages": [{"id", "name", "kind", "width_mm", "height_mm"}]}` in page order; `kind` is `drawing` or `sketch`, or for an output page its process (`laser` today), the word the page chip of `0098` shows. Written by a pure `curvyo-document-core::library_summary(&Document)` inside `pack` for a save (not for a relay snapshot or a recovery copy). Read up to 256 KiB; a newer `format` is read as absent.
   - `pages/<page id>.png`: 224 x 160 px (the 56 x 40 row picture at 4x, the `0095` rule), rendered by the frontend at save beside `thumbnail.png` and joined by the host (`with_thumbnail` generalised to several members). At most the first 32 pages **(Proposal)**, each at most `MAX_THUMBNAIL_BYTES`; a page that fails to render has no member.
   - **Fallback:** for a container without `library.json` the indexing thread unpacks it natively (`unpack` plus `library_summary`, never written back). Its page rows have no picture until the next save **(Proposal)**.
2. **Reading never opens.** The host opens files read-only, reads only the zip directory and named members (the `probe_document` pattern), and writes nothing next to them: no lock, no sidecar, no thumbnail write-back. The modification time is untouched (the access time is the operating system's mount option). No `WasmSession`, recovery replica, undo stack or room is created for a source.
3. **Sources and index** (`curvyo-storage-io::library_sources`, `curvyo-ui-core::library_index`):
   - Order: the open workspace, then the other workspaces of `recent-workspaces.toml`, then the added folders of `library-folders.toml` (app data folder, `format = 1`, the `recent-workspaces.toml` rules: a newer file is never written over, a damaged one is replaced). Remove deletes only the entry.
   - A folder is read recursively for `.curvyo` files, skipping hidden entries and symbolic links to folders, at most 10,000 documents per source **(Proposal)**.
   - `library-index.json` in the app data folder, `format = 1`, a cache: per document the source key (workspace id from `workspace.toml`, or the added folder's path), the path inside it, size, modification time, state (`readable`, `not_curvyo`, `damaged`, `newer` by the manifest's `format_version`), name and the page summaries. A damaged or newer index is rebuilt and overwritten (it holds no work). A document is re-read when its size or modification time differs. The scan runs when the tab is shown and on window focus while it is shown; no watcher, no polling.
   - A source that cannot be read carries its reason (`not_found`, `no_access`, `io`), the others go on (criterion 10).
   - Search: Unicode lower-case substring over document and page names; a `cpu_time` test holds 1,000 documents at most 100 ms (criterion 3). The rows for the virtualised tree come from `library_index` (criterion 13).
4. **Saved version** (criterion 11): the library always reads the file on disk. The label is shown when the document's path is this window's project path and the host's dirty flag is set. Other windows or processes: no label; their memory is never read (UX E7).
5. **A read-only source for preview and import.** The host command `library_read(source key, path)` resolves the path under the source root (the workspace `resolve` rule, nothing outside) and returns the bytes. `WasmLibrarySource::open(bytes)` unpacks them (newer or damaged files are refused as `0001` criterion 7) into a `Document` that is never packed, never committed into and has no undo, sync or recovery. It draws one page into the preview canvas through the session's render path and answers hits and marquees through `ui-core`'s Select hit functions. At most one is open; closing the pane drops it **(Proposal)**.
6. **Fragment** (`curvyo-document-core::fragment`):
   - `Document::extract_fragment(roots, page)` returns an owned in-memory value: each node's registers as deep copies (text containers with their marks, ADR 0021), child order, the page registers for a page import, and the bytes of every referenced asset (pictures, fonts, derived results). It is never written to disk.
   - `Document::insert_fragment(&fragment, target, came_from)` writes everything in one commit, labelled "Import page" or "Import elements", and returns an `ImportReport` (counts for the notice).
   - **One remap table** shared with `duplicate_objects` (one list, not two), covering `clone_link` (ADR 0015), `text_carrier` (0022), dimension slots (0027), `tiling_sources` (0030), `placement_link` (0045), sketch references (0026), test grid parts (0057) and generated objects (0080). A reference whose target is not in the fragment: a linked clone becomes an ordinary path (the `0049` orphan rule), a text on a carrier keeps its outline copy (0022), a dimension point becomes free (0027), and a placement of a page that is not imported becomes ordinary copies of its members at its matrix **(Proposal)**.
   - **Page import:** the members come from `ui-core`'s kind-aware membership read (a drawing page by centre, an output page's manufacturing with its operations, fixtures, placements and own items, a sketch page's sketch). The page gets a new id and its registers (name, kind, size, grid, the profile copy `mfg_profile` of ADR 0039). It goes after the last page by ADR 0051's layout, and its objects move by the change of the frame origin. If the manufacturing data cannot be read in this build (unknown process, damaged), the page imports as a drawing page and the notice names the reason (criterion 4). Material links and fixture-set ids are kept (the numbers are stored beside them, ADR 0054). Material records and document formats are not copied.
   - **Element import** goes into the active layer of the active page, centred on the drop point or on the visible canvas; a sketch page refuses it (`0099`).
   - **Assets** are inserted by hash under the budget of ADR 0047 §3. An import over the budget is refused whole and nothing changes. Embedded fonts come along; a font that is not embedded keeps its `FontRef` and falls back by `0107`.
7. **Came from record.**
   - **Where:** node meta key `came_from` and page register `page.<id>.came_from`, one atomic map each (one last-writer-wins value). Fields: `document` (file name without extension), `page` (page name), `page_id` (the source page's id), `item` (the root's own name, optional) and `at` (integer, Unix milliseconds UTC, passed in by the caller since core has no clock).
   - **Written on** the imported page and on every fragment root, never on their descendants. A second import makes a second copy (criterion 6).
   - **Reading:** a wrong type or a missing required field reads as absent for display, the value is kept untouched, and the file is never `Damaged`, since the record never changes drawing or output. Unknown keys are kept (ADR 0085 adds some).
   - **Carried** by Duplicate, copy and paste, and by conversions with one source (Object to path, Stroke to path). A result of several sources (Boolean, Combine, Weld) keeps it only when all sources carry an equal record **(Proposal)**. A group made of imports carries none.
   - **SVG export:** nothing **(Proposal)**; a `data-` attribute is a one-line change if wanted.
   - **Open source:** the index lists the documents that contain `page_id`. The first with the same `document` name wins, else the first in source order; it opens normally (the `0051` unsaved-changes check) and that page becomes active. If none is found, the button is absent.
8. **Format 58**: the next free number at merge (main writes 57 since `0145` slice 2). An older build refuses the file. Goldens: `came_from_v58.curvyo` (an imported page and an imported group) and `came_from_damaged_v58.curvyo` (a record of the wrong type opens and shows no line). There is also a container fixture with and one without `library.json`.
9. **Crates and network.** No new crate, no new dependency, no network in either build (criterion 12). The work lands in `document-core` (`fragment`, `came_from`, `library_summary`, the member read and the picture join of decision 10), `ui-core` (`library_index`, membership choice, notice texts), `storage-io` (`library_sources`, folders and scan), `curvyo-app` (commands, path and dirty), `editor-wasm` (`WasmLibrarySource`, the session's import entry, the browser's member read and picture join) and the frontend (tab, preview, page pictures at save, the browser source).
10. **The browser build** (Frage 341 C, answered 2026-11-24; criteria 1, 2, 3, 10, 11, 12, 13).
    - **Sources:** each workspace held in IndexedDB `curvyo-workspaces` is one source, the open one first, then the others in the order of the browser's workspace list, each as the workspace tree of `0095` **(Proposal)**. The source key is the workspace id, as on the desktop, so Open source by page id (decision 7) works across both builds. There is no recent list, no added folder and no "Add folder"; a file on the maker's disk is never read. A browser with no workspace shows the empty state.
    - **One reader for both builds.** The member read (manifest `format_version`, `library.json` within 256 KiB, the names and bytes of `pages/<id>.png` within `MAX_THUMBNAIL_BYTES`) and the picture join (`with_pictures`, today in `storage-io::container_thumbnail`) move into `curvyo-document-core` as pure functions over a zip archive. `storage-io` calls them with a `File`, the wasm with the entry bytes (`Cursor<&[u8]>`). `storage-io` keeps folders, the recursive scan, the read-only open and `library_read`.
    - **Page pictures at save:** the browser's save joins `thumbnail.png` and `pages/<id>.png` through the wasm before it stores or downloads the bytes, so its container equals a desktop save. The pictures are rendered by the same frontend code as on the desktop.
    - **Index:** the same `ui-core::library_index` records and `library-index.json` text (`format = 1`), kept under the key `l|index` of the `files` store of `curvyo-workspaces` **(Proposal)**. That is the only write the library makes in the browser, and it is a cache: damaged or newer is rebuilt. The change check per document is the entry's byte length and `modified`. A changed entry is loaded whole once, and its members are read by the wasm. The fallback for a container without `library.json` (`unpack` plus `library_summary`, never written back) and the member reads run in a Web Worker with the wasm module, like the recovery worker of ADR 0079, so the page stays responsive **(Proposal)**. The scan runs when the tab is shown and on window focus, as on the desktop.
    - **Pictures on expand:** the entry is loaded and all its `pages/` members are read in one pass. The pictures are held as object URLs while the row is expanded and are released when it collapses. There is no picture cache.
    - **Read-only rules (decision 2) in the browser:** the library opens only read-only IndexedDB transactions on `<id>|e|`, `<id>|m|`, `<id>|d|` and `w|` keys and never writes them. An entry's `modified` is untouched. It takes no Web Lock, creates no recovery copy, joins no room and opens no `WasmSession`. Preview and import use `readDocument` and `WasmLibrarySource::open(bytes)` as in decision 5.
    - **Saved version (decision 4):** the label is shown when the document is this tab's open workspace document and the page's dirty flag is set. Another tab of the same browser counts as another window: no label.
    - **Fragment, Came from, search and format:** decisions 6 to 8 unchanged. The `cpu_time` budget test covers both builds because search is the same Rust.

## Consequences

- A save renders up to 32 extra small pictures. The plan measures thread CPU for 8 pages. If it is too slow, rendering only changed pages is a debt entry, not a format change.
- Source document names, page names and import times travel inside the document, also when its workspace is shared or synced; participants can read them in the Came from line.
- Two copy paths exist (Duplicate inside a document, Fragment between documents) but share one remap table. Folding Duplicate into Fragment is a later refactor.
- Old files list their pages without pictures until saved again. In the browser this also holds for documents saved there before this slice and for those written by sync without pictures.
- In the browser, the first scan after many changes loads each changed container whole, one at a time, in the worker; a 500-document store costs its total size once, then only what changed.
- Browser documents are in workspaces, so the browser's tree groups them by workspace. A flat list would change only the row building in `ui-core`, not the storage or the index.
- **For the UX notes:** page pictures are stored at 224 x 160; the preview pane draws the real page from the read-only source; Saved version is certain only for this window or tab (E7); E4, E5, E6, E8, E9 and E10 are covered by decisions 1, 6 and 7; in the browser the tab has no "Add folder" and no recent list (decision 10).
