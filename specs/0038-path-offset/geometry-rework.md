# Offset geometry rework: clean curves instead of dense polylines

Architect, 2026-10-12. Input: the customer's rejection of PR #115 ("the
roundings are made of many small segments ... with a circle and a rectangle it's
a disaster ... probably adjustable"; "rectangles and circles/ellipses can stay
shapes, in both directions"). This note diagnoses the cause, proposes the
rework, and lists what changes in `specification.md` and `adrs.md`. Nothing is
built yet; the numbers marked "est." are estimates to be confirmed by the spike
in step 1 of "Plan".

## 1. Diagnosis

The dense output is the design of this slice, not a bug. The spec says so
("What the result is: a polyline", criterion 11, criterion 20), following
ADR 0003 §3/§4 ("results are polylines"). Two places multiply the nodes:

1. **The input is flattened before the offset.** `offset()` in
   `curvyo-geometry-core/src/offset.rs` calls `grid_paths` (closed) or
   `flatten_closed`/`flatten_open` (open) from `flatten.rs` at
   `tolerance - 2 x GRID` = 0.008 mm. Each cubic becomes chords of angle
   `2 sqrt(2 x 0.008 / R)`: a circle of R 20 mm is about 118 chords, a 5 mm
   corner arc about 15. Every curve the user drew is gone before
   `i_overlay` sees it.
2. **Every chord vertex becomes a join.** `IntOutlineOffset::outline` (closed)
   and `IntStrokeOffset::stroke` (open) treat each flattened vertex as a corner.
   With the Round join (`int_join` and `arc_options` in `offset.rs`) each corner
   gets at least one arc chord, so the output has about twice the input's
   vertices. A curved input therefore yields a polygon with about twice the
   flattened vertex count. Round joins on real corners are chords as well
   (`arc_options`: step `2 sqrt(2t/r)`).

After that, `boolean_cleanup::cleanup` only removes collinear and duplicate
vertices, and `canonical_millimetres` writes corner nodes without handles. No
step turns chords back into curves. Primitives (rectangle, ellipse, polygon,
star) take the same route: `curvyo-ui-core/src/offset_plan.rs` turns them into
outlines (`outline_of_rotated`) and the result is always a path with
rotation 0.

| Input (Outset 2 mm, Round) | Now (est.) | Exact result |
|---|---|---|
| Circle R 20 (4 cubics) | ~230 corner nodes | 1 circle (4 cubics) |
| Rect 100 x 50, corner radius 5 | ~120 corner nodes | rect 104 x 54, radius 7 (8 segments) |
| Rect 20 x 20, sharp | 4 lines + ~4 x 10 chords | rect 24 x 24, radius 2 |
| 200-node gear (`wavy(200)` in `tests/boolean_performance.rs`) | ~1,000 corner nodes | ~200-300 segments (est.) |

## 2. Options for clean output

All three keep `i_overlay` as the source of truth for topology: what survives,
holes, vanishing parts, self-intersection cleanup. That part works and is
deterministic. They differ in where the final shape of each outline comes from.

**(a) Fit cubics to the result rings.** Run the current pipeline, then fit
cubics to each ring within a tolerance (`kurbo::fit_to_bezpath[_opt]` or
`kurbo::simplify`).
- Segments (est., 0.01 mm): circle 4-8, rounded rect 8-12, gear 250-400.
- Cost: `fit_to_bezpath_opt` is the slow variant (it searches for the
  smallest segment count). On the gear's ~1,000 vertices that is about
  10-100 ms on top of today's 20/55 ms, which breaks criterion 3. The non-opt
  variant is cheaper but produces more segments.
- Risk: **high.** The fitter only sees chords, so it has to guess corners from
  turn angles. A chord turn of 3° on a small arc looks like a real 5° corner.
  Straight edges come back as near-straight cubics unless they are detected,
  and the fit has to share the 0.01 mm budget with the 0.008 mm flattening.
  Results wobble, and the node count depends on the tolerance.
- Upside: the same pass would also help Boolean results (the technical-debt
  entry "Boolean results are polylines").

**(b) Exact pieces for straight-edged input only.** Offset lines stay exact,
as now. Recognise the result vertices that lie at distance |d| from one input
corner as a round join, and replace them with a true circular arc (a cubic per
≤ 90°).
- Segments: rect, polygon and star exact (one segment per edge and per join).
  Circle, ellipse and Bézier input see no change, because their input is still
  flattened.
- Cost: under 1 ms. Risk: low.
- Not enough on its own: the customer's main example is the circle.

