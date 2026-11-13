# Plan for Test grid and materials (0112), Parts A to H

Branch `story/test-grid-and-materials`, on `main` after `0111` (PR #350, format 44, profile record
format 4). Built on `specification.md` (criteria 1 to 33), `adrs.md`, ADR 0057 (project file,
format 46), ADR 0058 (material record format 2, retired ids, photos) and `ux-notes.md` (G1 to G16,
M1 to M11). With this plan the story is Ready. One PR for the slice (the customer can try it only
after Part E); Parts A to G are commits on this branch, each ending with the gate of its crates;
Part H runs the full gate of `CLAUDE.md` §7.

**Frage 214 (LEGACY-FILES), answered B (2026-11-13, nothing is deployed): Part F is dropped.** A
format-1 record is skipped like a newer one, with the notice "Material <name> has an older format."
(Proposal), and a format 43 to 45 grid opens as a plain group whose `op_grid` row is an inert operation
kept and written back (ADR 0057 §6). The four format-43 grid goldens are "opens, runs nothing, saves
back" pins. ADR 0057 §7 and ADR 0058 §6 carry one line each.

Where the documents differ, this plan follows (for the product owner and architect to align before
the PR, all **(Proposal)**):
- Undo labels: the user-visible words of UX section 11 over adrs.md: keys `judge_test` "Choose best
  cell", `set_test_photo` "Add photo", new `crop_test_photo` "Crop photo", `remove_test_photo`,
  `edit_test_note` "Edit note", `delete_test`, `add_material_set` "Add set", new
  `delete_material_set` "Delete set", new `add_material` "Add material"; a material pick on a grid
  is `use_material` "Use material" (UX G8), not "Edit test grid". `add_test_grid`, `edit_test_grid`,
  `place_test_grid` as adrs.md.
- `rebuild_test_grid`, `mark_test_cell`, `unmark_test_cell`, `rename_test_grid`,
  `delete_test_grid_row`, `save_as_material` leave `history_names.rs`; old files whose history holds
  them show the raw key as every retired label does (check `history_rows` fallback in A0).

## Affected crates/modules

- `curvyo-laser-core` (pure, wasm32): `test_id.rs` (new), `test_grid.rs` (recipe v2, one serde type
  for the grid and the test record), `test_grid_layout.rs` (gap 2 mm constant, label texts with the
  id), `test_grid_ops.rs` (new: the derived operations a recipe resolves into), `material.rs` (lasers,
  sets with interval and source, tests, best cell, note, photo kind, judging), `material_edits.rs`,
  `material_file.rs` (format 2), `material_photo.rs` (new), `material_retired.rs` (new),
  `material_preferences.rs` (`test-grid-choice.json` format 2), `materials.rs` (store: taken ids),
  `operation_settings_rule.rs` (per-laser tie, ADR 0058 §7), `operation_naming.rs` (skip derived
  rows). Gone: `test_grid_run.rs`, the `grid_cell` numbers. Goldens `tests/fixtures/materials/`.
- `curvyo-document-core`: `test_grid_codec.rs` (recipe map, `test_grid_lasered`, `grid_cell`
  `{column,row}`, `grid_label` values, `op_grid_part`), `test_grid_commands.rs` (+ `_tests.rs`;
  place, regenerate in place, sweep on delete and off-page, copy with new id, recreate, new id on a
  Lasered edit), `operation_codec.rs` (`op_grid_part` read; `op_grid` reading leaves the run path),
  `document.rs` (`CURRENT_FORMAT_VERSION` 46). Goldens `tests/fixtures/`.
- `curvyo-ui-core`: `history_names.rs`, `step_scope.rs` (labels above).
- `curvyo-render-core`: `worked_chip.rs` goes (the Worked chip of `0028`).
- `curvyo-editor-wasm`: `session/test_grid/` (one owned struct `TestGridTool` like the Trace tool:
  tool state, bar draft, place, bar commits, regeneration, place from Materials, the Lasered hook),
  `session/materials/` (tab state: unfolded material, open test, laser folding; lasers and sets,
  tests, judge, note, photo bytes, retired ids, journal with photo bytes), `session/operations/`
  (`settings.rs`, `options.rs`, `assign.rs`, `kind_press.rs`, `settings_view.rs`, `grid_look.rs`),
  `session/select.rs` and `clones.rs` (grid selection, copy), `wasm_test_grid.rs`,
  `wasm_materials.rs`. Fixtures `tests/fixtures/test_grid_view/`, `materials_view/`,
  `material_writes/`; budget `tests/test_grid_materials_budget.rs` on `tests/cpu_time/`.
- `curvyo-app`: `materials.rs` (`read_material_photo`, `write_material_photo`,
  `remove_material_photo`, `read_retired_test_ids`, `write_retired_test_ids` over
  `curvyo-storage-io` `read_optional`, `write_atomic`, `remove_file`), `main.rs` registration.
