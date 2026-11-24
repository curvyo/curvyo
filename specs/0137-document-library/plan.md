# Plan for "Document library: take a page, or just the elements of a page, from any of my documents as a plain copy"

One slice, built to ADR 0084 (decisions 1 to 9) and `ux-notes.md` sections 1 to 13. Frage 341
decided C (customer, 2026-11-24): the browser build has the Library tab too and lists the
documents stored in the browser; the desktop path is built first, the web storage side follows the
ADR 0084 amendment (task 20). The "Add source..." button and the Online section belong to `0138` and
are left out (no placeholder). `format_version` 58 (next free on `main` at the time of the build;
kept in `CURRENT_FORMAT_VERSION` and the fixtures only, so a rebase can renumber it). No new crate,
no new dependency.

## Affected crates/modules

- `curvyo-document-core`: `came_from.rs` (new: the record, its codec on node meta and page
  registers, read tolerant of damage), `library_summary.rs` (new: `library_summary(&Document)`, the
  `library.json` member and its reader), `fragment.rs` and `fragment_insert.rs` (new: extract from a
  forked, pruned read-only copy; insert with the shared remap of Duplicate), `objects.rs`
  (`copy_subtree` takes a source and a target tree, so Duplicate and Fragment share one copy and
  one relink list), `container.rs` (`library.json` written for a save only), `document.rs`
  (format 58).
- `curvyo-storage-io`: `container_thumbnail.rs` (`with_members`: `thumbnail.png` and
  `pages/<id>.png` joined in one pass), `library_sources.rs` (new: `library-folders.toml`, the
  recursive scan, the member read of one document, `library_read` under a source root).
- `curvyo-ui-core`: `library_index.rs` (new: records, change check, search, the tree rows),
  `library_index_text.rs` (new: row lines, errors, notices, Came from line words).
- `curvyo-app`: `library.rs` (new commands: sources, scan, page pictures, read, add and remove a
  folder, Saved version), `project.rs` (page pictures at save).
- `curvyo-editor-wasm`: `wasm_library.rs` (new: `WasmLibrarySource`, the import entries, Came
  from of the selection and of a page), page pictures at save, CPU-time test of the search.
- `frontend`: the Library tab, the page preview pane, drag and Import, the Came from line, Open
  source, the page pictures at save.

## Tasks

M1 document-core (tests first):
- [x] 1. `came_from` record: fields `document`, `page`, `page_id`, `item`, `at`; node meta key and
  page register, one atomic map; damaged or partial reads as absent and is kept; unknown keys kept
  (fulfils AC 6, 9)
- [x] 2. Carried by Duplicate and paste (meta copy), by Object to path and Stroke to path; a
  Boolean, Combine or Weld keeps it only when all sources carry an equal one (fulfils AC 9)
- [x] 3. Format 58, goldens `came_from_v58.curvyo` and `came_from_damaged_v58.curvyo`, an older
  build refuses by version, a file of 57 opens unchanged (fulfils AC 9)
- [x] 4. `library_summary` and the `library.json` member, written by `pack` for a save only (not a
  relay snapshot or a recovery copy), read with a 256 KiB cap, a newer `format` read as absent;
  container fixtures with and without the member (fulfils AC 2)
- [x] 5. `Fragment`: `extract_fragment(roots, page)` from a fork of the source with everything
  else deleted, so every reference into the rest follows its own removal rule (a linked clone
  becomes a path, ADR 0049); counts for the notice; assets travel by hash (fulfils AC 5, 8)
- [x] 6. `insert_fragment`: elements into the active layer of the active page, centred on a point,
  one step "Import elements"; refused on a sketch page; a second insert is a second copy; the
  shared copy and relink of Duplicate (fulfils AC 5, 6, 8)
- [x] 7. Page import: a new page after the last page (ADR 0051 layout), its registers, its members
  moved by the change of the frame origin, layers kept by name, one step "Import page", the page
  becomes active; an output page whose data cannot be copied comes in as a drawing page with the
  reason (fulfils AC 4, 6)

M2 storage-io:
- [x] 8. `with_members` writes `thumbnail.png` and up to 32 `pages/<id>.png` in one pass (fulfils
  AC 2)
- [x] 9. `library-folders.toml` (format 1, newer never written over, damaged replaced), add and
  remove an entry, nothing on disk deleted (fulfils AC 1)
- [x] 10. The recursive scan (hidden entries and folder links skipped, 10,000 per source) and the
  member read of one document (`library.json`, `pages/<id>.png`, state by manifest), opened
  read-only; source reasons `not_found`, `no_access`, `io`; `library_read` under the source root
  (fulfils AC 1, 2, 10, 12)

M3 ui-core:
- [x] 11. `library_index`: records with size and modification time, the change check, the
  `library-index.json` cache (format 1, rebuilt when damaged or newer), the native fallback for a
  file without `library.json` (fulfils AC 1, 2, 3)
- [x] 12. Search: lower-case substring over document and page names, the P13 tree (matching
  documents collapsed, documents with matching pages expanded with those pages only); CPU-time
  test, 1,000 documents within 100 ms (fulfils AC 3)
- [x] 13. Rows of the virtualised tree for 500 x 8 within 200 ms CPU time; texts of rows, errors,
  Saved version, notices and the Came from line; Open source lookup by page id, same name first
  (fulfils AC 7, 10, 11, 13)

M4 app and wasm:
- [x] 14. Host commands: sources and index, page pictures, `library_read`, add and remove a
  folder, Saved version for this window's path and dirty flag (fulfils AC 1, 2, 10, 11, 12)
- [x] 15. Page pictures at save: the frontend renders up to 32 pages at 224 x 160, the host joins
  them (fulfils AC 2)
- [x] 16. `WasmLibrarySource`: open bytes read-only, draw one page, hit and marquee, extract;
  the session's import entries with `at` from the caller (fulfils AC 4, 5, 6)

M5 frontend:
- [x] 17. Library tab after Fonts (desktop build first, the browser with task 20): header, search, the virtualised tree with
  sources, documents and page rows, Add folder... and the armed Remove (fulfils AC 1, 2, 3, 10,
  11, 13)
- [x] 18. Page preview pane: picking by click, Shift+click and marquee, Import page and Import
  elements, drag of a page row and of picks onto the canvas (fulfils AC 4, 5)
- [x] 19. Came from line in the Style tab, the Document tab and the Layers and Pages tooltips;
  Open source (fulfils AC 7)

M6 browser build (after the ADR 0084 amendment for Frage 341 C):
- [x] 20. The browser's own stored documents as a library source: index, page pictures, read,
  import through the same `ui-core` index and `WasmLibrarySource` (fulfils AC 1, 2, 3, 10, 13)

Left for a follow-up (`docs/technical-debt.md`, "Document library"): the browser's worker of
task 20.

## Validation

- Golden files for format 58 and the container members; the older-build refusal by version.
- Unit tests in each crate for every rule above; CPU-time tests (thread CPU) for search and rows.
- The full gate of `CLAUDE.md` §7 once before the report.
- Manual check in the desktop build: add a folder, browse, import a page and elements, Came from
  and Open source.
