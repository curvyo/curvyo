# 0061 acceptance report (tester)

Verdict: PASS with findings (no criterion failed; two spec gaps and one reading-order defect).

Tests: `curvyo-ui-core/tests/acceptance_0061_tester.rs` (7 pass, 1 ignored = F1),
`curvyo-editor-wasm/tests/acceptance_0061_tester.rs` (17 pass), `frontend/tests/browser/align-distribute.cjs`
(PASS), `frontend/tests/browser/align-distribute-journey.cjs` (PASS). Gate logs: `target/0061-gate/`
(fmt, clippy, nextest per crate all green: 4192 editor-wasm, 2105 ui-core, others green; wasm32 build and clippy,
deny, doc, wasm build, tsc, vitest, oxlint (existing warnings only) green).

| Criterion | Test | Result |
|---|---|---|
| 1, 18 groups shown/hidden | ac1_ac18_group_visibility; journey (11 symbol buttons, no text) | pass |
| 2 six aligns | ac2_align_all_six_onto_reference | pass |
| 3, 12 selection box, no-op, one step, name, undo | ac3_ac12_align_selection_box_one_step_and_noop; ac3 plan test | pass |
| 6, 7 distribute 3+, fewer than 3 | ac6_ac7_*, ac7_fewer_than_three_* | pass |
| 8 typed gap, 2 objects | ac8_typed_gap_* | pass |
| 10 locked | ac10_*, ac15_* (locked never moves) | pass, see F2 |
| 11 linked clone | ac11_linked_clone_* | pass |
| 14 one object to active page, 6 buttons, page 2 off origin | ac14_* | pass |
| 15 nothing selected, two pages, locked, empty page | ac15_nothing_selected_*, ac15_empty_or_all_locked_* | pass |
| 16 page distribution 0/1/2/n | ac16_* (4 tests), frame plan test | pass |
| 17 history names, selection stays | ac14, ac15, ac16 (names) | pass |
| 19 refusal, cells < n | ac19_*; ac19_ac20 plan test | pass |
| 20, 21 grid 2x3 gap 5, reading order, one step, undo, no-op | ac20_ac21_grid_*, ac21_grid_that_changes_nothing_* | pass |
| 13, 22 keyboard | journey (Space on focused button, Ctrl+Z undo) | pass |
| 5 key object | browser only partly; not independently driven (no Ctrl-click seam in Session tests) | not covered |

## Findings
- F1 (ui-core, `grid_translations`): reading order sorts by centre y, so a top-aligned row with different heights is
  reordered. Repro: boxes (0,0,10x10) and (30,0,20x4), grid 1x2: the 20x4 box (right) is placed first.
  Ignored test `ac20_top_aligned_row_of_unequal_heights_keeps_left_to_right_order`.
- F2 (criteria 10, 21): locked objects are not pickable, so they are never part of a selection. They therefore do not
  count for the reference box nor occupy a grid cell, contrary to the wording. They never move. No Session seam exists
  to lock an already selected object, so this is untested beyond non-movement. Needs a spec clarification.
- F3 (observation, criterion 20): the grid origin is the selection box before the operation, so pressing the same grid
  again on objects of unequal size can shift the grid (the new union corner differs); not idempotent.
- Selecting via Ctrl+A is not page scoped (selects objects of all pages); not a 0061 matter.