- Frontend: `platform/host.ts` (IndexedDB `curvyo-material-photos`, `curvyo.retired-test-ids`),
  `lib/testGridView.ts`, `lib/materialsView.ts`, `lib/materialWrites.ts`, new `lib/photoSize.ts`,
  `hooks/useTestGridTool.ts` (replaces `useTestGrid.ts`), `hooks/useMaterials.ts`, new
  `hooks/useCamera.ts`; `ToolRail.tsx`, `railCard.ts`; new `components/TestGridBar.tsx`,
  `components/TestGridMaterialMenu.tsx`; `components/SelectToolbar.tsx` (grid selection);
  `panel/PanelStrip.tsx` (`materials` tab), new `panel/MaterialsTab.tsx` (from
  `MaterialsSection.tsx`), `MaterialRow.tsx`, new `panel/MaterialBody.tsx`, `panel/LaserSets.tsx`,
  `panel/TestRow.tsx`, `panel/TestDetail.tsx`, `panel/TestPhoto.tsx`, `panel/PhotoCrop.tsx`;
  `TestGridResult.tsx` moves into the detail; `OperationRow.tsx`/`OperationBody.tsx` (source line);
  `OperationsSection.tsx` (removals). Gone: `TestGridBlock.tsx`, `SaveAsMaterialForm.tsx`,
  `lib/testGridFields.ts` (folded into the bar), `lib/saveDraft.ts` grid parts.

## Part A: laser-core rules, record format 2, test ids, retired ids, photos (Opus, medium)

Tests first in every task. Each module's doc comment one sentence without "and".

- [x] A0. Removals that make room: `test_grid_run.rs` and its use in `tests/settings_goldens.rs`;
  `grid_cell` numbers in `test_grid.rs`; `render-core/worked_chip.rs`; the six retired labels of
  `history_names.rs` (history rows of old files keep reading). Gate stays green (AC 12, 15).
- [x] A1. `test_id.rs` ("makes and checks 5-character test ids"): `TestId` newtype (5 of `0-9A-Z`,
  parse refuses lowercase, length, other chars); `TestId::make(random: &[u8], taken: &impl
  Fn(&TestId) -> bool)` rejection-samples bytes into 36 symbols (a byte below 252 maps `% 36`), draws
  again while taken, `None` when the bytes run out (the session passes 64 bytes and retries with
  fresh ones). Tests: uniformity over all bytes 0..252, refusal of a taken id, exhaustion (AC 9).
- [x] A2. `test_grid.rs` recipe v2: `Recipe { id, kind, columns, rows, fixed, cell_mm, crosshatch,
  material }`, `Axis { param, min, max, count }`, `Param` (`power`, `speed`, `passes`, `interval`
  Engrave only), `Fixed` (each off-axis parameter once). One serde type with a `TestRecipe` view
  without `id` and `material` (ADR 0058 §2). Rules: the two axes differ; `set_axis_param` swaps when
  the other axis holds it (G6) and moves the freed parameter into `fixed` with its default; ranges
  (count 2 to 12, cell 3 to 50, power 1 to 100 at 0.1, speed 1 to max feed whole, passes 1 to 20,
  interval 0.01 to 2.00); Min above Max valid; validation texts of UX section 10 (at most 70
  characters); defaults of criterion 6 and G7 from the machine and an optional material set;
  `cell_values(column, row)` rounded to 0.1 % and whole mm/min (AC 3 to 6, 24).
