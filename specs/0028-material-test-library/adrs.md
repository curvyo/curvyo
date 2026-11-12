# 0028 material test library: architecture notes

Status: Proposed (architect, 2026-11-11), revised 2026-11-11 after the changed answers. Built on the customer's answers of 2026-11-11: Frage 183 A (one folded Test grid row), 184 A (per-user `materials/<uuid>.json` tied to a profile id), 185 B (an operation points to its material by id, numbers stored beside it), 186 B (label settings configurable, as in LightBurn), 187 B+C (stored numbers refresh on every material edit and every save), 188 B (a typed number keeps the link), 189 B (material edits are undo steps). Frage 190 has a default (§10). Decisions marked **(Proposal)** are the architect's and vetoable at the demo. The file shapes are [ADR 0054](../../docs/adr/0054-laser-settings-and-test-grids-in-the-project-file.md) (project file, format 43) and [ADR 0055](../../docs/adr/0055-material-records.md) (material record format 1), both `needs-customer`, accepted with the story PR.

Depends on and extends:
- [ADR 0046](../../docs/adr/0046-laser-operations-record.md): the operation node gains `op_settings` and `op_grid`; §4 (copy) gets the grid case; the nearest rule and the delete sweep are reused unchanged.
- [ADR 0038](../../docs/adr/0038-machine-profile-records.md): reading rules, host commands and the session's library pattern, applied to materials; its codec moves onto a shared envelope module.
- [ADR 0039](../../docs/adr/0039-machine-profile-copy-in-the-project-file.md): the profile copy's id is the machine tie of a material; max power and max feed are the bounds of the numbers.
- [ADR 0004](../../docs/adr/0004-persistence-and-cross-machine-sync.md) §6, §7: one file per record, test history append-only, folder sync.
- [ADR 0030](../../docs/adr/0030-stored-tiling-nodes.md): atomic recipe on a group, generated children tagged per cell (the grid follows it).
- [ADR 0033](../../docs/adr/0033-manufacturing-data-in-the-project-file.md) §3, §4, §7: `x.<process id>.*` keys on shared nodes, never repaired on read, the bump rule.
- [ADR 0052](../../docs/adr/0052-laser-fixture-jobs-in-the-project-file.md): fixture jobs keep no settings (spec, Where settings live 4); `op_settings` on a `fixture_job` node is not read.

## 1. The grid row (Frage 183 A)

The row is an ADR 0046 operation node with `op_kind` = the grid's kind (`cut`, `score`, `engrave`), `op_grid` = `true`, and one key `member.<top group id>`. So Show, Output, rename, armed trash, arrows, drag, the Cutting or Marking section, Cuts last and undo are the `0101` code paths. The run rule per leaf (label, cell, other) is in ADR 0054 §4.

Code points the implementer must not miss:
- The `0101` assign and unassign commands refuse a grid row (its one key is written only by the grid commands); Use material and Edit settings refuse it too.
- The ADR 0046 §4 copy rule skips grid rows; the copy of a top group gets a new row in the same batch (ADR 0054 §5).
- Deleting a top group deletes its row in the same batch; a row whose group a merge removed reads "no grid" until the next local step's member-key sweep deletes it (ADR 0054 §5, amended 2026-11-12).
- `read_operation` (`operation_codec.rs`) reads `op_settings` as plain values and `op_grid` as a boolean; a damaged `op_settings` must **not** make the kind unreadable (Damaged settings is not Unknown operation).

## 2. Settings in the document model (Frage 185 B)

