# Tester report: 0055 compound-path-node-editing

Verdict: PASS (2026-10-19).

Tests: `curvyo-ui-core/tests/acceptance_0055_tester.rs` (28 tests, written from the specification before the implementation was read) and `frontend/tests/acceptance_0055_tester.test.ts` (5 tests).

| Criterion | Tests | Result |
|---|---|---|
| 1 Nodes of every outline, pill removed | c1_c2_hit_reaches_extra_outline..., c1 pill test (TS) | PASS |
| 2 Selection across outlines, segment click | c1_c2_hit_... (coincident nodes too) | PASS |
| 3 Move, insert, segment commands, hole dragged out | c3_hole_dragged_out..., c3_move_across..., c3_insert_and_segment... | PASS |
| 4 Delete: outline removal, promotion, object removal | c4_* (4 tests), edge_coincident_nodes..., stale ids | PASS |
| 5 Join a, b, c, notice count | c5a, c5b (2), c5c (3), TS join notice | PASS |
| 6 Split at a node | c6_* (3 tests), TS split notices | PASS |
| 7 Close path | c7_* (2 tests), TS notice/tooltip/dimmed | PASS |
| 8 Pen continue | c8_pen_extends..., c8_extend_refuses..., c8_end_index... | PASS |
| 9 Pen connect | c9_pen_connect_moves_other_outlines_in | PASS |
| 10 One commit, one undo step | c10_* plus step-count asserts in c3 to c9 | PASS |
| 11 Performance | not re-written; implementer's `compound_node_editing_budget` run in release: passes (0.5 s total) | PASS (not own test) |
| 12 Save/reopen | c12 only exports JSON and Loro snapshot; no reopen seam at this level | PARTIAL |

Notes
- Join merges the two ends into one node (0006 criteria 9, 10); node counts in tests follow that.
- Not covered by tests: the hover preview of Close path (needs a browser), the Pen chip "more outlines" line, a real reopen of a saved file (c12).
- Gate: fmt, clippy -D warnings, nextest workspace (7422 passed), wasm32 builds of the 4 core crates, cargo deny, cargo doc -D warnings, frontend `npm test` (379 passed): all green.