- [x] A3. `test_grid_layout.rs`: gap 2 mm and text height 3 mm as constants; label items `id`,
  `column_title`, `row_title`, `column_value`, `row_value` (titles "Power %", "mm/min", "Passes",
  "Interval mm"); lane widths passed in; size and the bed-fit cause ("Grid is 260 mm wide, the bed
  is 200 mm.") (AC 5, 7, 10).
- [x] A4. `test_grid_ops.rs` ("the derived operations a recipe resolves into"): from a recipe, a
  layout and an optional label set, the wanted parts in list order: `labels` (Engrave, linked to the
  material's Engrave set for the page's laser else own 20 % and 3000 mm/min) and one `cell` per
  cell, top-left first, row by row, each with name ("60 % · 1950 · K7Q2M", "K7Q2M labels"; units
  only on power), `op_settings` own numbers, `op_options` Crosshatch for Engrave. Test: 5 x 5 gives
  26 parts with the expected names and numbers; 12 x 12 gives 145 (AC 10, 12).
- [x] A5. `material.rs` and `material_edits.rs` model of format 2: `Material { id, name,
  thickness, modified, lasers: Vec<LaserSets>, tests: Vec<TestRecord>, history }`; `LaserSets {
  machine, sets: cut|score|engrave }`; `Set { power, speed, passes, interval (Engrave required),
  source: Test(id) | Typed | History(entry) }`; `TestRecord { id, created, machine, recipe,
  lasered, best, note, photo: None|Jpg|Png }`. Edits: add laser with its first set (at most 16),
  add, edit, delete a set (a hand edit of a tested set becomes Typed, G13), rename, thickness;
  `write_test` (create at a pick, rewrite recipe while not Lasered, move between materials
  without retiring), `judge(test, cell)` (writes `best` and the laser's set of the recipe's kind
  from the cell's axis values over `fixed`, only the four numbers, appends a `set` entry; a tap on
  the best cell changes nothing, G14), `set_note` (at most 500 characters), `set_photo_kind`,
  `delete_test` (returns the id to retire); limits 200 tests, 500 history; `modified` as `max(now,
  previous + 1)`, `now` passed in (AC 17 to 26).
- [x] A6. `material_file.rs` format 2: canonical writer (key order of ADR 0058 §2, two-space indent,
  trailing newline), reader with every damage of ADR 0058 §6, text limit 512 KiB, newer than 2
  skipped. **Without Part F:** format 1 reads as `Skipped(OlderFormat)`. Goldens in
  `tests/fixtures/materials/`: `format2_full.json` (reads and writes back byte for byte),
  `format2_one_laser.json`, `format2_max.json` (16 lasers, 200 tests, 500 entries, 500-character
  notes; reads, fits 512 KiB), one damaged file per new rule (`duplicate_laser.json`,
  `set_wrong_kind.json`, `interval_missing.json`, `interval_on_cut.json`, `source_no_test.json`,
  `test_id_bad.json`, `test_id_twice.json`, `best_outside.json`, `note_501.json`,
  `photo_gif.json`, `tests_201.json`), `newer_format.json` to format 3. The format-1 goldens stay
  on disk for Part F (AC 19, 22, 30).
- [x] A7. `material_retired.rs` ("reads and writes the retired test ids"): format 1, sorted, at most
  20,000 (Proposal), append-only merge, damaged reads empty with one notice, never rewritten on read.
  Goldens `retired_ids.json`, `retired_ids_damaged.json` (AC 9, 17, 25).
- [x] A8. `material_photo.rs` ("checks a photo's bytes and names its file"): JPEG or PNG signature,
  width and height from PNG `IHDR` or JPEG `SOF0..SOF15` (no decoding), refuse long side above 600
  or more than 1 MiB, file name `<uuid>.<test id>.jpg|png`; returns `WritePhoto { name, bytes }` /
  `RemovePhoto { name }`. Goldens `photos/7k2qx_600.jpg`, `photos/wide_601.png`,
  `photos/not_an_image.jpg` (AC 27, 31).
- [x] A9. `materials.rs` store: `taken_test_ids()` = live test ids of every loaded record + retired
  (the session adds the open project's grid ids); `operation_settings_rule.rs` per-laser tie (live
  when the record has a `lasers` entry of the page's profile id with the kind's set, else stored
  with "Material has no <kind> set for this laser, using stored numbers"); only the four set
  numbers are ever copied into an operation (AC 19, 21).
- [x] A10. `material_preferences.rs`: `test-grid-choice.json` format 2 (the v2 recipe per machine);
  format 1 reads as empty (a preference, not a conversion); golden `test_grid_choice_v2.json` (AC 2).
- [x] A11. `operation_naming.rs` `last_of_kind` skips rows with a readable `op_grid_part` (ADR 0056
  §3) (AC 12).
- Gate A: fmt, clippy, nextest for laser-core, render-core, ui-core; wasm32 build of laser-core.

**Part A deviations (implementer, 2026-11-12, all (Proposal) for the architect):**
- The `0028` session in `curvyo-editor-wasm` still builds on the format-1 model, `GridRecipe` and
  the folded row, so the new code sits **beside** the old until Part C moves the session over: new
  public modules `curvyo_laser_core::grids` (test ids, recipe v2, layout, derived operations,
  `test-grid-choice` format 2) and `curvyo_laser_core::library` (record format 2, edits, retired
  ids, photos); `test_grids`, `material_library` and the record types of `settings` stay and go in
  Part C (then the new files may take the final names of adrs.md). New files: `test_id.rs`,
  `test_param.rs`, `test_recipe.rs` (+ `_tests.rs`), `test_recipe_json.rs` (stored form, shared by
  grid and record), `test_recipe_layout.rs`, `test_grid_fit.rs` (bed fit and free spot, moved out of
  the old layout and re-exported on both paths), `test_grid_ops.rs`, `test_grid_choice.rs`,
  `material_record.rs`, `material_record_edits.rs` (+ `_tests.rs`), `material_record_file.rs`,
  `material_test_file.rs`, `material_retired.rs`, `material_photo.rs`; `SetNumbers` added to
  `material.rs`. Goldens in `tests/fixtures/materials/` (+ `photos/`), read by
  `tests/material_records_v2.rs` and the unit tests; `newer_format.json` now says format 3.
- A0 moves to Part C: `test_grid_run.rs`, `render-core/worked_chip.rs` and the six labels are still
  used by the `0028` session; removing them first would break editor-wasm.
- A9: the taken set is `library::live_test_ids` plus `RetiredIds` (the session adds the project's
  grid ids); the per-laser tie is `Material::set_for` with `no_laser_set_text`.
  `operation_settings_rule` still takes the format-1 `MaterialRead`; it switches to
  `set_for` in Part C (one line in `live_material`), so A9 stays open.
- A11 moves to Part B: `OperationSnapshot` has no `op_grid_part` yet (B1 adds it; `GridPart::read`
  in laser-core is ready for it).
- Without Part F a format-1 record reads `MaterialRead::Older` ("Material has an older format.");
  a `{ "history": ... }` source and `tested` entries are refused in format 2 until Part F.
- Finding: `format2_max.json` (200 tests, 500-character notes, 500 entries, 16 lasers) is 460 KB
  with ASCII notes and 560 KB with two-byte characters, so 512 KiB holds the maximum only for
  ASCII notes; the store must refuse a write over the limit (Part C), as ADR 0055 §4.

## Part B: document-core grid object, derived rows, format 46 (Opus, medium)

- [x] B1. `test_grid_codec.rs`: read and write the recipe map (any damage of ADR 0057 §6 makes the
  group plain), `test_grid_lasered` (non-negative integer, else Not lasered), `grid_cell`
  `{column,row}`, `grid_label` values, `op_grid_part` (not a map: ignored); a tag counts only under
  a group with a readable recipe (AC 12, 15).
- [x] B2. `test_grid_commands.rs` `place_test_grid(page, at, recipe, layout, parts)`: one commit
  "Add test grid" writing the group, items and derived rows (new rows at the end of each kind's
  section) (AC 2, 12).
- [x] B3. `regenerate(grid, recipe, layout, parts)`: rows rewritten in place by part (node id and
  place kept; name, Show, Output, `op_settings`, `op_options`, membership overwritten), missing
  parts after the grid's last row, surplus rows and items deleted, items re-laid out; one commit
  with the caller's label; undo restores the old rows exactly (test with a hand-edited row) (AC 14).
- [x] B4. Lifetime: delete, cut and ungroup of the group and a move off the laser page delete every
  row whose `op_grid_part.grid` is the group in the same commit; an orphan row (merge) is swept by
  the next local step; copy rule of ADR 0046 §4 skips derived rows, and `copy_test_grid` writes a
  new group with the id the caller gives, no Lasered register, its own rows (AC 8, 13).
- [x] B5. New id on a Lasered grid: `renew_test_id_in(commit, grid, new_id)` writes the new `id`,
  deletes `test_grid_lasered`, rewrites the `id` label item; called by B3 and by every settings,
  options and membership write that touches a derived row (not rename, Show, Output, order). A
  grid that is not Lasered keeps its id (AC 11).
- [x] B6. `recreate_test_grid(page, at, recipe_with_id, lasered)`: "Place test grid", one commit
  (AC 28).
- [x] B7. Format 46: `CURRENT_FORMAT_VERSION` 46 and its doc block; a format 47 file refused.
  **Without Part F:** 43 to 45 files open unchanged, `op_grid` rows inert and written back. Goldens:
  `test_grid_v46.curvyo` (5 x 5 Cut grid with material, 26 rows, one hand-edited cell row; an
  Engrave grid with Crosshatch and Lasered) reads and writes back equal;
  `test_grid_damaged_v46.curvyo` (bad id, both axes on power, count 13, non-integer Lasered,
  `op_grid_part` as text; each opens per ADR 0057 §6 and saves back unchanged);
  `test_grid_orphan_rows_v46.curvyo` (AC 15, 29).
- [x] B8. ui-core labels and step scope of the list above (AC 2, 14, 17 to 28).
- Gate B: fmt, clippy, nextest for document-core, ui-core and the editor-wasm tests that pin the
  format; wasm32 build.

**Part B deviations (implementer, 2026-11-12, all (Proposal) for the architect):**
- The `0028` commands in `test_grid_commands.rs` stay until Part C (the session still uses them);
  the new commands sit beside them in `test_grid_object.rs` (place, regenerate, copy, renew id,
  off-page removal, Lasered hook; + `_tests.rs`, 540 lines of tests) and `test_grid_rows.rs` (the
  derived rows' registers, place in the list, the orphan sweep). A11 is done (`last_of_kind`
  skips a readable `op_grid_part`; `OperationSnapshot::grid_part` carries the plain value).
- document-core cannot read the recipe (it does not depend on laser-core): for lifetime and
  regeneration a grid is any group with a `test_grid` register; whether it reads is the
  session's call, which never regenerates a damaged one. Its rows are swept when the group goes.
  `op_grid_part` is read as `GridPart::read` reads it (exact keys); one that does not read is an
  ordinary row, never swept or rewritten.
- Delete, cut, ungroup and merge orphans: the sweep runs in every local step
  (`sweep_dead_member_keys`), so the rows go in the deleting step and undo restores them. A move
  off the laser page: page ownership is by centre with artwork (ui-core), so the session calls
  `remove_test_grid_rows(group)` inside the move's `Document::batch` (Part C).
- Copy: Duplicate and Paste copy the group bare (no Lasered register, no rows); the session calls
  `copy_test_grid(copy, write)` with a new id in the same batch (Part C). `regenerate_test_grid`
  enforces ADR 0057 §4: a Lasered grid whose recipe changes needs a new id (`IdKept`), any other
  keeps it (`IdChanged`); the session gives the id. `renew_test_id(group, id)` is its own command
  that the session calls in the batch of a hand edit of a derived row's settings, options or
  members (Part C); it also replaces the old id in the derived rows' names.
- Regeneration replaces the tagged items (new node ids) instead of rewriting them in place; the
  rows' members follow. New rows of a new grid go where the caller says (`DerivedRow::at`, the
  end of each kind's section is the session's), missing parts of an existing grid after its last
  row.
