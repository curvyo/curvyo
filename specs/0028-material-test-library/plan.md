# Plan for Material test library (0028), Parts A to E

Branch: `story/0028-material-test-library`. Built on `specification.md` (criteria 1 to 31,
Fragen 183 to 186 answered, changed answers 187 B+C, 188 B, 189 B, Frage 190 at its default A),
`adrs.md` and ADR 0054 (project file, `format_version` 43) and ADR 0055 (material record format 1)
as revised 2026-11-11, `ux-notes.md` (M1 to M16 applied).
One PR for the whole slice (the customer can try it only after Part E); Parts A to C are commits
on this branch. Each Part ends with the gate of its crates (Part E: the full gate of `CLAUDE.md` §7).

Where the documents differ, this plan follows (for the product owner to align before the PR):
- Criteria 1 to 3 still name an inline make block with "Make grid"; built is M1 and "Where settings
  live" 1 to 3 (one press adds the grid with remembered or default values, fields edit it).
- Neither UX 8.4 "Changed since saved" nor "stored: 60 %" is built (187 B+C, adrs.md S1b): the
  stored numbers are refreshed at every material edit in its step and at every save, so the line
  shows live, stored with reason, or "Differs from <name>" with overrides (188 B, S1, S1a).
- Material edits are undo steps (189 B, S8, S8a); criteria 15, 18, 21, 22 and the new limits
  criterion follow adrs.md §7 and ADR 0055 §9 until the spec is aligned.
- Undo labels are the full list of adrs.md §7 (`use_own_numbers`, `rename_test_grid`,
  `delete_test_grid_row`, `reset_to_material` and the four library steps included).
- Criterion 27 and M10 amend `0101` 4.4 (several rows selectable); built in Part D.

## Affected crates/modules

- `curvyo-laser-core` (pure, wasm32): new `operation_settings.rs`, `test_grid.rs`,
  `record_envelope.rs`, `material.rs`, `material_file.rs`, `materials.rs`; `machine_profile_file.rs`
  moves onto `record_envelope` (no behaviour change); `operation_sequence.rs` / `operation_kind_run.rs`
  learn the grid row's run rule. Goldens `tests/fixtures/materials/`.
- `curvyo-document-core`: `operation_codec.rs` (`op_settings` plain values, `op_grid`), new
  `operation_settings_commands.rs` (Use material, Edit settings, Reset to material, Use own numbers,
  the material-edit refresh with the root marker, the save refresh; `operation.rs` is 616 lines and
  does not grow), new `test_grid_codec.rs` (the `x.curvyo.laser.test_grid`, `test_grid_worked`,
  `grid_cell`, `grid_label` keys and reads), new `test_grid_commands.rs` (make, edit, rebuild, mark,
  unmark, the delete and copy cases); `manufacturing_codec.rs`, `document.rs` (format 43). Goldens
  `tests/fixtures/`.
- `curvyo-ui-core`: `history_names.rs`, `step_scope.rs` (labels of adrs.md §7 plus the three above).
- `curvyo-render-core`: the grid chip row and the Worked chip (`operation` look code beside
  `fixture_look`).
- `curvyo-editor-wasm`: `session/materials/` (state beside `MachineProfiles` in
  `session/manufacturing/machine.rs`, commands as document steps, `undo.rs` record journal,
  `view_json.rs`), `session/test_grid/` (make, edit,
  rebuild, mark, cell hit, label widths via `curvyo-text-core`, `view_json.rs`),
  `session/operations/settings.rs` (+ `view_json.rs` gains `settings`), `wasm_materials.rs`,
  `wasm_test_grid.rs`, `wasm_operations.rs` (settings entry points, multi-select).
- `curvyo-app`: new `materials.rs` (Tauri commands over `curvyo-storage-io`; check `remove_file`
  exists, else add it there); `frontend/src/platform/host.ts` (localStorage keys of ADR 0055).
- Frontend: `lib/materialsView.ts`, `lib/testGridView.ts`, `lib/operationsView.ts` (settings),
  `lib/materialRows.ts`, `lib/testGridFields.ts`, `hooks/useMaterials.ts`, `hooks/useTestGrid.ts`,
  `components/panel/` `OperationSettingsBlock.tsx`, `MaterialPick.tsx`, `MaterialsSection.tsx`,
  `MaterialRow.tsx`, `MaterialFields.tsx`, `TestGridBlock.tsx`, `TestGridResult.tsx`,
  `SaveAsMaterialForm.tsx`; `OperationsSection.tsx` (567 lines) only places the blocks.

