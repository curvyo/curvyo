# ADRs for "Cut at crossings"

Cut works on the exact curves, so it cannot use the polygon kernel for the
cut itself. It adds one curve-intersection module to `curvyo-geometry-core`,
built on `kurbo`'s cubic types and the existing `subdivide_at_parameter`
(`kurbo` 0.13 has line intersections only). **No new dependency, no new
crate, no new ADR, no `format_version` bump** (pieces are ordinary open
paths), no trait, no generic.

## Depends on

- [ADR 0003 §1, §2, §8](../../docs/adr/0003-geometry-kernel-booleans-offsetting-vcarving.md):
  curve math lives in `curvyo-geometry-core` on `kurbo`, with an explicit
  tolerance. There is no backend trait.
- [ADR 0002 §5, §6, §9](../../docs/adr/0002-document-model-units-and-svg-round-trip.md):
  pieces are new tree nodes at the operand's place, cubic segments and
  lines, written in one commit.
- [`specs/0016-boolean-operations/adrs.md`](../0016-boolean-operations/adrs.md):
  command pattern (availability, refusal codes with offenders, the refusal
  outline), `MAX_COORDINATE_MM`, `outline_of_rotated` for primitives,
  `operands_in_order` (stacking order, criterion 11).
- [`specs/0035-combine-and-break-apart/adrs.md`](../0035-combine-and-break-apart/adrs.md):
  the sweep of `outline_touch.rs` (chords sorted by minimum x, active list)
  and its `TOUCH_DISTANCE` of 0.001 mm, `session/refusal.rs`, the
  `AnchorIdMinter`.
- [`specs/0048-split-compound-path/adrs.md`](../0048-split-compound-path/adrs.md):
  `replace_with_pieces`, the generalised replace command that Cut writes
  through (0048 is built first).
- [`specs/0006-path-merge-split-and-node-types/adrs.md`](../0006-path-merge-split-and-node-types/adrs.md):
  splitting a path at a node (criterion 4).

## Feature-local decisions

- **2026-10-10: one new kernel module, `curvyo-geometry-core/src/curve_intersections.rs`.**
  `cut_parameters(contours: &[Contour]) -> Vec<Vec<CutParameter>>`, where
  `CutParameter = (segment index, t)`, sorted along each contour and
  deduplicated within 0.01 mm. The algorithm:
  1. Find candidate segment pairs with the sweep pattern of `outline_touch`,
     on segment bounding boxes grown by 0.001 mm.
  2. Subdivide each pair recursively at t = 0.5 (de Casteljau) while their
     boxes overlap, until both pieces are flat within 0.0005 mm.
  3. Intersect the two chords exactly. A crossing gives one point. Chords
     within `TOUCH_DISTANCE` of each other count as meeting (a touch or a
     T-junction). Collinear overlapping chords give the two ends of their
     shared stretch.
  4. Hits that chain along both curves over more than 0.01 mm merge into one
     overlap, and its two ends are the cut points (criterion 12, collinear
     lines).

  Self-intersection: a contour is tested against itself. Adjacent segments
  ignore the hit at their shared node. One cubic is split at t = 0.5 so that
  a loop is found. All of it uses only `+ - * /` and `sqrt` with no fused
  operations, so the same input gives bit-identical cuts on every target (criterion 11; a primitive's outline comes from the platform's `sin` and `cos`, so criterion 11 holds to 1e-6 mm).
  Rejected: flattening and intersecting polylines, because a polyline cut
  point is off the curve by up to the flatten tolerance and criterion 4
  needs 1e-6 mm on the curve. Also rejected: Bézier clipping, because it is
  faster on long curves but harder to keep robust at tangencies, and the
  budget of criterion 13 (lines) does not need it.
- **2026-10-10: splitting.** Pieces come from `subdivide_at_parameter`
  (`segment.rs`), applied in order with t renormalised after each split. It
  keeps a line a line. A cut within 0.001 mm of a node splits that node as
  0006 does, and adds no node (criterion 4). New end nodes are Corner nodes
  with a zero handle on the open side (criterion 5).
