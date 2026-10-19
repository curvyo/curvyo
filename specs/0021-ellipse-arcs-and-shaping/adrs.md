# ADRs for "Arc and curve handles: ellipse arcs, and a Curve handle on ellipse, polygon and star"

This feature adds optional parameters to three existing primitives (an arc and a
curve on the ellipse, a curve on the polygon and the star) and one shared
evaluation. Every other piece is machinery that exists (`ParamHandle`,
`apply_param`, the Select bar groups, the entry chip, `oriented_bounds`, Object
to path). **No new crate, no new dependency, no new ADR.** As with
`specs/0013-rectangle-corner-radii/adrs.md`, the change extends ADR 0002 §5 and
ADR 0009 §3 by registers of the same kind and introduces no new document-model
concept, so it is not `needs-customer` (`CLAUDE.md` §3). `format_version`
increases once, at merge.

Reference state: `main` with `CURRENT_FORMAT_VERSION` 14; `Shape::Ellipse
{ frame: EllipseFrame }`, `Shape::Polygon`, `Shape::Star` in
`curvyo-document-core/src/primitive_model.rs`; `ellipse_outline` in
`primitive_outline.rs` returns four `OutlineAnchor`s. Read, not built, on
2026-10-17.

## Depends on

- [ADR 0002 §3, §5, §9](../../docs/adr/0002-document-model-units-and-svg-round-trip.md):
  units are types (the arc angles use the existing angle newtype, the curve a
  new dimensionless `Curve` newtype holding a fraction from -4 to 4, the customer's range of
  2026-10-19); a primitive's
  parameters are keys in its meta map; one interaction is one commit.
- [ADR 0004 §9](../../docs/adr/0004-persistence-and-cross-machine-sync.md): a newer
  file is refused, never partly read (decision 3).
- [ADR 0009 §3](../../docs/adr/0009-concurrent-editing-semantics.md): per-register
  last-writer-wins; an unchanged register is never written.
- [ADR 0014](../../docs/adr/0014-history-undo-and-branches.md): one commit, one
  step; names come from `history_names.rs` (decision 5).
- [ADR 0015](../../docs/adr/0015-node-references-and-ancestor-resolved-state.md) and
  [ADR 0016](../../docs/adr/0016-clone-overrides.md): clones resolve to a path or
  compound path, so a primitive with arc or curve is converted first
  (`specs/0049-linked-clone/` criterion 2); this spec adds nothing to clones.
- [`specs/0003-primitive-shapes/adrs.md`](../0003-primitive-shapes/adrs.md): stored
  raw, clamped where evaluated; Object to path strips every primitive key.
- [`specs/0009-unified-object-editing/adrs.md`](../0009-unified-object-editing/adrs.md):
  one registry, one hit rule, `apply_param` for drag, entry and bar.
- [`specs/0013-rectangle-corner-radii/adrs.md`](../0013-rectangle-corner-radii/adrs.md):
  the habit for new registers (absent means default, unchanged guard, validation
  by `Damaged`, a generated golden fixture per version).
- [`specs/0056-mirror-and-negative-scale/adrs.md`](../0056-mirror-and-negative-scale/adrs.md)
  (D3: a mirrored primitive stays a primitive) and
  [`specs/0047-transform-polish/adrs.md`](../0047-transform-polish/adrs.md): the arc
  is mirrored by the angle rule of criterion 54; skew converts through the
  evaluation of decision 4; a non-uniform scale keeps an arc (decision 9).
- [`specs/0050-layers-and-objects-panel/adrs.md`](../0050-layers-and-objects-panel/adrs.md):
  hidden and locked objects are never in the selection; nothing to build here.

## Feature-local decisions

- **1. 2026-10-17: stored representation.** (A) chosen: for the ellipse one list
  register `ellipse_arc = [start, end, kind]` (angles in degrees as `f64`, kind a
  string `pie`, `arc`, `chord`); absent means whole. For all three shapes one flat
  register `curve` (a fraction, `f64`); absent means the shape's default (ellipse
  1.0, polygon and star 0.0). (B) three flat arc registers, rejected: "whole" would
  need three deletes and a half-written arc would be a state to define, while one
  list is replaced as a unit and a half-merge of start from one peer and kind from
  another is still a valid arc (two peers rarely drag both handles). (C) a nested
  map, rejected as in `0013` decision 1. Returning to the default (Whole ellipse,
  curve 100% on an ellipse, 0% on a polygon or star) **deletes the key** so an
  untouched shape is byte for byte what it was (criteria 14, 32, 50); Object to path
  already deletes primitive keys, so deletes are an existing operation. The build
  confirms with a test that a concurrent set and delete of one key resolve by LWW
  and the object reads valid either way.