## Part A: core settings, run rule, test grid recipe, format 43 (Opus, medium)

WIP of the first Part A attempt (written to the ADRs before the revision, not committed):
`/tmp/claude-1000/-home-marc-curvyo/f353e023-c91a-5318-bf48-5d3b382c55e5/scratchpad/partA-wip/`
(`material.rs`, `test_grid.rs`, `test_grid_layout.rs`, `test_grid_run.rs`, `operation_settings.rs`,
`lib.rs.diff`). `material.rs` and the `test_grid*.rs` files are reusable after a review against the
revised ADRs (`modified` rule of ADR 0055 §2); `operation_settings.rs` needs the new shape (A1).

- [x] A1. laser-core `operation_settings.rs`: newtypes `PowerPercent` (tenths, `u16`), `Passes`
  (`u8`), reuse `FeedRate`; `SettingsValue` from plain values with damage (not a map, unknown key,
  out of range, `outline` on another kind, link without id); `resolve(value, kind, materials,
  profile_id)` -> Live / Stored(reason: missing, damaged, newer, other machine, no set) / Own /
  None / Damaged; set choice per kind (Cut for cut and cut_out, Score, Engrave, Engrave + Outline:
  hatch Engrave, outline Score else Engrave); `MaterialLink.modified`, `overrides` per role
  (`SetOverride` with optional power, speed, passes), damage `BadOverride` (overrides without link,
  empty, unknown key, outline on another kind); Live and Stored carry the material numbers, the run
  numbers (overrides on top, "Differs from <name>") and the overridden fields; no `stored`/`behind()`;
  `refresh(read, kind, materials, profile_id, guard)` (only when live and differing; overrides kept;
  guard: record `modified` above the link's), `edit_number` (own: the number; linked: into or out of
  `overrides`), `reset_to_material`; `0029` reads `resolve(..).run()`; speed above max feed named,
  not damage (AC 24, 25, 26, 28; S1, S1a, S1b, S3; ADR 0054 §1 to §3). The rule takes
  `&[Material]`, so the in-memory model of `material.rs` (sets, history, limits, `modified` as
  `max(now, previous + 1)`; no file code) is built here first and Part B adds its file format.
  Tests first.
- [x] A2. laser-core `test_grid.rs`: recipe with limits (columns/rows 2 to 12, power 1 to 100 at
  0.1, speed 1 to max feed whole, cell 3 to 50, gap 0 to 20, labels: on, kind engrave|score, height,
  power, speed, passes 1 to 20), defaults per machine (criterion 1, label speed 50 % of max feed);
  per field validation texts of UX 6.3 (no clamping); cell values rounded to 0.1 % and whole mm/min;
  layout from label lane widths passed in (cells, power labels, speed labels, unit line; size with
  labels); bed fit with cause text "Grid is 260 mm wide, the bed is 200 mm."; first free spot left
  to right, top to bottom from given occupied boxes; mark kept when a new cell has the same power and
  speed, else cleared; hand-edited comparison of layout to tagged items with an explicit tolerance
  (AC 1 to 5, 7, 9, 10, 13; S10). Tests first.
- [x] A3. laser-core run rule of a grid row (`operation_kind_run.rs` / `operation_sequence.rs`):
  leaves whose nearest key is the row: labels with label settings, cells with their `grid_cell`
  numbers and the grid's kind, others not assigned; all labels before cells, cells row by row from
  the top left; a grid row runs in its kind's section (AC 5, 6, 7, 10). Tests first.
- [x] A4. document-core `operation_codec.rs`: `op_settings` as plain values, `op_grid` boolean;
  damaged settings keep the kind readable; an Unknown operation keeps its register; `op_settings`
  on a `fixture_job` is not read (AC 6, 26, 28; ADR 0054 §1). Tests first.
- [x] A5. document-core `operation_settings_commands.rs`: Use material (one or several ids, one
  commit, link with `modified`, overrides cleared, also on a re-pick, skipped ids returned), Edit
  settings (own: whole map; linked: link kept, field into `overrides`, removed when equal to the
  material number; Passes 1 when empty), Reset to material (removes `overrides`), Use own numbers
  (link and overrides dropped, run numbers kept as own); the material-edit write (refresh without
  guard into every linked operation plus root marker `x.curvyo.laser.material_edit` = `{ id,
  modified }`, in the caller's step); the save refresh (guard on, one commit with the reserved
  non-step label `refresh_material_numbers`, no history row, nothing committed when nothing
  differs, as `migrate_format` of ADR 0042 §9); grid rows refused; the `0101` assign/unassign
  commands refuse a grid row (AC 24 to 27; Frage 187 B+C, 188 B).
