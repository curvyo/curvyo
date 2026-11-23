# Plan for "Shape loops: a Rounded handle on polygon and star" (slice 1)

Slice 1 only: polygon and star (criteria 1 to 13, 19 and 20 as they apply: 19 and 20 concern
slice 2 and are not built). No rectangle, no ellipse, no Randomized. Built to `adrs.md`
(decisions 1 to 7) and `ux-notes.md` sections 1 to 8, 10, 12. `format_version` 54 (next free on
`main` at the time of the build; kept in `CURRENT_FORMAT_VERSION` and the fixtures only, so a
rebase can renumber it).

## Affected crates/modules

- `curvyo-document-core`: new `ring_rounded.rs` (`Rounded` newtype, `rounded_outline`, exact
  bounds), `primitive_model.rs` (`rounded` on `Shape::Polygon` and `Shape::Star`, frame box),
  `primitive_outline.rs` (outline arm ahead of Curve), `shape_arc_codec.rs` / `shape_codec.rs`
  (register `rounded`), `shape_arc.rs` (`set_rounded`, exclusion with Curve in one commit),
  `shapes.rs` (Object to path), `document.rs` (format 54, snapshot field).
- `curvyo-svg-core`: `shapes.rs` (`inkscape:rounded` accepted, `StarData.rounded`).
- `curvyo-ui-core`: `svg_import_shapes.rs`, `svg_export_shapes.rs`, `svg_import_shadow.rs`,
  `offset_primitive.rs`, `transform_primitive.rs` (treat `rounded` like `curve`),
  `param_edit.rs`, `param_handles.rs`, `shape_knobs.rs`, `shape_rounded_edit.rs` (new:
  pointer to value, snaps), `select_bar_arc.rs` (field state), `param_entry.rs`.
- `curvyo-editor-wasm`: bar surface, readout, cursor name, CPU-time test (criterion 12).
- `frontend`: the Rounded field after Curve (polygon/star), entry chip.
- `docs`: `design-system.md` rows (ux-notes section 12), `technical-debt.md`, `requirements.md`.

## Tasks

M1 document-core (one commit, tests first):
- [ ] 1. `Rounded` newtype and `rounded_outline` per `adrs.md` decision 1, hand-computed
  goldens (hexagon R=10 q=0.3 handles 3 mm along t_i; star), rotation by 360/N invariance, every
  vertex on the outline (fulfils AC 4)
- [ ] 2. Property test, 1000 seeded combinations N 3..1024, Ratio 0.01..0.99, q -10..10: finite,
  closed (fulfils AC 13)
- [ ] 3. Register `rounded`: stored raw, deleted at 0, non-finite is Damaged, limited on use,
  absent is 0; `format_version` 54 + generated golden project; Curve and Rounded both non-zero is
  valid, Rounded wins (fulfils AC 5, 11)
- [ ] 4. `set_rounded` command: one commit "Change rounding", zeroes Curve in the same commit;
  `set_curves` zeroes Rounded likewise; undo restores both (fulfils AC 3, 5)
- [ ] 5. Stays a live primitive over-range (Points change keeps q) (fulfils AC 6)
- [ ] 6. Bounds: union of today's box and the exact Bezier box; q = 0 unchanged; 4096 sampled
  points inside (fulfils AC 7)
- [ ] 7. Object to path: M smooth nodes, cubic handles, loops kept; q = 0 unchanged (fulfils AC 9)
- [ ] 8. Hit/fill: nonzero fill, click inside a loop selects (outline-based, existing path) (AC 8)

M2 svg (one commit): import `inkscape:rounded` live when the outline matches within 0.001 mm,
else path; export writes it for Curve 0; shadow appends q (old 7/8-number forms read as q = 0);
hand-computed golden fixture marked provisional, TODO linked to ISSUE-NEEDED (fulfils AC 18, 4).

M3 ui-core and wasm (one commit): `ParamValue::Rounded`, `ParamHandle::Rounded`, knob on the
vertex-0 ray at 15 px with U = D - 15, relative drag, snaps (0, 1 within max(0.015, 3 px/U);
Ctrl 0.25), limit +-10, one commit; bar state and field value; entry chip; readout; cpu_time test
(fulfils AC 1, 2, 3, 12).

M4 frontend (one commit): Rounded field after Curve, entry chip, design-system rows.

## Validation

Hand-computed golden fixtures (provisional until the customer exports them from Inkscape 1.4,
`adrs.md` decision 7); property test; CPU-time test (thread CPU time); full gate of `CLAUDE.md` §7.
Not verifiable here: the visual match to Inkscape's output and the canvas feel of the drag.