- Lasered hook in document-core: `#[doc(hidden)] mark_test_grid_lasered_for_tests`, label
  `mark_test_grid_lasered` named "Mark lasered" (the history-name guard needs a name).
- `crosshatch` is required in every recipe by the laser-core reader (`false` for Cut and Score);
  ADR 0057 §1 says "Engrave only": the architect aligns the wording.
- B8: the new labels are `MaterialStep` variants and constants in document-core and rows in
  `history_names.rs`/`step_scope.rs`; the six `0028` labels stay until Part C (A0).
- Format pins to 46 done here (G2 in part): document-core tests, editor-wasm
  `acceptance_0024_slice2_tester`, `acceptance_0106_tester`, the SVG export `*.document.json`.
  Without Part F, 43/44 goldens open unchanged (their tests pass). Golden reading by the laser
  extension: `curvyo-laser-core/tests/test_grid_goldens.rs`.

## Part C: session, view JSON, wasm bridge, budgets (Opus, medium)

- [x] C1. `session/test_grid/` `TestGridTool` (Finding 1): activate, leave, page activation hook;
  press places at the press point else the first free spot (G2), Enter places at the first free
  spot; a press on a grid selects it; hover outline; draft per machine from `test-grid-choice`
  (written on commit, not a step); ids from `TestId::make` over the taken set plus the open
  project's grid ids; caller random bytes as for material ids (AC 1, 2, 7, 9).
- [x] C2. Bar commits: one field per commit, invalid values kept in the draft with the cause and no
  write (no clamping), valid commits run B3 (and B5 when Lasered) with "Edit test grid" and, when a
  material is picked and the test is not Lasered, rewrite the test record in the same step through
  the material journal; status "New test id 8PL4D." (AC 3 to 6, 11, 14, 22).