**(c) Curve-native pieces, i_overlay only for topology (recommended).**
1. Build the exact **raw offset pieces** of the operand. Each line segment
   gives a parallel line. Each cubic gives `kurbo::offset::offset_cubic(c, d,
   tol)` (kurbo 0.13, already a dependency; usually 1-3 cubics). Each corner
   gives its join: Round = arc around the corner (centre and radius known),
   Miter = the extended lines plus the clip line, Bevel = a line. Open paths
   also get caps: a half circle, or lines. Each piece carries an id.
2. Run the current `i_overlay` pipeline unchanged. Its rings say which parts
   survive.
3. **Map back.** Walk each ring and label every vertex with the piece it lies
   on (distance ≤ tolerance, looked up through a bounding-box grid).
   Consecutive vertices with the same label form a run. The run ends are the
   intersection of the two neighbouring pieces: line/line and line/arc
   analytically, cubic/cubic with the existing
   `curve_intersections::cut_parameters` (0036). Where two pieces meet
   tangentially, the run ends at the shared piece endpoint.
4. Emit each run as its piece cut to the run ends
   (`segment::subdivide_at_parameter`). The node is smooth where the tangents
   match and a corner where they do not.
5. **Safety net:** if any emitted curve deviates from its ring by more than
   the tolerance, or any run has no piece, that ring is written as today's
   polyline. The result is never worse than PR #115.
- Segments: circle 4 (8 if R > 37 mm, see below), rounded rect 8, gear about
  200-300 at 2 mm outset (one or two per surviving input cubic plus one arc
  per real corner) and fewer at 5 mm, where the pockets fill (all est.).
  Segment counts barely depend on the tolerance: lines and arcs are exact, and
  only `offset_cubic` subdivides.
- Cost (est.): 1-3 ms for the pieces and 1-3 ms for the mapping, on top of
  today's 20 ms (2 mm) and 55 ms (5 mm). Criterion 3 still holds at 2 mm and
  -1 mm. The 5 mm case stays as overrun as the technical-debt entry already
  says (about 60 ms), and the worker resolution recorded there fixes both.
- Risk: **medium.**
  - Mapping. Pieces that cross at grazing angles give ambiguous labels near
    the crossing. The safety net covers this, and a property test should
    count how often it fires.
  - Determinism (criterion 12). `offset_cubic` uses kurbo's float math
    (`sqrt`, `cbrt`, trigonometry in its solvers). Values agree to about
    1e-12 mm across platforms. The danger is a subdivision decision that
    flips on the last bit and changes the segment count. Our own arcs and
    lines use only `+ - * /` and `sqrt`; for example `tan(θ/4)` for the arc
    handle comes from two half-angle `sqrt` steps. Mitigation: the
    cross-target determinism check of 0016 runs on curved offsets. If it ever
    fails, replace `offset_cubic` with our own subdivide-and-check offset
    (about 150 lines, `+ - * /` and `sqrt` only).
  - Arc accuracy. A cubic per 90° deviates from the circle by 2.7e-4 x R,
    which reaches 0.01 mm at R ≈ 37 mm. Above that, split arcs into 45°
    pieces (error 4.2e-6 x R).

**Recommendation: (c), with (b)'s exact lines and arcs as part of it. (a) is
not used for offsets.** The fitter in (a) has to guess what (c) already knows.
Fitting stays the tool for Boolean results and imports (ADR 0003 §6, the
technical-debt entry); it is not needed here.

## 3. Primitives stay primitives where the result is exactly one

Before the kernel runs, the primitive path in `offset_plan.rs` computes the
result from the primitive's parameters. It needs no flattening and no
`i_overlay`, takes microseconds, is exact, and keeps the primitive's rotation
and centre. A result that is not exactly a primitive goes through (c), which
gives a path of few exact segments (a "path" in the table below).
`r` = effective corner radius (`effective_corner_radii`, the CSS clamping);
`Rect.corner_radii` already exists (`document-core/src/corner_radii.rs`,
spec 0013), so there is no new field.

| Operand | Outset d | Inset \|d\| |
|---|---|---|
| Rect, sharp corners | Miter: rect +d each side, radii 0. Round: rect +d, every radius d. Bevel: path of 8 lines | rect −\|d\| each side, radii 0, every join (convex corners open no gap). Empty when min(w, h) ≤ 2\|d\| |
| Rect, radii r_i | per corner: r_i > 0 → r_i + d with any join (tangent, no gap); r_i = 0 → follows the Join as above (Bevel at any sharp corner → path) | rect −\|d\|, radius max(r_i − \|d\|, 0) per corner (the inset of an arc with r < \|d\| is a sharp corner). Same empty rule |
| Circle (rx = ry) | circle R + d, any join | circle R − \|d\|; empty when R ≤ \|d\| |
| Ellipse (rx ≠ ry) | **not an ellipse.** Path of offset cubics (est. 4-8 cubics), unless the result ellipse (rx + d, ry + d) stays within the tolerance | same; an inset with \|d\| > ry²/rx (the smallest radius of curvature) has sharp points at the ends of the major axis, so it is always a path |
| Regular polygon (N, circumradius R) | Miter: polygon with R + d / cos(π/N) (the apothem grows by d; corners ≥ 60° never reach the limit). Round: path of N lines + N arcs. Bevel: path of 2N lines | polygon R − \|d\| / cos(π/N) with every join; empty when the apothem ≤ \|d\| |
| Star (N, ratio k) | Miter within the miter limit at the tips: the star scaled about its centre by (h + d)/h, same k (all edge lines are at the same distance h from the centre). Past the limit, Round or Bevel: path | Miter within the limit at the inner corners: scaled by (h − \|d\|)/h, same k. Round or Bevel: path (the reflex corners get the join). Empty when h ≤ \|d\| |

