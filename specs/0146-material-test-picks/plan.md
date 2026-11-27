# Plan for material test picks (0146)

Branch `story/material-test-picks`, stacked on `fix/material-delete-test-source` (the reader fix of
ADR 0093 decision 4 is already there, not redone). Fragen 312 to 316 at default A. Build order is
core first, each slice a green commit; the branch is regrouped to 1 to 5 commits before the report.

## Affected crates/modules
- `curvyo-laser-core`: `material.rs` (`PickKind`), `material_record.rs`, `material_record_file.rs`,
  new `material_record_picks.rs`, `material_test_file.rs`, `material_record_edits.rs`,
  `operation_settings*.rs`, `test_cell_fill.rs`; fixtures `tests/fixtures/materials/`.
- `curvyo-document-core`: `CURRENT_FORMAT_VERSION` 66, `MAX_MATERIAL_TEXT_BYTES` 640 KiB, undo keys,
  goldens `material_pick_v66.curvyo`, `material_pick_damaged_v66.curvyo`.
- `curvyo-ui-core`: `history_names.rs`. `curvyo-app`: `materials.rs` limit text.
- `curvyo-editor-wasm`: `session/materials/`, `session/test_grid/items.rs`, operation settings block,
  `tests/cpu_time/`, `tests/fixtures/test_grid_items/engrave_3x3_10mm.json`.
- `frontend`: pick bar, pick pick, folded pick lines, canvas overlay, decoders, `index.css` tokens.

## Tasks
- [x] 1. `PickKind`, record format 5: marks on tests, `picks` per laser, reader/writer, damage rules,
  limit 640 KiB, golden fixtures (AC 10, 11, 23)
- [x] 2. `judge(kind)`, `unmark`, `Material::pick_set`, `put_test` drops outside marks (AC 2, 3, 5, 10)
- [x] 3. Operation link `pick`, `material_run` by pick, Engrave + Outline per pick (AC 13, 14, 15, 16)
- [x] 4. Project format 66, goldens, undo keys and words (AC 17)
- [x] 5. Round Engrave cell via `cell_shape` (ellipse primitive), tolerance tests, golden items (AC 18 to 22)
- [x] 6. Wasm session: mark/unmark pick, Use pick, views with picks and availability, multi-row skip notice
  (AC 2, 3, 9, 13 to 16)
- [x] 7. Budgets in `tests/cpu_time/` (AC 24)
- [x] 8. Frontend: pick bar, table frames, pick lines, materials pick lines, pick pick, canvas overlay,
  tokens (AC 1, 4, 6 to 9, 13, 15)
- [ ] 9. Docs: design-system rows final, ADR 0093 notes on 0054/0057/0058, spec Status at PR time
  (PR number placeholder for the lead)

## Validation
Golden files in `curvyo-laser-core/tests/fixtures/materials/` and `curvyo-document-core/tests/fixtures/`,
tolerance tests for the circle, CPU-time budgets, decoder tests and a contrast test in the frontend,
full gate of CLAUDE.md section 7.

## Progress notes
- Built as one core commit (record format 5, project format 66, wasm ops, round cell) and one frontend
  commit; regrouped before the PR. Fixtures: `curvyo-laser-core/tests/fixtures/materials/format5_*.json`,
  `format4_with_*.json`, `curvyo-document-core/tests/fixtures/material_pick_*_v66.curvyo`,
  `curvyo-editor-wasm/tests/fixtures/test_grid_items/engrave_3x3_10mm.json`,
  `materials_view/test_with_picks.json`.
- Decisions of this build (Proposal): `use_material` always writes Best (a pick is chosen with Use pick);
  unmarking Best turns its set Typed through the normal `set` history entry; a laser entry that loses its
  last pick set stays listed (as an entry without sets does today); the Engrave + Outline availability of a
  pick needs the pick's main (Engrave) set; the undo steps are `judge_test` ("Mark pick"), `unmark_pick`,
  `use_pick`.
- Not built, listed for the lead: (1) the 1px `--engrave-hatch` screen edge on test-grid Engrave circle cells
  (UX 7.2, P8; render-core, view only); (2) the folded pick lines of a kind that has picks but no main set
  (UX 8, "+ <Kind> set" case): the wasm view lists picks only under an existing main set; (3) the CPU budgets
  of criterion 24 are asserted in release builds only (`tests/acceptance_0146_budget.rs`), measure with
  `cargo test --release`; (4) dark values of the pick tokens (no dark theme yet).
- At PR time the lead sets `specification.md` Status to `Done (PR #n)`, fills "PR:" in its Links and the
  `specs/README.md` row; the design-system rows "Material test picks (`0146`, draft)" become final then.
