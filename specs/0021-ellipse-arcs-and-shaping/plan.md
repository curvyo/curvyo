# Plan for "Arc and curve handles: ellipse arcs, and a Curve handle on ellipse, polygon and star"

Built to `specification.md` with the customer answers of 2026-10-17 (PR #144):
ARC-KIND = A (a new arc is an open Arc), ARC-BOX = A (the box is the box of the
drawn part), a non-uniform scale keeps an ellipse arc (also rotated, `adrs.md`
decision 9), a skew converts to a path. `format_version` 15 (the next free one
on `main` at the time of the build; kept in one place, `CURRENT_FORMAT_VERSION`
and the fixtures, so a rebase can renumber it).

## Affected crates/modules

- `curvyo-document-core`: `primitive_model.rs` (the `Curve` newtype, `EllipseArc`,
  `ArcKind`, the new fields of `Shape::Ellipse`, `Shape::Polygon`, `Shape::Star`,
  the tight frame box), new `ring_curve.rs` (the curved ring: vertices, side arcs,
  middles, `k_min`, cubic control points, the inverse map of the handle), new
  `ellipse_arc.rs` (arc outline, box, angles, the arc under a linear map, the
  SVD of decision 9), `primitive_outline.rs` (`PrimitiveOutline { anchors,
  closed }`, the outline of an arc and of a curved shape, the fine outline the
  renderer flattens), new `shape_arc.rs` (commands `set_ellipse_arc`,
  `set_curve`), new `shape_arc_codec.rs` (the registers `ellipse_arc` and
  `curve`, validation), `shape_codec.rs`, `shapes.rs` (an open Arc converts to an
  open path), `document.rs` (format 15), `history_names`.
- `curvyo-render-core`: artwork and previews read `PrimitiveOutline` (an open
  arc is stroked open and filled closed).
- `curvyo-ui-core`: `param_handles.rs` (`ArcStart`, `ArcEnd`, `Curve`, the
  layout and the knob displacement), `param_edit.rs` (pointer to value,
  `apply_param`, commit), `param_entry.rs` (the "°" and "%" chips),
  `select_bar.rs` (ellipse group and curve group), `mirror.rs`,
  `group_transform.rs` and `conversion.rs` (stretch keeps an ellipse, skew
  converts), `transform_primitive.rs` (resize of an arc box), hit testing of an
  open arc.
- `curvyo-editor-wasm`: the bar surface, the readouts, the cursor names, the
  live-region text.
- `frontend`: the ellipse group and the Curve field of the Select bar, the entry
  chips.
- `docs/design-system.md`: the rows the UX notes name.

## Milestones and tasks

### M1 document-core: model, evaluation, format (one commit)

- [x] `Curve` newtype (fraction -1..=1), `EllipseArc { start, end, kind }`,
      `ArcKind`; new fields on the three shapes; every match site (17, 35).
- [x] `ring_curve.rs`: ring of M vertices, `phi`, chord, side arc, middle
      `m(k)`, `k_min`, effective curve, Bézier of a side (split above 90°),
      inverse map of the handle; property test of no self-crossing (18-21, 36-39,
      46, 48, 49, 51).
- [x] `ellipse_arc.rs`: angle normalisation and whole test (5, 64), outline of
      the three kinds split into `ceil(S/90°)` cubics (4, 13), tight box (9),
      the arc under a linear map (mirror, 54) and under any affine map by SVD
      (56).
- [x] `PrimitiveOutline { anchors, closed }`; every caller; a fine outline for
      drawing within 0.01 mm (67).
- [x] codec: `ellipse_arc` list register and `curve` register, deleted at the
      default; validation (14, 15, 32, 33, 50); `ALL_PRIMITIVE_KEYS`.
- [x] commands `set_ellipse_arc`, `set_curve` (one commit, nothing written when
      unchanged), history names "Change arc", "Change curve" (59).
- [x] Object to path: an open Arc becomes an open path (13, 29, 30, 48).
- [x] `format_version` 15, golden fixtures, damaged fixtures (14, 15, 33, 50);
      LWW test of a concurrent set and delete (decision 1).

### M2 ui-core: handles, values, bar, transforms (one commit)

- [x] `ParamHandle::{ArcStart, ArcEnd, Curve}` and their layout with the knob
      displacement rule (1, 22, 40, 53).
- [x] pointer to value, Ctrl snapping, snapping to the stops (2, 5, 7, 23, 41).
- [x] `apply_param` for arc angles, arc kind, whole, curve with the per-shape
      limit (3, 26, 31, 38, 44, 46, 63, 64).
- [x] entry chips (8, 24, 42).
- [x] bar state: ellipse group, curve group, greyed rules (3, 26, 31, 44, 52,
      65, 66).
- [x] resize of an arc box (10), rotation (11, 53), mirror (54), group stretch
      by SVD and skew conversion (56, 63), linked clone converts (55), copies
      (57).
- [x] hit test: open arc is the stroke only, pie and chord the closed outline
      (12, 28, 47).
- [x] benchmarks (62 a, b), `#[ignore]` (`tests/arc_curve_budget.rs`); (c) to (e) not measured.

### M3 editor-wasm and frontend (one commit)

- [x] wasm surface: bar view fields, setters for kind, whole, start, end,
      curve (preview, commit, cancel); readouts "start 30.0° · sweep 120.0°" and
      "curve -35.0 %" (6, 25, 43); live-region text (65).
- [x] `SelectToolbar.tsx`: ellipse group (two rows), curve group, tooltips,
      greyed controls (3, 26, 44, 65, 66).
- [x] chips for "Arc start", "Arc end", "Curve" (8, 24, 42).
- [x] design-system rows; `specs/README.md` format table row.

## Validation

- Test first in `curvyo-document-core` and `curvyo-ui-core` for every formula
  of the spec (check values of criteria 19, 29, 37, 38; equality of the
  polygon with N = 4 and the circle, criterion 51; the 64-sample mirror and
  stretch tests of criteria 54 and 56).
- Golden fixtures in `curvyo-document-core/tests/fixtures/` for all three arc
  kinds, the curves on circle and ellipse, polygon and star.
- Session tests in `curvyo-editor-wasm/tests/` for drag, entry, bar and history.
- Full gate of `CLAUDE.md` §7 plus the frontend checks.

## Notes from the build (2026-10-19)

- `PrimitiveOutline` dereferences to its anchors, so the many callers that only walk the
  nodes are unchanged; every caller that draws, hits, bounds or converts reads `closed`.
- `drawing_outline_of` splits arcs into pieces of at most 22.5° for the renderer
  (criterion 67); a whole ellipse at 100 % and straight polygons draw as before.
- `Document::resize_ellipse_with_arc` (new, additive) writes a resize that mirrors an arc
  in one commit with the label `resize_ellipse`; `transform_objects` writes a changed arc.
- The Proposal of criterion 56 (a rotated whole ellipse no longer converts on a group
  stretch) is not built: it changes `0047` criterion 7 and its tests; only an ellipse with
  an arc keeps its kind on a stretch. A curved ellipse converts like a rectangle.
- Knob displacement searches up to two pitches (28 px), not one: a knob exactly on a
  corner glyph cannot clear 15 px with 14.
- The kind buttons are a `role="group"` of `aria-pressed` buttons, not a `radiogroup`.
- Tester tests of earlier slices that assert "no parameter handle on an ellipse or a
  polygon" or a constant glyph count from 72 px now fail by criteria 1, 22 and 40; they are
  left for the tester.
