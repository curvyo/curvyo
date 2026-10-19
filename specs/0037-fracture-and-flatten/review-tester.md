# Tester and architect review: 0037 Fracture and Flatten

Date: 2026-10-19. Branch `story/fracture-and-flatten` (3 commits on `origin/main`).

## Verdict

- Acceptance criteria: **PASS** (all criteria covered; the 2 s budget of criterion 17 holds).
- Architecture: **PASS with one MUST-FIX** (Flatten's box pre-filter, below).
- Gate (CLAUDE.md §7 plus frontend): **green**.

## Tests added

Written from `specification.md` before the implementation was read, only where the
implementer's tests left a criterion open or weak.

`curvyo-editor-wasm/tests/acceptance_0037_tester.rs` (session level):

| AC | Test | Result |
|---|---|---|
| 2 | `c2_an_open_outline_is_closed_and_counted` (amended by `0054` criterion 24) | PASS |
| 2 | `c2_an_operand_without_area_is_refused_with_its_offender` (both commands, nothing changes) | PASS |
| 3 | `c3_flatten_refuses_touching_shapes` | PASS |
| 4 | `c4_more_than_5000_pieces_is_refused` (72 x 72 lattice; Flatten on the same stack applies) | PASS |
| 6, 9 | `c6_c9_an_operand_apart_is_kept_and_undo_restores_everything` (id, primitive kept, selected, one undo step) | PASS |
| 8, 18 | `c8_complete_style_and_pieces_ordered_left_to_right` (U under a bar: complete style, order by first node, Corner nodes) | PASS |
| 10, 12, 18 | `c10_c12_visible_regions_of_several_areas_or_with_a_hole_are_one_compound_path` | PASS |

`curvyo-geometry-core/tests/acceptance_0037_tester.rs` (kernel):

| AC | Test | Result |
|---|---|---|
| 15 | `c15_curved_operands_give_clean_outlines_within_tolerance` (nodes and chord midpoints within 0.01 mm, spacing, no collinear node, node budget) | PASS |
| 15 | `c15_a_deep_stack_of_discs_stays_clean_and_within_tolerance` (25 discs, fold drift) | PASS |
| 15, 17a | `c15_pieces_of_a_chain_of_squares_have_four_nodes` | PASS |

Already covered by the implementer, no new test: 1 (frontend `stackText.test.ts`), 5 (shared
Boolean card path), 7 (examples and seeded tiling), 9/11/14 (notices and commit labels), 13
(`strokes_follow`), 16 (selection order in `ui-core`; cross-platform is the nightly runs), 17
(`fracture_flatten.rs`).

Criterion 15 needed tests: nothing checked node spacing, collinear nodes or the 0.01 mm bound
on a fold result. Criterion 18 needs no test of its own: the pieces are ordinary or compound
paths of Corner nodes (asserted in the c8 and c10 tests), and the Node tool's handling of
those is `0016` criterion 38, already tested.

Release budget of criterion 17 (`cargo test --release -p curvyo-geometry-core --test
fracture_flatten`): (b) Fracture 10.5 ms, Flatten 7.6 ms; (a) and (c) far below 2 s.

## Findings

### MUST-FIX

1. **Flatten's box pre-filter is quadratic on nested stacks** (`curvyo-geometry-core/src/visible_region.rs:44-53`).
   `plan.md` replaces the running union `U` of `adrs.md` by "subtract every operand above whose
   box meets", justified as "same region, fewer and smaller overlays". On stacks where the boxes
   all meet, each operand is overlaid with the concatenation of every operand above it: O(n²)
   edges. Measured in release, concentric circles (radii 1 to 1 + 0.2 n mm):

   | n | Fracture | Flatten |
   |---|---|---|
   | 200 | 0.45 s | 1.24 s |
   | 500 | 4.2 s | 11.5 s |
   | 2,000 | 68 s | 297 s |

   500 random discs of radius 8 mm in a 100 mm square: Flatten 0.72 s. 2,000 identical squares:
   Flatten 0.29 s. A target or ripple pattern of a few hundred rings freezes the UI thread for
   seconds. Fix: go back to the running union of `adrs.md` (top down, `O − U`, then `U := U ∪
   O`). The box test can stay as a shortcut for the `Untouched` check. Repro: a release test
   with `(0..500).map(|i| vec![circle(0.0, 0.0, 1.0 + 0.2 * f64::from(500 - i))])` through
   `visible_regions`.

### Should-fix

1. Fracture has the same nesting cost (4.2 s at n = 500): `fracture.rs:68` runs an
   `Intersect` overlay for every cell whose box meets, and nested rings all meet. The fold is
   what `adrs.md` chose; a cheaper pre-test (for example, skip a cell whose region lies inside a
   hole of O, or test the cell's box against O's region) would help. Not a criterion.
2. `fracture_cells` is 75 lines (`curvyo-geometry-core/src/fracture.rs:53-127`), over the ~60
   of §5: split out the per-operand step, or give the reason in the PR.
3. `curvyo-ui-core/src/fracture.rs:1-2`: the module doc needs "and" (Fracture's plan *and* the
   stack, refusals and plan shared with Flatten). Move `Stack`, `StackPlan`, `StackRefusal`,
   `MAX_STACK_PIECES` and `piece_outlines` to their own module (for example `stack_plan.rs`).
4. `curvyo-geometry-core/src/fracture.rs:97`: `touched[cell.covering[0]]` indexes without the `// invariant:` comment
   that line 105 gives for the same fact.

### Reviewed, no change

- `replace_with_pieces` (`curvyo-document-core/src/replace.rs:146`): empty `pieces` with
  non-empty `removed` is now a delete in one labelled commit. The doc comment says so; Split,
  Break apart and Cut always pass a piece for each removed object (their planners refuse
  before), so their behaviour is unchanged. The changed implementer case (`write(&[], &[])`
  still refuses) keeps the guard. Acceptable as a public API change.
- New geometry modules: one responsibility each (`stack_regions.rs` is the shared grid
  helpers, one sentence), all under 250 lines; `Tolerance` is a type, grid comparisons are
  integer and exact, area noise goes through `is_sliver`; no I/O, clock or threads; wasm32
  builds; no `unwrap`/`expect` outside tests. `Stack::run<T>` has two concrete `T`.
- Open outlines closed and counted: as the spec's status line and the UX notes say (`0054`
  criterion 24). Tested above.
- The 5,000 limit counts new objects only: consistent with the notice ("would make more than
  5000 pieces"); for Flatten it can only trigger above 5,000 trimmed objects. Accepted.
- Notice counts: Fracture counts the operands that took part, Flatten all operands (as
  criterion 11's example requires). The two differ, but each matches its criterion.

## Gate

`cargo fmt --check`, `clippy -D warnings`, `nextest` (7,696 passed, 78 skipped), wasm32 build
of every `*-core` crate, `cargo deny`, `cargo doc -D warnings`: green. Frontend: `npm run
build` green, `npm test` 402/402, `oxlint` warnings only in files this branch does not touch.