- [x] A6. document-core `test_grid_codec.rs` + `test_grid_commands.rs`: make (group "Test grid <N>",
  cells group, labels group, row with `op_grid` and `member.<top group>`, one commit), edit (recipe
  write plus rebuild unless hand edited), rebuild, mark and unmark, delete of the top group deletes
  its row, delete of the row keeps the items, orphan row reads "no grid", copy/duplicate/paste of a
  top group drops the mark and adds its own row in the same commit (ADR 0046 §4 copy path skips grid
  rows); tags inert outside a readable `test_grid` (AC 3, 4, 6, 9 to 11, 13).
- [x] A7. document-core format 43: `CURRENT_FORMAT_VERSION` 43 with its doc note, a format 44 file
  refused, format 42 files open with "No settings" and save back unchanged; goldens
  `laser_settings_v43.curvyo` (with a power override on a linked hatch, a `material_edit` marker),
  `laser_settings_damaged_v43.curvyo` (plus link without `modified`, overrides without link, empty
  overrides, `overrides.outline` on a Cut),
  `test_grid_hand_edited_v43.curvyo`, `test_grid_orphan_row_v43.curvyo`; older goldens unchanged
  (AC 28). Superseded format 42 pins in tester tests updated (Haiku, mechanical).
- [x] A8. ui-core labels `add_test_grid`, `edit_test_grid`, `rebuild_test_grid`, `mark_test_cell`,
  `unmark_test_cell`, `use_material`, `use_own_numbers`, `edit_operation_settings`,
  `rename_test_grid`, `delete_test_grid_row`, `reset_to_material`, `save_as_material`,
  `edit_material`, `rename_material`, `delete_material` with names, each in one step-scope list;
  `refresh_material_numbers` reserved with no name and no row (AC 3, 9, 13, 15, 18, 21 to 25;
  adrs.md §7).
- Gate A: fmt, clippy, nextest for laser-core, document-core, ui-core and editor-wasm binaries that
  pin the format; wasm32 build of the core crates.
- Built (2026-11-11), where it differs from the tasks above:
  - A1: the value codec is `operation_settings.rs`, the rule (`resolve`, `refresh`, `edit_number`,
    `reset_to_material`, `use_own_numbers`, `use_material`) `operation_settings_rule.rs` (size).
    `Material.modified` and history `date` are `u64` like `MaterialLink.modified`; the rule is
    `next_modified`. `FeedRate` (all bounded integers) gains `Hash`.
  - A2, A3: the recipe is `test_grid.rs`, the layout `test_grid_layout.rs`, the grid row's run rule
    `test_grid_run.rs` (not in `operation_kind_run.rs`); the row's section follows its `op_kind`
    with no new code. `next_test_grid_name` joins `operation_naming.rs`.
  - A5: document-core writes plain values only (ADR 0054 §2): `write_operation_settings` (one step
    for one or several operations, `SettingsStep` names the label), `commit_material_edit` (root
    marker plus refreshed values, `MaterialStep`), `refresh_material_numbers` (reserved commit via
    the generalised `commit_reserved`). Which operations are skipped, the link, the overrides and the
    guard are laser-core's; the session (C1, C2) joins the two.
  - A6: a grid row whose key path dies in a local step is deleted by the member-key sweep, so
    deleting, cutting or ungrouping the top group takes the row in the same step. An orphan row
    left by a merge reads "no grid" until the next local step, which then removes it like a dead
    key (ADR 0054 §5 keeps it renamable; for the architect). Rename and delete of a grid row use
    `rename_test_grid` and `delete_test_grid_row`. Rebuild repairs linked clones of replaced items.
  - A7: the laser extension's reading of the four goldens is
    `curvyo-laser-core/tests/settings_goldens.rs`. The SVG export documents of `curvyo-svg-core`
    pin format 43.
  - A8: all fifteen labels are in `FREE_STEPS`.

## Part B: material records and library I/O (Opus, medium; host glue B5 Sonnet)

- [x] B1. laser-core `record_envelope.rs`: id and file-name check, `format_version` gate, size
  limit, `new_record_id` (renamed from ADR 0038 §5); `machine_profile_file.rs` moves onto it, its
  goldens unchanged (refactor commit, behaviour kept).
