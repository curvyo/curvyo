# ADR 0053: Editable curves and width profiles

**Status:** Accepted by the customer on 2026-11-07 (CURVES A, WIDTH A,
OUTPUT A and ADR 0053). Detailed interaction and implementation choices below
remain proposals for the story demo.

## Context

ADR 0002 §6 stores paths as Bézier segments and lines. Converting Spiro and
B-spline to Bézier at completion would reuse that model, but would lose their
control points and editing behaviour. Flattening a variable-width stroke to a
filled outline would similarly lose its centre line and editable widths. Both
alternatives conflict with the approved direction. A width in absolute units
would also fail to scale with changes to the base stroke width.

## Decision

1. Extend ADR 0002 §6: a single-outline path may retain a curve kind (Bézier,
   Spiro or B-spline) and the control data needed to reconstruct that curve.
   Bézier is the absent/default kind. Straight and freehand results use ordinary
   path geometry. Derive a Bézier approximation for existing geometry consumers;
   never replace the editable data with that approximation on save. Specify and
   test endpoint, corner, open/closed and node-edit behaviour for each kind.
2. A path may carry a width profile on its centre line, independent of nodes
   and stroke style. Each point has a stable identity, a normalised arc-length
   position and a finite, nonnegative width multiplier. Actual width is base
   stroke width times the multiplier; absent profile means uniform multiplier
   one. A zero base width remains zero. A clone resolves the source profile
   while its resolved stroke width comes from its own effective style.
3. Proposal: interpolate smoothly without overshoot or negative widths; extend
   the first and last values to the ends of open paths. For closed paths use
   periodic interpolation across the seam. Order points by position with a
   stable ID tie-break after concurrent edits; define duplicate-position
   handling deterministically. Store points as independently editable CRDT
   entries, rather than one whole-profile register, so distinct edits merge.
   The implementation must specify the exact interpolation and test it before
   shipping; no invalid nonfinite value may reach geometry generation.
4. Profiles apply to single-outline paths in this slice. Editing a centre line
   preserves the normalised positions, so a point follows its fraction of the
   new length. Copy and linked-clone semantics preserve the data. Split, join,
   combine, break apart, booleans and other operations that cannot preserve the
   profile or curve data must refuse with a clear action to convert first;
   never silently discard them. Transforms must preserve visible geometry,
   including nonuniform transforms, or explicitly refuse unsupported cases.
5. Render, selection, hit testing and Stroke to path use the same derived
   outline. Stroke to path is an explicit undoable conversion. Preserve the
   centre-line output contract for manufacturing; it does not generate satin
   stitches. SVG export must preserve the visible result using ordinary SVG
   geometry and must not claim editable profile or curve round-trip. Define and
   test dash, cap, join, marker and fill behaviour; where a combination is not
   supported, prevent it or report it rather than silently dropping data.
6. Bump `format_version` to the next free version at merge, following the
   existing version policy. Old paths need no rewrite: absent new data has the
   old meaning. Older readers must reject the new version. Validate corrupt,
   unknown and unsupported data without silently treating it as a plain path;
   preserve source data where safe and surface an error otherwise. Cover old
   files, new files, future versions, malformed data, undo and concurrent edits.

## Consequences

Geometry consumers need an explicit shared conversion boundary and stable
tolerances; approximate curves must be tested against their defining algorithms.
Variable outlines and self-intersections are drawn, not refused; only truly
degenerate input fails, with an actionable error (amendment 2026-11-10 below).
Full editable SVG round-trip,
tablet pressure capture, brushes and stitch generation remain separate work.
The final implementation must document any dependency, its licence and wasm
support, and demonstrate the approved workflow end to end.

## Implementation notes (Proposal, 2026-11-07)

These notes specify the approved model without replacing its accepted decision.

- Measure position by arc length of the evaluated centre line in document
  coordinates, divided by total length. Open paths use [0, 1]; closed paths use
  [0, 1), with 0 at the first anchor and the path's stored traversal direction.
  A zero-length centre line has no usable fraction: retain its data, refuse
  width editing and outline generation with an explanation, and do not emit
  invalid geometry.
