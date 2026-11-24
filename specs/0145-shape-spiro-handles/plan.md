# Plan for "Shape loops: a Rounded handle on polygon, star and ellipse"

## Slice 2: ellipse only (Frage 326 at default B; criteria 14, 15, 16, 19)

Built to `adrs.md` decision 9 and `ux-notes.md` section 9. The rectangle keeps its corner radii
(`0013`); no rectangle work. Slice 1's `Rounded`, `rounded_outline`, `set_rounded`, knob, edit,
bar and SVG code are extended, not duplicated. `format_version` 57 (renumbered at rebase if
`main` moved; it lives in `CURRENT_FORMAT_VERSION` and the golden fixture only).

### Affected crates/modules

- `curvyo-document-core`: `primitive_model.rs` (`rounded` on `Shape::Ellipse`, frame box),
  `ring_rounded.rs` (handles scaled by the ellipse's `(rx, ry)`, `effective_rounded`, `Q_C`),
  `primitive_outline.rs` (the Rounded arm for an ellipse without an arc, after the arc arm),
  `ring_curve.rs` (`effective_curve` is 0 while Rounded is set), `shape_arc.rs` /
  `shape_arc_codec.rs` / `shape_codec.rs` (`set_rounded` accepts an ellipse, Curve zeroes
  Rounded on an ellipse, register read and write, the ellipse with an arc ignores a stored
  `q`), `document.rs` (format 57, snapshot field), `shapes.rs` (Object to path).
- `curvyo-svg-core`: nothing (Inkscape has no live ellipse with Rounded).
- `curvyo-ui-core`: `svg_export_shapes.rs` / `svg_import_shadow.rs` (a rounded ellipse
  exports as a path with the shadow, `q` appended), `offset_primitive.rs`,
  `transform_primitive.rs`, `param_edit.rs`, `param_handles.rs`, `shape_knobs.rs`
  (knob on E, rest at `q_c`, `q_c` tick), `shape_rounded_edit.rs` (snap to `q_c`, no write at
  `q_c` when untouched), `select_bar_arc.rs` (field for an ellipse, greyed with an arc).
- `curvyo-editor-wasm`: bar surface, readout, CPU-time test for the ellipse.
- `frontend`: the Rounded field for an ellipse, greyed with the arc note.
- `docs`: `design-system.md`, `technical-debt.md`.

### Tasks

M1 document-core (one commit, tests first):
- [ ] 1. `Rounded` on an ellipse: unit 4-ring E, S, W, N then affine scale; `Q_C = KAPPA/sqrt(2)`
  gives exactly today's `ellipse_outline` within 0.05 % of `rx`, `ry`; a typed 0.39 too
  (fulfils AC 15)
- [ ] 2. Arc wins: an ellipse with an arc ignores a stored `q`; `set_rounded` writes it anyway
  but the outline is the arc (fulfils AC 19)
- [ ] 3. Register `rounded` on the ellipse, deleted at 0, absent is 0, non-finite is Damaged;
  `format_version` 57 plus a generated golden project; both Curve and Rounded non-zero is
  valid, Rounded wins (fulfils AC 14, 16)
- [ ] 4. `set_rounded` zeroes Curve on an ellipse in the same commit and `set_curves` zeroes
  Rounded; undo restores both (fulfils AC 14)
- [ ] 5. Stays a live ellipse over-range, bounds are the union with the exact Bezier box, Object
  to path keeps the loops, the 4096-point and the finiteness property tests (fulfils AC 16)

M2 svg (one commit with M1 or M3): a rounded ellipse exports as a path with the shadow (`q`
appended, the older number forms still read as `q = 0`) and imports back as a live ellipse
(fulfils AC 16).

M3 ui-core and wasm: the knob on the E ray 15 px inside, `U = rx on screen - 15`, rest at `q_c`
while untouched (`q = 0`, Curve 100 %), `q_c` tick and snap within 2 percent, release at `q_c`
writes nothing, arc knobs vanish while Curve is held at 0, bar field state for an ellipse and
greyed with an arc, entry chip; CPU-time test in `curvyo-editor-wasm/tests/cpu_time/`, thread
CPU time, min of several runs, budgets only when `TIMED` (fulfils AC 14, 15, 16, 19).

M4 frontend: the field for the ellipse and its greyed state, design-system rows.

Built as three commits (plan, Rust, frontend).

## Slice 1 (Done, PR #461)

(Original title: Plan for "Shape loops: a Rounded handle on polygon and star".)


Slice 1 only: polygon and star (criteria 1 to 13, 19 and 20 as they apply: 19 and 20 concern
slice 2 and are not built). No rectangle, no ellipse, no Randomized. Built to `adrs.md`
(decisions 1 to 7) and `ux-notes.md` sections 1 to 8, 10, 12. `format_version` 56 (next free on
`main` at the time of the build; kept in `CURRENT_FORMAT_VERSION` and the fixtures only, so a
rebase can renumber it).

### Affected crates/modules

- `curvyo-document-core`: new `ring_rounded.rs` (`Rounded` newtype, `rounded_outline`, exact
  bounds), `primitive_model.rs` (`rounded` on `Shape::Polygon` and `Shape::Star`, frame box),
  `primitive_outline.rs` (outline arm ahead of Curve), `shape_arc_codec.rs` / `shape_codec.rs`
  (register `rounded`), `shape_arc.rs` (`set_rounded`, exclusion with Curve in one commit),
  `shapes.rs` (Object to path), `document.rs` (format 56, snapshot field).
- `curvyo-svg-core`: `shapes.rs` (`inkscape:rounded` accepted, `StarData.rounded`).
- `curvyo-ui-core`: `svg_import_shapes.rs`, `svg_export_shapes.rs`, `svg_import_shadow.rs`,
  `offset_primitive.rs`, `transform_primitive.rs` (treat `rounded` like `curve`),
  `param_edit.rs`, `param_handles.rs`, `shape_knobs.rs`, `shape_rounded_edit.rs` (new:
  pointer to value, snaps), `select_bar_arc.rs` (field state), `param_entry.rs`.
- `curvyo-editor-wasm`: bar surface, readout, cursor name, CPU-time test (criterion 12).
- `frontend`: the Rounded field after Curve (polygon/star), entry chip.
- `docs`: `design-system.md` rows (ux-notes section 12), `technical-debt.md`, `requirements.md`.

### Tasks

M1 document-core (one commit, tests first):
- [x] 1. `Rounded` newtype and `rounded_outline` per `adrs.md` decision 1, hand-computed
  goldens (hexagon R=10 q=0.3 handles 3 mm along t_i; star), rotation by 360/N invariance, every
  vertex on the outline (fulfils AC 4)
- [x] 2. Property test, 1000 seeded combinations N 3..1024, Ratio 0.01..0.99, q -10..10: finite,
  closed (fulfils AC 13)
- [x] 3. Register `rounded`: stored raw, deleted at 0, non-finite is Damaged, limited on use,
  absent is 0; `format_version` 56 + generated golden project; Curve and Rounded both non-zero is
  valid, Rounded wins (fulfils AC 5, 11)
- [x] 4. `set_rounded` command: one commit "Change rounding", zeroes Curve in the same commit;
  `set_curves` zeroes Rounded likewise; undo restores both (fulfils AC 3, 5)
- [x] 5. Stays a live primitive over-range (Points change keeps q) (fulfils AC 6)
- [x] 6. Bounds: union of today's box and the exact Bezier box; q = 0 unchanged; 4096 sampled
  points inside (fulfils AC 7)
- [x] 7. Object to path: M smooth nodes, cubic handles, loops kept; q = 0 unchanged (fulfils AC 9)
- [x] 8. Hit/fill: nonzero fill, click inside a loop selects (outline-based, existing path) (AC 8)

M2 svg (one commit): import `inkscape:rounded` live when the outline matches within 0.001 mm,
else path; export writes it for Curve 0; shadow appends q (old 7/8-number forms read as q = 0);
hand-computed golden fixture marked provisional, TODO linked to ISSUE-NEEDED (fulfils AC 18, 4).

M3 ui-core and wasm (one commit): `ParamValue::Rounded`, `ParamHandle::Rounded`, knob on the
vertex-0 ray at 15 px with U = D - 15, relative drag, snaps (0, 1 within max(0.015, 3 px/U);
Ctrl 0.25), limit +-10, one commit; bar state and field value; entry chip; readout; cpu_time test
(fulfils AC 1, 2, 3, 12).

M4 frontend (one commit): Rounded field after Curve, entry chip, design-system rows.

Built as three commits (plan, Rust, frontend). Not built: minor ticks under Ctrl, the Drag ghost
render of the looped shape, the centre handle yielding on hover (`docs/technical-debt.md`).

### Validation

Hand-computed golden fixtures (provisional until the customer exports them from Inkscape 1.4,
`adrs.md` decision 7); property test; CPU-time test (thread CPU time); full gate of `CLAUDE.md` §7.
Not verifiable here: the visual match to Inkscape's output and the canvas feel of the drag.