- [x] C3. Material pick: "Use material" step writes the recipe's material, regenerates (label row
  linked, shared values from the set), creates or moves the test record; "New material" from the
  bar opens the form and Add picks it (G8, G9) (AC 6, 22).
- [x] C4. Selection and editing: one selected grid shows the bar with the Select tool (G3); the
  `0111` kind buttons are absent for a grid selection (G4); handles off except move (no resize,
  rotate, skew; S, R, K inert); Duplicate and Paste go through B4 with a new id; derived-row hand
  edits in `operations/settings.rs`, `options.rs`, `assign.rs` call B5; `kind_press.rs` never
  reuses a derived row (AC 7, 8, 11).
- [x] C5. `session/materials/`: tab state (one unfolded material, open test, laser folding; not
  steps); add material (form values, "Add material"), rename, thickness, delete (retires its test
  ids, removes photos, "Delete material"); lasers and sets ("This page" first, others in added
  order, add laser by its first set, "Add set", "Edit material", "Delete set"); profiles of the
  library not yet in the material for "+ Sets for another laser" (AC 16 to 21).
- [x] C6. Tests: list newest first with state and page name from the open project's grids; open
  test; judge (A5 judge, "Choose best cell"); note ("Edit note"); photo bytes in (A8 check, host
  write `write_photo`, "Add photo" / "Crop photo"), remove ("Remove photo"); delete test (retire,
  remove photo, "Delete test"); journal entries hold photo before/after bytes within 16 MiB; retired
  file written as a host write, not a step (AC 22 to 27).
- [x] C7. "Place on <page>" (B6 on the active laser page, first free spot) and "Select on page"
  (selects the grid, activates its page) (AC 23, 28).
- [x] C8. Lasered hook: `#[doc(hidden)] pub fn mark_test_grid_lasered_for_tests(grid, at_ms)`,
  Rust only, not in `wasm_test_grid.rs`; writes the register and the record's state in one commit
  (AC 11).
- [x] C9. View JSON and shared fixtures (Finding 3): `test_grid_view` (recipe, fields with ranges
  and causes, fit cause, id, Lasered, material) and `materials_view` (rows with counts, unfolded
  body with lasers and sets with source, tests with state, test detail with table, best cell, set
  line, note, photo name or missing, place target); the host write list gains `write_photo` and
  `remove_photo` (byte handle) and `retired`. Fixtures (replace the `0028` ones):
  `test_grid_view/engrave_crosshatch.json`, `lasered.json`, `no_material.json`, `invalid_value.json`,
  `does_not_fit.json`; `materials_view/two_lasers.json`, `typed_set.json`, `test_with_photo.json`,
  `test_without_photo.json`, `format1_record.json` (Part F; without it the skipped row),
  `damaged_record.json`, `empty.json`; `material_writes/judge.json`, `photo_write.json`,
  `photo_remove.json`, `delete_material_retires.json`. Contract test
  `tests/test_grid_materials_view_contract.rs` builds each scenario and compares the JSON value
  (AC 3 to 6, 17 to 27). The `operations_view` fixtures gain the source line of derived rows.
- [x] C10. `wasm_test_grid.rs` / `wasm_materials.rs`: tool activate, place at, bar field commit,
  axis parameter, kind, material pick, place from test; materials add, rename, thickness, delete,
  add set, edit set, delete set, open test, judge, note, photo bytes (`&[u8]`), remove photo,
  delete test, load retired text, take writes. Old `0028` bindings (save as material, rebuild,
  mark, unmark) go.
- [x] C11. Budgets `tests/test_grid_materials_budget.rs` (release, thread CPU, `tests/cpu_time/`,
  in the `performance-budgets` job): place a 12 x 12 grid within 50 ms; a bar edit regenerating it
  within 30 ms; Materials tab JSON with 200 materials within 50 ms; load 200 format-2 records within
  200 ms; Operations view with a 12 x 12 grid within 16 ms; delete a 12 x 12 grid with its rows
  within 30 ms. A failed budget is recorded in `adrs.md`, never silently changed (AC 32).
  `material_test_library_budget.rs` and `acceptance_0028_budget_tester.rs` lose the superseded cases.
- Gate C: fmt, clippy (host and wasm32), nextest for editor-wasm (in chunks), ui-core,
  document-core; wasm32 build; rustdoc.

**Part C deviations (implementer, 2026-11-12, all (Proposal) for the architect):**
- Files: `session/test_grid/` (`mod.rs` the one `TestGridTool` field, `items.rs`, `bar.rs`,
  `place.rs`, `view_json.rs`, `tests.rs`) and `session/materials/` (`mod.rs`, `step.rs` one
  library change as one step, `commands.rs`, `test_records.rs`, `undo.rs`, `refresh.rs`,
  `view_json.rs`, `tests.rs`); laser-core `material_store.rs` (`library::Library`: format 2, the
  order, revert, a record over 512 KiB refused with "Could not save <name>: it is over 512 KiB.").
- A0 done here: laser-core `test_grid`, `test_grid_layout`, `test_grid_run`, `material_file`,
  `material_edits`, `materials` and the format-1 types of `material.rs` go; `material_preferences`
  keeps `material-order.json` only. Part F takes the format-1 reader back from git (`a649a665^`
  `material_file.rs`) into `material_file_v1.rs`. document-core: `add_test_grid`, `edit_`,
  `rebuild_`, `set_test_grid_mark`, their labels and `SaveAsMaterial` go; the format-43 goldens are
  frozen (no builder). render-core `worked_chip.rs` and the six labels go (old rows read the
  unknown-name fallback). A Materials tab in ui-core (`PanelTab::Materials`).