- A profile with no points is uniform 1; one point is uniform at that point's
  multiplier. Between two distinct positions use `a + (b - a) * (3t² - 2t³)`,
  where `t` is the relative interval position. This is continuous, has zero
  slope at each point and cannot overshoot. On an open path extend the nearest
  endpoint value. On a closed path interpolate across the seam from the last
  point to the first, treating the first position as increased by 1. Duplicate
  positions produced by merging are retained, ordered by stable point ID, and
  the greatest ID supplies the effective value at that position. Positions and
  multipliers must be finite; reject out-of-range positions and negative widths
  rather than clamping or silently replacing them. A drag is one undoable edit.
- Width points are geometry, not style: a linked clone uses its source's
  profile with the clone's resolved stroke width. A duplicate copies the data
  into independently editable points. Uniform scaling scales the base width
  according to the existing stroke-scaling setting; profile multipliers remain
  unchanged. Translation, rotation and reflection retain positions and
  multipliers; reflection preserves traversal order. Until nonuniform scaling
  can preserve the complete rendered outline, refuse that operation for a
  profiled path with a visible explanation. Do not alter the object on refusal.
- Construct the outline with the stroke's existing cap, join and miter-limit
  semantics. Apply dashes to centre-line arc length first and evaluate widths at
  each dash boundary; each dash receives the selected caps. A closed undashed
  path has joins at its seam and no end caps. Markers remain on the centre line,
  use existing placement and style semantics, and are not included in Stroke to
  path unless the existing command already includes them. Preserve the ordinary
  fill independently from the outlined stroke. Where an existing geometry engine
  cannot honour a combination, disable or refuse the change or conversion with a
  clear notice; do not fall back to an unprofiled stroke. A uniform multiplier
  of 1 must match an ordinary stroke within the existing geometry tolerance,
  including caps, joins, dashes and markers.
- SVG export emits the original fill plus a filled outline of the visible
  variable stroke in the stroke paint, preserving drawing order and markers;
  an editable centre line/profile is not promised on SVG re-import. Spiro and
  B-spline export as evaluated ordinary SVG path geometry. Machine jobs use the
  evaluated centre line and existing role rules, ignoring width for geometry;
  explicit Stroke to path changes the object to its outline and thereafter the
  resulting geometry follows ordinary job rules. If export cannot represent an
  enabled feature accurately, fail with an actionable error instead of dropping
  it. Preserve the existing frozen-snapshot export contract.
- Split, join, combine, break apart, booleans, offsets and node operations
  which cannot preserve both curve controls and profiles must leave all inputs
  unchanged and offer conversion first. Conversion of a curve to Bézier must be
  explicit and undoable. Stroke to path is a separate explicit conversion of
  the stroke to outline; it must preserve the original fill and markers as
  separate resulting geometry or refuse with an explanation. Normal node edits
  which the selected curve kind supports preserve its kind and width profile.
  Duplicating or deleting the object and changing its style remain available.

### B-spline evaluation (Proposal, 2026-11-07)

Use non-rational B-splines with unit weights, oriented by Inkscape's cubic
B-spline drawing mode; this defines Curvyo's geometry and does not promise
byte-identical Inkscape path-effect output or its adjustable weights. Control
points and corner flags remain the persisted source, not the resulting Béziers.

- An open smooth run with `n >= 2` controls has degree `min(3, n - 1)` and an
  open clamped uniform knot vector: first and last knots have multiplicity
  degree + 1; each interior knot is one unit apart. Thus the curve starts and
  ends at the first and last controls; two controls form a line and three form
  a quadratic. Convert each nonzero knot span exactly to cubic Bézier, degree
  elevating lines/quadratics where needed.
- A closed run without corners and with at least three controls uses degree 3
  with a uniform periodic knot vector, repeating the first three controls for
  evaluation and taking exactly `n` spans. The seam is smooth; the first span
  starts at `(P[n-1] + 4 P[0] + P[1]) / 6`, with indices modulo `n`. A closed
  path with fewer than three distinct control entries is refused without a
  commit; opening it remains possible.
- A corner flag breaks the curve at that control. Split into successive open
  runs that share the corner as their endpoint, evaluate each with the open
  rule, and join at the exact corner without imposing tangent continuity. For
  a closed path with corners, traverse cyclically from a corner to the next,
  including the wraparound run; with one corner its run starts and ends at that
  same corner. No extra seam segment is introduced. With only one distinct
  entry in a run, produce no span. Coincident coordinates are retained as
  controls, not silently removed; a fully coincident run has zero length.
