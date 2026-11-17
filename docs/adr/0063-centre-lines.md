# ADR 0063: Centre lines of filled shapes: a CDT medial axis in `geometry-core`

**Status:** Proposed 2026-11-15 (architect) for `specs/0117-centre-lines`; not `needs-customer`
(a geometry algorithm inside the story the customer decided on 2026-11-15, CENTRE-MEANING, BLOBS,
JUNCTION, INNER and SCOPE all A; no format, model or platform change). Accepted with the story PR.
Constants and rules marked (Proposal) are the architect's, vetoable at the demo. One question for
the customer (Frage 218, default A) at the end.
Builds on [ADR 0003](0003-geometry-kernel-booleans-offsetting-vcarving.md) §5 (approximate medial
axis from a `spade` constrained Delaunay triangulation, accepted for V-carve) and takes the module
name `geometry-core::medial_axis` from [ADR 0036](0036-variable-width-strokes.md) §6 (only
Proposed; its storage was replaced by [ADR 0053](0053-editable-curves-and-width-profiles.md)). The
results are ADR 0053 width-profile paths. The job runs in the worker of
[ADR 0023](0023-raster-trace-crate-pipeline-and-worker.md) §5. Nothing goes into `vectrace`
([ADR 0048](0048-vectrace-crate.md)): this works on vector shapes, traced or not.

## Context

A subject is a filled region (outer outlines and holes) with traced, noisy outlines and thousands
of nodes. We need its skeleton with a width (twice the clearance), a main path through junctions,
branches cut at an overlap, a Bézier fit within `T(w) = max(0.05 mm, 3 % of w)`, a width profile
with few points, blobs kept, and inner areas. This must be deterministic, run on wasm in the CPU
budgets of criterion 18, and add no crate. Options for the skeleton:

- **A. CDT of the flattened boundary, chordal-axis topology, axis points refined to maximum
  clearance (chosen).** `spade` 2.15.1 (MIT OR Apache-2.0, pure Rust; dependencies `robust` 1.2
  (MIT OR Apache-2.0, new in the lock), `allocator-api2` 0.2.21 (MIT OR Apache-2.0, new in the lock, through `hashbrown`'s default features), `hashbrown`, `num-traits`, `smallvec`, already in the lock).
  `O(n log n)`, exact predicates, and the triangle classes give the topology directly (ends,
  sleeves, junctions). Accuracy does not depend on sampling density, because each axis point is
  searched along its chord. It is already the accepted direction of ADR 0003 §5.
- **B. Voronoi of dense boundary samples (circumcentres of a plain Delaunay triangulation) with
  λ-pruning.** Uses the same dependency, but needs a spacing well below the thinnest width
  everywhere: about 60,000 points for the whale and millions at 100,000 nodes. Every pair of
  samples also grows a hair spur that has to be pruned. Lost on point count and noise.
- **C. Segment-site Voronoi (`boostvoronoi`, BSL-1.0).** The exact skeleton, but its own README
  lists open bugs, it has few users, its parabolic arcs need sampling, and every reflex jaggy
  still spawns a spur. It stays the escalation path of ADR 0003 §5 (see Decision 10).
- **D. Distance-field thinning on a raster.** `T(w)` at 0.05 mm needs pixels of 0.025 mm or
  smaller: about 32 Mpx for a 200 x 100 mm whale, which misses the budget. Holes and thin necks
  also lose exactness. Rejected.
- **E. Straight skeleton.** Its width is offset time, not Euclidean distance, so it is wrong at
  bends. Every reflex vertex of a noisy outline spawns a spike, robust implementations are
  super-linear, and there is no maintained pure-Rust wasm crate. Rejected.
- **`i_triangle` instead of `spade`** (same authors as `i_overlay`, integer-based). Version 0.51
  requires `i_overlay ^9.1.0`, but the kernel is pinned at `=9.0.1` (ADR 0003 §3, golden files),
  so every triangulator bump would move the boolean goldens. Rejected.

## Decision

1. **Input (Proposal).** Each subject is evaluated as it is drawn. Clones are resolved; Spiro and
   B-spline are taken as their derived Béziers; the subject's own stroke is ignored. The outline
   is mapped to document space and normalised by the boolean kernel (the subject's fill rule,
   0.001 mm grid, the `canonical_millimetres` order). This gives regions, each one outer ring with
   its holes, so self-intersections and touching holes are resolved before any skeleton work.
   Flattening uses the kernel's 0.002 mm chord tolerance.
2. **Skeleton: `geometry-core::medial_axis`.** One region in, a graph of axis vertices with a
   clearance radius `r` out. The same graph serves a later V-carve.
   - **Triangulation.** A CDT in grid units (integers as `f64`, so they are exact). Points are
     inserted in canonical order, and only the non-panicking `try_*` constraint calls are used.
     Triangles outside the region are dropped.
   - **Densify (Proposal).** A boundary edge longer than the height `h` of its inner triangle over
     it is split into `ceil(len / h)` parts, for at most 3 rounds and never below 0.01 mm. Each
     chord then crosses the stroke.
   - **Topology.** A triangle with 2 boundary edges is an end, one with 1 is a sleeve, one with 0
     is a junction.
   - **Axis points.** On every internal edge (a chord), the axis point is where the exact distance
     to the flattened boundary is largest. A golden-section search finds it to 0.001 mm. Its
     width is `w = 2 r`, twice that distance, never the chord length. Boundary segments are kept
     in a uniform grid index, so this search is fast.
   - **Junctions and ends.** A junction vertex is the junction triangle's circumcentre, or the
     midpoint of its longest edge when the circumcentre lies outside the triangle. An end
     connects its apex (the tip) to its chord's axis point.
3. **Graph clean-up (Proposal).**
   - **End trim.** From each tip, axis points whose disc contains the tip (`|p - tip| < r(p)`) are
     dropped. A rounded end then stops at the centre of its end circle, `w/2` before the tip
     (criterion 8). A tapering end keeps reaching the tip, because there `r < |p - tip|`.
   - **Spur prune.** A leaf branch (leaf to the nearest junction) shorter than `2 r_J` (the local
     width at its junction) is removed, shortest first; ties go to the lower grid position. This
     repeats until nothing changes, and degree-2 vertices are merged.
   - **What follows from the prune.** Jaggy spurs go, and so do the corner forks of a flat end
     (about `0.71 w` long), so a flat end ends like a rounded one (criterion 8 Proposal). A stub
     shorter than its width only shapes its parent's end (criterion 10 Proposal).
   - **Rings.** A component whose vertices all have degree 2 is a ring and becomes a closed path.
4. **Main path and branches (Proposal).**
   - **Junction zone.** The zone is the disc of radius `r_J` around junction `J`. On the spec's T
     the true axis dips by `w_s² / (8 w_b)` there (w/8, 0.25 mm, at equal 2 mm widths) and
     `r_J = 1.25 w/2`, a 25 % width bump. So every measure and fit ignores samples inside the
     zone.
   - **Merged junctions.** Junction vertices closer together than `max(r_J1, r_J2)` become one
     junction before pairing. Its degree is the sum of theirs and its zone is the union of their
     zones. A crossing or a noisy T, which often triangulates as two junction triangles, is
     therefore paired once.
   - **Edge measure.** For each edge leaving `J`, the samples between `r_J` and `3 r_J` from `J`
     give a direction `d` (unit vector from first to last, pointing away from `J`) and a width
     `w` (their median).
   - **Pair cost.** `c(a, b) = (1 + d_a·d_b) / 2 + |w_a - w_b| / max(w_a, w_b)`, computed with a
     dot product only. Straight on at equal width costs 0.
     - On the T, bar to bar costs 0 and bar to stem costs 0.5.
     - On the Y, stem to the straight 2 mm arm costs 0, and stem to the 60°, 1 mm arm costs
       0.25 + 0.5 = 0.75.
   - **Pairing.** At each junction exactly one pair continues: the lowest cost. Costs within 0.02
     count as equal; then the pair with the longer combined edge length wins, then the lower start
     position. "Lower" means the lexicographically smaller `(x, y)` in grid integers of the
     edge's first sample outside the zone. All other edges end there.
   - **Chains.** Chains follow the pairs. An open chain starts at its lexicographically smaller
     end. A closed chain starts at its smallest grid point and runs with positive orientation.
   - **Main and branches.** The main path is the longest chain with no unpaired junction end.
     If there is none, it is the longest chain overall, with ties as in criterion 9. Every other
     chain is a branch. At each unpaired end, a branch attaches to the chain that passes through
     that junction, so branches of branches follow the same rule, and a chain unpaired at both
     ends attaches at both (criterion 10).
   - **Branch start.** A branch is cut where its distance to its parent's *fitted* centre curve
     equals `(0.5 - p) w`, with `w` the parent's width at the nearest point. This is the first
     root walking outward from `J`, found by bisection to 0.001 mm. `p` ranges from -50 % to
     +50 % (Proposal, as `ux-notes.md`).
   - **Inside the zone.** Inside the junction zone, the branch continues as the straight
     extension of its edge measure: direction `d`, width `w`. The cut is solved on that
     extension. So a root exists for every `p` in the range, and the start width is the branch's
     own width within `T(w)`. On the 2 mm T, all three tested starts lie inside the zone, at 0.6,
     1.0 and 1.4 mm from the bar's centre, and the raw axis gives 2.15 mm at 0.6 mm.
5. **Fit (Proposal).**
   - **Split budget.** The axis error and the fit error each get half of `T(w)`.
   - **Corners.** An axis turn of more than 45° within one local width is a corner anchor.
   - **Curve.** Between corners, `kurbo` 0.13's `simplify_bezpath` (already a dependency) fits
     cubic Béziers to the bridged axis polyline with accuracy `T(w_min) / 2`, where `w_min` is the
     chain's smallest width. Bridged means junction-zone samples are left out on through chains.
   - **Profile.**
     - **Samples.** Each axis sample is projected onto the fitted curve, giving an arc-length
       fraction and the measured width.
     - **Points.** A greedy farthest-reach pass picks the profile points. Each next point is the
       farthest sample for which every sample in between lies within `T(w_i) / 2`, measured with
       ADR 0053's own smoothstep evaluator (`width_profile.rs`), not linear interpolation.
     - **Seeds.** An open path is seeded with both ends. A closed path is seeded with its seam
       point and stays periodic, so the seam step is at most `T(w)` (criterion 7).
     - **Base and multipliers.** The base stroke width is the chain's largest width; multipliers
       are `w / base`.
     - **Why at least five intervals.** Smoothstep deviates from linear by up to 9.62 % of the
       width change, so the 3 to 0.5 mm taper needs at least five intervals.
6. **Blobs (Proposal).** The test runs per region. Let `w_max = 2 max r` over its axis and `D` be
   the diameter of its outer ring (rotating calipers on the hull).
   - **Rule.** A region is a blob when `w_max + 2 T(w_max) >= threshold · D`. The tolerance term
     keeps a flattened disc a blob at threshold 1.0. The threshold ranges from 0.05 to 1.0.
   - **Whole blob subject.** It stays unchanged and is counted.
   - **Compound subject with some blob regions.** Those regions become filled paths in the result
     group, with the subject's style.
7. **Inner areas (Proposal).** With Keep inner areas on, each hole `H` becomes
   `offset(H, -d)`, using the existing `geometry-core::offset` with a Round join (`d` from -5 to
   +5 mm, as `ux-notes.md`). Where a selected
   subject `S` lies inside `H` (checked with `outline_nesting`), the inner area is
   `offset(H, -d) - offset(S, +d)`, using the boolean difference.
   - **Result.** Holes in the result survive. An empty result gives no area. Stroke is off and the
     fill follows CENTRE-INNERFILL. Inner areas stack below the subject's centre paths.
   - **Blobs.** A subject kept as a blob gives no inner areas.
8. **Placement, API, undo, refusals (Proposal; no new crate, ADR 0011 layout unchanged).**
   - **`curvyo-geometry-core`** adds `spade` and the modules `medial_axis` and `centre_line/`
     (`chains`, `fit`, `branch_trim`, `blob`, `inner`). It has two pure, staged entries:
     - `CentreSkeleton::build(region, Tolerance) -> Result<CentreSkeleton, CentreLineError>`
       covers Decisions 2 to 5 except the branch cut.
     - `CentreSkeleton::finish(&self, &CentreOptions) -> CentreLines` covers the branch cut, blobs
       and inner areas.
     - Public values use `Length`, `Point` and `Tolerance`. The `thiserror` error covers
       nonfinite input, out of range (`MAX_COORDINATE_MM`) and a failed triangulation.
   - **`curvyo-ui-core::centre_lines_plan`** handles:
     - subjects (criterion 1) and refusals that make no history entry (criterion 4);
     - the node count, refused above 100,000 with the count (criterion 18; interim 20,000, see the
       amendment of 2026-11-16);
     - the request, and the document change built from the result: style (criterion 3), groups
       and names, stacking order, and parent space.
     - A uniform parent scale divides the widths. A subject under a non-uniform parent transform
       is skipped and counted (ADR 0053 refuses non-uniform scaling of profiles).
   - **`curvyo-editor-wasm`** gets `session/centre_lines.rs`, which owns the preview state, and
     `centre_lines_glue.rs` with `centre_lines_job(request) -> wire`, an `f64` JSON wire like
     trace.
     - **Apply.** It writes the shown result as one commit labelled `centre_lines`. Undo and redo
       restore the document and the selection through the existing history (criterion 2).
     - **Stale preview.** If the document version or the selection changed since the preview,
       Apply recomputes first.
   - **Worker.** Both preview and Apply run in the trace worker of ADR 0023 §5, as a new message
     `centre_lines`: same module, cancel by terminate.
     - The worker memoises each region's skeleton by its canonical geometry. So an option change
       (overlap, blob threshold, inner areas, offset) reruns only `finish`.
     - A failed region is kept as it is and counted. The main thread only builds the request, the
       preview and the commit.
9. **Determinism.** The pipeline runs on canonical input, has no `HashMap` iteration in output
   order, and computes the lengths that decide the topology (the densify piece counts) as
   `sqrt(dx² + dy²)`. The same input gives a bit-identical result on every run of one platform,
   also after permuting subpaths or ring start vertices. Native and wasm agree within the
   goldens' 0.01 mm: kurbo (built with `std`) takes `hypot`, `cbrt`, `atan2` and `acos` from the
   platform (amended 2026-11-16; kurbo's `libm` feature would be a separate chore).
10. **CPU budgets (criterion 18): confirmed as wasm targets in the worker.**
    - **Wasm targets.**
      - Whale fixture (5,000 nodes, 20 shapes): Apply ≤ 1.0 s, option change ≤ 0.3 s.
      - 100,000 nodes: Apply ≤ 15 s, and main-thread work ≤ 100 ms per call.
      - Above 100,000 nodes: refused (interim on the main thread: above 20,000, see the amendment
        of 2026-11-16).
    - **Estimate.** The whale flattens to about 10,000 to 20,000 boundary points. The CDT then
      takes tens of milliseconds, the search about 25 distance queries per chord, and the fit is
      linear. That is about 0.1 to 0.2 s native, so there is headroom.
    - **CI gates.** CI gates native thread CPU time at half of each wasm target (0.5 s, 0.15 s,
      7.5 s and 50 ms for main-thread work). This is the wasm/native factor 2 that ADR 0053's
      outline budgets use (2 ms and 4 ms). **Exception (Proposal, 2026-11-19):** the native gate of a
      slider change and of an option change is 0.2 s, not 0.15 s. The CI runner measured whale ×29
      Optimise at 0.177 s against 0.115 s on the dev machine (CI is about 25 % slower, and no
      cheap speedup of the refit was found); the wasm limit of 0.3 s, which holds at 0.17 s, is
      the binding one. All other gates stay. The tests live in `curvyo-editor-wasm/tests/cpu_time/`. The PR reports wasm
      numbers, as `0025` criterion 25 does.
    - **Step 0, the first commit of the story.** This is the `spike/medial-axis` that ADR 0036
      asked for, done in place.
      - **Contents.** `medial_axis` with the analytic fixtures (straight, arc, S, taper, ring, T)
        and a native timing of the whale skeleton.
      - **Gates.** Axis and width errors each ≤ `T(w)/2`, and the whale skeleton ≤ 0.3 s native.
      - **If it misses.** The implementer stops and reports to the architect. The escalation is
        `boostvoronoi` (option C) by an amendment.
      - **Record.** The measurements go into `specs/0117-centre-lines/adrs.md`.
11. **Format.** No change (criterion 17). Results are ordinary paths with an ADR 0053 profile
    (format 42 and later), groups, and filled paths. SVG export follows ADR 0053.
12. **Tests and goldens (Proposal).**
    - **Fixtures.** They live in `curvyo-geometry-core/tests/fixtures/centre_lines/` as text: input
      outlines, and the expected centre nodes, profile points, branch starts, inner areas and
      kept blobs. Values are in mm to 4 decimals and compared within 0.01 mm. They are
      regenerated only by an explicit bless run that is reviewed in the PR.
    - **Synthetic subjects.** They are generated in the tests by the existing `variable_stroke`
      outliner from a known centre line and width: straight, arc and S at 2 mm, the 3 to 0.5 mm
      taper, ring, T, Y, a loop with a branch, a flat-ended bar, a two-hole compound, and a mouth
      line with round ends. Each also comes in a noisy copy with ±0.05 mm jaggies from a
      fixed-seed generator. A synthetic whale is assembled from them.
    - **Real line art.** A whale-like drawing made for the repo (own work, CC0) is traced once by
      `vectrace` at fixed settings, and the traced outlines (about 5,000 nodes, 20 shapes) are
      stored as the input text. A `vectrace` bump therefore never moves this golden. The same
      input tiled 20 times is the 100,000-node CPU fixture.
    - **Checks.** Criterion 5's painted check uses the `variable_stroke` outline, with an area
      difference ≤ 2 %. Criterion 6 allows at most `ceil(0.0962 · Δw / (T(w_min)/2)) + 2` points.
      On the T:
      - the bar is straight within `T(w)`, and its width is 2 mm within `T(w)` through the
        junction;
      - at `p` = +20 %, 0 and -20 %, the stem's start lies at 0.6, 1.0 and 1.4 mm within 0.01 mm,
        with a start width of 2 mm within `T(w)`;
      - an X crossing gives one main path and two branches.
    - **Property tests (`proptest`, native).**
      - Rotation by any angle, uniform scale from 0.5 to 4, and translation keep the result
        within `T(w)`: the same path and branch counts, centre curves within `T(w)` by Hausdorff
        distance, and widths at 20 positions within `T(w)`. Exact-tie fixtures are excluded.
      - Two runs, a permuted subpath order and a permuted ring start each give a bit-identical
        result.
    - **Wasm.** The goldens also run in Node, like `boolean_golden_wasm.rs`.
    - **Session tests.** They cover criteria 2, 4, 15 and 16, the refusal above the node limit (20,000 interim, 100,000 with the worker), and
      that no history entry is made on a refusal.

## Amendment 2026-11-16 (implementer, Proposals decided by the lead)

What is built differs from the Decision; the measurements are in `specs/0117-centre-lines/adrs.md`.

- **Interim: Apply runs on the main thread (customer question 219, default A; reversed by the
  customer on 2026-11-18, see the amendment of that date).** The worker job,
  the live preview and the skeleton memo of Decision 8 are not built; they, criterion 15 and the
  100 ms part of criterion 18 move to a follow-up story. Until then the operation is refused above
  `CENTRE_LINES_MAIN_THREAD_LIMIT` = 20,000 nodes; `CENTRE_LINES_NODE_LIMIT` = 100,000 stays for
  the worker. A 20,000-node Apply freezes the main thread for seconds (`docs/technical-debt.md`).
- **API as built (Decision 8).** `CentreSkeleton::build(&[Outline]) -> Result<CentreSkeleton,
  CentreLineError>` and `CentreSkeleton::finish(&CentreOptions, cutouts: &[Outline]) ->
  CentreLines`; the wasm surface is `curvyo-editor-wasm/src/wasm_centre_lines.rs` (no
  `centre_lines_glue.rs` until the worker). The subject's fill rule (Decision 1) and the parent
  scale (Decision 8) are struck: paths carry no fill rule (nonzero) and results are written
  without a transform. The document commit writes every path's width points in one batched write
  (`Document::set_width_profiles`).
- **Decision 1:** outlines are flattened at 0.0125 mm, `T(w_min)/4`.
- **Decision 2:** densify splits the outline inside one CDT, at most 8 rounds and 8 pieces per
  edge and round, no piece under 0.05 mm; an end triangle's long edge is measured against the
  triangle across its chord.
- **Decision 3:** the end trim's slack is 0.02 mm; a tree hanging off a junction within twice its
  local radius is pruned like a spur (round corners of thick strokes); a junction left with two
  edges moves to where their straight runs meet (the corner of a round-joined L); an end cut back
  by the steep trim settles where the stroke's own disc touches the end (a flat end ends `w/2`
  before its edge).
- **Decision 4:** a junction-to-same-junction edge that leaves the zone is a loop and is kept.
- **Decision 5:** pieces are fitted in local coordinates (kurbo 0.13.1 overflow), near-straight or
  degenerate pieces are lines, a profile point takes the median width around it where every
  sample still fits within `T(w)/2`, and a branch start reads its parent's width on pieces of at
  most 0.1 mm.
- **Optimise (Decision 5, customer 2026-11-16 and 2026-11-17, CENTRE-DEFAULT A):**
  `CentreOptions::optimise` from 0 to 1, default 0.75. The curve and the profile keep a quarter
  of `T(w)` at 0 and half of it at 0.5 (amended 2026-11-19: the slider was dead below 0.5; the
  dolphin goes from 2,157 to 1,534 nodes plus width points over 0 to 50 %, every 5 % step fewer),
  grow to all of `T(w)` at the default (criterion 5's painted area relaxed to
  4 %) and to three times `T(w)` at 1. Corners keep their detail at every setting: an anchor at
  each corner of Decision 5 and the half share within two local widths of it; a spur pruned at a
  bend settles the bend at its edges' meeting point in both prune passes. The build fits at the
  default; another setting refits the main and through chains in `finish`, the cheap stage. The
  curve's `T` comes from the lowest quarter of the chain's widths, not the least width.
- **Round branch end (CENTRE-TIP A, 2026-11-17):** four support points at 0.4, 0.825, 0.975 and
  1 of the radius, placed so that the smoothstep stays within 0.08 r of the semicircle (0.077 r).
- **Decision 6:** fat parts of a region (clearance at least 2.5 times the region's median, small
  holes filled) are opened off and kept filled.
- **Decision 10, numbers now:** native release thread CPU Apply 0.47 s and option change 0.10 s
  at 5,046 nodes; 9.2 s at 99,876 nodes against 7.5 s (that test is ignored; the limit is 20,000
  for now). Wasm (Node, process CPU time): 0.67 s, 0.18 s and 13.6 s against 1.0 s, 0.3 s and
  15 s.

## Amendment 2026-11-18 (architect, customer decisions 2026-11-18)

The customer reversed question 219 A: live preview in a worker, Apply off the main thread, the
operation in the Node tool, sliders, **Thick parts** in mm instead of the blob ratio, kept-filled
shapes keep their nodes, Optimise more aggressive where nodes are dense, and the orca's thick wave
bands as an acceptance case (spec criteria 12, 15, 15a, 15b, 18, 19, 20). Everything below is a
Proposal unless marked otherwise. It replaces the worker paragraph of Decision 8 and the interim
of 2026-11-16; the interim 20,000-node limit goes, the 100,000 refusal stays.

**W. Worker (replaces Decision 8, Worker).**
- **Instance.** A second `Worker` from the trace worker's script and compiled module
  (`centreLineJobs.ts`), not the trace worker's instance: a trace cancel terminates its worker
  and would drop the skeleton memo, and a running trace would queue the preview. Rejected: a new
  worker script (duplicate init) and sharing the instance.
- **Where things run.** The worker owns `curvyo-editor-wasm/src/centre_lines_glue.rs`, a
  `CentreLinesWorker` holding the memo; it calls only `geometry-core` pure functions. The session
  builds the request, holds the shown preview and commits; it never runs the kernel.
- **Messages** (all numbers in transferred `Float64Array`s, no JSON; this corrects Decision 8):
  - `centre-subjects {gen, version, subjects}`: the subjects' outlines in document space, sent
    when the bar opens and when the document version or selection changes. The worker builds
    the skeletons it lacks (memo keyed by a subject's outline bytes, compared exactly) and evicts
    the rest. Builds over 1 s report `progress`.
  - `centre-finish {gen, options}` → `centre-preview {gen, outlines, counts}`: the result's
    painted outlines (below) and the notice's counts. The worker keeps the latest result.
  - `centre-result {gen}` → `{gen, wire}`: the full result of that generation for Apply.
- **Staleness.** The page keeps one job in flight and one pending slot that each newer request
  overwrites; a slider drag never terminates. Every reply whose `gen` is not the latest request
  is dropped. The worker is terminated (memo and linear memory freed) on Cancel, Escape, deselect
  and when the bar closes after Apply.
- **Watchdog (Proposal, implementer).** A worker that owes an answer (ready, a preview or a
  result) and stays silent for 60 s (each progress report restarts it) counts as hung: it is
  terminated, the bar closes with "Centre lines failed. Nothing changed." and the next request
  starts a fresh worker. 60 s is four times the 15 s wasm budget of Apply on 100,000 nodes.
- **Staged memo per subject.** build (none) → fit (Optimise, last 4 values) → thick cut (Thick
  parts) → branch cut (Overlap) → inner areas (Offset, per hole). A slider reruns its stage and
  the ones after it only.
  **Built (Round B, accepted by the architect review of 2026-11-19):** the stages follow the order
  the code runs: cut (Thick parts) → through fit (+ Optimise) → branches and kept parts
  (+ Overlap) → inner areas (Thick parts, Offset, cut-outs), with **one cache entry per stage**
  keyed by the options that stage reads (not the last four Optimise values). Overlap reruns only
  the branches, Offset only the inner areas. `CentreSkeleton::finish` runs the same stages with a
  fresh memo, so both give the same result.
- **Preview drawing.** The worker sends each result path's variable-width artwork at the canvas
  tolerance (0.01 mm, the ADR 0053 outline `session/width_outlines.rs` draws), the inner areas
  and the kept-filled parts as polygons. The session draws them, nonzero, in place of the
  subjects (one `Path2D` per colour per generation in the canvas). The preview therefore paints
  exactly what Apply writes. **Built:** each path's painted outline flattened to **polygons** at
  0.01 mm (two numbers a point), the kept parts with the subject's own nodes, the inner areas;
  35 MB at 93,438 nodes, asserted under 64 MB in `centre_lines_cpu_time.rs`, so no `Float32Array`
  fallback is built. The implementer records transfer sizes at 5,000 and 100,000 nodes;
  limit 64 MB per message at 100,000 nodes, else the preview drops to `Float32Array` ribbons
  (axis samples ± `w/2`) and the memo is not seeded.
- **Memory.** Worker linear memory ≤ 320 MB after the first painted preview of 93,438 nodes
  (537 traced whales), checked in `frontend/tests/centreLinesWorkerMemory.test.ts` on the
  browser's wasm module. **(Proposal**, implementer: it was 256 MB, set when only the kernel's
  56 MB was known; 306 MB measured with the painted outlines, `docs/technical-debt.md`**)**.

**A. Apply (replaces Decision 8, Apply and Stale preview).**
- Apply commits the wire of the latest generation; while a job runs, Apply waits for it (the
  button shows busy). If the document version or the selection changed since `centre-subjects`,
  the session refuses the wire and the bar resends the subjects: there is no main-thread
  recompute.
- **Step 1, measure** where the 6.6 s native commit of the 100,000-node result goes. About
  0.3 ms per path at 2,250 paths and 0.6 ms at about 11,000 suggest a superlinear insert or
  delete; fix that inside `document-core` behind its current API.
- **Step 2, prepare in slices** off the document: snapshots, minted ids, profile points, and the
  artwork seeded into the `width_outlines` memo (so the first frame after Apply computes
  nothing). A slice is a fixed count of result nodes plus width points (deterministic, not a
  clock), set so a slice costs ≤ 25 ms native. Input is blocked while slices run ("Applying n %");
  Escape drops the prepared work and nothing is written.
- **Step 3, one `document.batch`** labelled `centre_lines`: one undo step (criterion 2), ≤ 50 ms
  native for the 100,000-node result. If it cannot get there, a transaction open across slices is
  a `document-core` API change and needs its own amendment and architect review first.

**B. Thick bands (criteria 19, 20).** Hypotheses, most likely first, each with its measure:
1. **The fat-part rule fires on bands.** The thin inner lines pull the region's median clearance
   down, so a band reaches 2.5 times it and is opened off as Round offsets with many arc nodes.
   Measure: openings per region and their nodes, with the rule on and off.
2. **Acute crest spurs.** Along a corner bisector `r = d sin(θ/2)`; the steep trim only drops
   slopes above 0.65 (θ > about 81°), and the spur's length `r_J / sin(θ/2)` can exceed the
   `2 r_J` prune. Measure: kept branches shorter than `4 r_J` that end at a convex outline vertex.
3. **Width noise.** Both traced edges add their noise to `w`, which exceeds `T(w)/2` on wide
   bands, so the greedy profile places a point every few samples and the axis wiggles. Measure:
   σ of `w` around a running median over one width, against `T(w)/2`, and width points per 100 mm.
4. **Tolerance from the thinnest quarter.** A band paired with a thin line at a tangential merge
   is fitted at the thin line's `T`. Measure: per path the width quartiles and the `T` used.
5. **Junction clusters.** Thin lines joining a band leave junctions closer than their zones, and
   short chains between them give a mesh of short pieces. Measure: junctions and merged clusters
   per 100 mm of band.

The implementer adds an ignored measurement test on the orca that prints, per region and per path:
role, length, width quartiles, nodes, width points, corner anchors, σ of `w`, the `T` used,
junctions and spurs; it runs it before and after each fix and records the table in `adrs.md`.
Fix directions, in the same order: (1) drop the 2.5 times rule (K below); (2) at a junction whose
other two edges pair at cost ≤ 0.25, prune a leaf ending at a convex outline vertex with `r`
falling about linearly, up to `r_J / sin(θ/2) + T(w)` long; a tapering stroke end is a chain end,
not a side leaf, and is tested to survive; (3) D below; (4) `T` per fitted piece from that piece's
lowest quartile, and pair cost weighting the width term by 2 at a tangential merge (directions
within 20°); (5) junctions joined by a chain shorter than `r_J1 + r_J2` merge into one cluster.
Criterion 20's budget: per path at most `2 + ceil(0.12 L)` nodes and `6 + 4 p + ceil(0.12 L)`
width points (`L` in mm, `p` the path's pointed ends, 0 to 2: ends whose width falls to 0; the 2
and 6 cover the ends and a round branch tip, the 4 per pointed end are criterion 8's four taper
support points), the painted area within 4 % of the band. **(Proposal**, implementer from the
measured fixture: `6 + ceil(0.12 L)` left the dolphin's bands 10 and 38, each with one pointed
end, over by 3 and 2 width points; the tester judged `6 + 4 p` fair**)**.

**D. Density-aware simplification (criterion 15a).** Applied after the fit, inside Optimise:
- **Merge distance** `m(opt)·w`, `w` the local width: `m` is 0 up to 50 %, 0.5 at the default
  75 % and 0.75 at 100 %, linear in between.
- **Axis.** Adjacent anchors closer than that merge: the anchor whose removal costs less goes,
  and the span is refitted as one cubic against its samples. The merge is kept only within the
  setting's tolerance plus `T(w)/2`. Corner anchors, path ends and branch starts never go; within
  two widths of a corner the half share of Decision 5 stays.
- **Width profile.** Before the greedy pass the width samples take a running median over one
  local width (removes the traced stairs, keeps steps longer than a width). Profile points closer
  than `m·w` merge into one at their median width when the profile still fits its tolerance.
- **Bound.** The fit keeps three times `T(w)` at 100 % and the merge may add `T(w)/2`, so the
  bound is 3.5 times `T(w)` as in criterion 15a (the 2026-11-16 amendment said three times).
- **Tests.** For the whale, the orca stand-in and the synthetic fixtures, Optimise at 0 to 100 %
  in 5 % steps never gives more nodes plus width points at a higher step; corner anchors are
  identical at every step.

**K. Thick parts (criterion 12; replaces Decision 6's ratio and the 2.5 times fat-part rule).**
- **Option.** `CentreOptions::thick_parts: Option<Length>`, the largest inscribed width
  `w = 2r` (Decision 2) a part may have; `None` is the slider's maximum. `blob_ratio` goes.
- **Rule, in `finish`, on the existing skeleton.** Axis vertices with `w > thick_parts` are cut
  away. The part they cover stays filled: its boundary is the original outline plus the CDT chords
  where `w` crosses the threshold. **Built (accepted 2026-11-19):** the kept part is the **region
  less the swept strokes** (each piece's samples swept by their discs, 0.05 mm wider), the
  components holding a thick sample, outlined by the subject's own Béziers where it runs along
  them and by straight chords where the swept edge dents it: CDT chords gave no pair where a
  thick chain hangs off a thin through chain (the iris on its ring). The painted dolphin stays
  within 2.9 % of its area (limit 4 %). Chains are cut there by arc length (no refit), and a main
  path cut in two gives two main paths. A slider move therefore never rebuilds.
- **Kept nodes.** A kept part writes the subject's original Bézier segments, split only at the
  chord ends (projected onto the original segments within the flattening tolerance); a fully kept
  region writes its original subpaths unchanged. If a projection fails (the normalisation changed
  that boundary), that part falls back to its flattened ring and is counted in `adrs.md`.
- **Internal rule.** Only the disc test stays: a region whose cut axis is empty, or whose
  `w_max + 2 T(w_max) ≥ D`, stays filled at every setting (criterion 12's maximum). **Built:**
  with a limit, the old blob ratio stays internal (a region at least half as wide as long stays
  filled; the orca's oval pupil); from the slider's end (20 mm) to the field's maximum (100 mm)
  that ratio fades linearly into the disc test, so no region jumps from filled to lines where
  the limit ends (`blob.rs`).
- **The whale's eye.** At 3 mm the iris (4.28 mm) stays filled with its own nodes and the ring
  gets a centre line, as before, through the cut instead of the opening.

**C. Budgets (criteria 15b and 18), thread CPU; native CI gates at half the wasm limit.**

| Case | Wasm limit | Native gate |
|---|---|---|
| First preview, whale 5,046 nodes (subjects + finish + outlines, worker) | 1.0 s | 0.5 s |
| A slider change, whale (its stages + outlines, worker) | 0.3 s | 0.2 s (was 0.15 s, see §10) |
| Main thread per call at 100,000 nodes (request, preview, each slice, the batch) | 100 ms | 50 ms |
| First preview, 100,000 nodes (worker, with progress) | 15 s | 7.5 s |
| Apply from a shown preview, 100,000 nodes (slices + batch, main thread) | 15 s | 7.5 s |

The lead's target of 100 ms wasm per Overlap, Thick parts and Optimise change at 5,000 nodes is a
target, not a gate: the whole `finish` measures 0.18 s wasm today, and the staged memo is what
should reach it. The PR reports the measured numbers.

## Amendment 2026-11-20 (implementer, customer CENTRE-THROUGH A: better junctions and through-paths, no new option; Proposal)

The customer's screenshots of the dolphin and the orca showed branches whose butt ends poke out
of the path they join, wave crests split into two paths at their tip, and a fin cut off as a
stub. Amends Decision 4 and criterion 20's budget; measurements in `specs/0117-centre-lines/adrs.md`
(Round D).

- **Crest tips (Decision 4).** At a junction of three edges, a free spur is a crest's tip when
  it ends in a point (`r ≤ r_J / 4`), points out along the outer bisector of the other two edges
  (within 30°), is shorter than both, and is no longer than `1.5 r_J / sin(θ/2) + T`, θ the
  angle between them (their miter); the two arms then make no crest when they lie within 15° of
  each other or 150° of a straight line. The arms pair, and their path runs through the tip: a
  straight leg from the first arm into the tip, its width falling linearly to 0, and one out to
  the other arm. The fit cuts the curve at the tip and where each leg begins, so each leg is one
  straight piece (measured up to a 60° apex; a wider crest's spur is pruned as before). A crest tip counts as two pointed ends in criterion 20's budget (a taper in
  and one out).
- **Pairing (Decision 4).** The pair with the smaller direction term `(1 + d_a·d_b) / 2` wins
  outright when the two differ by more than 0.1; only within 0.1 does the full cost (with the
  width term) decide, then the length and the start as before. So a band runs on into the thin
  fin that continues it instead of turning into its other arm (the orca's fin, screenshot 23).
- **Branch start (Decision 4).** The centre at `(0.5 - p) w` as before, unless the butt's
  farther corner then lies less than `p/2 w` inside the parent's paint (the union of its discs):
  then deeper, back along the extension past the junction's centre by at most one parent width,
  to where it does. A gap (`p < 0`) also keeps the nearer corner out of the parent's paint.
  Where the nearer corner would leave the parent's far side (an acute junction), the branch's
  first samples narrow until it stays inside. A branch at right angles starts where it did.
- **Fit guard (Decision 5).** kurbo measures a candidate's error from the source only and so
  accepted a cubic running 100 mm past a crest's leg; a fitted cubic straying more than three
  times its accuracy from its piece is split in two (`safe_fit.rs`).

## Consequences

- **Gained.** Centre lines arrive with editable profiles, and the axis graph is reusable for V-carve
  (R-MFG-CNC-002) and later auto-satin. `spade`, `robust` and `allocator-api2` are the only new dependencies.
- **Accuracy.** It is set by the boundary flattening (0.0125 mm since the amendment) and the per-chord search, not by
  sampling density. The junction zones are bridged, not measured, so the centre near a junction
  is a fit, not the true medial axis (which dips there).
- **Pruning is a heuristic.** A real stub shorter than its width disappears, and a jaggy longer
  than the local width survives as a short branch. Both are visible at the demo and tunable by the
  Proposal constants.
- **Rotation invariance.** Exact ties are broken by grid position, so a perfectly symmetric shape
  can change its main path under rotation. The property tests exclude such ties.
- **Main-thread risk.** The commit of a 100,000-node result is the riskiest part of the 100 ms
  rule. If it misses the 50 ms native gate, the lead records it in `docs/technical-debt.md` with a
  link here, and the architect revisits it (for example a smaller result through a coarser fit).
- **Accepted cost.** The memo lives in the worker and is lost on cancel; the cost is time, never a
  different result.
- **kurbo's fitter is guarded (2026-11-19).** `kurbo` 0.13.1's `cubic_fit` has no bound on a
  candidate's arm lengths; a candidate that passes the ray test on a spicy range sends `inv_arclen`
  into a shift overflow (debug) or an endless loop (release, so a stalled worker in wasm).
  `centre_line/safe_fit.rs` recomputes kurbo's candidates and ray test for each range before kurbo
  fits it and reports a range that would measure a candidate longer than 10 m as a cusp, so kurbo
  splits it; the anchor merge skips such a range. The goldens did not move. The real fix is a
  maximum arm length upstream (`docs/technical-debt.md`).

## Frage 218 (CENTRE-CAPS): caps of branches

A stroke has one cap for both ends (`LineCap`). Criterion 10 asks for a Butt start, while criteria
3 and 8 ask for a Round cap. Per-end caps would change the format, which criterion 17 excludes.
- **A.** Main paths and rings use Round. Branches use Butt on both ends, and a branch's free
  rounded end is drawn by its profile instead: the axis is extended by `r` with widths
  `2 sqrt(r² - s²)`, so it ends at the tip at width 0 and paints the same round end. A negative
  overlap stays a real gap.
- **B.** Round everywhere. A branch's start cap then reaches `w_b/2` back toward its parent, so a
  gap of -20 % becomes an overlap.
- **C.** Per-end caps: a `format_version` bump, in a later story.

Recommendation A. Default A. **Decided: A (customer, 2026-11-15)**; criteria 3 and 8 of the spec are
amended for the free ends of branches. The customer notes it may change later: option C (per-end
caps, `format_version` bump) would be its own story.

## Frage 228 (CENTRE-ORCA-LICENCE): the orca as a committed fixture

Renumbered from 220 (main's Frage 220 is GATE-BLOCKS of spec 0029). **Outcome (2026-11-18):** none
of the options below: the customer's own dolphin drawing (thick wave bands, thin lines joining
them, a filled eye) was checked in instead with the customer's approval and is the fixture of
criteria 19 and 20; the orca is only measured locally and never committed.

Criteria 19 and 20 store the demo's orca in `tests/fixtures/centre_lines/`. The picture carries
the watermark of a colouring-page site, and the real line art of Decision 12 is own work (CC0).
- **A.** Commit an own-work stand-in drawn for the repo with the orca's features (thick nested
  wave bands, thin inner lines joining them at acute crests, a filled pupil), traced once like the
  whale; the real orca is only measured locally and its numbers go into `adrs.md`.
- **B.** You hold the rights to the picture: the traced orca itself is committed.
- **C.** No orca fixture; only the synthetic band fixtures.

Recommendation A. Default A.