- **2026-10-10: the plan lives in `curvyo-ui-core/src/cut.rs`.** It holds
  availability (one or more objects, Select tool), the refusals (group,
  compound path, no division, more than 10,000 pieces, out of range), and
  the pieces with fresh anchor ids. The piece limit is checked before any
  write. Compound paths are refused before the kernel runs.
- **2026-10-10: the write goes through 0048's `replace_with_pieces`, plus
  one fill choice.** Cut adds `PieceFill { Keep, None }`: `None` turns Fill
  off and keeps the stored fill colour and opacity (criterion 6). Split and
  Break apart pass `Keep`. Pieces of one operand sit consecutively at its
  place. Undivided operands are not passed and keep their ids (criterion
  2). Label `cut_at_crossings`.
- **2026-10-10: session and frontend.** `session/cut.rs` and `wasm_cut.rs`
  follow the pattern of `session/combine.rs`. The rail button goes in the
  Path card after Split, per the UX notes. No shortcut, and Ctrl+X is not
  touched.
- **2026-10-10: tests and budget.** Criteria 12 and 12a are unit tests in
  `curve_intersections.rs` and `cut.rs`. Criterion 13: the 50 x 50 grid is
  built by a deterministic generator in the test, as 0035 does, and the
  expected count of 5,100 is the golden value. It runs in under 2 s in a
  release build, in the `boolean-budgets` CI job. A property test checks
  random cubic pairs: every reported point lies within 0.01 mm of both
  curves, and every split half lies within 1e-6 mm of its original.

## Flagged

None for the customer. The spec's own questions all have defaults.

## Notes made while building (2026-10-11, implementer)

- **A figure eight gives two pieces.** The contour passes the crossing twice and is cut at both
  passes, one piece per lobe. Criterion 12 and the definition of a cut point were corrected on
  2026-10-11 (lead decision, the architect agrees): the first text said "one open piece", which
  contradicted the definition of a piece (the part between two consecutive cut points) and would
  leave a piece that still crosses itself.
- **No `kurbo` in the curve module.** `curve_intersections.rs` and `curve_chords.rs` do their own
  de Casteljau and chord arithmetic with `+ - * /`, `sqrt` and `f64::midpoint`, so criterion 11
  (identical on every target) does not depend on the maths library of the platform. The split of
  the module into three files (`curve_chords.rs`, `curve_intersections.rs`, `curve_settle.rs`)
  keeps each under the 500 lines of `CLAUDE.md` §5.
- **Crossings are made exact.** A crossing found on flattened chords is refined by Newton's
  method on both curves (at most six steps, kept only if the miss is below 1e-9 mm), so the two
  contours are cut at the same point and not a flattening tolerance apart; touches and tangents
  keep the chord result.
- **Merging cuts along the contour.** Two cuts are one when they are within 0.01 mm in space and
  along the contour (chord lengths); two cuts at one place far apart along the contour (the two
  passes of a figure eight) stay two. Meetings at the join of neighbouring segments (a polyline
  run of at most 0.002 mm between them) are no cuts.
- **The split of a contour is `curvyo-ui-core/src/cut_pieces.rs`**, written through the
  `replace_with_pieces` of 0048 with `PieceFill::None`.
- **A loop in one cubic** is found by meeting the two halves of every piece that could loop (its
  control polygon goes backwards along its chord), recursively. Splitting once at t = 0.5 missed
  loops that lie in one half. A self-crossing whose one pass lies within 0.01 mm of a split point
  is taken for the join and dropped.
- **Near-touches are one cut.** Curves that come within the touch distance along a stretch without
  coinciding (tangent circles) make one contact: the touches and the crossings of a lens thinner
  than 0.01 mm (two Bézier circles that touch from outside cross twice by their own rounding) are
  cut once, in the middle. Stretches along which the curves coincide to 1e-6 mm (doubled lines,
  identical circles) keep a cut at each end.
- **2026-10-11: fixed tolerances.** `cut_parameters` uses fixed tolerances derived from
  `TOUCH_DISTANCE_MM` (0.001 mm) and the 0.01 mm kernel tolerance, as `touching_outlines` does; no
  caller needs another value. A `Tolerance` parameter comes with the first caller that does.