- Reject nonfinite control coordinates. Degenerate spans must remain finite;
  zero-length spans are skipped for length measurement but do not remove
  controls or change the stored curve. Preview, saved geometry, hit testing and
  exports use the same evaluator. These rules need mathematical reference tests
  independent of the evaluator implementation.

### Compatibility with older sync peers (Proposal, 2026-11-07)

A file-format bump protects file opening but does not protect a replica of an
older build already receiving Loro updates: that build can ignore unknown path
keys. Until a compatible peer-capability protocol is available, a document that
has contained editable curves or width profiles must not be shared or uploaded;
creating either feature in an already shared document is refused before mutation.
This applies to active links, offline copies and reconnect, workspace background
writers, snapshots, join and relay queues, not just the Share button. Refusals
must be visible and leave the document and pending local work intact.

The compatibility invariant is a monotone root marker
`extended_path_history = true`. The first operation introducing either feature
writes it in the same commit as the feature data. All creation, copy, import,
conversion and restore paths that can introduce such data must preserve or
establish it. Undo, reset, object deletion and history restore cannot remove it;
a history wipe carries it into the retained shallow state. A marker is never
cleared merely because no live object has extended data: retained CRDT history
or queued updates may still contain it. A live extended path without the marker
is invalid. The safety check also examines retained history for marker writes,
so a deleted marker cannot make retained history appear safe.

Ordinary older documents, including valid shallow snapshots made by the existing
history wipe, remain shareable: they predate these features and have no marker.
A shallow snapshot is not by itself evidence of extended data. This conclusion
requires the invariant above for every legitimate writer and the existing rule
that shared history cannot be wiped. A malformed or deliberately altered file
whose marker and all evidence were removed before truncation cannot be proven
safe from its remaining bytes; the marker is a compatibility mechanism, not an
authentication or tamper-proof mechanism. Old offline replicas already in a
room cannot be protected by a check added only to the new client, so the feature
must remain blocked for that shared document until a separately verified
capability/upgrade protocol exists. Do not rotate the history-wipe epoch or
silently discard offline edits to work around this restriction.

Required tests cover: an ordinary pre-feature file after wipe remains shareable;
feature creation followed by undo, deletion or reset, wipe and reopen remains
blocked; missing marker with live feature is rejected; copies, unlink, import
and history restore retain the invariant; starting sharing and active uploads
are blocked; offline and relay reconnect, queued records, workspace snapshots
and background writers cannot bypass it; and refused operations preserve the
local state and show a useful notice. Verify each case for both curves and
width profiles. The export guard and its persisted marker are a temporary
compatibility policy, not a claim that older peers understand the new format.

### Spiro evaluation in the session (Proposal, amended 2026-11-10 and 2026-11-11)

Replaces the worker boundary of 2026-11-07. The document stores the ordered
Spiro controls, control kinds and closure as its only authoritative curve
geometry. Evaluated Bézier geometry is derived and is never written into Loro,
undo history or the project file; after a merge each replica evaluates the
merged snapshot.

- The solver is a pure function in `geometry-core` (controls, closure and
  tolerance in; Bézier geometry or a typed failure out), verified on native and
  wasm against the libspiro reference fixtures, including corners, endpoints,
  closed paths and degenerate input.
- The session calls it synchronously and memoises the result on the complete
  key (controls, kinds, closure, placement, solver version, tolerance), compared
  by value, not by hash alone. The cache is disposable and starts empty on open.
  There is no worker, job API, request generation, pending state or Retry;
  draw, hit testing, bounds, conversion and export all see the same result for
  the current snapshot. Export, Stroke to path and machine output evaluate the
  frozen snapshot they captured, synchronously.
- Stability rule (customer decision 2026-11-11, replaces the straight-line
  fallback and the notice): the solver is warm-started from the previous
  solution, a segment turns by at most 90 degrees and the sweep changes by at
  most 270 degrees when a point is added or removed (Proposals). A control
  change with no solution within these limits keeps the last solvable result;
  nothing is shown and no other spline is substituted. A Spiro read from an
  older build without a stored solution is drawn as straight lines, stays
  editable, and operations that need its evaluated geometry refuse with "This
  Spiro curve comes from an older build..." and change nothing.
