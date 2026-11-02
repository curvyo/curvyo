# Plan for Pattern fill

Implementer, 2026-11-02, branch `story/0071-pattern-fill` (from `main` + the 0071 docs commits).
Built under `adrs.md` and ADR 0029 with the open questions at their defaults (PAT-STORE A, PAT-NEW
A) and the lead's decisions of 2026-11-02: Convert to paths is the panel button only (criterion 24's
menus do not exist; PO rewords later), Crosshatch has Spacing / Angle / Second angle (135°), a shape
edit that pushes a pattern over the hard limit keeps the edit and draws no pattern, with the red
state line.

## Affected crates/modules

- **`curvyo-document-core`:** `pattern_model.rs` (new: `FillPaint`, `PatternFill`,
  `PatternGenerator`, `GeneratorKind`, `PatternParam`, `PatternMode`, `PatternSeed`,
  `PatternCount`, `PatternFraction`, SplitMix64), `pattern_schema.rs` (new: per-generator parameter
  table with wire name, label, kind, frozen default, minimum), `pattern_codec.rs` (new: the keys
  `fill_paint` and `fill_pattern`, lenient read, change write, strict validation); `style_model.rs`
  (`Fill` gains `paint` and `pattern`), `style_codec.rs` / `style_validation.rs` (the two keys),
  `styles.rs` (`StyleEdit` pattern edits, fresh seed per node), `document.rs` (format 32).
- **`curvyo-geometry-core`:** `pattern/` (new module group): `mod.rs` (`pattern_pieces`, frame,
  limits, errors), `region.rs` (region on the grid, margin inset, cell classification),
  `clip.rs` (keep / string clip / intersection), `lines.rs` (Hatch, Crosshatch, Grid, Waves),
  `cells.rs` (Dots, Honeycomb, Brick wall), `leaves.rs` (Leaf forest), `scales.rs` (Sea waves),
  `blend.rs` (Interpolation waves), `seed.rs` (seeded stream). `libm` becomes a direct dependency.
- **`curvyo-render-core`:** none beyond reading `Fill::paints_solid` (a pattern fill never draws as a
  solid colour anywhere, e.g. in the history preview); no geometry-core dependency (ADR 0011).
- **`curvyo-ui-core`:** `pattern_export.rs` (SVG export of the pieces), `pattern_convert.rs`
  (Convert to paths plan), `pattern_fields.rs` (panel scales, field views, typed entry),
  `svg_export_leaf.rs` (a pattern fill writes `fill="none"`), history name.
- **`curvyo-editor-wasm`:** `session/pattern.rs` (panel read, commands, state lines, over-limit
  refusal, convert), `session/draw.rs` (artwork expansion through the memo), `wasm_pattern.rs`
  (bindings). The memo itself is `curvyo-ui-core/src/pattern_cache.rs`: the session has no
  geometry-core dependency (ADR 0011), so the pure memo lives where the kernel is reachable.
- **`frontend`:** `PatternBlock.tsx`, `TileGroup.tsx`, `PatternIcons.tsx`, `PaintRow` with a third
  option, `FillSection.tsx`, `useStylePanel.ts`, browser script `tests/browser/pattern-fill.cjs`.

## Tasks

- [x] 1. Model and format 32, tests first: typed model and schema round trip for every generator,
  absent keys read as before, damage rules, `StyleEdit` pattern edits (paint switch keeps the
  pattern, generator switch carries Spacing/Angle/Margin/Mode/Seed, fresh seed from the node id,
  re-roll), one commit per edit, undo restores, copy/duplicate keeps the seed, clone reads it,
  golden `pattern_v32.curvyo` (AC 1, 3, 17, 20, 21, 22).
- [x] 2. Geometry kernel, one generator at a time with goldens (native and wasm32 byte-equal):
  region + margin + holes + clipping (AC 13, 14, 15), Hatch / Crosshatch / Grid (AC 5), Dots (AC 6),
  Honeycomb (AC 7), Brick wall (AC 8), Leaf forest (AC 9, 17), Sea waves (AC 10), Waves (AC 11),
  Interpolation waves (AC 12), Lines / Shapes (AC 16), open regions (AC 23), count limits and the
  thread-CPU measurements (AC 18).
