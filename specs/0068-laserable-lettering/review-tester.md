# Tester and architect review: 0068 Weld

Date 2026-10-20, branch `story/weld` (3 commits on `story/text-objects`). Verdict: **PASS**.
No MUST-FIX.

## Tester

The acceptance tests were written from `specification.md` before the implementation was read,
one per criterion that the implementer's tests left uncovered or covered weakly:

| Criterion | Test | Result |
|---|---|---|
| 5 | `ui-core/tests/acceptance_0068.rs` `ac05_a_ring_keeps_its_full_hole_and_an_island_is_a_piece` | pass |
| 9, 10 | `ac09_ac10_an_open_line_welds_as_its_band_and_dashes_as_solid` (style to band, round caps, dashed as solid) | pass |
| 10, 28 | `ac10_ac28_open_outlines_without_stroke_close_or_refuse` | pass |
| 11 | `ac11_a_closed_stroke_does_not_widen_the_result` | pass |
| 13 | `ac13_nested_groups_and_an_overlapping_compound_weld` (3 levels; Combine of overlapping squares) | pass |
| 14 | `ac14_one_lone_rectangle_still_welds` | pass |
| 19 | `ac19_two_selected_groups_go_with_their_members` | pass |
| 19, 20 | `ac19_ac20_place_style_and_untouched_neighbours` (entered group, lowest style) | pass |
| 24 | `assert_clean` in the tests above (corner nodes, no near-duplicate or collinear nodes) | pass |
| 26 | `ac26_a_welded_result_round_trips` | pass |
| 28 | `ac28_a_coordinate_out_of_range_refuses` | pass |
| 23, Terms | `editor-wasm/tests/acceptance_0068.rs` `ac23_the_dot_of_an_i_is_a_piece` | pass |
| 12, 22 | `ac12_ac22_texts_and_a_shape_weld_and_one_undo_restores_them` (exact snapshots, redo, editable) | pass |
| 18, 30 | `geometry-core/tests/acceptance_0068.rs` `ac18_weld_at_zero_gap_is_union_node_for_node` (proptest, 128 cases, fixed seed) | pass |
| 29 | `ac29_order_does_not_matter_and_every_operand_is_inside` (proptest) | pass |
| edges | `a_hairline_band_is_empty_not_refused`, `a_negative_or_nan_gap_counts_as_zero` | pass |

The implementer's tests already cover criteria 4, 6, 7 (touching at g = 0 and g > 0), 8, 15 to 17,
27, 31 and 32. Release budgets (one run): (a) Annabelle 18 ms, 2 pieces; (b) 500 characters
767 ms; (c) at most 138 ms (fixture (i)); (d) under 1 ms.

## Architect

- `Document::replace_with_path_at`: sound. All checks (base among the operands, outlines, anchor
  ids, place an operand or a group of operands only) run before the first write; one commit; the
  emptied place group and other emptied groups go through `remove_emptied_groups`.
  `replace_with_path` delegates with `place = style_from = base` and no name, so its behaviour is
  unchanged.
- `union_region` in `boolean.rs`: the same flatten-and-normalize as Union before. Union is
  unchanged, and the property test shows Weld at g = 0 equals Union node for node.
- `weld.rs`, `stroke_band.rs`, `weld_plan.rs`: they follow decisions 1 to 9. The deviations
  (`StrokeBand` with the document's join and cap types, the write through
  `replace_with_path_at` instead of `replace_with_pieces`) are recorded in the build notes.
  Integer offsetting only; tolerances are explicit; no new crate or dependency. The Gap is kept
  in `localStorage` with `try`/`catch`, as decision 7 says.

Should-fix:

1. `curvyo-geometry-core/src/weld.rs:48` and `adrs.md:77` say a gap under two grid pitches
   (0.002 mm) skips the closing. The code (`weld.rs:124`, `round(g/2/0.001)`) skips only below
   0.001 mm. Correct the text.
2. `curvyo-geometry-core/src/offset_polyline.rs:22`: `MITER_LIMIT_ANGLE` was made `pub(crate)`
   but is used only in that file. Revert the visibility.
3. `curvyo-geometry-core/src/stroke_band.rs:41`: `Result<Paths64, ()>` is a unit error. Return
   the existing `OffsetError` (or an `Option`) instead.
4. Function length (§5, about 60 lines): `plan_weld` (`weld_plan.rs:143-243`, 100 lines),
   `replace_with_path_at` (`replace.rs:77-157`, 80 lines) and `weld` (`weld.rs:55-119`,
   65 lines). Give a reason in the PR or split them.

Wasm budget: the 29.6 KiB does not count against ADR 0020's text-engine budget. That budget
(1.4 MiB) was set for the text engine and was measured as the `0074` slice's delta against
`main` (engine about 1,290 KiB, slice code about 135 KiB). The technical-debt entry reopens it
for "the next feature on the text engine". Weld adds no text-engine code: it calls the existing
`texts_as_paths`. The growth comes from the geometry kernel (`i_overlay`'s stroke builder about
15 KiB, the rest Weld's own code). The editor wasm has no budget of its own, so the PR should
state the growth.

## Gate

`cargo fmt --check`, `cargo clippy -D warnings`, `cargo nextest` (7,795 tests passed),
wasm32 builds of all 5 core crates, `cargo deny check`, `cargo doc -D warnings`, frontend
`npm run build` and `npm test` (410 passed): all green.
