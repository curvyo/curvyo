# 0069 Tiling: acceptance report (tester)

Date: 2026-11-02. Branch `story/0069-tiling` at `be3e4102`. Tests written from
`specification.md` before reading the implementation; public APIs read afterwards to compile.

Test files:

- `curvyo-ui-core/tests/acceptance_0069_tester.rs` (49 tests: layout, PCG32, variance, counts,
  Tile button, history names, SVG export)
- `curvyo-document-core/tests/acceptance_0069_tester.rs` (37 tests: stored tiling, refusals,
  registers, format, re-apply, hand edits, delete, duplicate, release, merge repair)
- `curvyo-editor-wasm/tests/acceptance_0069_tester.rs` (15 tests: session flow)

**Verdict: FAIL** (one failing white-box test; criterion 31 cannot be met in full, criteria 34
and 35 are partly built).

**Update 2026-11-02 (implementer, after the architect review):** finding 1 is fixed
(`apply_tiling` refuses a cell key outside the layout; `c04_cells_that_do_not_fit_the_recipe_are_refused`
passes unchanged). Criteria 13, 16, 23 and 31 were reworded by the lead (notes 2 to 4). Criteria
14, 34, 35 and 39 are now built and covered by the implementer's tests named in the table; they
await the tester's re-run. The helper `keyed` of the document-core tester file now returns a
`BTreeMap` instead of a `HashMap`: `c23_clones_have_own_placement_and_the_cell_affine_as_matrix`
compared the value order of two separately built hash maps and failed at random; no assertion
changed.

## Gate (CLAUDE.md §7, run locally)

| Step | Result |
|---|---|
| `cargo fmt --all --check` | green |
| `cargo clippy --workspace --all-targets -D warnings` | green |
| `cargo nextest` document-core | 2443/2444, fails `c04_cells_that_do_not_fit_the_recipe_are_refused` |
| `cargo nextest` ui-core | 2315/2315 |
| `cargo nextest` editor-wasm | 4380/4381; `acceptance_0025_slice3_tester::c55_undo_of_a_big_trace_is_not_quadratic` failed under load, passes alone (8.6 s); not tiling-related |
| `cargo nextest` other crates | 1751/1751 |
| wasm32 build and clippy, every core crate and editor-wasm | green |
| `cargo deny check` | green |
| `cargo doc -D warnings` | green |
| frontend `build:wasm`, `tsc`, `npm test`, `oxlint` | green (oxlint warnings only, outside tiling) |

## Criteria

