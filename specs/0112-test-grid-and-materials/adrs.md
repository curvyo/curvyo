# ADRs for test grid and materials

Status: Proposed (architect, 2026-11-12). Builds second, after `0111` (needs its `op_options`, `interval_mm` and format 44). File shapes: [ADR 0057](../../docs/adr/0057-test-grid-object-with-derived-operations.md) (project file, format 46) and [ADR 0058](../../docs/adr/0058-material-records-shared-across-lasers.md) (material record format 2), both `needs-customer`, accepted with the story PR. Decisions marked **(Proposal)** are the architect's and vetoable at the demo.

- [ADR 0057](../../docs/adr/0057-test-grid-object-with-derived-operations.md): the grid is one tagged group with its recipe and test id in one atomic map, a Lasered register, derived ordinary operations with `op_grid_part`, regeneration in place, delete and off-page removal, copy with a new id, recreate from Materials, the Lasered test hook, migration of format-43 grids; replaces ADR 0054 §4 and §5.
- [ADR 0058](../../docs/adr/0058-material-records-shared-across-lasers.md): record format 2 (lasers with sets, tests with recipe, best cell, note, photo, state), retired ids file, photo files and the host boundary, format-1 read path, the per-laser link rule.
- [ADR 0056](../../docs/adr/0056-laser-operation-tools-in-the-project-file.md): `main.interval_mm` (now linked to the Engrave set), `op_options` (Crosshatch of an Engrave grid's cells), the kind-button rule that skips derived rows.
- [ADR 0054](../../docs/adr/0054-laser-settings-and-test-grids-in-the-project-file.md) §1 to §3: settings, link, overrides and refresh, unchanged; §2's machine tie reads the record's `lasers` entry (ADR 0058 §7).
- [ADR 0055](../../docs/adr/0055-material-records.md): location, envelope, order, undo journal (now with photo bytes), limits of the journal; record format amended by ADR 0058.
- [ADR 0046](../../docs/adr/0046-laser-operations-record.md) §3, §4: delete sweep and copy rule, extended by ADR 0057 §3.
- [ADR 0042](../../docs/adr/0042-page-kinds.md) §9: the `migrate_format` reserved commit reused for old grids.
- [ADR 0038](../../docs/adr/0038-machine-profile-records.md): reading rules and folder sync, applied to records, photos and the retired file.

## Feature-local decisions

- 2026-11-12: **Format.** Project `format_version` 46, material record format 2, `test-grid-choice.json` format 2, new `retired-test-ids.json` format 1. Numbers taken at merge.
- 2026-11-12: **Which hand edits of a derived row give a Lasered grid a new id**: settings, material, options and membership; not rename, Show, Output or order (what was burned does not change) **(Proposal)**.
- 2026-11-12: **Regeneration overwrites everything of a derived row** (name, Show, Output, settings, options, membership; Frage 204 A), keeps its node id and place; new rows go after the grid's last row **(Proposal)**.
- 2026-11-12: **The test record follows the grid** while not Lasered (each bar commit rewrites its recipe in the same step); a material pick moves it between materials without retiring the id **(Proposal)**.
- 2026-11-13: **Legacy conversion dropped** (Frage 214 B, nothing is deployed; `0112` criteria 29, 30): format 43/44 grids open as ordinary operations without a grid object; a format-1 material record is skipped as "older format"; ADR 0057 §7 and ADR 0058 §6 amended. (Replaces the 2026-11-12 migration entry.)
- 2026-11-12: **Lasered hook**: a Rust-only `#[doc(hidden)]` session method, not bound to wasm; Rust acceptance tests call it; no UI, flag or cargo feature.
- 2026-11-12: **Photos**: the frontend crops and encodes (JPEG 0.85, PNG kept), core checks signature and header size (no decoding), the host writes `materials/<uuid>.<test id>.jpg|png` (desktop) or IndexedDB `curvyo-material-photos` (browser). The camera stays in the browser; nothing leaves the machine.
- 2026-11-12, amended 2026-11-13: **Undo labels** (constants in document-core, names in `curvyo-ui-core/src/history_names.rs`) are the words of UX notes section 11: `add_test_grid` "Add test grid", `edit_test_grid` "Edit test grid" (bar commits), `use_material` "Use material" (a material pick on a grid, UX G8), `place_test_grid` "Place test grid", `judge_test` "Choose best cell", `edit_test_note` "Edit note", `set_test_photo` "Add photo", `crop_test_photo` "Crop photo", `remove_test_photo` "Remove photo", `delete_test` "Delete test", `add_material` "Add material", `add_material_set` "Add set", `delete_material_set` "Delete set", plus the material steps of ADR 0055 §9 and the test hook's `mark_test_grid_lasered` "Mark lasered" (ADR 0057 notes). Removed with the folded row: `rebuild_test_grid`, `mark_test_cell`, `unmark_test_cell`, `rename_test_grid`, `delete_test_grid_row`, `save_as_material`; old histories show the raw key.

- 2026-11-13: **Recipe `crosshatch`** is present in every stored recipe (`false` for Cut and Score); ADR 0057 §1 amended.
- 2026-11-13: **Lifetime joins** (delete sweep in document-core by the `test_grid` register; off-page removal and Duplicate's copy joined by the session after the command's step; a copy's test record at its next commit): ADR 0057 notes **(Proposal)**.
- 2026-11-13: **Random bytes for ids** come from the host (`feed_test_grid_random`, `crypto.getRandomValues`); the session's xorshift stream from the peer id is only a fallback for tests and a host that gave none, never a second source in the product.

## Crate placement (no new crate)

- `curvyo-laser-core` (as built, amended 2026-11-13): public modules `grids` (`test_id.rs`, `test_param.rs`, `test_recipe.rs` with its stored form `test_recipe_json.rs`, `test_recipe_layout.rs`, `test_grid_fit.rs`, `test_grid_ops.rs`, `test_grid_choice.rs`) and `library` (`material_record.rs`, `material_record_edits.rs`, `material_record_file.rs`, `material_test_file.rs`, `material_store.rs`, `material_retired.rs`, `material_photo.rs`); `operation_settings_rule.rs` (the per-laser tie). `test_grid_run.rs`, the format-1 store and the `grid_cell` numbers are gone.
- `curvyo-document-core`: `test_grid_codec.rs` and `test_grid_commands.rs` for the new registers, the derived-row sweep in the delete and off-page paths, the migration (called by the session, which supplies ids).
- `curvyo-editor-wasm`: `session/test_grid/` (tool, bar, regeneration, place), `session/materials/` (tab, tests, judge, note, photo bytes, journal).
- `curvyo-app`: `materials.rs` gains `read_material_photo`, `write_material_photo`, `remove_material_photo`, and the retired file, over `curvyo-storage-io` (`read_optional`, `write_atomic`, `remove_file`); `frontend/src/platform/host.ts` the browser side.

## Architecture review findings (2026-11-06) in this slice

- **Finding 1:** the Test grid tool state lives in `session/test_grid/` as one owned struct (like the Trace tool); the materials tab state in `session/materials/`; page activation calls one method of each.
- **Finding 2:** the tool bar, the Materials tab, the test detail and camera and crop get their own components and hooks (`useTestGridTool`, `useMaterials`, `PhotoCrop`); camera and crop code never enters `useEditorSession.ts`.
- **Finding 3:** `test_grid_view` (recipe, fields, ranges, fit cause, id, Lasered), `materials_view` (lasers, sets with source, tests with state, test detail with table, best cell, note, photo name) and the host write list (adds `write_photo` and `remove_photo` with a byte handle) are checked decoders (`testGridView.ts`, `materialsView.ts`). Fixtures in `curvyo-editor-wasm/tests/fixtures/test_grid_view/`, `materials_view/` and `material_writes/`: at least an Engrave grid, a Lasered grid, a grid without material, a material on two lasers, a typed set, a test with and without photo, a format-1 record, a damaged record. Rust pins the produced JSON; TypeScript decodes each and rejects a missing and a mistyped field.

## Goldens and budgets

- Goldens: ADR 0057 §8 (project), ADR 0058 §10 (records, retired ids, photos).
- Budgets confirmed (criterion 32, thread CPU time, `curvyo-editor-wasm/tests/cpu_time/`): placing a 12 x 12 grid (145 rows, label texts shaped once per distinct text) within 50 ms; a bar edit regenerating it within 30 ms; the Materials list with 200 materials within 50 ms. Added: loading 200 format-2 records within 200 ms (ADR 0055 budget); the Operations view of a page with a 12 x 12 grid (about 150 rows) within 16 ms; deleting a 12 x 12 grid with its rows within 30 ms.

## Spec changes for the product owner

- S1. Criterion 15: shape and format 46 as ADR 0057.
- S2. Criterion 22: the test is written into the material at the pick and follows the grid until Lasered; a pick of another material moves it.
- S3. Criterion 29: old grids migrate at open (not an undo step); their burned id appears at the first bar change; the old mark becomes the best cell when the grid's first test is made.
- S4. Criterion 27: the camera image and any file are encoded as JPEG unless the file is PNG; at most 1 MiB after encoding.
- S5. Criterion 11: the list of hand edits that count (feature-local decision above).

## Questions for the customer

None new. Both ADRs are `needs-customer` and are accepted with the story PR.
