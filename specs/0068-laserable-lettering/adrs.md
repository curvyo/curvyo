# ADRs for "Weld"

Weld is the n-ary union of 0016, followed by a morphological closing that uses
the integer offsetter of 0038. Open outlines contribute a stroke band built by
the same library. **No new ADR, no new crate, no new dependency, no
`Cargo.toml` change and no `format_version` bump.** The result is an ordinary
path or compound path. The change adds no trait and no generic.

## Depends on

- [ADR 0003 §1, §3, §4, §8](../../docs/adr/0003-geometry-kernel-booleans-offsetting-vcarving.md):
  the kernel runs on flattened polygons on the 0.001 mm grid with `i_overlay`
  `=9.0.1` in its `i64` engine. Results are polylines. Offsetting uses
  `i_overlay`'s integer offsetter (§4 amendment of 2026-10-10). The curve
  rebuild of the 2026-10-12 amendment applies to Offset only, not to Weld.
- [ADR 0002 §5, §9](../../docs/adr/0002-document-model-units-and-svg-round-trip.md):
  the result takes the lowest operand's place in sibling order and is written
  in one commit.
- [ADR 0014](../../docs/adr/0014-history-undo-and-branches.md): one step,
  labelled `weld`. Undo restores the texts and groups.
- [ADR 0015](../../docs/adr/0015-node-references-and-ancestor-resolved-state.md):
  a linked clone contributes the resolved path it shows. Operands are read in
  world coordinates.
- [ADR 0020 §1, §5](../../docs/adr/0020-text-nodes-text-engine-and-font-sources.md)
  and [`specs/0074-text-objects/adrs.md`](../0074-text-objects/adrs.md): a
  text enters a path command as the compound path of its glyphs. A substitute
  face is refused.
- [ADR 0004 §6, §7, §12](../../docs/adr/0004-persistence-and-cross-machine-sync.md):
  the scope of what is synced. The Gap value is outside that scope (see
  decision 7).
- [ADR 0011](../../docs/adr/0011-workspace-and-crate-layout.md): the crates
  are unchanged.
- [`specs/0016-boolean-operations/adrs.md`](../0016-boolean-operations/adrs.md):
  `grid_paths`, `normalize`, `cleanup`, `canonical_millimetres`, the refusal
  codes with offenders, `MAX_COORDINATE_MM`, and fixtures (a) to (i).
- [`specs/0037-fracture-and-flatten/adrs.md`](../0037-fracture-and-flatten/adrs.md):
  `connected_pieces`, `replace_with_pieces`, and the `boolean-budgets` job.
- [`specs/0038-path-offset/adrs.md`](../0038-path-offset/adrs.md): the
  integer outline offset (`polyline_offset`), the arc step, the miter limit
  constant, and `miter_min_turn`.
- [`specs/0054-combine-open-paths/adrs.md`](../0054-combine-open-paths/adrs.md):
  the fill area of an open outline is closed by a straight line.

## Feature-local decisions

1. **2026-10-20: shared engine: the union of 0016, not the visible-region
   fold.**
   - `boolean.rs` gets a crate-private `union_region(regions: &[Paths64]) ->
     Paths64`. It is steps 4 and 5 for `Union`: each operand is normalized,
     then one n-ary overlay runs. `boolean(BooleanOp::Union, ..)` and `weld`
     both call it, so the project has one union algorithm (criterion 30).
   - Rejected: the fold of `visible_region.rs` with its edges dissolved. It
     gives the same region by the spec's own argument, but it takes 2n
     overlays instead of one and computes regions per operand that Weld
     throws away. It also needs a final union anyway.

2. **2026-10-20: gap closing on the grid.** In `weld.rs`, the steps run in
   this order:
   - `U = union_region(contributions)`.
   - `r = round(g / 2 / 0.001 mm)` grid units.
   - `C = outline(outline(U, +r, Round), -r, Round)`. This uses the integer
     `IntOutlineOffset` of `polyline_offset` with `MathMode::Integer`. That
     function is split into a crate-private helper that takes a `Paths64`.
   - `R = run(Union, C, U)`.
   - Then `cleanup` and `canonical_millimetres`, once.

   The rounded arcs have their vertices on the circle, so the chords lie
   inside it and the closing can cut a convex corner by up to one sagitta.
   The final `∪ U` therefore makes "U lies inside the result" exact on the
   grid (criterion 15) and restores the corners of convex operands
   (criterion 16).

   The arc step is `2 x acos(1 - 0.004 mm / r)`, clamped by the library to
   0.35° to 45°. That keeps every chord within 0.004 mm, so the outer bound
   is g/2 + 0.004 mm + the flattening error, which is under g/2 + 0.01 mm.

   When `r = 0` (g = 0, or g < 0.001 mm), both offsets and the extra union are
   skipped, and the result is exactly the Union region (criterion 18).

   Angles are computed from `sqrt` and constants only, as in 0038, so the
   result is the same on every target (criterion 29).