- [x] 3. Session cache and canvas: synthetic snapshot under the stroke, live during drags and
  node edits, interior hit, over-limit draws nothing (AC 2, 16, 18, 19, 22, 23).
- [x] 4. SVG export: region `fill="none"` plus a group of piece paths, no `<pattern>` (AC 26).
- [x] 5. Convert to paths: one commit, group above the shape, fill off, undo (AC 24, 25).
- [x] 6. Panel: fill kind None / Solid / Pattern, tile picker, parameter fields with preview and
  one commit, Margin, Seed + re-roll, Draw as, state line, Convert button (AC 1 to 4, 16, 18, 23).
- [x] 7. Browser script and screenshots of all ten generators (AC 28).

### Follow-ups (architecture review, acceptance report), 2026-11-02

- [x] 8. Review change 1: the memo drops entries once per draw (`PatternCache::retain_only`), not
  in `expand`; session test `ac19_a_draw_in_two_parts_keeps_both_parts_patterns` (AC 19).
- [x] 9. Review change 2: Convert's availability comes from the memo's `PatternState` in the
  panel read; `pattern_convert_available` and its own generation are gone (AC 24).
- [x] 10. Review change 3: SVG writes the pieces before the outline; test
  `ac25_the_export_writes_the_pieces_before_the_outline`, golden regenerated (AC 25, 26).
- [x] 11. Review change 4: Leaf forest stops after 512 drops in a row and after 2,000,000
  throws, cells in a dense grid instead of a hash map; measured below (AC 9, 17, 18).
- [x] 12. Review change 5: `pattern_values.rs` (value table view, per-kind helpers) split from
  `pattern_model.rs`; `pattern_draw.rs` split from `pattern_cache.rs`.