- [x] B2. laser-core `material_file.rs` (on A1's `material.rs`): record
  format 1, canonical writer (key order, two-space indent, trailing newline, read-write-read
  identity), `deny_unknown_fields`, every damage of ADR 0055 §3 with one reason text, newer format
  skipped; `material-order.json`, `test-grid-choice.json` (100 machines, damaged reads empty)
  (AC 16 to 18, 22, 29; S6, S9, S10). Tests first, goldens in
  `curvyo-laser-core/tests/fixtures/materials/`: `full.json`, `generic_laser.json`, `no_sets.json`,
  `history_max.json`, damaged `unknown_key.json`, `power_0_5.json`, `power_100_5.json`,
  `speed_0.json`, `passes_21.json`, `thickness_0.json`, `thickness_100_1.json`, `name_61.json`,
  `source_no_entry.json`, `history_501.json`, `id_mismatch.json`, `newer_format.json`; `order.json`,
  `order_unknown_id.json`, `test_grid_choice.json`.
- [x] B3. laser-core `materials.rs`: load from `(name, text)` pairs (1,000 records, file-name order,
  one notice per record), filter by profile id, save a test (new record or add/replace a kind set,
  history `tested`), edit a set (`edited`), remove a set (`removed`), rename, thickness, delete,
  order, `revert` (write an earlier text back only if the current text is the expected one; undo
  drops the entry the step added, redo adds it with the same id, Frage 190 A; new `modified`);
  each returns `Write { id, text }` / `Remove { id }`; refused past 500 entries or 256 KiB
  (AC 15 to 23, 29; ADR 0055 §2, §9). Tests first.
- [x] B4. `docs/technical-debt.md`: `curvyo-library-core` deferred again (Haiku).
- [x] B5. `curvyo-app/src/materials.rs`: `list_materials`, `write_material`, `remove_material`,
  read/write of `material-order.json` (data dir) and `test-grid-choice.json` (config dir);
  `frontend/src/platform/host.ts`: `curvyo.materials/<id>`, `curvyo.material-order`,
  `curvyo.test-grid-choice` (AC 17, 31: no network).
- Gate B: fmt, clippy, nextest laser-core and curvyo-app, wasm32 build of laser-core.
- Built (2026-11-11), where it differs from the tasks above:
  - B1: `LibraryWrite` moves into `record_envelope` too; `new_profile_id` and `is_profile_id` stay
    as re-exported names of `new_record_id` and `is_record_id`, so their callers are unchanged.
  - B2: the record is `material_file.rs`, the order and grid-choice files `material_preferences.rs`
    (size). Each damage has one reason text ("Power 0.5 is outside 1 to 100.", "Unknown key
    colour.", ...). A 101st remembered machine drops the one with the lowest id (Proposal: the file
    keeps no dates).
  - B3: the list is `materials.rs`, the record edits `material_edits.rs` (size). A damaged or newer
    record shows on the page of the machine its JSON names, or on every page when it names none
    (Proposal). `revert` re-stamps `modified` and returns the text it wrote, which the journal
    expects at the next undo or redo; a damaged earlier text goes back verbatim. `edit_set` refuses
    a kind without a set (a set is added only by Save as material). Load notices read
    "<file name>: <reason>". The 256 KiB refusal has no test (a valid 500-entry record stays below).
  - B4: the debt entry already carried the 2026-11-11 update; nothing changed.
  - B5: `curvyo-storage-io` has `remove_file`. The commands are `list_materials`, `write_material`,
    `remove_material`, `read_material_order`, `write_material_order`, `read_test_grid_choice`,
    `write_test_grid_choice`. `frontend/src/lib/materialsFile.ts` (read all, perform one write) comes
    now, as `acceptance_0051_tester` requires the page to invoke every registered command; C1
    wires it into the session.
  - Part A's `operation_settings_rule.rs` doc comment wraps `<name>` in backticks (rustdoc).

## Part C: session, view JSON, wasm bridges, budgets (Opus, medium)

- [x] C1. `session/materials/`: load at start beside `MachineProfiles`; each library edit of B3
  (Save as material, set edit/removal, thickness, rename, delete; not the order) is one document
  step of the open project: A5's material-edit write (root marker, refresh of every linked
  operation) plus the host write list; `undo.rs` keeps `{ id, before, after }` record texts per step
  in memory (at most 16 MiB, oldest record parts dropped first: "<name> can no longer be undone.");
  Ctrl+Z/Redo do the document part by ADR 0014, then `materials::revert`, else the notice "<name>
  changed since, the material was not undone."; a failed host write leaves the record with a
  notice; the journal ends with New, Open, quit; History-list actions do not touch records; the save
  path runs A5's `refresh_material_numbers` commit before the file text is taken; use counts from
  the open project (AC 15 to 23, 21 count; S8, S8a; ADR 0055 §9 (a) to (f)).
