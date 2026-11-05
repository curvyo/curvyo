# Plan for page kinds

Implementer, 2026-11-05. Branch `story/0098-page-kinds`. ADR 0042 (Proposed), `adrs.md`.

## Affected crates/modules

- **`curvyo-document-core`**: `page_kind.rs` (new: `PageKind`, the kind read of ADR 0042 §2),
  `pages.rs` (`Page.kind`, every page listed, `add_page(kind)`, size refusal for output and
  unknown pages, delete of an output page with its manufacturing), `manufacturing.rs`
  (`add_output_page`, corner-anchored `set_manufacturing_profile`, `delete_placement`; removed
  `add_manufacturing`, `delete_manufacturing`, `set_work_area`), `sketch_page.rs` (only sketch
  pages own a sketch), `page_kind_migration.rs` (new: the format-36 open migration),
  `history` (reserved label `migrate_format`), `document.rs` (`format_version` 37), goldens.
- **`curvyo-ui-core`**: tab list per workbench (Sketches and Parameters only on sketch pages).
- **`curvyo-editor-wasm`**: workbench derived from the active page (`set_workbench` removed),
  page kind list, `pages_json` with kind and chip, status text, output pages drawn in the plane
  (page, fixtures, placements, origin marker), placement select and delete, Ctrl+Page Up/Down,
  removed manufacturing add/delete/work-area exports, profile work-area edit; CPU-time test.
- **`frontend`**: Add page group with the Kind switch and machine list, kind chip (page row,
  panel header, canvas label, status bar), switcher removed, laser page Document tab (Machine,
  Fixtures, notice when off), Ruler sub-row in the profile list, registry keyed by kind.

## Tasks

- [x] 1. Plan (this file).
- [x] 2. Core: `PageKind` and its read, `Page.kind`, output pages in `pages()`; tests for the
  read rules incl. damaged and unknown (AC 5, 10, 18 c, 20).
- [x] 3. Core: `add_page(kind)` with size of the last page of the kind, names "Page N",
  "Sketch N"; `add_output_page`; `delete_page` of an output page with its manufacturing; size
  refusal; removal of add/delete manufacturing and `set_work_area` (AC 2, 3, 4, 5, 16, 18 d).
- [x] 4. Core: corner-anchored profile resize, placements compensated (AC 14, 15).
- [x] 5. Core: `create_page_sketch` only on sketch pages (AC 3, 19 b).
- [x] 6. Core: `migrate_format` open migration from 36, format 37, the four goldens, old
  goldens open through it (AC 19, 20).
- [x] 7. Wasm: derived workbench, tabs follow the page, last tab per kind, page kind list,
  Ctrl+Page keys, status and pages JSON with kind; sketch tests switch by page (AC 7, 8, 9, 18).
- [x] 8. Wasm: output pages drawn in the plane for every workbench, vector tools on laser pages,
  placement select/delete, laser page Document view, profile work-area edit (AC 10 to 15, 17).
- [x] 9. Wasm: CPU-time test for page activation (AC 21).
- [x] 10. Frontend: Add page group, kind chips, switcher removed, laser Document tab, Ruler
  sub-row, registry keyed by kind, tests (AC 1, 6, 7, 13, 15, 18 a, b).
- [x] 11. Earlier tests updated per the spec's "Changes to accepted specs" table; full gate;
  browser check (AC 22: no network, no I/O added).

## Notes for the lead

- The first page of a new document and SVG import pages get no `kind` register (absent reads as
  Drawing): writing it would shift every node id the earlier goldens expect.
- Criterion 17: laser pages are pages of the SVG export with their own items; a placed page is
  not drawn into the export yet (`0100`).
- Session tests of earlier specs reach the Sketch workbench by a sketch page laid over the
  drawing page (`src/session/test_pages.rs`, `tests/page_kind_support/`).

## Validation

Golden files for format 37 and the 36 migration, the CPU-time test under
`curvyo-editor-wasm/tests/cpu_time/`, node tests for the kind registry, and an
end-to-end browser run (screenshots in the session scratchpad).
