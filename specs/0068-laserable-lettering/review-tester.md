# 0068 Weld: tester and architect review

Date: 2026-10-20. Branch `story/weld-v2` (3 commits on `origin/main`). Reviewer: tester (combined
with the architect review at the lead's request).

## Verdict

**FAIL** on criterion 14 (a line is cut twice). Everything else passes.

## Acceptance tests added

Written from `specification.md` before the implementation was read.

| Criterion | Test | Result |
|---|---|---|
| 4 | `curvyo-geometry-core/tests/acceptance_0068_tester.rs` `ac04_two_offset_squares_give_140_mm_against_flattens_160` | PASS |
| 27 (d) | `ac27d_twenty_squares_give_420_mm` | PASS |
| 5 | `ac05_a_hidden_disc_is_removed` | PASS |
| 5, 9, 15 | `ac05_ac09_a_disc_that_fills_a_rings_hole_is_kept` (disc below and above the ring) | PASS |
| 14 | `ac14_every_visible_boundary_is_in_the_result` | PASS |
| 14 | `ac14_no_line_is_doubled_on_200_random_sets` (proptest ChaCha seed, 0.002 mm) | **FAIL** (1 of 200) |
| 14 | `ac14_a_disc_crossing_a_rectangle_edge_leaves_it_once` (smallest failing set) | **FAIL** |
| 10 | `curvyo-editor-wasm/tests/acceptance_0068_tester.rs` `ac10_overlapping_script_letters_are_cut_apart` | PASS |
| 11, 20 | `ac11_ac20_one_undo_restores_the_editable_text` | PASS |
| 12, 19 | `ac12_a_single_selected_object_welds` | PASS |

## Finding 1 (MUST-FIX): doubled lines up to 14.5 mm long, one grid unit apart

Reproduction: weld a rectangle (22.5, 5.5)-(31.5, 9) below a disc centre (28, 10), r = 5.5.
The result holds the rectangle's right edge (31.5, 5.5)-(31.5, 5.757) in the closed outline and
again as the open line (31.499, 5.756)-(31.5, 5.5).

A scan of 3,000 sets of the same generator finds 12 failing sets, with doubles of 14.47, 11.76,
5.97, 1.97, 0.71, 0.56 and 0.49 mm. Each pair is at most 0.001 mm apart (one grid unit), so the
laser cuts it twice. The implementer's property test misses them because it compares at exactly
0.001 mm, and the far end lies 0.001 mm plus float noise away.

Cause: the fold and the final union round the same crossing to different grid points.
`weld.rs` `noded` then inserts the fold's vertex, not the vertex of `U`. The piece then lies one
unit inside `U` and survives the `STRICTLY_INSIDE` clip. `weld_lines.rs` `runs_along` would drop
it, but `chain_lines` asks it only for lines under 10 units (`SHORT_LINE_UNITS`).

Cheap fix (the fallback of `adrs.md` decision 3): drop every inner line segment whose endpoints
and midpoint lie within two units of the same edge of `∂U`, whatever its length. Look the edges
up through an edge index (`region_contact.rs` `EdgeIndex` or a grid bucket) so that fixtures (b)
and (e) stay in budget. Then rerun the 3,000-set scan at 0.002 mm.

## Architect review

- Edge extraction, chaining, `cleanup_open`: sound and deterministic (BTree order, fixed
  endpoints). `cleanup_open` shares Douglas-Peucker with the closed form, which is the right
  factoring. Finding 1 is the one gap.
- `replace_with_path_at(&Style)`: correct. It keeps the Weld rule out of `curvyo-document-core`
  and shares `write_replacement` with `replace_with_path`. No format change.
- Short open lines deviation: accepted. Dropping every line under 0.01 mm removed real trim
  edges, and a 2-node line of 0.001 mm or more meets `0016` criterion 24.
- Near-double fix (`noded`): it helps but is not enough (Finding 1). "4 of 3,000 under
  0.02 mm" understates the problem.
- Shared refusal text ("The selected object ...") applies to Offset, Cut, Split and Combine
  as well. No spec or test quotes "1 of 1", so this is fine. The 0037 test change
  (`slice(5, 7)`) still pins buttons 6 and 7 and does not weaken the test.
- wasm +199 KB: `i_overlay` 9.0.1 has no feature gate for `string`, already
  `default-features = false`, release has LTO. Nothing cheap to trim; accept.
- Window minimum height 640: matches Frage 16 default A; `tauri.conf.json` and the
  design-system row only.
- Should-fix: `ui-core/src/weld_plan.rs` `plan_weld` is about 85 lines (CLAUDE.md §5: give a
  reason in the PR or split off the operand collection).