- [x] C2. `session/operations/settings.rs`: the settings line per selected operation through A1's
  rule (live numbers change in the same frame as a material edit; "Differs from <name>" with the
  material number beside each override), Use material, Edit settings (override on a linked row),
  Reset to material, Use own numbers, several selected rows with the skip notice of at most 70 characters (AC 24 to 27).
- [x] C3. `session/test_grid/`: Add test grid with remembered or default values, place or refuse
  ("Test grid is ... wide, the bed is ...", "No free spot on the bed for a ... grid."), selected, its
  row selected; field edits with A2 validation; rebuild; mark and unmark; label widths measured once
  per distinct text with `curvyo-text-core`; canvas selection of the grid selects its row and asks
  the Operations tab (M3); grid selection shows box and centre move only (no resize, rotate, skew);
  cell selection highlights its table cell; remembered values written to `test-grid-choice.json`
  (AC 1 to 14).
- [x] C4. render-core: cells and labels in their kind look, one chip row per grid box, the Worked
  chip (20px, never output or exported), Show/Output off looks (AC 12; UX §11).
- [x] C5. View JSON with one serde producer each: `materials_view`, `test_grid_view`,
  `operations_view.settings`, the host write list; shared fixtures
  `curvyo-editor-wasm/tests/fixtures/materials_view/`, `test_grid_view/`,
  `operations_view/settings_*.json`, `material_writes/` with the cases of adrs.md §6 (empty
  library, damaged, newer, live, stored per reason, own, no settings, damaged settings, grid with
  and without mark, hand edited, does not fit, live link with overrides, stored per reason with
  overrides); contract test
  `tests/material_test_library_view_contract.rs` builds each named scenario and compares the JSON
  value to the file.
- [x] C6. `wasm_materials.rs`, `wasm_test_grid.rs`, `wasm_operations.rs` additions: strings and
  scalars only (ADR 0001 §5).
- [x] C7. Budgets in thread CPU time with `tests/cpu_time/`, `tests/material_test_library_budget.rs`:
  make a 12 x 12 grid with 25 label texts <= 50 ms, one field edit (rebuild) <= 30 ms, Materials
  view of 200 records <= 50 ms, load of 200 records with history <= 200 ms; added to the CI
  `performance-budgets` job (AC 30).