**Ellipse error.** Approximating the offset of an ellipse by the ellipse
(rx + d, ry + d) gives this largest error, computed by sampling:

| Ellipse | d = +2 | d = +5 | d = −2 | d = −5 |
|---|---|---|---|---|
| 50 x 25 | 0.11 mm | 0.25 mm | 0.12 mm | 0.33 mm |
| 20 x 10 | 0.10 mm | 0.21 mm | 0.13 mm | — |
| 30 x 25 | 0.008 mm | 0.018 mm | — | — |

The best-fitting ellipse roughly halves this error. That is still 5-25 times
the 0.01 mm tolerance for any visibly oval shape, which is too much for a kerf
offset. **Recommendation:** keep it an ellipse only if the largest error,
sampled deterministically, is at most the kernel tolerance; that happens only
for nearly round ellipses. Otherwise write a path of offset cubics (4-8
segments), which is still clean and still editable.

Spec consequences for primitive results: the result keeps the operand's
rotation and is the same primitive kind. Criterion 11's "rotation 0" applies to
path results only. The write needs an "add primitive above" counterpart to
`add_paths_above` in `curvyo-document-core/src/replace.rs`; the commit label
stays `offset_path`.

## 4. Precision setting

Not in this slice. With (c), lines, arcs and primitives are exact, and only
`offset_cubic` depends on the tolerance, weakly (its error falls with about the
5th to 6th power of the segment length, so halving the tolerance adds about
12 % segments). Most of the density the customer saw disappears without a
setting, and an unused option breaks `CLAUDE.md` §5 ("no speculative config
options").

If the customer still wants one after seeing the rework, it should be an
**app preference "Curve precision"**: range 0.001-0.1 mm, default 0.01 mm,
stored with the app settings and not in the document (so no format change). It
would feed the `Tolerance` of offset, Boolean operations and later refitting.
It should not be a field in the Offset section, because one precision should
hold across all operations. Default if the customer does not answer: no
setting.

## 5. ADR, format and dependency impact

- **ADR:** no new ADR. A dated **amendment to ADR 0003 §4**: offset results
  are curves rebuilt from exact pieces, `i_overlay` decides topology only, and
  the polyline is the fallback. This lifts §4's "results are polylines" for
  offsetting. §2 already names `kurbo` for single-curve offsetting. Mark it
  `needs-customer` as the earlier §4 amendment was; the customer's feedback is
  effectively the approval. `adrs.md` gets the pipeline note, the primitive
  table and the determinism mitigation.
- **Format:** **no `format_version` bump.** Results are ordinary paths with
  cubic segments, or existing primitives with existing fields (corner radii,
  point count, inner ratio).
- **Dependency:** **none new.** `kurbo` 0.13 is already in
  `curvyo-geometry-core`; `offset_cubic` is in its default build (confirm in
  the spike). `i_overlay` stays at `=9.0.1`.
- **Spec (PO):** remove "What the result is: a polyline" and "Out of scope:
  keeping curves as curves". Rewrite criterion 11: curves allowed; smooth
  nodes where tangent; within 0.01 mm of the exact offset of the *original*
  curves for Round (Miter and Bevel at chord corners no longer apply); a
  segment budget such as "a circle gives 4 segments, a straight edge 1, a
  round join 1 per 90°". Rewrite criterion 20 (circle → circle primitive).
  Add primitive criteria from §3. Tie criterion 12 to the new determinism
  check.

## Plan (for the implementer, on `story/path-offset`)

1. Spike on `spike/offset-pieces` (not merged): `offset_cubic` on the gear,
   piece mapping, segment counts and timings. Record the numbers here.
2. Primitive fast path (§3), test-first from the tables.
3. Pieces + mapping + safety net in `geometry-core` (new module
   `offset_pieces.rs`; `offset.rs` stays the entry point).
4. Determinism check on curved offsets; budget test prints segment counts.