3. **2026-10-20: stroke band of open outlines: `i_overlay`'s
   `IntStrokeOffset`.** It goes in a new kernel module, `stroke_band.rs`,
   which the later Stroke to path command reuses.
   - Each open outline is flattened with `tolerance - 2 x GRID` and snapped.
     Then `stroke(width, closed = false)` runs with
     `IntStrokeStyle`:
     - join: Round → Round; Bevel → Bevel; Miter →
       `Miter(MITER_LIMIT_ANGLE)`, the SVG limit of 4, which is the
       `MITER_LIMIT` of `render-core`'s lyon stroke. The constant is shared
       from `offset_polyline.rs`, never copied.
     - cap: Butt, Round or Square.
     - `miter_min_turn` and the arc step as in decision 2.
   - The band is already a union, and it enters `union_region` as one more
     region.
   - Dashes are ignored.
   - Rejected:
     - `kurbo::stroke`: it is float-only, and its output would still need a
       union and a determinism check.
     - The lyon tessellation of `render-core`: it is f32 triangles at
       display tolerance, and it would point the dependency from geometry
       to rendering.
   - Per outline, `ui-core` decides what it contributes:
     - a closed outline always gives its region;
     - an open one gives its fill area when Fill is on or Stroke is off;
     - an open one gives its band when Stroke is on.
   - An operand with no area is dropped and reported for the notice. Only
     an empty `U` is refused (`EmptyResult`, criterion 28).

4. **2026-10-20: collecting the operands (`ui-core/src/weld_plan.rs`).**
   - The selection is expanded to leaves with `Document::leaves_of`, at any
     group depth. The leaves are read in world coordinates, from the
     session's resolved snapshots, with clones resolved.
   - `texts_refusal` comes first and refuses all or nothing (criterion 27).
     Then `texts_as_paths` turns each text into its glyph compound path under
     the text's id. That is the one conversion of 0074. Object to path's
     intermediate group is never written.
   - Operands are sorted by stacking order, never by selection order.
   - The base operand is the lowest leaf, which gives the style. The place is
     the lowest selected top-level object, inside the entered group if there
     is one.
   - The write is `replace_with_pieces` (0048/0037) with one piece, and
     `removed` = every selected object, groups included.
   - If `replace_with_pieces` cannot take the place from one node and the
     style from another, it gets a `style_from: NodeId` field. That is not a
     new command.
   - The label is `weld`. `history_names.rs` maps it to "Weld".

5. **2026-10-20: curves.** Every outline is flattened, including untouched
   ones, as in Union. The output is deterministic and there is one code path.
   Keeping the curves of untouched outlines would mix curves and polylines in
   one result, and it would need the closing to prove an outline unchanged.
   Refitting stays the follow-up of 0016.

6. **2026-10-20: piece count.** This deviates from the spec's "pure function in
   `ui-core`": the count comes from the kernel. `weld` returns
   `Weld { result: BooleanResult, pieces: usize }`, computed by the existing
   `connected_pieces` (outlines of even depth from `outline_nesting`). There
   is no second nesting walk in `ui-core`.

   Criterion 7 (two squares touching at one corner give 2 pieces at g = 0) is
   a unit test. If `i_overlay` returns one outline that visits the touching
   vertex twice, `weld.rs` splits that outline at the repeated vertex before
   `canonical_millimetres`.

7. **2026-10-20: Gap setting (ADR 0004).**
   - The value lives in the webview's `localStorage`, key
     `curvyo.weld-gap-mm`, in both builds. This is the rule of 0051 decision
     8: every access is in `try/catch`, and a failure gives 0.1 mm.
   - It is per OS user and per machine, and it is not synced. It is not a
     project key, not a file in the ADR 0004 data directory, and not a host
     command.
   - A tolerance preference is not one of the records ADR 0004 syncs
     (projects, the material database, asset libraries). The data directory
     is for records that must follow the maker, such as
     `document-formats.toml`.
   - Range clamping and the clamp notice are a pure function in
     `ui-core/src/weld_gap.rs`. The frontend passes the clamped value to the
     wasm call. It writes no history step.

8. **2026-10-20: budgets and thread.** Weld runs synchronously on the UI
   thread, as Union does, with no busy state and no new operand or node
   limit.
   - Estimate for 31(b): about 500 glyphs x about 200 vertices, giving about
     100k vertices. That is four overlays or offsets, about 0.2 to 0.5 s
     native.
   - Fixtures (a) to (d) are added to the `boolean-budgets` job (release,
     native). The wasm run takes at most 2x.
   - If 31(b) misses 2 s on CI, the implementer logs it in
     `docs/technical-debt.md` ("Weld runs on the main thread") and the slice
     ships. It does not get a limit or a refusal.