- Gate C: fmt, clippy, nextest editor-wasm (lib and the new binaries), render-core; wasm32 build.
- Built (2026-11-11), where it differs from the tasks above:
  - Session state: one field per feature module, `materials` (`session/materials/`: library, journal,
    outbox, selected row, notice) and `test_grids` (`session/test_grid/`: remembered values, pressed
    row, kept invalid value, add refusal, label width cache, the canvas-selection edge for M3).
    `useEditorSession.ts` does not grow: the host wiring is `lib/materialsFile.ts` (read once, apply
    to every session, flush after workbench commands and after undo/redo in `useHistory.ts`) and
    `lib/materialWrites.ts` (the write list decoder, tested against `material_writes/`).
  - C1: the material clock is the history clock in whole seconds times 1000 (`next_modified` keeps
    `modified` rising); hosts pass random bytes only. A library step records its journal entry only
    when the stack grew; a delete stamps the marker past the record's `modified`, so the step always
    writes. Undo notices follow UX notes §13 ("<name> changed since; the material was not undone.",
    "Only the project was undone; <name> is too old.", names cut at 24 characters). Not built: the
    History-list notice "Only the project was restored; materials stay as they are." (History-list
    actions simply do not touch records) and the undo write-failure notice text, which Part D shows
    from `flushMaterialWrites`' returned reason. "This machine is not in your profiles." refuses Save
    as material on a page whose copy names no profile id (Proposal).
  - C2: the settings line is per row (`rows[].settings`, `null` for grid rows and unknown kinds) with
    the needed set kind and the disabled pick entries; `operations_view.pick` lists this machine's
    readable materials; `rows[].grid` carries the grid row's group and size. Multi-select is the
    host's: `pick_operation_material` takes a JSON list of ids. No settings keeps typed numbers in a
    session draft until power and speed are both there (outline numbers of a No settings Engrave +
    Outline are not drafted; the first commit fills the outline from the main set). Skip notice for
    several kinds: "N operations skipped: no fitting set." (Proposal).
  - C3: document-core gains `edit_test_grid` (`TestGridEdit`: recipe, mark, optional items for a
    hand-edited grid, optional row kind and index for a Kind change) beside `rebuild_test_grid`, as
    criteria 8 to 10 need a field edit that keeps a hand-edited grid's items and a Kind change that
    moves the row in the same step. Labels are made in the included sans (`fonts.fallback`); while it
    loads, Add test grid refuses with "The label font is still loading." (Proposal). Not built: the
    canvas restriction of a grid selection to box and centre move (no resize, rotate, skew; UX 6.5)
    and the Select bar's hidden commands: the Select tool has no per-selection handle switch yet
    (for Part E with the ux-engineer).
  - C4: render-core gains `worked_chip.rs` only; the grid looks are session code
    (`operations/grid_look.rs`) over the existing kind looks and chip row.
  - C5: fixtures `materials_view/{empty,damaged_newer_and_used,selected_fields}.json`,
    `test_grid_view/{default_unmarked,marked,does_not_fit,invalid_value}.json`,
    `operations_view/settings_{live,stored,own_none_damaged}.json` (the `pick` and per-row
    `settings`/`grid` part; the five `0101` fixtures were regenerated with the new keys),
    `material_writes/{save_as_material,undo_save_as_material}.json`. A hand-edited grid is pinned by
    the session tests, not by a fixture. Superseded `0101` pins updated (machine-setting keys allowed
    since `0028`): `acceptance_0101_v2_tester.rs` and `.test.ts`, and `operationsView.test.ts` skips
    the `settings_*` files (Part D decodes them).
  - C7: release times on the dev machine: make 12 x 12 3.5 ms, field edit 6.2 ms, Materials view of
    200 records 0.1 ms, load of 200 records with 100 history entries each 16.6 ms.
  - Part A deviation 2 stays: an orphan grid row left by a merge is still removed by the next local
    step's member-key sweep. Keeping it renamable (ADR 0054 §5) needs the sweep to tell a key that
    died in this step from one a merge left dead, which every import path would have to record; not
    cheap, for the architect.
  - `curvyo-document-core`'s `add_test_grid` doc comment wraps `<N>` in backticks (rustdoc).

## Part D: frontend Operations settings and Materials (Sonnet, medium)

- [x] D1. Decoders `lib/materialsView.ts`, `lib/operationsView.ts` (settings) checking every field;
  tests read the Rust fixtures and reject a missing field and a wrong type
  (`tests/materialsView.test.ts`, `tests/operationsView.test.ts`).
- [x] D2. `OperationSettingsBlock.tsx` + `MaterialPick.tsx` (UX §8): head, inline pick (no popup,
  disabled "No <kind> set", Filter above 8), number fields with chips (editable also when linked),
  the states live, stored with reason, "Differs from <name>" with the material number beside each
  typed one and "Reset to material", own, "No settings"; "Edit material"; multi-select of rows
  (Ctrl, Shift, Shift+arrows) with "Mixed" and the skip notice (AC 24 to 27).
- [x] D3. `MaterialsSection.tsx`, `MaterialRow.tsx`, `MaterialFields.tsx`, `lib/materialRows.ts`,
  `hooks/useMaterials.ts` (UX §10): collapsed "Materials (N)", rows with thickness and symbols,
  rename by double-click/F2, armed trash with the use count, arrows, drag, Alt+arrows, set fields
  with source line and set trash, damaged and newer rows, window above 60 rows (AC 19 to 23, 29).
  Tests `tests/materialRows.test.ts`.