| # | Tests | Result |
|---|---|---|
| 1 | `c01_entering_shows_the_defaults_and_writes_nothing` | PASS |
| 2 | `c02_cancel_and_escape_leave_the_document_byte_identical`, `c02_c37_apply_is_one_step_tile_and_undo_redo_are_one_step_each` | PASS (session; Enter/Escape keys are wired in `useTiling.ts`) |
| 3 | `c03_tile_disabled_with_nothing_or_tiling_plus_others` (wasm), `c03_tile_is_disabled_without_selection_and_for_a_tiling_with_others` (ui) | PASS |
| 4 | `c04_*` in document-core (dimensions only, locked source, locked layer, clone chain 8, orphan clone, count, NaN, zero, radius, tiling in source, group depth, cells outside the layout), `c04_a_refusal_keeps_the_bar_open_and_writes_nothing` | PASS (finding 1 fixed 2026-11-02) |
| 5 | `c05_cell_box_is_the_union_box_and_a_group_counts_as_its_leaves`, `c05_nested_groups_are_flattened_inside_each_instance` | PASS |
| 6 | `c06_*` (defaults, origin, 1 x 1, 0 rows/columns) | PASS |
| 7 | `c07_*` (gap, touch, negative gap, pitch, conversion round trip, pitch > 0), `c07_switching_spacing_mode_keeps_the_preview_still` | PASS |
| 8 | `c08_*` | PASS |
| 9 | `c09_*` (offset rows/columns, brick, shift range) | PASS |
| 10 | `c10_mirror_flips_about_the_offset_centre` | PASS |
| 11 | `c11_*` (defaults, CCW on screen, negative sweep, start/centre, ranges) | PASS |
| 12 | `c12_*` (360 vs < 360, count 1 for any sweep) | PASS |
| 13 | `c13_*` | PASS (criterion reworded, note 3) |
| 14 | implementer's `the_radius_handle_follows_a_drag`, `both_circle_handles_snap_to_the_grid_and_the_modifier_suspends`, `a_handle_drag_shows_its_readout_and_the_fields_follow` (`tiling_session.rs`) | PASS (implementer's tests, 2026-11-02; not re-tested by the tester) |
| 15 | `c15_*` (circle per step, uniform scale, scale <= -100 %) | PASS |
| 16 | `c16_*` (pinned formula, stepped draws nothing, mixed, range signs) | PASS (scale clamp at 0.01 now in the criterion, note 4) |
| 17 | `c17_*` | PASS |
| 18 | `c18_hue_variance_gives_own_style_and_leaves_the_source_untouched`, `c18_opacity_variance_is_clamped_to_0_to_100_percent` | PASS |
| 19 | `c19_seed_extremes_work_and_different_seeds_differ`; dice and fresh seed are frontend (`tilingText.test.ts`), not re-tested | PASS (core part) |
| 20 | `c20_*` (independent reference PCG32, golden file, advance, unit value, non-finite refused) | PASS |
| 21 | `c16_c21_random_values_follow_the_pinned_draw_order`, `c21_*` | PASS (desktop vs wasm not run by tester) |
| 22 | `c22_*`, `c17_c22_the_kept_source_is_never_varied` | PASS |
| 23 | `c23_*` (own placement, primitives converted in one commit, copies, image copy, text clone, sketch clone) | PASS |
| 24 | `c23_c24_multi_source_with_image_and_text` | PASS |
| 25 | `c25_*` (registers, format 32, newer file refused, 18 malformed variants Damaged, residue opens) | PASS |
| 26 | `c26_c27_reopen_shows_stored_values_and_edit_is_one_step` (copies tiling) | PASS |
| 27 | `c27_*`, `c26_c27_*` | PASS |
| 28 | `c28_*` | PASS |
| 29 | `c29_a_source_edit_flows_into_clone_instances`, `c23_copies_make_independent_leaves_mapped_by_the_affine`, `c23_clones_have_own_placement_*` | PASS |
| 30 | `c30_deleting_a_source_*` (doc), `c30_a_deleted_source_offers_only_release` (wasm) | PASS |
| 31 | `c31_deleting_the_tiling_*`, `c31_a_duplicate_is_an_independent_tiling_*`, `c31_an_instance_copied_out_of_its_tiling_is_a_plain_object` | PASS: delete, undo, duplicate. Copy/cut/paste apply once an object clipboard exists (criterion reworded by the lead 2026-11-02) |
| 32 | `c32_*` (doc and wasm, incl. concurrent apply repair converging on both peers) | PASS |
| 33 | `c33_*` (ui and wasm, save/reopen) | PASS |
| 34 | `c34_*`; implementer's `the_preview_draws_ghost_fills_and_the_dashed_circle`, `a_reopen_hides_only_the_managed_instances`, frontend `tilingText.test.ts` (debounce, live region) | PASS (debounce, ghost fill and refused-field marking built 2026-11-02; not re-tested by the tester) |
| 35 | `c35_undo_is_locked_and_switching_tool_cancels`; implementer's `a_page_switch_cancels_the_bar` | PASS (page switch built 2026-11-02) |
| 36 | `c36_*` (ui and wasm, limits 10,000 / 100,000, 99,999 cells laid out) | PASS |
| 37 | `c37_history_names`, `c02_c37_*`, `c26_c27_*`, `c32_*` | PASS |
| 38 | `c38_moving_the_tiling_keeps_the_stored_layout_numbers` | PASS (pages and grid snapping not tested) |
| 39 | implementer's `align_moves_a_tiling_as_one_object`, `distribute_moves_a_tiling_as_one_object` (`tiling_session.rs`) | PASS (2026-11-02; Align and Distribute now settle clone matrices by the clone rule) |
| 40 | not re-run (release-only asserts); `plan.md` lists 5.0 / 196 / 266 ms | NOT VERIFIED |
| 41 | `c41_layout_goldens_hold_every_named_case` plus independent checks above | PASS |
| 42 | `c42_shipped_goldens_open_and_damage_files_are_refused`, `c33_*` | PASS |
| 43 | covered by the rows above (copy and paste of a tiling deferred with criterion 31) | PASS |

## Findings

1. `Document::apply_tiling` accepts a cell whose key lies outside the recipe's layout and
   writes a keyed instance for it. Repro: a 3 x 1 recipe, cells `(5,0)` and `(0,2)`, `apply_tiling`
   returns `Ok(written: true)`; expected `Err(TilingError::Cells)` and an unchanged version
   (ADR 0028 §3 and `TilingError::Cells` "the cells are not the layout of the recipe"). Only
   reachable through the API (the session always passes `tiling_layout`), so low severity.
   A non-finite cell affine is refused correctly (`c04_a_non_finite_cell_affine_is_refused`).
   Fixed 2026-11-02.
2. Criterion 31 copy/cut/paste (also into another document): not possible, no object clipboard.
   Resolved by the lead's rewording: they apply once an object clipboard exists.
3. Criterion 13 with a start angle other than 0: an instance is turned by `angle - start`, not
   by its absolute angle. The relative orientation to the radius is kept and the kept source
   stays unturned, so this fits "keeps its orientation relative to the radius"; the spec
   wording "rotated by its angle on the circle" should say so. Reworded: "its angle on the circle
   minus the start angle".
4. Scale factors are clamped at 0.01 (scale variance <= -100 %): no degenerate matrices; the
   spec does not say what happens there. Now in criterion 16 (Proposal).