- A9: the rule reads sets per laser (ADR 0058 §7); `StoredReason::OtherMachine` becomes `NoSet`
  ("... for this laser ..."), `Older` is new. The Engrave set's interval fills only an operation
  without its own: a stored interval stays on every material path (`0111` criterion 23 tests);
  adrs.md says "linked to the Engrave set", the architect aligns.
- Ids: the host feeds bytes (`feed_test_grid_random`); without them a stream from the peer id.
- Copies and off-page removal: no move or duplicate path calls the commands itself;
  `recording_unguarded` reconciles after a command's step (`continue_step` joins it): rows of a
  grid whose box centre left their laser page go; after `duplicate_objects` a selected grid
  without rows gets `copy_test_grid` with a new id. The copy's test record, like the record of the
  new id after a hand edit (`renew_test_id` only), is made at its next bar commit or pick.
- A press on a grid selects it by its box (cells are outlines); Enter is `test_grid_press(None)`.
- "+ <kind> set" adds the set with the G7 start numbers instead of empty fields (a record set
  holds no empty number). Undo of a photo step writes the earlier bytes back when the session
  knows them (written here or `load_material_photo`), else removes the file.
- The `0028` edge that switched to the Operations tab on a grid selection goes (the bar edits it).
- Superseded tests removed: editor-wasm `acceptance_0028_tester`, `acceptance_0028_budget_tester`,
  `material_test_library_budget`, `material_test_library_view_contract`; laser-core
  `acceptance_0028_tester` and the grid tests of `settings_goldens`; document-core
  `test_grid_commands_tests`. Tester files adapted in format or API only:
  `acceptance_0111_settings_tester` (format-2 records, the laser argument, the inert v43 grid row),
  `acceptance_0111_tester` (the tool for the button), `acceptance_0098_tester` and
  `manufacturing/tests.rs` (Materials tab), `acceptance_0020_tester_more` (the v46 goldens of Part
  B join the explicit-steps list).
- Budgets (release, thread CPU): place 7.5 ms, bar edit 6.9 ms, Materials tab of 200 0.12 ms, load
  of 200 records 25 ms, Operations view of 145 rows 1.9 ms (38 ms before the copy was read once per
  view), delete 1.3 ms. The CI performance job runs `test_grid_materials_budget`.
- Open: no `operations_view` fixture with a derived row's `source` yet (all rows `null`); the
  frontend still calls the `0028` bindings, so `tsc` and `npm test` fail until Part E;
  `materials_view/format1_record.json` is the Older row until Part F.

## Part D: host files for photos and retired ids (Opus, medium)

- [x] D1. `curvyo-app/src/materials.rs`: `read_material_photo(name)`, `write_material_photo(name,
  bytes)`, `remove_material_photo(name)` accept only `<uuid>.<test id>.jpg|png` (test like
  `only_record_ids_reach_the_materials_folder`); `read_retired_test_ids`, `write_retired_test_ids`
  next to `materials/`; registered in `main.rs`. No new dependency (AC 27, 31, 33).
- [x] D2. `frontend/src/platform/host.ts`: IndexedDB `curvyo-material-photos` (key the desktop file
  name), `localStorage` `curvyo.retired-test-ids`; a refused write keeps the record and gives
  "Could not save the photo: <reason>."; a missing or unreadable photo resolves to "missing"
  (AC 27, 31). Follow the IndexedDB pattern of `lib/browserFonts.ts`; no new dev dependency, Vitest
  uses a hand-written double behind the host interface.
- Gate D: fmt, clippy, nextest for curvyo-app; `npx tsc -b --noEmit`, `npm test`.

**Part D deviations (implementer, 2026-11-12, all (Proposal)):**
- The write glue lives in `frontend/src/lib/materialHostWrites.ts` (alias-free, so `node --test`
  runs it against the `material_writes/` fixtures with a host double): records, photo bytes taken
  by handle and sent as the raw body, `retired`, order and grid choice; every write is tried after
  a refusal. `materialsFile.ts` only binds it to `invoke` and the session: it reads and loads
  `retired-test-ids.json`, `loadMaterialPhoto(session, name)` reads a photo (also handing it to
  `load_material_photo` for undo) and answers `"missing"` for none, empty or unreadable.
- The browser photo commands and the IndexedDB store are `frontend/src/lib/materialPhotoStore.ts`
  (store `photos`, out-of-line key = file name, value the bytes); `host.ts` adds the three photo
  commands and `curvyo.retired-test-ids`. `PhotoStore` is the interface behind which the tests use
  a hand-written double (`node --test`, not Vitest).
- `MaterialWriteFailure` gains `photo: boolean`; a refused photo keeps the record (no reload) and
  its notice is `photoFailureText` "Could not save the photo: <reason>."; Part E shows it. A
  refused retired file is silent like the order (no step; the next retire writes all ids).
- The 512 KiB limit is also a host rule: the glue refuses a record text over 512 KiB in UTF-8
  before the host ("it is over 512 KiB", the record-failure path reloads the library), and
  `write_material` refuses it on the desktop and in the browser, so a written record always lists.
  Photos over 1 MiB are refused on write and on read (desktop and browser).
- `materialWrites.ts` decodes `retired`, `write_photo` (`name`, non-negative integer `handle`) and
  `remove_photo`; its test reads the four `0112` fixtures (E1 adds the view decoders).
- Left for Part E: `tsc` errors only in `hooks/useMaterials.ts` (5) and `hooks/useTestGrid.ts`
  (6); `npm test` 46 failures, all in decoders and fixtures of the `0028` views.