9. **2026-10-20: placement.**
   - `curvyo-geometry-core`:
     - new: `weld.rs` (contributions, union, closing, pieces) and
       `stroke_band.rs`;
     - changed: `union_region` in `boolean.rs`, and the `Paths64` offset
       helper in `offset_polyline.rs`. Both are crate-private.
     - exported: `weld`, `Weld`, `WeldOperand { regions: &[Outline], banded:
       &[Outline], band: Option<(Length, OffsetJoin, OffsetCap)> }`. These
       reuse 0038's join and cap types, with no new style types.
   - `curvyo-ui-core`: new `weld_plan.rs` (operands, plan, notice text with
     the dash sentence and the sentences of 0054 criteria 21 and 22) and
     `weld_gap.rs`.
   - `curvyo-editor-wasm`: new `session/weld.rs` and `wasm_weld.rs`.
   - Frontend: the button and the Gap field in the Select bar.
   - Fixtures go in `tests/fixtures/weld/`, with golden files for native and
     wasm at 1e-6 mm.
   - There is no new crate and no new ADR. There is no `format_version`
     bump, because Weld writes only existing kinds (criterion 26).

## Flagged to the PO (no customer question)

1. **Pieces and islands.** Resolved 2026-10-20, default taken: a piece is one outline of even
   depth with the holes directly inside it, so an island in a hole is a piece of its own and
   criterion 5 reports 2 pieces. Terms and criterion 5 of `specification.md` say so;
   `connected_pieces` counts it unchanged.

## Notes from the build (implementer, 2026-10-20)

- **Dimmed tooltip line kept.** The customer rule "no hint labels by default" would drop the
  third line "Select an object first." of the dimmed Weld button. It stays, because
  `docs/design-system.md` (row "Flip buttons") gives the dimmed Flip buttons beside it the same
  line: it names the state, and the bar stays consistent. No new hint is introduced.
- **Decision 4, the write.** Weld writes one object, as Union does, so the contingency is taken
  on the one-object write, not on `replace_with_pieces`: `Document::replace_with_path_at
  (operands, style_from, place, name, outlines, label)` is the general form, and
  `replace_with_path` now calls it with `place = style_from = base` and no name. `place` may be
  an operand or a group whose every leaf is an operand; that group is emptied and removed in the
  same commit, so the result takes its place (criterion 20). The name (criterion 25, the base
  text's `default_name`, 24 characters) is written in the same commit. A public addition to
  `curvyo-document-core`; no stored format changes.
- **Decision 3, types.** `StrokeBand { width: Length, join: LineJoin, cap: LineCap }` takes the
  document's own join and cap types (`0038` has `OffsetJoin` but no cap type); the join maps to
  `OffsetJoin` inside the kernel, so `int_join` (with its miter limit) and `miter_min_turn` of
  `offset_polyline.rs` are shared, not copied. Round arcs of the band and of the closing use a
  chord tolerance of 0.004 mm.
- **Decision 6, pinch split.** Not needed: the n-ary union already returns two outlines for two
  squares that touch at one corner (`weld.rs` test `ac07`, and `0016` fixture (c)).
- **Criterion 10, open outlines.** The sentences of `0054` criteria 21 and 22 count the open
  outlines whose whole contribution is their straight-line closure (Stroke off). An open outline
  with Stroke and Fill on contributes its band and its closed fill area, and adds no sentence.
- **Criterion 28, the no-area sentence.** The exact `0016` sentence with "Weld": "Weld needs
  shapes that enclose an area. 3 of 3 selected objects have no area. Nothing was changed." (the
  UX notes ask for the exact `0016` text; its paraphrase there is not used).
- **Gap closing and concave corners.** At g > 0 a concave corner of the union gets a rounded
  fill of radius g/2 (what a closing does; within the bound of criterion 15). The golden results
  of the `0016` fixtures at g = 0.1 mm show these extra nodes.
- **Fixtures.** `curvyo-geometry-core/tests/fixtures/weld/weld.golden` (criterion 31 (c), (d),
  compared natively and in the wasm build) and `curvyo-editor-wasm/tests/fixtures/weld/
  annabelle.golden` (criterion 31 (a)). Measured on the dev VM, release: (a) 22 ms, 2 pieces;
  (b) 0.77 s; (c) at most 0.14 s (fixture (i)); (d) under 1 ms. No technical-debt entry needed.
- **Wasm size.** The release editor wasm grows by 30,319 bytes (29.6 KiB) against
  `story/text-objects`; about 15 KiB of it is `i_overlay`'s stroke builder.