- Part D done (Sonnet). Deviations and notes:
  - Tests are `node:test` over pure `lib/*.ts` (the repo has no vitest or DOM test setup); components stay
    thin and are checked in Part E's Chromium run. Decoders live in `lib/operationSettingsView.ts` (settings
    and pick, called by `operationsView.ts`) and `lib/materialsView.ts`; text and rules in
    `lib/operationSettingsRows.ts`, `lib/materialRows.ts`, `lib/rowSelection.ts`, `lib/settingsFieldText.ts`.
  - Hooks: `useOperationSettings` (pick, edit, reset, 5 s skip notice), `useMaterials` (commands, records handed
    to the host after each, "Edit material" reveal), `useScrollWindow`, `useMaterialDrag`; `useEditorSession`
    and `useOperations` did not grow. `flushMaterialWrites` now returns `{reason, name} | null`; `useHistory`
    shows "Could not undo/redo <name>: <reason>." (alert, 8 s). The "changed since" and "too old" notices
    already come from the session through the history notice.
  - Number fields reuse `EntryField` (commit on Enter or leaving, chip text from the session as message).
    Up/Down stepping (UX 12) is not built (`stepOf` exists, unused); Part E or the ux-engineer decides.
  - Fields height under a selected material is fixed (`fieldsHeight`) so the window is exact; the real
    height is for the Chromium check. Pick list and Materials list window above 60 rows.
  - Not built: the "Speed above the machine" line is shown from `over_feed` as a state line (done); the
    History-list notice "Only the project was restored" is not wired (needs a host signal, for Part E).
- Gate D: fmt, clippy, nextest of touched crates, `npm run build:wasm && npx tsc -b --noEmit &&
  npm test && npx oxlint`.

## Part E: grid UI, Worked, Save as material (Sonnet, medium)

- [x] E1. Add group's sixth button "Add test grid" with refusals under it; Test grid row (kind
  glyph, line 2 "<Kind> test grid · C × R cells", no digit, no tick, section by kind, rename, armed
  trash text "Its 25 cells and labels become Not assigned.") (AC 1, 3, 6).
- [x] E2. `lib/testGridView.ts` (decoder, tests read the Rust fixtures), `lib/testGridFields.ts`,
  `hooks/useTestGrid.ts`, `TestGridBlock.tsx` (UX 6.2: head with Select, machine and size, Kind
  radiogroup, Power, Speed, Cell, Labels switch and fields, invalid values kept with chips, "Edited
  by hand" with Rebuild) (AC 1, 2, 7 to 10).
- [x] E3. `TestGridResult.tsx` (UX §7: `role="grid"`, roving focus, mark/unmark, canvas highlight,
  Worked line) and `SaveAsMaterialForm.tsx` (UX §9: Add to, Name, Thickness, Passes, Replace with
  sentence and 400 ms guard, validation, write failure notice, draft rules) (AC 12 to 15).
- [ ] E4. `specification.md` gets `Status: Done (PR #n)` with the PR; design-system rows of UX §17
  (ux-engineer at review); `docs/adr/0046` status note (architect).
- Part E built (Sonnet, 2026-11-11); E4 stays for the PR (spec status, UX §17 rows, ADR 0046 note).
  Deviations and notes:
  - Add test grid is the sixth button of the add group (8px gap); the refusal shows under the group, 8 s. The Test
    grid row has no digit and no tick; its trash text names `row.items`. Rows now show `--value-fill` while selected
    (it had no ground). The list selection follows the canvas: a grid selected on the canvas selects its row.
  - Grid block: `TestGridBlock.tsx` (fields), `TestGridResult.tsx` (table, Worked line), `SaveAsMaterialForm.tsx`,
    `SegmentGroup.tsx` (Kind, Add to), hook `useTestGrid` (called in `PropertiesPanel`; `useEditorSession` did not
    grow), decoders `lib/testGridView.ts` (tests read the Rust fixtures), `testGridFields.ts`, `testGridResult.ts`,
    `saveDraft.ts`. Label fields are Power, Speed, Passes only (UX 6.2 lists no label kind or height field). A value
    the grid did not take stays in the field's own text with its chip until Escape (the view's `invalid` is decoded
    but not used to refill a remounted field).
  - Existing list of the form is a plain scrolling radiogroup (max 140 px), not the windowed pick; picking selects the
    material in the Materials section to read the set it would replace ("Replaces the Cut set (...)"). The Replace
    guard is `aria-disabled` for 400 ms. The draft survives a tab change and is dropped by Cancel, Escape, a save, a
    mark change, Rebuild, Kind change and another grid.
  - Up and Down (Shift 10) step the typed text and commit on key-up in `EntryField` (`stepText`), also in the Part D
    settings fields.
  - Move-only canvas handling: `TransformHandleTolerances.move_only` (new public field of `curvyo-ui-core`, for the
    architect): `group_handles` and `drawn_edit_handles` keep only the centre move when set, so draw, hover, cursor and
    press all follow. The session sets it while the selection is exactly one grid's cells and labels
    (`whole_test_grid_selected`); S, R and K are consumed for it (M still opens the Move chip). One cell or part of the
    grid, entered by double-click, keeps its handles. Not built: the Select bar's hidden Flip, Guide, Object to path and
    Linked clone for a grid selection.
  - Part C fix found in Chromium: Add test grid and the Grid block's Select set the selection to the top group's id,
    so no box was drawn; both now select the grid's leaves like every group selection.
  - Still not built: the History-list notice "Only the project was restored" (needs a host signal).
  - `fieldsHeight` measured in Chromium (1, 2, 3 sets, with and without a use line): exact (136, 240, 344, 364 px,
    no overflow). Demo path 1 to 8 ran in Chromium (screenshots in the scratchpad `0028/`); step 4 uses 60 %,
    1950 mm/min (the default grid has no 3000 row). "Edited by hand" and Rebuild also ran there.