One atomic map `op_settings` per operation: `main`, `outline` (Engrave + Outline only), `material` `{ id, name, outline_from, modified }` and `overrides` (typed numbers per field, only with a link) (ADR 0054 §1). Atomic so a merge never mixes two sets or keeps an override over a material it was not typed for. Which numbers run is one pure rule (`curvyo-laser-core/src/operation_settings.rs`): the material numbers are live from the library, else the stored ones (reason: missing, damaged, newer, other machine, no set); the overrides go on top ("Differs from <name>"); else own, else none or damaged. Writes: Use material (clears overrides), Edit settings (own: the number; linked: an override, Frage 188 B), Reset to material, Use own numbers, a material edit in this project (refresh inside its step, Frage 187 B), save (refresh as the reserved non-step commit `refresh_material_numbers`, only where the record's `modified` is newer, 187 C), and the `0029` output commit. Open, library loads and remote imports never write it. The "stored: 60 %" state goes.

Grid data uses `x.curvyo.laser.*` keys because the group and its items are ordinary drawing nodes: a build with the laser extension disabled keeps them as plain groups and items (ADR 0033 §3). Cells keep their own numbers (`grid_cell`), so a moved cell runs with what it was made with (criterion 10). "Edited by hand" is derived by comparing the pure layout to the tagged items (Proposal; a stored flag is wrong after undo or merge).

## 3. `format_version` 43

New data on existing nodes, but a build of 42 would treat a grid row as an ordinary operation and silently break the grid on copy and delete (ADR 0054 §6). Bump 42 to 43, no migration, four goldens in `curvyo-document-core/tests/fixtures/`. `0110` took 42 on `main`; 43 is the next free number (rebased 2026-11-12); the number is taken at merge (ADR 0033 §7). `CURRENT_FORMAT_VERSION` gets a format 43 note in its doc block. The material record is a separate file with its own `format_version` 1 and needs no project bump.

## 4. Crate placement: `curvyo-laser-core`, no new crate

New modules (each under 500 lines, one sentence each): `record_envelope`, `material`, `material_file`, `materials` (ADR 0055 §7), `operation_settings` (ADR 0054 §2), `test_grid` ("the test grid recipe, its limits and its layout": cell numbers rounded to 0.1 % and whole mm/min, positions, size, the bed fit check with its cause text, the hand-edited comparison). `curvyo-library-core` is deferred again: two laser-only records and no merge are not a second user of a crate (`CLAUDE.md` §5); `docs/technical-debt.md` is updated. `curvyo-document-core` gains the register constants, the plain-value reads and the write commands; it interprets nothing.

## 5. Session and wasm boundary; core stays I/O-free

- **Core** takes texts, random bytes, the time and the label lane widths from the caller and returns writes. No file, clock, `localStorage` or text shaping in `curvyo-laser-core`.
- **Session** (`curvyo-editor-wasm/src/session/`): `materials/` (library state beside `MachineProfiles`; commands, each one document step; `undo.rs`, the record journal; `view_json.rs`), `test_grid/` (make, edit, rebuild, mark, cell hit on the canvas; `view_json.rs`), and `operations/settings.rs` (the settings line, Use material, Edit settings, Reset to material); the save path runs the refresh commit before the file text is taken. The session measures the label widths with `curvyo-text-core` and passes them into the layout. Grid writes are one document batch each.
- **wasm** (`wasm_materials.rs`, `wasm_test_grid.rs`): strings and scalars (ADR 0001 §5); library edits return the host writes as one JSON list.
- **Host** (`curvyo-app/src/materials.rs`, `frontend/src/platform/host.ts`): moves text, decides nothing. The frontend never parses a record file.

## 6. JSON view contracts (architecture review 2026-11-06, finding 3)

Every JSON that crosses wasm to TypeScript in this slice has one Rust producer (`view_json.rs` serde types) and one TypeScript decoder that checks each field (no `as` cast on unchecked data), following `fixtureJobsView.ts`:
- `materials_view` (rows, damaged rows with reason, the selected material's fields), decoder `frontend/src/lib/materialsView.ts`;
- `test_grid_view` (fields, ranges, fit cause, hand edited, result table, Worked line), `frontend/src/lib/testGridView.ts`;
- `operations_view` gains `settings` (state, numbers per role, reason, material numbers and overridden fields, "Reset to material" when overridden, pick options with disabled reasons), extended in `frontend/src/lib/operationsView.ts`;
- the host write list (`[{ "op": "write", "id", "text" } | { "op": "remove", "id" }]`).
Shared fixtures in `curvyo-editor-wasm/tests/fixtures/materials_view/`, `test_grid_view/`, `operations_view/settings_*.json`, `material_writes/`. The **Rust** test builds each fixture's named session scenario and compares the produced JSON value to the file (the producer is pinned, not only parsed). The **TypeScript** test decodes every fixture and rejects a copy with a missing field and one with a wrong type. Cases at least: empty library, a damaged record, a newer record, live link, live link with overrides, stored link per reason with overrides, own numbers, no settings, damaged settings, a grid with and without mark, a hand-edited grid, a grid that does not fit.

## 7. Undo labels and material undo (Frage 189 B)

Document steps (constants in `curvyo-document-core`, names in `curvyo-ui-core/src/history_names.rs` as `0103` §7): `add_test_grid` "Add test grid", `edit_test_grid` "Edit test grid", `rebuild_test_grid` "Rebuild test grid", `mark_test_cell` "Mark cell", `unmark_test_cell` "Unmark cell", `use_material` "Use material", `edit_operation_settings` "Edit settings", and (Proposal) `use_own_numbers` "Use own numbers", `rename_test_grid` "Rename test grid", `delete_test_grid_row` "Delete test grid row", `reset_to_material` "Reset to material", and the library steps `save_as_material` "Save as material", `edit_material` "Edit material" (set edit, set removal, thickness), `rename_material` "Rename material", `delete_material` "Delete material". Reserved, no step and no row: `refresh_material_numbers` (save). The material order and the remembered grid values are not steps (Proposal).

How a library step works (ADR 0055 §9): it is a document step of the open project (root marker `x.curvyo.laser.material_edit` plus the refresh of every linked operation), and the session keeps `{ id, before, after }` record texts per step in memory (`session/materials/undo.rs`, at most 16 MiB). Undo and redo do the document part by ADR 0014, then rewrite the record only if it is still what the step left (`materials::revert` in core, pure); else a notice. Limits: only Ctrl+Z and Redo of the session in which the edit was made; a later session, another project and the History-list actions do not reach the record; an unloaded change on disk is overwritten as by any library write.

## 8. Budgets (criterion 30, thread CPU time)

Confirmed: a 12 x 12 grid made within 50 ms and a field edit (rebuild) within 30 ms, both including the 25 label texts (each distinct text shaped once per build); the Materials list of 200 records within 50 ms. Added: loading 200 records with history within 200 ms (ADR 0038's load budget). Tests under `curvyo-editor-wasm/tests/cpu_time/`.

## 9. Spec changes this build needs (for the product owner)

- S1. Criterion 24 (188 B): a typed number on a linked operation keeps the link and is stored as an override; the line shows "Differs from <name>" with the material's number beside each typed one, and "Reset to material"; a number typed equal to the material's removes that override; the pick's "Own numbers" ends the link and keeps the numbers that ran. Linked numbers become editable fields (no longer read-only text).
- S1a. Criterion 25 and 27: a pick (also of the linked material again) clears the overrides.
- S1b. Criterion 26 (187 B+C): the "stored: 60 %" note goes; (b) applies the overrides over the stored numbers; the last sentence becomes "nothing in the project is written until a material edit, a save, a pick or an output".
- S2. Where settings live 3: `0029` reads the rule of ADR 0054 §2, overrides included, and its output commit refreshes the stored numbers (not the overrides).
- S3. Power is 1 to 100 with at most one decimal; grid values round to 0.1 % and whole mm/min.
- S4. Criterion 6: deleting the grid deletes its row; deleting the row keeps the items, which then run nothing; items cannot be assigned to a grid row; no settings line on a grid row.
- S5. Criterion 7 (186 B): the label fields are on or off, kind (Engrave or Score, never Cut), height, power, speed, passes; the spec lists the final set before the build.
- S6. Criterion 16: a set's source is a reference to its history entry, which holds kind, numbers, grid and date.
- S7. Criteria 19 and 22: ordered materials first, the others by name.
- S8. Criteria 15, 18, 21, 22 (189 B): Save as material, a set edit or removal, rename, thickness and delete are undo steps of the open project that also refresh every pointing operation; Ctrl+Z in the same session also takes back the delete; the order is not a step. Undo of a step removes the history entry it added (Frage 190 A).
- S8a. New criterion (189 B limits): a material edit is undone only by Ctrl+Z of the session in which it was made; if the record changed since, the record is left and a notice names it; a later session, another project and the History-list actions do not reach the record.
- S9. Criterion 29 also damages: a source naming no entry, more than 500 entries, text over 256 KiB.
- S10. Criterion 2: remembered values are per computer (config directory), not synced.

## 10. Questions for the customer

Answered 2026-11-11: **187 B+C** (refresh at each material edit in its step and at each save, plus pick and output), **188 B** (typed number keeps the link as an override), **189 B** (material edits are undo steps).

**Frage 190 (MAT-UNDO-HISTORY): Does Ctrl+Z of a material edit also remove the history entry the edit added to the record?**
- A: yes; after Ctrl+Z the record is as before the edit (an undone "Save as material" also takes its test entry back); Redo adds the entry again.
- B: no; the entry stays and an "Undone" entry is added; the history shows every edit, also undone ones, and fills its 500 entries faster.
- Recommendation: A (Ctrl+Z means "as if not done"; the grid and its mark are still in the project, so the test can be saved again). Default if unanswered: A.