- Remote updates are checked on a kept copy of the document instead of the live
  one (`import_updates_checked`); this doubles the log's memory for a live-shared
  document (`docs/technical-debt.md`).

Required tests: libspiro references on native and wasm, the stability rule (last solvable result kept),
reopen with an empty cache, clones sharing one evaluation under different
transforms and styles, undo and remote merge, and the CPU budget in
`spiro_cpu_budget.rs` (thread CPU time).

### Affine placement of Spiro controls (accepted, 2026-11-07)

The approved CURVES A keeps Spiro controls editable. A nonuniform scale or skew
of those controls followed by solving again generally changes the shape compared
with applying the same transform to the already evaluated curve. The existing
selection workflow expects its preview and committed transform to agree. This
amendment preserves both properties. The customer approved SPIRO-TRANSFORM A on
2026-11-07: persist the affine placement while keeping the controls editable.

1. Store a path-local affine placement `A` for a Spiro path, with six finite
   coefficients as one atomic register, absent meaning identity. Its ordered
   control points remain in local millimetre coordinates. Evaluate Spiro in
   that local space, then apply `A` to the resulting Bézier geometry. Do not
   store the evaluated geometry. Translation, rotation, scale, reflection and
   skew compose the selection's world transform `M` as `A' = M × A`, using the
   existing fixed-point and stroke-scaling rules. Reject nonfinite or
   noninvertible results without changing any selected object; retain existing
   zero-size recovery rules where they produce an invertible map.
2. Show control points at `A × p`. Convert a node gesture in document space
   through `A⁻¹` before committing the local point, leaving `A` unchanged.
   Handles, snapping, hit testing and bounds use the same world-space evaluated
   curve; control selection uses the transformed control locations. Node edit
   and object transform preview the exact same composed geometry later committed.
   This introduces no new user-facing mode: the maker sees and transforms the
   path with the existing Select and Node tools.
3. A linked clone resolves its root controls and root placement, followed by
   its own existing clone-chain placement and resolved style. Transforming a
   clone edits its placement, not the root. Copy or unlink preserves the visible
   geometry, controls and composed placement while giving the new path its own
   independently editable data. Avoid baking either clone or root placement
   into the controls and then solving them again.
4. The placement register is a single LWW value: concurrent edits to the same
   placement resolve as one complete affine map, never a hybrid of components.
   Independent edits to controls continue to merge by their existing IDs. The
   journal records a transform as one undoable step; undo and redo restore the
   controls and placement without replacing them with cached Béziers. In-flight
   drags use the frozen source snapshot and the existing conflict policy; stale
   cached solver results for an older snapshot cannot overwrite the model.
5. Explicit Convert to Bézier evaluates the frozen control snapshot, applies
   `A`, writes ordinary world-space Bézier anchors and clears Spiro placement
   in the same undoable commit. Stroke to path uses that same evaluated world
   geometry without first destructively converting the editable source. SVG
   and manufacturing output use the evaluated transformed centre line subject
   to this ADR's existing output rules. Derived geometry remains a cache only.
6. Reserve the next free `format_version` when this representation is merged;
   absent placement in old or already-written Spiro paths means identity, so
   existing controls require no rewrite. Older readers must reject the newer
   file, and the separate older-peer sync restrictions above remain in force.
   Validate all coefficients on read, including invertibility; damaged or
   unknown placement must be reported rather than silently treated as identity.
   Cache keys include local controls, closure, placement, solver version and
   tolerance; for a clone, tolerance accounts for the total composed map.

Required verification includes nonuniform scale and skew with preview versus
reopened result, transformed Node edits and snapping, reflection, clone chains,
copy/unlink, simultaneous control and placement edits from separate replicas,
undo/redo and versioned SVG/manufacturing output. Compare the transformed
result against the original evaluated curve under the same affine map at the
specified world-space tolerance.

Implementation order: add and validate the placement register and its
next-format migration in `document-core`; extend the read model and journal
commands before wiring Select and Node editing; evaluate the local controls in
the session and compose placement for canvas, hit testing, clones and exports;
then cover the workflows and cross-replica behaviour above. The customer can
transform and reopen the same editable Spiro path in the final demo.

### Variable-width outline, replaced (amendment, 2026-11-10)

The customer chose option A on 2026-11-10: replace the certified
variable-width outline (interval-arithmetic proofs per curve piece) with a
fast, uncertified one. It needed 352 ms (release) for a smooth 4-node S-curve,
12 s and then failed on a 12-node kinked path, and refused a two-node arc. The
document model, profile semantics and `format_version` 42 are unchanged; the
outline is derived and never stored. Method details are a Proposal.

1. **Constant profile** (every multiplier equal): the existing `stroke_outline`
   with width × multiplier, so uniform 1 is the ordinary stroke exactly.
2. **Varying profile**, in `geometry-core` as one pure function:
   - Sample the evaluated centre line by adaptive de Casteljau subdivision:
     split a sub-cubic until its control points are within 0.002 mm of its
     chord and its control polygon turns by at most `sqrt(8 · 0.004 mm /
     w_max)` (`w_max` the largest width on it), and until the linear
     interpolation of the smoothstep profile is within 0.002 mm. Retracted
     handles and cusps are thus split finely. Every anchor and width-point
     position is a forced sample. Position is cumulative chord length over
     total length.
   - At each sample offset by width/2 along the exact unit normal (derivative
     of the cubic; the chord direction where the derivative vanishes).
   - The stroke is this normal offset, not the envelope of width discs.
     Consecutive steps whose two sides both advance form one positively
     oriented strip ring; a run of consecutive steps where a side folds back
     (inner side of a tight bend) is filled as one ring per side instead: the
     centre line forward and that side's offset points back, the folding side
     split into its positive parts with the NonZero rule. (Amended
     2026-11-10: a convex hull per folding step made the union quadratic in
     the number of steps, 1.1 s on the benchmark paths.)
     Corner anchors get a join piece with the width there (round
     disc, bevel triangle, miter up to the existing limit 4); open ends get
     the style's cap. All pieces are positive and go into one `i_overlay`
     union with the NonZero rule on the existing 0.001 mm grid.
   - Dashes split the samples by centre-line arc length, each dash with caps
     and widths at its ends. Markers use the existing `stroke_markers` loops
     at the base width, united in the same overlay.
3. **Tolerance** 0.01 mm in document space (the existing kernel tolerance):
   0.002 mm chord, 0.002 mm grid snapping (two pitches, as in `stroke_outline`),
   0.004 mm turning and 0.002 mm width interpolation. It is a construction
   target, checked by tests against the same definition evaluated at 64× the
   sample density with the same caps and joins (boundary within a 0.01 mm
   band, area and containment), not proven per piece.
4. **Budget**, thread CPU time, release: ≤ 2 ms native and ≤ 4 ms wasm for a
   path of up to 20 nodes and 8 width points (the S-curve, the 12-node kinked
   path and the two-node arc above are the benchmark), ≤ 16 ms wasm for a
   200-node freehand path. It is a regression test, not a product limit.
5. **Refused** only: nonfinite coordinates or widths, a reach beyond
   `MAX_COORDINATE_MM`, a zero-length centre line, and compound paths (§4).
   Zero widths, tapers to zero, cusps, kinks, self-overlaps and tight arcs are
   drawn. A stroke that paints nothing is `Empty`, not a refusal.
6. **Synchronous**: draw, hit test, bounds, nudge, Stroke to path and export
   call the outline in the session, memoised on the exact evaluated centre
   line, profile, resolved style and tolerance. A width drag recomputes it on
   every move within the budget. There is no width worker, job or pending
   state; a path is never drawn with a stand-in stroke.
7. **Deleted**, not kept beside the new code: every `variable_stroke_*`
   module except the rewritten `variable_stroke.rs`, the certification
   modules `curve_interval_subdivision`, `curve_length_bounds`,
   `curve_normal_bounds` and `curve_offset_monotonicity` (`width_profile`
   first moves to plain `f64`), helpers in `stroke_markers.rs` that only the
   old kernel used, the Spiro spike tests
   `spiro_solver_experiment.rs` and `spiro_experiment/`, the width-job code in
   `curvyo-editor-wasm` and the frontend width worker. Tests of the old kernel
   keep only their behavioural assertions.

Consequence: Stroke to path and SVG export of a varying profile give line
segments within 0.01 mm instead of Bézier pieces (more nodes); tracked in
`docs/technical-debt.md`.