- UX review fixes (2026-11-12, Opus), tests in `session/test_grid/tests.rs` and `tests/testGridView.test.ts`:
  - Criterion 6 (Proposal, one sentence added to the spec): grid items are never operation targets.
    A kind button with a grid selected adds the operation empty; a tick, a drag onto a row or a digit
    with grid items refuses them with "Test grid items run with their grid row only." (45 chars);
    the other items of a mixed selection are assigned. Clone members of a cloned grid are not
    filtered (no route reaches them from the grid UI).
  - Criterion 9: a field edit, a Kind change and Rebuild re-select the new leaves when the selection
    was in that grid (it held dead ids before, so the next Add operation failed silently). A kind
    button the document refuses shows "The operation could not be added."; a selection of only
    dimensions still adds nothing without a notice (`0101` criterion 10, accepted behaviour).
  - UX §5, §12: digits count the shown rows without grid rows, in the session's number keys and
    in the row's line 2 (`rowDigit`); before, the keys used the stored order and counted grid rows.
  - UX §6.5: Linked clone and Object to path (already, as for any group) are hidden for a whole
    grid in Rust; Flip and Guide by `select_bar_hides_grid_commands` in `SelectToolbar.tsx`. This
    differs from `0056` D7 (Flip in every selection state, so the row does not move): the row moves
    for a grid selection. H and V still flip a grid (UX notes name only S, R and K); for the ux-engineer.
  - Found while testing: `align_test_support::draw_rect` returns `objects().last()`, which after a
    test grid is a grid label, not the rectangle; the new tests create their circle directly.
- Gate E: the full gate of `CLAUDE.md` §7; Chromium check of the demo path (one agent drives the
  browser).

## Validation

- Tests first for A1 to A3, B2, B3; goldens read, write back equal and damage per case; shared
  Rust/TS fixtures pin the producer and the decoder; budgets in thread CPU time only.
- Tester writes acceptance tests from the spec before reading this code (`acceptance_0028_*`).
- AC 31: no network, device, G-code or output code is added; the tester checks the diff.

## Demo path (desktop and browser)

1. New project, laser page with Generic laser; Operations tab shows six add buttons.
2. Add test grid: a 5 x 5 Cut grid with labels at the bed's top left, row "Test grid 1" in Cutting,
   Grid block open. Columns to 6: rebuilt in place, one undo step; Columns 13: chip, grid unchanged.
3. Press the cell 60 %, 3000 mm/min in the result table: tick chip on the canvas, "Worked: ...".
4. Save as material: "Birch 3 mm", 3.0 mm: Materials (1) with the Cut symbol.
5. Add Cut, select it, pick Birch 3 mm: live line "60 %, 3000 mm/min, 1 ×". Edit the Cut set in the
   Materials fields to 65 %: the line changes at once (one step "Edit material"); Ctrl+Z puts the
   record and the line back to 60 %, Redo to 65 %.
6. Type 70 on the operation: link kept, "Differs from Birch 3 mm" with 65 % beside 70; Reset to
   material: 65 % again; type 70 and pick Birch again: overrides cleared. "Own numbers" in the pick:
   link gone, the numbers kept.
7. Pick Birch, save, reopen: grid, mark, row, link and stored numbers as before. Delete Birch
   (armed trash, step "Delete material"): the operation shows "Material not found, using stored
   numbers"; Ctrl+Z brings Birch back. Delete again, save: the file saves with the stored numbers.
8. Reopen the project in a new session: Ctrl+Z no longer reaches the library (limits of ADR 0055
   §9); the History list restores only the project.