- [x] 13. Finding 3: lines are cut one by one by a new clipper (`pattern/string_clip.rs`, exact
  integer predicates, the region's edges indexed by cell, local winding), so each line part is
  its own path; parts keep the line's exact points (AC 5, 13 to 15, 24).
- [x] 14. Finding 2: the limit counts work (`pattern/cost.rs`): the work of making the raw
  pieces is known before anything is made, the work of cutting is charged step by step and ends
  the cutting when it runs out (`PatternError::TooDetailed`, state line "Not generated: too
  detailed for this shape. ..."); closed pieces are cut against the region near them
  (`pattern/local.rs`, tiles of the cell map cut from the tile above) (AC 18).
- [x] 15. Finding 4 (Proposal): Shapes merges crossing strips (Grid, Crosshatch, Sea waves) into
  one outline; parallel strips (Hatch, Waves) are cut in batches of 32 lines and merged across
  batches only when they can overlap; Interpolation waves' strips are cut one by one (AC 16).
- [x] 16. Goldens regenerated and reviewed: `cases.golden`, the tester's
  `acceptance_0071_bits.golden` (native = wasm32 in Node), `pattern_export.svg`.

### UI review follow-ups (`ux-notes.md`, "UI review (2026-11-02)"), 2026-11-02

- [x] 17. Item 1: the state lines render first in the Pattern block, directly under the Fill kind
  row, same `role="alert"` rule (AC 18, 23). A new generator drops a shown refusal, so an old
  count never stands under another generator; other commands keep it for its 8 s, so the rows do
  not move under a double-click.
- [ ] 18. Item 2: blocked, see open point. Convert sets the selection to the new group ids, but
  the selection holds leaves (as after Group), so nothing shows as selected and the Style tab is
  disabled after History. Fix ready (select the groups' leaves, `editor.sync()` after Convert, a
  browser assertion): `target/0071-browser/ux-fixed/item2-convert-selects-leaves.patch`. It
  breaks the tester's `c24_c25_...` assertion `selected_object_count() == 1` (the count is the
  leaves); that assertion needs the tester's change first. The `sync()` call is committed (AC 24).
- [x] 19. Item 3: Sea waves counts only the rings that have a radius (`scales::rings_drawn`), in
  the piece count and in the Shapes crossing work; 500 rings at the default radius are 3 rings
  and generate. Interpolation waves counts its lines, which the 650,000 was not (that was the
  Sea waves refusal still shown). Test `ac18_sea_waves_count_only_the_rings_that_fit` (AC 18).
- [x] 20. Item 4: "Not generated: too detailed for this shape. Make the pattern coarser." Test
  `ac18_too_detailed_names_no_lever` (AC 18).
- [x] 21. Item 5: a cross-field refusal names the other field (`PatternField::misfit`): Wall "the
  cell size", Joint the smaller of "the brick height" / "the brick width", Cell size / Width /
  Height "larger than the wall" / "the joint". Tests in `pattern_fields.rs` and
  `ac5_joint_over_the_brick_names_the_brick_height` (AC 2).
- [x] 22. Debt 6 (Proposal): the Stroke dash text row is labelled "Dashes" (fits the 60 px label
  column; "Dash pattern" does not); `docs/design-system.md` updated.
- [x] 23. Debt 7: where a value field's bar edge would cross the label or the value text, it is
  drawn as two 6 px ticks above and below the text row (`lib/valueEdge.ts`, test
  `valueEdge.test.ts`); `docs/design-system.md` "Fill bar" updated.

### Whole pieces only (customer request, 2026-11-02; AC 29)

- [x] 24. `PatternFill::whole_pieces` (default off) and `GeneratorKind::has_whole_pieces` (Dots,
  Honeycomb, Brick wall, Leaf forest, Sea waves) in `curvyo-document-core`; register field
  `whole_pieces: bool`, absent reads off, another type is damage (format 32 is unreleased, so it
  is extended, no new version); v32 golden fixture regenerated; damaged-register tests.
- [x] 25. `StyleEdit::PatternWholePieces`, commit `pattern_whole_pieces`, history name "Pattern:
  whole pieces"; an unchanged value writes no step.
- [x] 26. `curvyo-geometry-core`: `Cutter::keep_whole` drops every raw piece that meets the
  inset region's boundary, lies outside, or holds a contour (hole or island); a Sea waves ring
  `follows` its outer ring so a scale goes as one; in Shapes mode a strip must keep its width
  inside. Charged to the same budget; the estimate charges the boundary test instead of the
  overlay. Tests in `pattern_pieces.rs`, goldens in `cases.golden`, CPU-time cases.
- [x] 27. Session panel `whole_offered` / `whole_pieces`, `set_pattern_whole_pieces`; frontend
  `PanelSwitch` row "Whole pieces only" under Draw as; browser check in `pattern-fill.cjs`.
- [x] 28. SVG export and Convert to paths read the same pieces (`pattern_output.rs`).

Decision (Proposal): the switch is not offered for the line generators. Their Shapes mode gives
strips (merged bands for Hatch, Grid and Waves), not closed cells, so there is no whole piece to
keep; Honeycomb and Brick wall already are cells in both modes.

## Validation

- Piece goldens per generator in `curvyo-geometry-core/tests/fixtures/pattern/`, compared natively
  and with `cargo test --target wasm32-unknown-unknown -p curvyo-geometry-core --test
  pattern_golden` (Node runner), which shows native/wasm byte equality (AC 17).
- Thread-CPU timing per generator on a 100 mm square (`tests/pattern_cpu_time.rs`); numbers below.
- `pattern_v32.curvyo` document golden; SVG export golden of the same drawing.
- Browser script `frontend/tests/browser/pattern-fill.cjs`, screenshots in `target/0071-browser/`.

## Dependencies

- `libm` 0.2 (MIT OR Apache-2.0, rust-lang, pure Rust, no `unsafe` in the used functions'
  public API, already in `Cargo.lock` through `i_overlay`/`kurbo`): made a direct dependency of
  `curvyo-geometry-core` because the generators need `sin`/`cos` that are bit-identical on every
  platform and in wasm (ADR 0029 §5; `f64::sin` calls the platform's libm). Passes `cargo deny`.

## Decisions taken here (Proposal, vetoable at the demo)

- The pattern is kept while the paint is Solid or None (ADR 0029 §1); a later Pattern restores it.
  The UX notes' "starts at Hatch defaults again" is not followed: the ADR is the format authority
  and criterion 3 holds either way.
- Shapes mode for line generators (Hatch, Crosshatch, Grid, Waves, Sea waves, Interpolation waves):
  each line becomes a closed strip as wide as the shape's stroke width, cut at the region, so Shapes
  shows exactly what Lines shows, as fillable outlines.
- Dots: rows one Spacing apart, every second row shifted by half a Spacing. Honeycomb: Cell size is
  the distance between neighbouring cell centres; each hexagon is that minus the Wall across flats.
  Brick wall: Width and Height are the brick pitch, a brick is (Width - Joint) x (Height - Joint),
  so the row shift is Width / 2 (criterion 8: 10 mm for 20 mm). Sea waves: half circles above their
  centre, rows one radius apart, every second row shifted by one radius (half a scale width).
- Lines mode (lead decision 2026-11-02, told to the customer as Proposal, `adrs.md` decision 3):
  the pieces use the shape's stroke settings (width, colour, opacity, dash) even while the
  outline's own stroke is off, so a texture can be engraved without cutting the outline. No state
  line for it. The UX notes' "Lines are drawn with the stroke, which is off" line is not built;
  the UX notes are the UX engineer's to update.
- Convert to paths takes paths and primitives; a linked clone (styled by its source) is skipped.
- History names per UX notes §6: each pattern edit commits under its own label ("Fill: Pattern",
  "Pattern: Honeycomb", "Pattern spacing", "Pattern margin", "Pattern: Lines" / "Pattern: Shapes",
  "Pattern: new seed", "Convert pattern to paths"); Solid, None and the colour edits stay
  "Change style".
- render-core reads `Fill::paints_solid` (one line in `artwork.rs` and `shared_outline.rs`) so a
  pattern never paints as a solid colour in any preview; no geometry-core dependency is added.
- Format pins in tester-owned tests (`acceptance_0070_tester.rs`, `acceptance_0024_slice2_tester.rs`)
  moved to the new version the way the format-31 bump moved them (`>=` / `CURRENT + 1`, or the
  extended pin); no assertion was weakened beyond the version number.
- Over the hard limit while dragging a field: the last valid pattern stays on the canvas with the
  red count line (UX notes §7); the release is refused (nothing committed). A shape edit that
  pushes a placed pattern over the limit keeps the edit and draws no pattern (lead decision).
- Interpolation waves (customer 2026-11-02: like Inkscape's Interpolate): the first and last lines
  are the region's top and bottom runs vertex for vertex; the lines between blend the two runs,
  resampled to the same point count by arc length, step by step. Golden case "wavy top" and
  screenshot `target/0071-browser/gen-interpolation_waves-wavy.png`.

## Measurements (AC 18)

`curvyo-geometry-core/tests/pattern_cpu_time.rs`, release build, thread CPU time, 2026-11-02 on
midna. Before = branch at `37515997`, after = this change.

### The acceptance report's cases (100 mm circle, Lines)

| Case | Before | After |
|---|---|---|
| Interpolation waves, 1,000 lines | 2.1 s (wall) / 1.97 s | 26 ms |
| Interpolation waves, 4,000 lines | 21 s | refused (too detailed) in under 50 ms |
| Interpolation waves, 100,000 lines | over 9 min | refused in 0.1 ms, before any work |
| Grid at 0.1 mm | 3.1 s, 1.5 M nodes (staircases) | 4.5 ms, 2 nodes per line |
| Hatch at 0.05 mm | 194 ms | 9 ms |
| Waves, wavelength 0.05 mm | 349 ms | 59 ms |
| Leaf forest at density 100 %, near the hard limit | 4.5 s | 104 ms (130,052 leaves) |
| Honeycomb 6 mm on an 8,000-edge star | 6.6 s | 112 ms |
| Hatch 0.2 mm, Shapes, 8,000-edge star | 15.4 s | refused in under 30 ms |

### Every generator at its finest accepted setting

Bisection of each generator's size parameter against the work limit
(`ac18_every_generator_at_its_finest_accepted_setting_stays_within_the_budget`), on a 100 mm
circle and on a star of 4,000 needles 10 mm deep (8,000 edges, the worst outline measured):

| Shape | Mode | Slowest accepted | Slowest refusal |
|---|---|---|---|
| circle | Lines | 97 ms (Leaf forest, 103,152 leaves) | 57 ms |
| circle | Shapes | 105 ms (Grid 0.46 mm, one merged mesh) | 65 ms |
| star | Lines | 126 ms (Brick wall 4.8 mm) | 120 ms |
| star | Shapes | 138 ms (Crosshatch 48 mm) | 120 ms |

Every generator at its defaults on a 100 mm square: at most 61 ms (Sea waves, Shapes), 35 ms
(Waves, Shapes), 33 ms (Crosshatch, Shapes, merged); every Lines case under 1.5 ms. A refusal
before any work (over the piece limit, or too many raw anchors): under 0.2 ms.

The numbers above are the first calibration; the CI recalibration below replaces the work limit's
meaning and the finest settings.

### CI recalibration (2026-11-02, PR #248 CI)

The `performance-budgets` CI job (GitHub ubuntu runner) failed: Sea waves at its defaults, Shapes,
101.2 ms over 100 ms; Grid 0.463 mm, Shapes, on the circle 173.6 ms over 150 ms; Leaf forest at
density 100 % near the hard limit 166.0 ms over 150 ms (the star cases never ran: the test stops at
the first failure, and at 125 to 142 ms here they would have failed too). The same cases here (3
runs each): 60 to 63 ms, 105 to 110 ms, 98 to 110 ms; CI over this VM is 1.5 to 1.7 for overlay
work and 1.9 for the allocation-bound Dots refusal (151 ms on CI, 80 ms here). **Assumed CI factor:
1.9**, so the local targets are 100 / 1.9 = 52 ms at the defaults and 150 / 1.9 = 79 ms for
anything made or refused. The budgets were not raised.

What got faster (outputs identical unless noted):

- Crossing strips whose crossings are rounded (Crosshatch, Sea waves, Grid off the axes) are cut
  in one overlay that splits long edges into fragments first (`i_overlay` `Solver::FRAG`) instead
  of a union followed by an intersection: Sea waves defaults 60 to 41 ms, Crosshatch defaults 34
  to 19 ms, Crosshatch 1.57 mm on the circle 55 to 28 ms. Strips on the axes keep the union then
  intersection (one overlay is three times slower for them: 296 ms for Grid 0.463 mm). **Goldens
  changed** for Crosshatch and Sea waves in Shapes mode only (`cases.golden` 4 cases,
  `acceptance_0071_bits.golden` 4 lines, `pattern_export.svg` the Sea waves path): where a strip
  meets the region's boundary its crossing is now rounded to the 0.001 mm grid once instead of
  twice, so boundary points move by at most one grid unit and a few collinear points come or go;
  piece and outline counts are unchanged; native and wasm32 still match bit for bit.
- Cutting a piece against a region of many edges: the overlay compared every pair of edges below
  4,000 edges and used a tree above, both slow along the needle star's long side-by-side edges.
  Fragments are used above 4,000 edges and for a piece of at most 64 edges against at least 128
  region edges (`clip::solver`; slower for a few long strips against a smooth region, which keeps
  the default): Brick wall 4.8 mm on the star 130 to 99 ms, Waves Shapes on the star 118 to 70 ms.
  Same goldens.
- Leaf forest: each accepted leaf's circle is kept with its link in the cell lists, and the
  candidate's own cell is tested first (where a full box usually fails): 1.3 M throws 91 to 65 ms.
  Same test, same leaves.

Recalibration: every case of the test was measured against the work it was charged (time per
unit). Before, it reached 1.0 to 1.28 ns per unit (Grid on the axes, Honeycomb and Brick wall on
the star, Leaf forest); now no case measured goes above about 0.8 and almost all stay below 0.7,
with `PATTERN_MAX_WORK` unchanged at 100,000,000:

| Step | Before | After | Why |
|---|---|---|---|
| `Throw` | 15 | 35 | 50 ns per throw measured (after the speed-up) |
| `Crossing` (on the axes) | 1,300 | 2,600 | union then intersection of 188,000 edges |
| `RoundedCrossing` | 10,500 | 4,000 | the one-overlay cut is about 2.5 times faster |
| `Scan` | 150 | 60 | cutting the tiles costs about 18 ns per edge; the overlay was what grew |
| `AroundEdge` (new) | (`OverlayEdge` 140) | 1,300 | a region edge near a piece: 1.2 to 2.9 µs each on the star |
| `AroundPair` (new) | - | 3 | per pair of those edges: the overlay grows with their square |
| `StripEdge` | 30,000 | 36,000 | Waves Shapes on the star |
| `MergedEdge` | 450 | 600 | Waves Shapes on the circle |
| `RegionEdge` | 3,500 | 4,500 | the 8,000-edge star's region: 24 ms |

Early refusal: closed pieces (Dots, Honeycomb, Brick wall, Leaf forest) also estimate before any
work the least work of cutting the pieces that meet the boundary (the region's share of boundary
cells times the pieces, each one overlay of three piece and three region edges); when it does not
fit with the raw work the pattern is refused at once. It is not charged twice: the cut still
charges what it does. Dots near the hard limit: refused after 80 ms here (151 ms on CI) before,
now in 0.14 ms. Leaf forest at 100 % near the hard limit is now refused at once (0.13 ms; its 2 M
throws alone are over the limit) instead of being made in about 100 ms.

After (3 runs, slowest of the three; this VM):

| Case | Before | After |
|---|---|---|
| Slowest default (Sea waves, Shapes) | 63 ms | 44 ms (one cold first run: 62 ms) |
| Crosshatch / Waves defaults, Shapes | 34 to 42 / 34 to 37 ms | 19 / 34 ms (cold first run: 31 / 40 ms) |
| Slowest made, circle | 110 ms (Grid 0.463 mm, Shapes) | 68 ms (Crosshatch 1.03 mm, Shapes) |
| Slowest made, star | 142 ms (Crosshatch 47.8 mm, Shapes) | 62 ms (Brick wall 40 mm, Shapes) |
| Slowest refusal by work | 130 ms (Honeycomb on the star) | 67 ms (Waves 4.28 mm, Shapes, circle) |
| Leaf forest 100 % near the hard limit | 98 to 110 ms, made | 0.2 ms, refused before any work |
| Dots near the hard limit | 80 ms, refused late | 0.15 ms, refused before any work |

Finest accepted settings now (100 mm circle, Shapes): Grid 0.63 mm (was 0.46), Crosshatch 1.03 mm
(1.57), Sea waves radius 4.0 mm, Waves 4.8 mm (4.1), Interpolation waves 99 lines, Hatch 0.13 mm;
Lines mode: Dots 0.18 mm (252,000 dots; was 0.17), Honeycomb 0.23 mm, Brick wall 0.18 mm, Leaf
forest 0.29 mm (41,000 leaves; was 0.16 mm), the line generators unchanged. On the needle star,
Honeycomb (and, as before, Sea waves and Interpolation waves in Shapes mode) and Waves in Shapes
mode are refused even at their coarsest; that outline is the stress case, not a drawing.

## Decisions taken in the follow-ups (Proposal, vetoable at the demo)

- Over the work limit the state line reads "Not generated: too detailed for this shape. Use a
  larger spacing, size or fewer lines." (starts with "Not generated" like the count refusal, so
  the block shows it the same way). The UX notes are the UX engineer's to update.
- A refusal by work can take up to the limit itself (at most about 70 ms here) when it is only
  known while cutting; refusals known before any work stay under a millisecond.
- Shapes mode merges the strips of Hatch and Waves too when they can overlap (spacing below the
  strip's width across the lines): overlapping strips are one band for a laser as well.
  Interpolation waves' strips are not merged: their lines meet in fans at the region's sides,
  where a union of 1,000 strips took 10.5 s on a circle.
- Leaf forest: a forest stops after 512 drops in a row (density above about 60 % gives no more
  leaves than that) or after 2,000,000 throws; the defaults are not reached by either (frozen by
  the goldens from now on).
- Cut lines keep the exact points of the line (the inside is decided on the grid); a line lying
  along a hole edge is kept whole (on the boundary counts as inside, as before).
