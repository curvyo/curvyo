# 0070 Measure and dimension: acceptance report

Tester, 2026-11-01, branch `story/0070-measure-and-dimension` at `c59b582b` plus the tester commit.
Tests written from `specification.md`, ADR 0027 and `ux-notes.md` before reading the
implementation; only public names and the overlay JSON shape were looked up.

## Verdict: FAIL

Two defects against criteria 4 and 6 (below). Everything else passes, including the full gate.

## Tests

- `curvyo-document-core/tests/acceptance_0070_tester.rs` (31 tests, all pass)
- `curvyo-editor-wasm/tests/acceptance_0070_tester.rs` (39 tests, 37 pass, 2 fail = D1, D2)
- `frontend/tests/browser/measure-tester.cjs` (17 checks, 16 pass, 1 fail = D1)
- `frontend/tests/browser/measure.cjs` (implementer's): PASS

| AC | Tests | Result |
|---|---|---|
| 1 | `c1_drag_shows_length_angle_and_deltas_and_writes_nothing`, `c1_click_click_…`, `c1_negative_deltas_…`, `c1_readout_is_in_the_document_unit`, `c1_pinned_measurement_ends_on_{press_escape_tool_change…, undo, redo, a_document_edit}`; browser readout and Undo | PASS (see O1) |
| 2 | `c2_m_is_the_measure_tool_except_in_select_with_a_selection`, `c2_the_mode_label_…`; browser names, tooltips, "More tools", no bar | PASS |
| 3 | `c3_placement_region_chooses_…` (9 positions), `c3_without_shift_…`, `c3_a_curve_pick_is_radius_and_a_segment_pick_gives_its_length`, `c3_two_non_parallel_segment_picks_give_an_angle` | FAIL (D1) |
| 4 | `c4_shift_at_the_second_pick_…`, `c4_shift_is_read_live_…`, `c4_shift_released_…_escape_cancels`, `c4_ctrl_locks_the_direction…`, doc `c4_create_is_one_step…`, `c4_bad_creations_write_nothing`; browser Shift swap | PASS except D1 |
| 5 | `c5_selection_first_places_on_one_click_and_enter_…`, `c5_enter_places_for_a_selected_two_node_path_…`, `c5_a_press_on_a_pick_source_…`; browser Enter | PASS |
| 6 | `c6_anchors_snap_to_centres_only_with_snapping_and_alt_held_at_the_pick_turns_it_off`, `c6_snaps_to_centres_and_midpoints`, `c6_alt_cycles_radius_and_diameter_…`, `c6_alt_does_nothing_in_a_two_point_placement_…` | FAIL (D2) |
| 7 | `c7_vector_dimension_text_is_never_hidden_even_for_a_tiny_span`; browser screenshots | PASS (see O2) |
| 8 | `c8_dragging_the_text_is_one_step_move_dimension`, doc `c8_move_dimension_is_one_step_…` | PASS |
| 9 | doc `c9_a_dimension_is_never_an_output_node_and_is_a_guide`, `c9_make_clone_skips_a_dimension` | PASS |
| 10 | `c10_guide_button_on_a_dimension_converts_…` (session, fonts loaded), doc `c10_convert_…` (2) | PASS (see O3) |
| 11 | `c11_a_node_tool_drag_updates_the_number_live_…`, `c11_a_select_move_…`, doc `c11_*` (4); 200-dimension frame: implementer's `dimension_frame_budget.rs` passes | PASS |
| 12 | session `c12_deleting_the_measured_path_unlinks_…`; doc delete, node delete, Boolean, join, convert to path, non-uniform scale, clone unlink, last value after a move | PASS |
| 13 | `c13_node_tool_moves_kept_points_of_an_unlinked_dimension_…`, doc `c13_*` (2) | PASS |
| 14 | doc `c14_true_value_is_stored_unrounded_in_mm`; 3-decimal readout tests | PASS |
| 15 | `c15_hidden_objects_are_not_picked_locked_ones_are`, `c15_existing_dimensions_are_not_pick_targets`, doc `c15_*` | PASS |
| 16 | doc `c16_format_version_is_31…`, golden round trip, own round trip, older (v29, v30) open, v32 refused, four damaged goldens, own damaged registers (7), stale kept value on open | PASS |
| 17 | `c17_layers_row_announcer_and_delete`, `c17_layers_row_uses_the_display_unit`; browser Layers row | PASS |
| UX §3 | `live_region_speaks_on_pin_and_on_placement_not_on_every_move` | PASS |

## Defects

**D1 (criteria 3, 4): Shift at the second segment pick of an angle does not leave a dimension.**
Path (60,0)-(0,0)-(60,-60), Measure tool: click the first segment at (37,0) without Shift,
Shift+click the second segment at (37,-37), move to (30,-10): the mode label reads "Angle" (not
"Dimension, angle"), the readout stays, and the next click stores nothing. A dimension results only
when Shift is held at the first segment pick (which `measure.cjs` does). The spec puts the decision
at "the pick of the second anchor (… the second click)". Browser: `measure-tester.cjs` check "c4
angle …" (mode "Angle", no label added).

**D2 (criterion 6, UX §2): with snapping off, picks still snap to centres.** Two circles r 25 at
(50,50) and (150,50), document snapping disabled, drag (50.4,50.3) to (149.7,50.2): readout
"Δ 100, 0 mm". UX notes §2 make a pick a free point "while snapping is off"; criterion 6 ties anchor
snapping to `0060`. Alt held at the pick does turn it off (passes). If always-on feature picking is
intended (plan.md proposal), the spec and UX notes need that wording instead.

## Observations (no change requested)

- O1: a pinned measurement is dropped lazily on the next pointer event after an undo, redo or edit
  (`measure_drop_stale_pin`), not by the history call itself. In the browser it ends at once
  (Ctrl+Z passes), so the host re-sends the pointer; the session tests send one hover.
- O2: horizontal and vertical texts carry the `0066` axis glyph ("↔ 40 mm", "↕ 40 mm"), and the
  Layers row repeats it ("Dimension ↕ 40 mm"). Consistent with `0066` ac45; the spec examples show
  no glyph.
- O3: Convert reads the marks of the last drawn frame; with no frame drawn since the selection
  `toggle_guide` returns false. The host always draws, so only relevant to native callers.
- Layers row reads "Dimension 50 mm not linked" (no comma); the spec says "appended".

## Review of the implementer's change to three tester tests

`acceptance_0019_tester` ac37, `acceptance_edit_polish_pr3_tester` c59 and
`edit_polish_typed_skew_move::the_entry_keys_that_cannot_act_give_the_hints`: removing the "M gives
Select first" assertions is correct (criterion 2). Corrected: the deletions left M untested there,
so each now asserts the new rule positively (M gives `ToolChanged` and `Tool::Measure`, no move
entry, nothing written) in the same states.

## Gate (logs in `target/0070-gate/`)

fmt, clippy (workspace, all targets), wasm32 build and clippy of every core crate and
`curvyo-editor-wasm`, `cargo deny`, rustdoc, `npm run build:wasm`, `tsc`, `npm test`
(651 pass), oxlint (0 errors, existing warnings): green. nextest per crate: all green except the two
D1/D2 tests; `acceptance_0020_tester_more::c30…` run once separately (344 s, pass).
