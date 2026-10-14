# Tester review: 0053 node marquee selection

Verdict: PASS. Branch `test/node-marquee` on `story/node-marquee-selection` (8a3c9bf).
Tests: `curvyo-document-core/tests/acceptance_0053_tester.rs` (12),
`curvyo-ui-core/tests/acceptance_0053_tester.rs` (7), `curvyo-editor-wasm/tests/acceptance_0053_tester.rs`
(about 45). Written from the specification and public signatures before the implementation diff.

## Criteria

| AC | Tests | Result |
|---|---|---|
| 1, 3 | ac1_3_box_selects..., ac1_3_drag_direction..., ac1_press_over_fill... | PASS |
| 2, 10 | ac10_legend_text..., ac10_no_legend_before_dead_zone, ac10_the_box_is_drawn... | PASS (box colour and ring look are not asserted, only that geometry is added) |
| 4 | ac4_box_works_across_paths..., ac4_compound_paths_hidden_and_locked... | PASS |
| 5 to 8 | ac5_6, ac5_7, ac5_8, ac6, ac7, ac8, ac5_ctrl_wins..., ac5_modifiers_are_read_at_the_release; ui-core `apply_*` | PASS |
| 9 | ac9_marquee_order..., ac9_order_of_the_added_nodes..., ac7_add_keeps_existing_order...; ui-core order test | PASS |
| 11 | ac11_escape_cancels..., ac11_escape_keeps_order_for_join | PASS |
| 12 | ac12_click_on_nothing..., ac12_a_move_under_three_pixels... | PASS |
| 13 | ac13_a_press_on_a_node..., ac13_dragging_a_selected_node... | PASS |
| 14, 15 | ac14_zoom_during..., ac15_leaving_the_viewport... | PASS |
| 16 | ac16_zero_height..., ac16_boxes_beyond_1e7..., ui-core degenerate boxes | PASS |
| 17, 28 | ac17_marquee_click_and_ctrl_a_write_nothing | PASS |
| 18 | ac18_* (document layer: one commit, group emptied, stale ids; session: one step "Delete nodes") | PASS |
| 19 | ac19_join_reasons | PASS |
| 20 | ac20_* (document and session) | PASS |
| 21 to 24 | ac22_*, ac23_counts..., ac24_announcement... | PASS |
| 25 | `node_marquee_interactivity` (release): anchors_in_box 51 us, apply 418 us, marquee frame 12.2 ms, release to frame 3.9 ms | PASS |
| 26 | frontend, not testable here | not verified |
| 27 | ac27_the_select_tool_marquee... plus the unchanged 0014 tests | PASS |

`PathSnapshot::all_anchors` (for 0055): storage order with scrambled ids and positions, closed outlines
do not repeat the first anchor, four-outline compound path (closed and open mixed), no repeats, order
stable across save and reopen, 40 pseudo-random outline shapes. All pass.

## Findings

1. Spec example, criterion 18: a press at (5, -1) is 3.8 px from the segment at 100 % zoom and the
   segment hit tolerance is about 5 px, so that press selects the segment, not a marquee. The
   example is correct as a rule but not reachable as written. Our test starts at (5, -2). Spec nit only.
2. ADR decision 8 asks for a marquee frame of at most 1.1 times the plain Node frame. Measured 3.2
   times (12.2 ms against 3.8 ms). The implementer's plan records this; criterion 25's absolute
   16.7 ms holds with 4.5 ms to spare on this machine. The real-window check on the reference
   desktop is still open.
3. A node marquee in flight survives `set_tool(Select)` then `set_tool(Node)`, and the next release
   applies it. Not reachable by keyboard (tool keys are ignored with the button down); no test kept.

## Changed earlier tests

- `acceptance_0044_tester.rs` c3_other_tools_ignore_ctrl_a: Node now expects `SelectedAll`. Right:
  criterion 22 amends 0044 criterion 3 and the object selection is still asserted empty. The loop
  still covers the other tools.
- `acceptance_advanced_selection_tester.rs`, legend test: Node expects a legend containing "nodes".
  Right, criterion 10. All other tools still assert no legend.
- Same file, minus badge: Node expects the badge, others do not (`==` on `tool == Tool::Node`).
  Right, UX notes. Both are narrowed amendments, not weakened assertions.
- Mechanical signature migrations (`convert_anchor_kind`, `pointer_up`, `select_all`) in
  `acceptance_0006`, `0002`, `0003`, `0016_pr2`, `0018`, `0023`, `groups_commands`: assertions unchanged.

## Gate (local, 4 jobs)

fmt, clippy `--workspace --all-targets -D warnings`, nextest 6629 passed, wasm32 build and clippy for
all five core crates and `curvyo-editor-wasm`, `cargo deny`, `cargo doc -D warnings`, all release jobs of
ci.yml including `node_marquee_interactivity`: green. Frontend: `tsc`, lint (warnings only), `npm test`
355 of 355 three times, no flakes.
