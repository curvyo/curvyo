# ADRs for the document library

Architect, 2026-11-24. One new ADR; Fragen 268 B and C, 269 A and 270 A are fixed; Frage 341
(DOCLIB-WEB) answered 2026-11-24: C, the browser build has the tab and lists its own stored
documents (ADR 0084 decision 10). No new crate, no new dependency, no network.

- [ADR 0084](../../docs/adr/0084-document-library-index-read-only-sources-and-copy-import.md)
  (Proposed, `needs-customer`: document model and file format):
  - container members `library.json` and `pages/<page id>.png` written at save, with a native fallback for old files (criteria 1, 2, 13);
  - reading without opening, lock or write (2);
  - sources, `library-folders.toml` and the `library-index.json` cache (1, 3, 10);
  - Saved version for this window (11);
  - the read-only `WasmLibrarySource` (5);
  - `Fragment` extract and insert with one shared remap table (4, 5, 8);
  - the `came_from` record and Open source by page id (6, 7, 9);
  - format 58 (9);
  - the browser build (decision 10, Frage 341 C, answered 2026-11-24): one source per workspace
    held in IndexedDB `curvyo-workspaces`, the member read and picture join moved into
    `document-core` for both builds, the index text under key `l|index`, the reads in a worker,
    read-only transactions only (1, 2, 3, 10, 11, 12, 13).
- [ADR 0047](../../docs/adr/0047-document-fonts.md) §1, §3, §7: unchanged; fonts travel as assets
  in the fragment, under the same budget; a font that is not embedded falls back (criterion 8).
- [ADR 0037](../../docs/adr/0037-shared-workspace-list-rooms-and-keys.md): unchanged; the source
  key of a workspace is its `workspace.toml` id; a synced folder is read as files (criterion 12).
- [ADR 0042](../../docs/adr/0042-page-kinds.md), [ADR 0043](../../docs/adr/0043-sketch-pages-content-by-ownership.md),
  [ADR 0051](../../docs/adr/0051-page-auto-layout-and-ownership-by-centre.md): the existing
  membership read chooses what a page import takes; the page goes after the last page by 0051.
- [ADR 0015](../../docs/adr/0015-node-references-and-ancestor-resolved-state.md), 0022, 0027,
  0030, 0045, 0057, 0080: their references are remapped through the one table of ADR 0084
  decision 6; dangling ones follow each ADR's own rule for a removed target.
- [ADR 0033](../../docs/adr/0033-manufacturing-data-in-the-project-file.md) §7: format 58, the next
  free number at merge. Main writes 57 since `0145` slice 2.

## Notes for the PO and UX (ADR 0084 Consequences)

1. E8: page pictures are written at save, 224 x 160 px, at most 32 pages **(Proposal)**; a document
   saved before this slice lists its pages without pictures until it is saved again.
2. E9: Open source finds the source by the page id in the record, so a moved or renamed document
   is still found; with several copies the one with the same name wins.
3. E7: "Saved version" is certain only for the document open in this window.
4. Criterion 4: a placed page whose drawing page is not imported comes in as ordinary copies; a
   laser page whose manufacturing data this build cannot read comes in as a drawing page.
5. Criterion 9: SVG export writes nothing of the record **(Proposal)**.
6. A result of a Boolean, Combine or Weld keeps Came from only when all its sources had the same
   one **(Proposal)**.

7. Criterion 1 (browser): the browser keeps documents only in the workspaces it holds
   (`0096` criteria 35 and 38), so its tree groups them by workspace **(Proposal)**; the spec's
   "there are no workspaces" there should read "no recent list and no added folders".
8. Browser: documents stored before this slice, or written by sync without pictures, list their
   pages without pictures until they are saved there again.

## Note 2026-11-24: Frage 341 answered C

The customer answered Frage 341 (DOCLIB-WEB) with C on 2026-11-24, not the default A. ADR 0084
gains decision 10 and an option table; plan task 20 builds it. For the implementer: the member
read (`library.json`, `pages/` names and bytes, manifest format) and `with_pictures` move from
`storage-io` into `document-core` as pure functions over a zip archive, used with a `File` on the
desktop and with the entry bytes in the wasm; the browser's `save_project_bytes` joins the
pictures through the wasm before storing or downloading.

No question for the customer beyond accepting ADR 0084 (document model and format).

## Note 2026-11-24: architect review of the implementation

Built as ADR 0084 decides, with three differences, each reversible in one PR:

1. Decision 6, placements: a placed page inside an imported laser page whose drawing page is not
   imported is copied as a placement (it draws nothing in the target) instead of becoming ordinary
   copies. Listed in `docs/technical-debt.md` ("Document library"); criterion 4 names this case,
   so the PR says it is open.
2. Decision 1 and 10, the picture join: both builds join `pages/<id>.png` through the wasm
   (`curvyo_document_core::with_pictures`); the desktop host still joins `thumbnail.png` with
   `curvyo_storage_io::with_thumbnail`, so a desktop save rewrites the zip twice. Folding the
   thumbnail into the wasm join on the desktop removes `with_thumbnail` and its copies of
   `THUMBNAIL_MEMBER`, `MAX_THUMBNAIL_BYTES` and `read_member` in `storage-io`.
3. Decision 10, the worker: the browser's member reads run on the page's thread (debt entry).