- **2. 2026-10-17: validation and the arc/curve conflict.** On open: non-finite or
  out-of-range curve, non-finite angle, a kind outside the three, an arc list of
  the wrong length, or an arc together with a curve other than the ellipse default
  is `Damaged` with a named error (criteria 33, 50). A live merge that yields both
  is not refused: evaluation lets the arc win and ignores the curve until the arc
  is made whole (the spec's suggestion); that state is never written by one peer.
  The stored curve is raw; the effective one is `max(stored, k_min)` on a polygon
  or star, computed in the evaluation (criterion 38).
- **3. 2026-10-17: `format_version`.** One bump for Parts A and B together, the next
  free number at merge (14 today; 0058 may take 15 first). Reason: an older build
  would draw an arc as a whole ellipse and a curved polygon as straight, which for a
  cutting tool is a wrong cut, so it must refuse the file (criteria 15, 33, 50).
  The manifest version rises with the writer as in every earlier bump, so a file
  saved by the new build is refused whether or not it holds an arc or curve; the
  `CURRENT_FORMAT_VERSION` comment gets its paragraph, `future_format_version.curvyo`
  stays newer than current. Fixtures: `arc_pie_v*.curvyo`, `arc_arc_v*.curvyo`,
  `arc_chord_v*.curvyo`, `curve_ellipse_v*.curvyo`, `curve_polygon_star_v*.curvyo`,
  and damaged ones (arc with curve, curve 2.0, unknown kind). `specs/README.md`
  "format_version plan" has the row.
- **4. 2026-10-17: one shared evaluation, in `curvyo-document-core`.** New module
  `ring_curve.rs` ("Evaluation of a curved ring of vertices: side arcs, middles,
  the curve limit and the cubic control points"; pure, no Loro, wasm32-clean) and
  `ellipse_arc.rs` ("The outline, box and cubic form of an elliptical arc");
  `primitive_outline.rs` (now 400+ lines with tests) calls them and does not grow
  by their code. The ellipse is the 4-ring (vertices at the cross nodes), so
  criterion 51 holds by construction. `ellipse_outline` and the polygon and star
  outline return a small `Outline { anchors, closed }` (an open Arc is not closed);
  the implementer checks every caller of the present `Vec<OutlineAnchor>`.
  `render-core` flattens the true circular arcs (criterion 67) from the same
  functions; `ui-core` takes handle positions, the box and the inverse maps
  (criteria 23, 41) from the same functions; no copy of the formulas anywhere.
  Rejected: evaluation in `geometry-core` (it has no primitives) and one copy per
  consumer (the outline and the converted path would drift).
- **5. 2026-10-17: ui-core, wasm and history.** `ParamHandle` gains `ArcStart`,
  `ArcEnd`, `Curve`; their layout, including the "Knob displacement" rule of the UX
  notes, is one function in `param_handles.rs`; `apply_param` resolves drag, entry
  and bar. The bar gets one new `select_bar` group for the ellipse and a field in a
  curve group; the frontend renders what the bar model says (no logic in
  TypeScript). Document commands: `set_ellipse_arc`, `set_curve`, in a new
  `shape_arc.rs` ("Document commands that write an ellipse's arc and a shape's
  curve"), because `shapes.rs` is over 500 non-test lines. `history_names.rs` gets
  `set_ellipse_arc` and `set_curve` mapped to "Change arc" and "Change curve"
  (criterion 59). The step is written only when a value changes.
- **6. 2026-10-17: Object to path nodes.** The cubic construction of criteria 13,
  29 and 48 is one function (arc to cubics, with the 0.1 % bound), tested against
  the true arc; Symmetric nodes between arc segments, Corner at ends (criterion 13)
  and at every vertex of a polygon or star away from 100 % (criterion 49). The
  converted path keeps the `NodeId` and strips the new keys with the others
  (`ALL_PRIMITIVE_KEYS` gains `ellipse_arc` and `curve`).
- **7. 2026-10-17: mirror.** `0056`'s primitive mirror writer calls a pure function
  `mirror_arc(arc, flip)` (start' = image of end, end' = image of start; own-axis
  and document-axis cases of criterion 54) and leaves the curve alone. Tested by
  sampling the outline (64 points) and by flipping twice.
- **9. 2026-10-17: non-uniform scale keeps the arc, skew converts (customer).**
  The ellipse model (centre, `rx`, `ry`, rotation) represents the image of an
  ellipse under any affine map exactly, so no new parametrisation is needed. A
  pure function `ellipse_arc_under_affine(frame, rotation, arc, linear_part)`
  does the singular value decomposition of `M * R * diag(rx, ry)` and returns
  the new frame, rotation and arc (both angles shifted by the angle of `V`;
  `mirror_arc` on top when the determinant is negative). Called by the group
  stretch of `0019`/`0047` for an ellipse with an arc instead of the conversion
  of decision 4; skew still converts (`0047`). Curved ellipse, polygon, star
  keep converting on stretch. Rejected: converting on any stretch of a rotated
  arc (loses the arc, the customer asked for it to work); storing a skew in the
  ellipse (a new concept in the document model, against the skew rule of `0008`).
- **8. Not decided here, left to the build:** the real bar row heights (UX notes,
  "Open for the build review"), the file split of `shape_codec.rs` if the new
  keys push it over 500 lines (move the arc and curve codec to `shape_arc_codec.rs`
  in a pure-move first commit, as `0013` did), and the budgets of criterion 62
  (the implementer may propose other numbers with a measurement).

## ADR index

No new ADR file. Nothing in `docs/adr/` changes.

## Licence

No new dependency, so nothing to check with `cargo deny` and no licence
question. The circular-arc-to-cubic construction is written from the standard
formulas (criteria 29, 48) in our own code.