## Part E: frontend (Sonnet, medium; after the finished UX notes)

- [x] E1. Decoders `testGridView.ts`, `materialsView.ts`, `materialWrites.ts` check every field;
  tests decode each C9 fixture and reject a missing and a mistyped field (AC 3 to 6, 17 to 27).
- [x] E2. `ToolRail.tsx`/`railCard.ts`: third button of "More tools", `TestGridGlyph`, dimmed with
  "Laser pages only." off laser pages, no letter key (G1) (AC 1).
- [x] E3. `useTestGridTool.ts` (only caller of the grid bindings; camera and crop never in
  `useEditorSession.ts`) and `TestGridBar.tsx`: two rows per UX 4.1, mono id, Kind and parameter
  radiogroups, plain fields with units, Crosshatch toggle, whole-group wrapping and the one break
  (G5), invalid values kept with the Action notice, sr status for a new id; `SelectToolbar.tsx`
  shows the bar for one selected grid (AC 2 to 5, 11, 14).
- [x] E4. `TestGridMaterialMenu.tsx`: the Trace Preset menu with "New material" first, thickness,
  Filter above 8 (AC 6).
- [x] E5. Rows: `OperationBody.tsx` source line of derived rows, no material pick on cell rows
  (G10); `OperationsSection.tsx` loses the Test grid toolbar, `TestGridBlock`, the Materials
  section and `SaveAsMaterialForm` (AC 12, 16).
- [x] E6. `PanelStrip.tsx` `materials: SwatchBook` after Operations, laser pages only;
  `MaterialsTab.tsx` (New material form of `0045`, windowed flat list), `MaterialRow.tsx` (line 2
  counts, arrows, rename, armed trash with its status), `MaterialBody.tsx`, `LaserSets.tsx` (laser
  blocks, sets, "+ <kind> set", "+ Sets for another laser" in place) (AC 16 to 21).
- [x] E7. `TestRow.tsx`, `TestDetail.tsx` (back, head with trash, state and axes lines,
  `TestGridResult` with one-tap best cell and the set line, Note at 500, place button) (AC 22 to
  26, 28).
- [x] E8. `TestPhoto.tsx`, `PhotoCrop.tsx`, `useCamera.ts`, `lib/photoSize.ts`: File (JPEG/PNG,
  "Use a JPEG or PNG image."), Camera only with a camera and permission (stream stops on cancel,
  take, tab and test change), Crop with eight handles and keyboard, canvas encode JPEG 0.85 (PNG
  stays PNG) at 600 px long side; "Photo not found". `photoSize.ts` pure and tested (AC 27, 31).
- Gate E: `npm run build:wasm && npx tsc -b --noEmit && npm test && npx oxlint`.

**Part E deviations (implementer, 2026-11-13, all (Proposal)):**
- Gate E green: `build:wasm` 0, `tsc -b --noEmit` 0, `npm test` 0 (1133 pass, 0 fail), `oxlint` 0, `vite build` 0.
  No browser here: canvas press, hover outline, wrapping, camera and crop pointer work are for Part H.
- Files differ from the list: `hooks/useTestGridTool.ts` (takes the tool state and the canvas
  container for Enter), `lib/testGridBar.ts` (field logic, new), `lib/testGridResult.ts` is the judge
  table helper, `components/panel/JudgeTable.tsx` (was `TestGridResult.tsx`), `NewMaterialForm.tsx`,
  `LaserSets.tsx`, `MaterialBody.tsx`; `MaterialRow.tsx` is edited in place, not renamed. The
  Test grid button is wired through `workbenches/builtin.tsx` (`test_grid_tool_available`), the
  bar through `App.tsx` (tool `test-grid`, or one selected grid with the Select tool).
- `acceptance_0028_tester.test.ts` is cut to the three cases `0112` keeps (damaged title, Alt+arrows,
  no colour); the Worked cell, Save as material form, Replace sentence and operation use count of
  `0028` are superseded (UX notes 15) and replaced by `materialRows.test.ts` and `photoSize.test.ts`.
  `acceptance_0098_tester.test.ts` AC6 lists `materials` among the page-scoped tabs (the kind chip
  sits in its header, UX 7.1). The `operationRows` / `operationUnfold` tests drop the folded grid row.
- Digits 1 to 9 now count all rows (no grid rows exist any more); the row `grid` field is gone,
  replaced by `source` (`{grid, cell, text}`).
- The armed test status leaves out "Its photo goes with it." when the test has no photo.
- "Edit material" in an operation unfolds the material and switches to the Materials tab (no
  scroll-to-row focus); the bar's "New material" calls `open_material_form(<group JSON>)` and the
  panel switches tab. The New material draft (name, thickness) lives in `useMaterials`, so a tab
  change keeps it.
- Photo from a file or camera is stored, then Crop opens on the stored 600 px copy at once; Cancel
  keeps the stored photo (G16). Crop of an existing photo cuts a decoded copy and writes with
  `cropped: true`. The camera button is offered when `getUserMedia` exists and the permission query
  does not say `denied`; a refusal at start hides it and shows "Camera access was refused.".
- Notices of the Materials tab (refusals, "Use a JPEG or PNG image.", save failures) show under the
  list as one status or alert line for 8 s, not under the control (Part H may move them).

## Part F: legacy conversion (Opus, medium) — dropped (Frage 214 B, 2026-11-13); kept below for the record

