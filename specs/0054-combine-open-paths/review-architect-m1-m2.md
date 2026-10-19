# Architect review: 0054 milestones M1 (kernel) and M2 (file)

Date: 2026-10-18. Branch `story/combine-open-paths`, commits `12140ed` (M1) and `e727887` (M2),
reviewed on `git diff origin/main...HEAD`. Gate run on the touched crates: nextest for
`curvyo-geometry-core`, `curvyo-document-core`, `curvyo-ui-core` (3637 passed), clippy
`-D warnings` clean, wasm32 builds of both core crates green, the release budget tests of the
`performance-budgets` job that touch this code (`boolean_performance`, `outline_touch_nesting`,
`combine_interactivity`, `compound_path_interactivity`, `boolean_interactivity`) green and well
inside their budgets.

## Verdict

**CHANGES REQUESTED** (one must-fix, finding 1). M2 on its own would be APPROVE.

## Findings

1. **Must fix. `curvyo-geometry-core/src/outline_nesting.rs`, `contains_touching` /
   `test_points`: quadratic per touching pair (§5 functions with a stated cost; `adrs.md`
   "Cost: ... linear in their vertex counts"; criterion 32's 2 s Combine budget).**
   `test_points` crosses every chord of B with every chord of A, then every test point runs
   `distance_to_polyline` and `winding` over all of A, and every vertex of A runs both over B:
   O(n·m) per pair. A throwaway probe (not committed) on two closed outlines, one touching the
   other from inside, via the public `outline_nesting`, release build:

   | Input | Time |
   |---|---|
   | circles R 500 / r 250 mm (flattened at 0.0005 mm) | 130 ms |
   | circles R 1000 / r 500 mm | 261 ms |
   | polygons 20 000 / 20 000 nodes, internally tangent | **13.2 s** |
   | polygons 50 000 / 50 000 nodes, internally tangent | **84.7 s** |

   The first-node test this replaces was O(n) per pair. 20 000-node outlines are ordinary for
   traced or imported work (DXF/SVG polylines, vectorization), and Combine runs on the main
   thread in the browser: the app hangs. The budget inputs of criterion 32 use 4-node squares and
   do not see it. The `adrs.md` cost sentence was wrong (mine); I correct it with the fix.
   Fix, keeping the definition of "contains" unchanged:
   - Find the crossing cuts with the x-sorted chord sweep that `outline_touch::sweep` already
     uses, not the all-pairs loop.
   - Decide "distance > touch distance" and the winding of all test points in one pass: sort
     the test points by y and sweep A's chords by their y range (a scanline crossing count), or
     bucket A's chords into a uniform grid over the pair's box. Either is O((n + m) log(n + m))
     plus output for realistic input, uses `+ - * /` only, and is deterministic.
   - Stop at the first test point of B outside A (already done) and the first vertex of A
     inside B.
   - Add budget input (e) to `curvyo-geometry-core/tests/outline_touch_nesting.rs` (already in
     the `performance-budgets` job): two closed outlines of 20 000 nodes, one tangent to the
     other from inside, plus the same pair crossing; `outline_nesting` under 200 ms in release,
     depths pinned ([0, 1] and [0, 0]).
   *Rejected alternative:* deciding containment by area through `i_overlay` (B ∖ A empty, A ∖ B
   not). It is O(n log n) too, but it changes the definition to an area tolerance and couples
   depth to the grid; the sweep keeps the spec's vertex definition.

2. **Should fix. `curvyo-geometry-core/src/outline_touch.rs` `point_segment_distance` (line
   277) and `self_touches`, `outline_nesting.rs` `distance_to_polyline`: `f64::hypot` (criterion
   33, the module docs' "only `+ - * /` and `sqrt`").** `hypot` is a libm call: glibc on Linux,
   the CRT on Windows, Rust's `libm` port on wasm32. None promises correct rounding, so a point
   at the touch distance can be judged differently per target and change a depth, a part of
   Split or a winding. Pre-existing since `0035`, but depth now depends on it in many more
   places and the new module doc states the opposite. Fix: `(dx * dx + dy * dy).sqrt()` in the
   three places (IEEE `sqrt` is correctly rounded everywhere). Values are compared to 0.001 and
   0.002 mm only, far above any overflow concern at 10⁷ mm.

3. **Note for M7, no change in M1/M2. `specs/0016-boolean-operations/adrs.md` lines 178 and
   283** still describe `OpenOperands` as a kernel refusal. I add the amendment marks with the
   `adrs.md` correction of finding 1.

## Checked, no finding

- **Flattening respects `closed`.** `flatten(anchors, closed, tol)` drops the closing cubic for
  an open outline (`index >= segments`), empty and single-anchor inputs stay as before. Every
  caller passes `outline.closed`: `boolean::grid_paths`, `offset::without_inverted_rings` (its
  `near` test still wraps, which is right there: the region's boundary includes the straight
  closing line), `Flat::of`. `Flat::chord_count` gives n − 1 chords for an open outline;
  `pair_touches`, `self_touches` (no "backward" run for an open outline) and
  `distance_to_polyline` use it, so the touch test no longer wraps round an open outline. Ends
  that nearly meet still touch, as they should. Pinned in `flatten.rs` unit tests and
  `open_outlines_0054.rs::touch_reads_open_outlines_open`.
- **Boolean kernel accepts open outlines.** Step 1's open refusal and
  `BooleanError::OpenOperands` are gone; the grid polygon of an open outline is closed by its
  straight chord implicitly, which is the fill area. `ac15_withdrawn_*` prove "open" and "closed
  by a straight segment" give identical results for all five operations. Straight and
  zero-length open outlines are dropped by `normalize`; a lines-only operand is still
  `EmptyOperands`. `outline_in_range` still checks the unused end handles of an open outline:
  correct, a non-finite number there is damage. **Public API change: covered.** `adrs.md`
  "Booleans" names the removed variant and plan.md line 42 calls it a public-API change; the
  only consumer, `curvyo-ui-core/src/boolean.rs::refusal_of`, drops its arm. The PR's Review
  guide must list it.
- **Nesting robustness, apart from finding 1.** Coincident outlines have no qualifying test
  point and are both shapes (7d); a hole sharing an edge with its plate is a hole (7b); crossing
  squares are both shapes (7c); the notch case is caught by the cut midpoints; tangent circles
  give [0, 1] from inside and [0, 0] from outside; open outlines get depth 0 and contain
  nothing. Every test point compared is farther than `TOUCH_DISTANCE_MM` from the other outline,
  so `winding` is never asked about a point on an edge. `crossing_parameter`'s exact
  `denominator == 0.0` is a degeneracy guard (parallel or zero-length chord), not a geometric
  comparison; near-parallel chords give a cut somewhere on the chord, and any cut position
  leaves the midpoint on the chord: acceptable under §5. Sorting uses `total_cmp`; results do
  not depend on HashMap order or threads: deterministic (`nesting_of_touching_outlines_is_deterministic`).
- **File (M2).** `validate_extra_subpaths` now requires `closed` as `LoroValue::Bool` on every
  extra outline. That is safe: every writer of `extra_subpaths` (`replace.rs`, `clone_write.rs`,
  `offset_write.rs`) goes through `write_extra_subpaths`, which has written `closed` since the
  first commit of `0016` (`909a2cc`, format 8), and `compound_v8.curvyo` and the later goldens
  still open. The first outline keeps "absent = open", as `adrs.md` decided. Damage tests cover
  absent, string and number; a single-anchor outline opens.
- **No format bump: right.** ADR 0004 §9 and ADR 0002 §11 bump only when a reader of the
  current version would misread stored data. A build between `0050` and this slice reads the
  same keys with the same meaning (windings are stored, `closed` per outline exists since
  format 8), and its renderer, hit test, bounds and Break apart already treat open outlines
  open. What differs is command behaviour, not reading: its touch and depth tests treat an open
  outline as closed by the cubic and judge crossing outlines by their first node, which only
  changes how its Split groups (criterion 29, every node kept). Its Combine, Booleans and Offset
  refuse open compound paths, so it cannot write a wrong result from that difference. Mixed
  peers in sync may Split the same object differently; each result is a valid document and
  merges by the existing rules (ADR 0009 §3). A bump would lock those peers out for no data
  safety.
- **Golden `compound_open.curvyo`.** Holds the four outlines criterion 29 names, declares format
  14, round-trips, is not rewritten on open (deep-value compare of the Loro state), and the
  `document.json` view lists every outline's state. It has no `_vNN` suffix like the other
  fixtures; the spec names the file, so it stays.
- **§5 and core purity.** No new trait, generic, crate or dependency; no `unwrap`/`expect` in
  `src`; no I/O, clock or threads; modules 212/278/280 lines, functions well under 60.
  Tolerances are the named `TOUCH_DISTANCE_MM`, `TOUCH_FLATTEN_MM`, `CHORD_REACH_MM` and the
  existing one-grid-cell area. Lengths stay `f64` millimetres with `_mm` names inside the
  kernel, as accepted since `0016`. wasm32 builds green.
- **Performance budgets (`ci.yml` `performance-budgets`).** Boolean 2 × 10 000 nodes 72-86 ms;
  touch and nesting of 1001 outlines 30 ms; Combine/Split/Break apart budgets 21-413 ms against
  0.5-2 s. Not measured by any budget: dense touching pairs (finding 1). `outline_nesting` now
  runs the touch sweep itself and Combine also calls `touching_outlines`, so Combine flattens
  and sweeps twice; within budget today, worth sharing the `Flat`s in M3 if the numbers move.

## Navigation

ProjectAtlas: 4 calls (`atlas_symbols` ×2, `atlas_watch_once` ×1, `atlas_search` ×1); the first
returned `refresh_required` and was fixed by one background `watch_once`. About 16 shell reads
(the diff, spec, tests, CI) because the review starts from the branch diff.

## Re-check 2026-10-19 (fix in `1ebb8c69`)

Finding 1 resolved: `test_points` takes the crossings from `sweep`, `every_point` judges all test points in one y-sorted scanline pass with the same touch-distance and winding rules, and `outline_touch_nesting` asserts a 200 ms release budget for 2 × 20 000 nodes. Finding 2 resolved: `hypot` replaced by `sqrt` of the sum of squares. Verdict for M1 and M2: **APPROVE**; the `adrs.md` cost sentence is corrected.
