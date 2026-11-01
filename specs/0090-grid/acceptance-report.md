# Acceptance report: 0090 Grid

Date: 2026-11-01. Branch `story/0090-grid` at `ff48b130`. Verdict: **PASS**.

Tests were written from `specification.md` and `adrs.md` before reading the implementation.
Files (abbreviated below as DC, UI, ED):

- DC: `curvyo-document-core/tests/acceptance_0090_tester.rs` (19 tests)
- UI: `curvyo-ui-core/tests/acceptance_0090_tester.rs` (27 tests)
- ED: `curvyo-editor-wasm/tests/acceptance_0090_tester.rs` (21 tests)

Full gate (`CLAUDE.md` §7) green: fmt, workspace clippy, nextest (10283 passed, 152 skipped;
`pages::two_hundred_pages_cpu_budget` passed first time), wasm32 build and clippy for every
`*-core` crate and `curvyo-editor-wasm`, `cargo deny`, rustdoc, frontend `build:wasm`, `tsc`,
`npm test`, `oxlint`.

| # | Tests | Result |
|---|---|---|
| 1 | DC `ac1_*` (defaults, format version 29, round trip with no write on read, per page, Add page copies including after reorder, move/resize keep offset, old files v26/v27 open with defaults and no write, new document writes no register, 23 malformed registers refused as `Damaged` including on a deleted page, valid hand-made registers open, fieldwise merge); ED `ac1_add_page_copies_the_active_grid_and_delete_undo_restores_it` | PASS |
| 2 | ED `ac2_shift_click_toggle_is_one_labelled_history_step_on_the_active_page`, `ac2_entry_toggle_and_panel_act_on_the_active_page_only_and_name_it`; rail entry UI covered by the implementer's `frontend/tests/grid.test.ts` (no browser check) | PASS |
| 3 | DC `ac3_*` (refusals write nothing, extreme valid values, angle modulo 360, one step per change); ED `ac3_every_control_works_with_the_grid_hidden_and_each_change_is_one_step`, `ac3_invalid_values_are_refused_without_a_step_and_the_angle_wraps`; UI `ac3_subdivision_2_and_huge_subdivision_still_lay_out_and_nest`; notice length in `frontend/tests/grid.test.ts` | PASS |
| 4 | UI `ac4_*` (hidden draws nothing, clipped to page ∩ visible at 0/30/90/137°, off-screen page, majors stronger); ED `ac4_grid_is_only_in_the_frame_list_never_in_the_artwork_list`, `ac4_a_click_on_a_grid_line_hits_nothing` | PASS |
| 5 | UI `ac5_adaptive_minor_spacing_follows_the_zoom_25_5_1_0_2_mm`, `ac5_finer_lines_fade_in_between_levels`, `ac5_zooming_never_makes_a_line_jump_in_opacity_or_weight` (0.2 % zoom steps from 3 % to 6000 %), `ac5_levels_nest_*` | PASS |
| 6 | UI `ac6_fixed_grid_keeps_the_base_spacing_and_thins_out_when_dense`, `ac6_ac8_ac9_drawn_neighbours_are_never_closer_than_4_px` | PASS |
| 7 | UI `ac7_rotation_is_clockwise_around_the_grid_origin_with_unchanged_spacing`, `ac7_angle_360_draws_like_angle_0` | PASS |
| 8 | UI `ac8_log_lines_sit_at_log10_n_of_each_repeating_decade` (repeat, not mirror), `ac8_adaptive_log_adds_the_next_subdivision_in_cells_wider_than_48_px` | PASS |
| 9 | UI `ac9_line_count_is_bounded_by_the_screen_not_the_page_or_zoom` (20 km page, 1e-6 to 1e6 mm spacing, 2 % to 8000 %); CPU budget by the implementer's `curvyo-editor-wasm/tests/grid_budget.rs` (passed in the gate) | PASS |
| 10 | DC `ac10_ac11_grid_target_defaults_on_*`; ED `ac10_default_snapping_settings_include_the_grid_target`, `ac10_ac12_hidden_grid_or_grid_switch_off_does_not_pull`; UI `ac10_ac12_hidden_grid_pasteboard_and_switch_off_give_no_target` | PASS |
| 11 | DC `ac11_grid_point_is_document_wide_not_per_page`; ED `ac11_box_choice_*`, `ac11_nodes_choice_snaps_a_moving_path_anchor`, `ac11_nodes_choice_on_an_object_without_nodes_uses_the_box_centre`, `ac11_creation_snaps_the_placed_point_whatever_the_choice`, `ac11_resize_snaps_the_dragged_handle`, `ac11_node_tool_snaps_the_dragged_anchor` | PASS |
| 12 | UI `ac12_*` (intersection, line, only full-opacity lines pull, top page decides and a hidden top grid blocks, hysteresis, exact tie lost to every other kind, nearer grid beats farther guide); ED `ac12_alt_inverts_grid_snapping`, `ac12_pasteboard_has_no_grid_target`, `ac12_overlapping_pages_the_page_on_top_decides`, `ac12_page_move_keeps_the_grid_on_the_page` | PASS |
| 13 | UI `ac13_rotated_grid_snaps_in_grid_coordinates`, `ac13_x_and_y_switches_refer_to_the_grid_axes`; ED `ac13_rotated_grid_snaps_the_centre_to_a_rotated_intersection` | PASS |
| 14 | UI `ac14_log_grid_targets_are_the_drawn_log_lines` | PASS |
| 15 | UI `ac12_ac15_intersection_snap_is_exact_on_the_grid`; ED `ac10_ac11_ac15_centre_snaps_exactly_to_a_grid_intersection_as_one_step` (1e-8 mm, one undo step); DC `ac3_ac15_*` | PASS |

## Findings

1. Minor, not a criterion failure: writing an angle that differs by a full turn from the
   stored one (30° stored, 390° typed) makes one history step, because `rem_euclid(TAU)` leaves
   a 1-ulp difference. The stored value stays 30° to 1e-12 rad. Reproduce:
   `set_page_grid(id, g{angle: π/6})`, then `set_page_grid(id, g{angle: π/6 + 2π})` returns
   `Ok(true)`. Expected `Ok(false)` with no step. Covered loosely in
   `ac3_ac15_each_change_is_exactly_one_history_step_and_an_equal_write_is_none`.
2. Note: `grid_layout` given an un-normalised angle of exactly 2π (never stored, since the
   document normalises on write) gives a different line count than 0°. The test goes through
   `PageGrid::normalized`, which is the path the document takes.
3. Not covered here: the rail entry's blue state, icon and keyboard behaviour in a real
   browser, and the eyedropper (the eyedropper reads document geometry, and the grid is not
   in the artwork list).