- [ ] F1. laser-core `material_file_v1.rs` (the format-1 reader moves here from `material_file.rs`,
  keeping each under 500 lines): one laser entry from `machine`, sets without interval (Engrave
  without interval allowed only from format 1), no tests, history kept with `tested` entries, a
  source naming a `tested` entry reads `History(entry)` ("Test of <date>"); written as format 2 at
  the next change only. Goldens: the format-1 files (`full.json`, `generic_laser.json`,
  `no_sets.json`, `history_max.json`, the damaged ones) still read; `format1_as_format2.json` is
  what `full.json` writes after a change (AC 30).
- [ ] F2. document-core `test_grid_migration.rs` (beside `page_kind_migration.rs`): one
  `migrate_format` commit at open for files below 46: each format-43 recipe becomes v2 (columns
  power, rows speed, `fixed.passes` 1, its cell size and kind, the id the session gives), one derived
  row per cell with the cell's stored numbers (items not re-laid out), one labels row with the old
  label settings, the `op_grid` row deleted; `test_grid_worked` kept inert; the validator accepts
  `op_grid` and old tags below 46. Goldens: `laser_settings_v43`, `test_grid_hand_edited_v43`,
  `test_grid_orphan_row_v43`, `laser_settings_damaged_v43` open by the migration with their run
  numbers unchanged and nothing lost; a 44 golden (`operation_tools_v44` plus a grid) too (AC 29).
- [ ] F3. Session: calls F2 after open with fresh ids (not an undo step, file not written until
  saved); the first test record of a migrated grid takes `test_grid_worked` as its best cell and
  deletes it in that commit; the `id` label appears at the first bar change. View fixture
  `materials_view/format1_record.json` (AC 29, 30).
- Gate F: nextest for laser-core, document-core, editor-wasm (affected tests).

## Part G: mechanical follow-ups (Haiku)

- [ ] G1. Superseded tests (rows of "Changes to accepted specs"): `acceptance_0028_*` of laser-core
  and editor-wasm (folded row, Worked, Rebuild, Save as material, Materials section), the `0111`
  testers that name the Test grid button, `material_test_library_view_contract.rs`; each change
  listed in the PR.
- [ ] G2. Format pins to 46: document-core tests, `acceptance_0106_tester`,
  `acceptance_0024_slice2_tester`, the SVG export `*.document.json` goldens of `curvyo-svg-core`;
  `newer_format.json` pins.
- [ ] G3. Docs at acceptance: ADR 0057 and 0058 status; notes on ADR 0054 §4 to §6, 0055 §1 to §4,
  §8, §9, 0046 §3 §4; `docs/adr/index.md`; `docs/design-system.md` rows of UX section 14;
  `docs/technical-debt.md` if a budget or a deviation is left; spec `Status: Done (PR #n)` (lead).

## Part H: verify (Sonnet, medium)

- [ ] H1. Chromium run of the demo path (one agent drives the browser; the camera step with a fake
  device flag); screenshots to the scratchpad `0112/`; UX review by the ux-engineer; architect
  review (format 46, record format 2, photo files, host commands).
- [ ] H2. Full gate of `CLAUDE.md` §7, steps separately; report to the lead.

## Validation

- Tests first for A1 to A11, B1 to B7, C1 to C8, F1 to F3; goldens read and write back equal, one
  damaged golden per rule; shared Rust/TS fixtures pin producer and decoder; budgets in thread CPU
  time only.
- The tester writes `acceptance_0112_*` from the spec before reading this code; the Lasered hook
  (C8) is its way to criterion 11 and 28.
- AC 33: no network, device I/O, G-code or telemetry code; the camera stream and the image bytes
  stay in the browser and the data directory; the tester checks the diff.

## Demo path (desktop and browser)

1. New project, laser page with Generic laser; Materials tab after Operations, empty but
   "+ New material". On a drawing page the Test grid tool is dimmed, "Laser pages only.".
2. Test grid tool: the outline follows the pointer; press: a 5 x 5 Cut grid at the press, one
   step "Add test grid", id in row 1, the Operations tab holds 26 rows ("... · K7Q2M",
   "K7Q2M labels"); the id, axis titles and values are on the canvas.
3. Bar: Columns 3 to 6: grid and rows regenerate (one step); Rows parameter Power: the axes swap;
   Cell size 60: kept, marked, "Cell size is 3 to 50 mm."; Count 12 on a small bed: "Grid is ...
   wide, the bed is ... mm."; Kind Engrave: Interval offered, Crosshatch toggle.
4. Material "New material": the Materials tab opens on the form; Add "Birch 3 mm": the grid uses it,
   "Tests (1)" lists the id "Not lasered · <page>".
5. Hand-edit one cell row's power, then a bar change: the edit is overwritten; Undo brings it back.
6. Open the test: tap a cell: check and frame, "Cut set of Generic laser: 60 %, 1950 mm/min, 1 ×";
   the material shows the Cut set "Test K7Q2M". Note text, leave: one step. Photo from a file,
   crop, then the camera; the row shows the photo glyph.
7. "+ Sets for another laser": pick another profile, "+ Engrave set", type numbers ("Typed").
   On a page with that laser an operation linked to the material without a Cut set reads "Material
   has no Cut set for this laser, using stored numbers".
8. Duplicate the grid: new id, own rows; delete it: its rows go. Drag the first grid onto a
   drawing page: its rows go; the test reads "not on a page"; "Place on <page>" brings it back with
   the same id.
9. Delete the test (armed trash): the photo goes; a new grid never gets that id (session test).
10. Save, reopen: grids, rows, tests, best cell, note and photo as before. Open a format-43 project
    with an old grid (Part F dropped): the old row opens as an ordinary operation.
