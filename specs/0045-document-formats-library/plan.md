# Plan for Document formats library

Built together with `0043-properties-tabs` as one PR on
`story/properties-tabs-and-formats`; the milestones are commits. Spec, UX notes
and ADR notes: `specification.md` and `adrs.md` in this folder (from the docs PR).
Decided defaults (customer, 2026-10-10): the maker's formats are stored per user in
`document-formats.toml` in the app data directory; A0 to A6 on, Slides off but
starred; same-size duplicates refused; import and export of a formats file.

## Affected crates/modules
- `curvyo-document-core`: `document_presets.rs` (schema 2), new `format_library.rs`
  (state and edits), `format_library_file.rs` (user file reader and writer),
  `format_library_import.rs` (import merge, export); fixtures in
  `tests/fixtures/formats/`. Workspace `Cargo.toml`: `toml` gains `display`.
- `curvyo-ui-core`: `document_presets_view.rs` (quick selection from favourites),
  new `format_list_view.rs` and `format_form.rs`.
- `curvyo-editor-wasm`: `Session.formats`, `session/formats.rs`, `wasm_formats.rs`.
- `curvyo-storage-io`: `read_optional`, `ensure_parent_dir`, `rename_replacing`.
- `curvyo-app`: `formats.rs` (commands that move text, native dialogs).
- `frontend`: `lib/formatsFile.ts`, `platform/host.ts` (browser fallback),
  `hooks/useFormats.ts`, `panel/FormatList.tsx`, `FormatForm.tsx`,
  `FormatsBrokenBlock.tsx`, `ui/switch.tsx`.

## Tasks
- [x] 1. Built-in schema 2 (`enabled`, `favourite`, unit `cm`), shipped file with Slides off (AC 1, 7, 32).
- [x] 2. User file reader and canonical writer, overlay rules, limits, one failing fixture per rule, round trip (AC 2 to 5, 33, 34).
- [x] 3. Library edits: add, edit, delete, group edit and delete, stars, Show switches, deterministic ids (AC 15, 16, 20 to 22, 24, 32).
- [x] 4. Import merge and export with golden fixtures (AC 26, 27).
- [x] 5. `ui-core` views: quick selection from favourites, subject over groups that are on, list view, form prefill and validation messages (AC 8 to 11, 13, 17, 18 to 21).
- [x] 6. Session glue and `wasm_formats.rs`; session tests including the project file knowing nothing of the library (AC 6, 10, 14 to 16, 20 to 22, 24, 28).
- [x] 7. `storage-io` and `curvyo-app` commands; browser fallback in `host.ts` (AC 29, 30, 31).
- [x] 8. Frontend: disclosure, inline list with roving focus, switches, stars, add and edit forms, delete confirm, group form, broken-file block, import and export (AC 12 to 25).
- [ ] 9. Browser check of the Document tab at 800 x 600 (needs the Browser pane).

## Validation
Golden fixtures for the user file (valid, round trip, export, import merge, one
failing file per rule); table tests in `ui-core`; session tests for the glue;
`storage-io` tests for the three file helpers. The frontend holds typed text only
and is checked in the browser.

## Changes to tests written for `0030`
Built-in schema 2 and Slides off by default change what the `0030` tests saw:
the fixtures and inline files say `format = 2`, `unit = "cm"` is valid now
(`6e_unit_pt`), `format = 1` is the refused value (`6b_format_one`), and tests
that press Slides switch the group on first.

## Part H: formats panel restyle (criteria 35 to 44, `story/formats-panel-restyle`)

Customer answers of 2026-11-10: FMT-OPENS-AS A, FMT-RENAME-APPLY A, FMT-LOWER-BAR A,
FMT-GROUP-ARM A, FMT-PAGES-UNIT (Unit row stays), FMT-FILE-VERSION default (`format` stays 1).
FMT-NOTICES decided otherwise than the default: no success notice at all (customer rule), so
"Added", "Saved", "Imported ..." and "Moved to ..." go; failures stay; the sr-only regions announce.

### Affected crates/modules
- `curvyo-document-core`: `format_edit.rs` (no same-size refusal, `FormatReason::SameSizeAs` gone),
  `format_library_file.rs` (user file may repeat sizes), `format_library_import.rs` (skip by group,
  name and size), `document_presets.rs` (`with_sides` gone; the built-in file keeps its rule).
  Fixtures `tests/fixtures/formats/`: `fail_same_size.toml` becomes `valid_same_size.toml`.
- `curvyo-ui-core`: `format_list_view.rs` (armed and rename names, no edit-group, prompt or
  orientation), `format_form.rs` (no orientation, no success notices).
- `curvyo-editor-wasm`: `session/formats.rs` (`rename_format`, `rename_format_group`, no notices,
  no orientation), `wasm_formats.rs`, `session/pages.rs` and `wasm_pages.rs` (`set_page_position`
  and the page `xText`/`yText` go: no caller left).
- `frontend`: `components/CornerControl.tsx` (shared by Machine block and Grid pane),
  `panel/FormatList.tsx`, `FormatForm.tsx`, `FormatsBrokenBlock.tsx`, `PagesBlock.tsx`,
  `hooks/useFormats.ts` (uses `useDeleteArm`).

### Tasks
- [x] 1. Core: duplicate sizes allowed in add, edit, user file and import; import skips by group and name; the user-file loader refuses a repeated name in a group; built-in file keeps the rule (AC 39). Rust tests first: `formats_of_one_size_are_allowed_and_survive_a_reload`, import golden files, tester tests changed by design.
- [x] 2. `ui-core` list view: armed names, rename names, no group edit, no orientation, no success notices (AC 35 to 37, 42, 43); only the first of two formats of one size is selected (AC 39).
- [x] 3. Session glue and wasm: `rename_format`, `rename_format_group`; new groups are portrait; success returns no notice (AC 42, 43).
- [x] 4. Pages block without X and Y; `set_page_position` removed (AC 44).
- [x] 5. Shared origin corner control for the Grid & Rulers pane (AC 41).
- [x] 6. Formats panel: symbol buttons in the "All formats" row, armed delete, no lower bar, no "Opens as", inline rename (AC 35 to 38, 40, 42, 43).
- [x] 7. Chromium check of the panel, the Origin picker and the Pages block (`acceptance_0045_formats_panel_browser.test.ts`).

### Validation
Rust tests first for the duplicate-size rule, import and rename; view tests in `ui-core`;
Chromium test for armed delete, symbol buttons, rename, Origin picker and the Pages block.
