# 0108 Trace library extraction: inventory (slice 0)

Status: done by the architect, 2026-11-10, on `story/trace-library-extraction` at `6b7fa378`. Method:
real `use` lines and calls per file (grep for `curvyo_document_core`, `curvyo_geometry_core`,
`curvyo_text_core`, `crate::`), callers outside each crate, function and module lengths. No code
changed. The design is [ADR 0048](../../docs/adr/0048-vectrace-crate.md).

Marks: **move** (to `vectrace` as is), **split** (pure part moves, adapter part stays), **stay**
(Curvyo), **delete**. Lines include tests.

## 1. Where the plan's assumptions do not hold

1. **`Affine` is not `kurbo::Affine`.** `document-core`'s `compose`, `apply_point` and
   `determinant` use `mul_add` (one rounding), kurbo does not. `vectorize-core` uses
   `determinant` for the speck and region thresholds, and `trace_text` uses `compose` and
   `apply_point`. Swapping in kurbo changes bits. `vectrace` gets its own `PixelMap` with the
   same arithmetic, copied as is (ADR 0048 §3).
2. **Mask clip and part join need Curvyo's boolean kernel.** `trace_region` (clip) and `trace_parts`
   (composite join, `visible_regions`) call `curvyo-geometry-core::boolean`, about 1,000 lines on
   `i_overlay`, and that crate depends on `document-core`. Both **stay**. The jobs that clip
   inside the worker get a `Mask` trait instead (ADR 0048 §4).
3. **"fit" cannot move.** `trace_fit`, `trace_fit_guess` and `trace_fit_turn` match against fonts
   (`curvyo-text-core` `Face`, `layout`, `Recogniser`, `text_recognise_job`). They **stay**.
   `trace_words` is pure, but its only caller is `trace_fit_guess`, so it stays next to it.
4. **`trace_plan` has no pure part.** It builds document nodes, styles and names. It **stays
   whole**; nothing to split.
5. **Reading shapes back needs the document's outline.** `read_trace_body` calls
   `with_primitives`, which runs `keeps_fill` on `drawing_outline_of_rotated`. That is behaviour (a
   region whose fill changes keeps its traced paths), so it **stays** in the adapter. vectrace's
   wire reader returns the shapes without that step.
6. **The shape tests change type.** `ui-core/tests/trace_shapes.rs`, `trace_shape_sweep.rs`,
   `acceptance_0088_c16_tester.rs` and `acceptance_0088_c16_join_tester.rs` (about 1,750 lines)
   assert on document `Shape`. They move to `vectrace/tests` and switch to `Primitive`.
7. **Extraction blockers outside the plan.** In dev-dependencies, `vectorize-core` uses
   `curvyo-image-core` and `curvyo-svg-core`, and `ocr-core` uses `curvyo-image-core`. The OCR
   record test reads `frontend/public/models/`. 16 `vectorize-core` test files import
   `document-core`. Slice 1 fixes the imports. The dev-dependencies stay until slice 5 (§6).
8. **No line target for ui-core `trace_*` (customer, 2026-11-10).** About 3,100 lines stay for good reasons (points 2 to 5). Soft goal: as lean as reasonable, everything keeps working; report the number.
9. **Crate name without a suffix.** `vectrace` breaks the `CLAUDE.md` §6 suffix list. ADR 0048
   §1 treats it as a core crate in every rule. The lead amends `CLAUDE.md` §6 and the §7 gate list.

Effort: slice 2 is about 2 to 3 implementer days (shape types, `Mask`, `Job`), inside the 3-day
limit of the plan's risk section. ui-core has 23 `trace_*` files (the plan says 25), 7,923 lines.

## 2. `curvyo-vectorize-core/src` (9,942 lines, 33 files): all move

| File | Lines | Curvyo edge | Mark | Note |
|---|---|---|---|---|
| `trace.rs` | 360 | `Affine` | move | `trace_with` 72 and `masked` 80 lines, split after the move; `to_kurbo` becomes `PixelMap::to_kurbo` |
| `colour_trace.rs` | 195 | `Affine` | move | |
| `colour_map.rs` | 304 | `Affine` | move | `map.determinant()` must keep `mul_add` |
| `params.rs` | 460 | `Length` | move | `specks: Length` becomes `Mm` |
| `outline.rs` | 1,037 | none | move, then split | `impl Chain` is about 400 lines; split into `outline.rs` (Chain, Span, Contour, Room) and `outline_fit.rs` (Limits, fit_span, fit_closed, assemble, to_path, node_count) |
| `palette.rs` | 733 | none | move, then split | `palette.rs` (palette, merging) and `palette_cut.rs` (distinct, cut, split, otsu) |
| `guarded.rs` | 225 | none | move | `guarded` is 109 lines, split |
| `regions.rs`, `guard.rs`, `filter.rs`, `line.rs` | 254, 257, 195, 348 | none | move | `label` 82, `find` 70, `median` 62, `straight_runs` 62: split or justify in the PR |
| `select.rs` | 317 | none | move | private `distance` is a copy of `palette::distance`: delete |
| `analysis.rs`, `analysis/*.rs`, `schema.rs`, `raster.rs`, `picker.rs`, `background.rs`, `edges.rs`, `fit.rs`, `segment.rs`, `shared_fit.rs`, `corners.rs`, `corner_place.rs`, `clearance.rs`, `boundary.rs`, `contour.rs`, `smooth.rs`, `method.rs`, `progress.rs`, `lib.rs` | rest | none | move | no traits, no `TODO`, no dead-code allows |

Public items with no caller outside the crate (become `pub(crate)` when the vectrace API is set):
`LOW_CONFIDENCE`, `DEFAULT_COLOURS`, `MIN_COLOURS`, `UPSCALE_AUTO_BELOW_PX`, `DEFAULT_CORNERS`,
`MAX_SPECKS_MM`, `MIN_TOLERANCE_PX`, `MAX_TOLERANCE_PX`. `Diagnostics::fallbacks` is read only
by a `println!` in `tests/colour.rs`; keep it (harness output).

## 3. `curvyo-ocr-core/src` (1,189 lines): all move, to `vectrace::ocr`

| File | Lines | Mark | Note |
|---|---|---|---|
| `lib.rs` | 127 | move as `ocr.rs` | `read_lines`, `crop_lines`, `TextLine`, `OcrError`, `OcrProgress` |
| `detect.rs`, `lines.rs`, `model.rs`, `resize.rs`, `read.rs` | 223, 134, 104, 146, 455 | move to `ocr/` | `principal_box` 66 and `crop` 64 lines: justify (numeric pipeline) |

## 4. `curvyo-ui-core/src/trace*.rs` (7,923 lines)

| File | Lines | Curvyo edges | Mark | Target |
|---|---|---|---|---|
| `trace_transfer.rs` | 715 | `Affine`, `Length` | split | jobs and wire to `vectrace/src/job.rs`; the call to `with_primitives` and the 4 `Raster` forwarders (`trace_auto_threshold`, `trace_analyse`, `trace_suggest_background`, `trace_sample_colour`) leave (§5) |
| `trace_text_wire.rs` | 379 | `Affine`, `TraceRegion` | move | `job_text.rs`; the region becomes `Option<&dyn Mask>` |
| `trace_text_colour.rs` | 462 | `Affine`, `TraceRegion` | move | `job_text_colour.rs` |
| `trace_text.rs` | 468 | `Affine`, `Point`, `Vec2`, `Length` | move | `text_pass.rs`; `text_pass` is 115 lines, split; private `luma` is a copy of `raster::luma`: delete |
| `trace_inpaint.rs` | 104 | none | move | `inpaint.rs` |
| `trace_ocr.rs` | 191 | none | move | `ocr/accept.rs` (`OcrLine`, confidence gates, `read_text`) |
| `trace_shapes.rs` | 594 | `Shape` family, `drawing_outline_of_rotated`, `svg_import_match` | split | `shapes.rs`: recognise, wire section, `trace_add_shapes`. Stay: `primitive_path`, `primitive_path_like`, `with_primitives`, `keeps_fill` (about 70 lines). `recognise_layer` 78 lines, split |
| `trace_shape_fit.rs`, `_star`, `_band`, `_join`, `_hole_band` | 332, 399, 406, 239, 272 | `Shape`, `Angle`, `Length`, `Point`, `LineJoin`, `PointCount`, `InnerRatio`, `Curve`, `CornerRadii` | move | `shapes/*.rs`; the document types become `Primitive`, `Angle`, `Mm`, `Join` (§7) |
| `trace_shape_sample.rs`, `trace_shape_math.rs` | 251, 99 | none | move | `shapes/sample.rs`, `shapes/math.rs` |
| `trace_region.rs` | 620 | `geometry-core::boolean`, `ObjectSnapshot`, `Tolerance` | stay | gains `impl vectrace::Mask for TraceRegion`; `clip_trace` and `clip_colours` (counts after the clip) move into the job; private `node_count` is a copy: delete |
| `trace_region_raster.rs` | 194 | `TraceRegion`, `Affine` | stay | calls `vectrace::analyse` directly |
| `trace_parts.rs` | 412 | `geometry-core` booleans, `visible_regions` | stay | reads `vectrace::Output`; `join_trace_parts` 66 lines |
| `trace_plan.rs` | 534 | document nodes and styles | stay whole | takes `vectrace::ShapeRegion`, converts with the adapter; `plan_trace` 63 lines |
| `trace_fit.rs`, `trace_fit_guess.rs`, `trace_fit_turn.rs` | 504, 182, 35 | `text-core`, `text_recognise_job` | stay | `guessed_texts` 90 lines |
| `trace_words.rs` | 378 | none | stay | only caller is `trace_fit_guess`; `plan_words` 97 lines |
| `trace_text_notice.rs` | 153 | none (UI words) | stay | |

Total: about 4,820 lines move from ui-core and about 3,100 stay (3,012 in whole files plus about
90 lines of adapter).

## 5. Delete before moving (subtract first)

| # | What | Callers today | Replacement |
|---|---|---|---|
| D1 | the five `_with` job wrappers (`trace_job_with`, `trace_colour_job_with`, `trace_text_job_with`, `trace_text_regions_with`, `trace_text_colour_job_with`) | tests only | the `_in` form with `None` (then `Job`, §8) |
| D2 | `ui_core::trace_job` (the slice-1 signature with three bar values) | 6 test files only | tests build `TraceParams` inline (one helper per test crate) |
| D3 | the 4 `Raster` forwarders in `trace_transfer` | `wasm_trace.rs` | `vectrace::{auto_threshold, analyse, suggest_background, sample_colour}` |
| D4 | the renamed re-exports of vectorize-core and ocr-core in `ui-core/src/lib.rs` (`TRACE_PARAMS`, `TraceColourParams`, `OCR_LINE_HEIGHT`, ...) | editor-wasm | `editor-wasm` imports `vectrace` (no re-exports, plan slice 3) |
| D5 | `trace_inpaint` alias, `text_pass`/`TextPass`/`TextRegion`, `primitive_path`, `SHAPE_TOLERANCE_PX`/`_SHARE`, `TRACE_REGION_TOLERANCE` as public `ui-core` items | none outside ui-core (or tests only) | `pub(crate)` |
| D6 | copies: `trace_text::luma`, `trace_region::node_count`, `select::distance` | internal | the original in the same crate |

## 6. Document-core types and what replaces them

| Today (`document-core`) | Used by (moving code) | In vectrace | Adapter in ui-core converts |
|---|---|---|---|
| `Affine` | trace, colour, text pass, jobs | `PixelMap([f64; 6])`: `compose`, `apply_point`, `determinant`, `inverse`, `is_finite` copied with `mul_add`; `to_kurbo()` | nothing: the worker passes `[f64; 6]` |
| `Length` | `TraceParams::specks`, shapes | `Mm(f64)` | `Length::from_mm(m.as_mm())` |
| `Angle` | shapes | `Angle(f64)` radians | `Angle::from_radians` |
| `Point`, `Vec2` | text pass, shapes, region filter | `kurbo::Point`, `kurbo::Vec2` (mm) | `Point::new(p.x, p.y)`; plain field copies, no rounding |
| `Shape`, `RectBounds`, `EllipseFrame`, `StarFrame`, `CornerRadii` | shapes, wire | `Primitive` enum, fields as on the wire (rect: origin, width, height, one radius) | `Shape` with `CornerRadii::uniform`, `arc: None` |
| `PointCount`, `InnerRatio` | star, band, join, wire | `u32` and `f64`, checked against `POINTS = 3..=1024` and the open interval (0, 1) | `PointCount::new(..)` and `InnerRatio::new(..)` under `// invariant:`; a test asserts the bounds are equal |
| `Curve` | shapes (defaults only) | none | `Curve::ELLIPSE_DEFAULT` / `POLYGON_DEFAULT` |
| `LineJoin` | band, hole band | `Join { Miter, Round, Bevel }` | one `match` |
| `Tolerance` | region clip | none (stays with `TraceRegion`) | none |
| `Color` | `trace_plan` | `Rgb8` (exists) | `Color` from r, g, b |
| `FontRef`, `Face` | fit | none (fit stays) | none |
| `ObjectSnapshot` | `mask_operand` | none (stays) | none |

## 7. Long functions and modules (§5 limits)

Over 60 lines (production code): `guarded` 109, `text_pass` 115, `plan_words` 97, `guessed_texts`
90, `label` 82, `masked` 80, `recognise_layer` 78, `trace_with` 72, `find` 70, `join_trace_parts`
66, `principal_box` 66, `crop` 64, `plan_trace` 63, `median` 62, `straight_runs` 62, `open_span`
62, `sort_into_colours` 61. Over 500 lines: `outline.rs` 1,037, `palette.rs` 733,
`trace_transfer.rs` 715 (about 650 after the split), `trace_region.rs` 620 (stays; 95 of them are
tests, justify), `trace_shapes.rs` 594 (about 520 after), `trace_plan.rs` 534 (stays, justify).
Rule for the series: split only **after** the `git mv` commit, so `git log --follow` keeps the
history.

## 8. Tasks for slices 1 to 3

Every step: the affected crates' tests plus the goldens of
`curvyo-vectorize-core/tests/fixtures/trace/` byte-identical; one commit each unless stated.

**Slice 1: `vectorize-core` without `document-core`** (refactorer, Haiku)
1. Add `src/units.rs`: `Mm`, `Angle`, `PixelMap`. Copy the bodies of `document-core`'s
   `Affine::{compose, apply_point, apply_vector, determinant, inverse, is_finite}` as they are
   (`mul_add` stays). Add a unit test that compares each one bit for bit with `document-core`
   (as a dev-dependency for that test only).
2. Replace `Affine` with `PixelMap` in `trace.rs`, `colour_trace.rs` and `colour_map.rs`; replace
   `to_kurbo` with `PixelMap::to_kurbo`. Replace `Length` with `Mm` in `params.rs`.
3. Fix the callers in `ui-core` (`Affine::from_array(m)` becomes `PixelMap::new(m)`) and the
   16 test files. Remove `curvyo-document-core` from `[dependencies]` of `vectorize-core`.
4. Proof (pstack `blast-radius`): goldens, `native == wasm` check, criteria 25/26 of 0089.

**Slice 2: pure `ui-core` trace modules** (implementer, Sonnet; D-steps by the refactorer first)
1. Refactorer: D1, D2, D5, D6 (§5). Tests unchanged in what they assert.
2. Implementer: add `Primitive`, `Join`, `Band`, `ShapeRegion` (vectrace names) to
   `trace_shapes.rs`. Port `trace_shape_*` from `Shape`/`Length`/`Angle` to them; keep every
   formula and every order of operations. Move `primitive_path`, `primitive_path_like`,
   `with_primitives` and `keeps_fill` to a new `trace_adapter.rs` with
   `shape_of(&Primitive, Angle) -> (Shape, Angle)`.
3. Implementer: add the `Mask` trait and `Clipped`. `TraceRegion` implements it (its
   `clip_counting` becomes `clip`). The jobs take `Option<&dyn Mask>`; the count update of
   `clip_trace`/`clip_colours` moves into the jobs. Add a box-mask test double in the job tests.
4. Implementer: fold the job functions into `Job` and `run` / `text_regions` (ADR 0048 §5),
   including `shapes: bool` (today `with_shapes` in `wasm_trace.rs`). Split `read_trace_wire`
   into the pure read and the adapter's `with_primitives`.
5. Implementer: `trace_text` and the jobs use `PixelMap`, `kurbo::Point` and `Vec2`. After this,
   `grep curvyo_document_core` over the moving files gives nothing, and the moving files use
   nothing from `crate::` outside the moving set.
6. Refactorer: D3, then change the shape tests (§1 point 6) to `Primitive`.

**Slice 3: create `vectrace`** (refactorer, Haiku; the architect reviews the public API)
1. `git mv curvyo-vectorize-core vectrace`; `git mv curvyo-ocr-core/src/lib.rs vectrace/src/ocr.rs`
   and the other `src` files to `vectrace/src/ocr/`; the ocr tests and fixtures to
   `vectrace/tests/`. `git mv` the moving `ui-core` files to the targets of §4. In this commit,
   change only paths, `mod` lines and `use` lines.
2. `vectrace/Cargo.toml`: `name = "vectrace"`, `license = "MIT OR Apache-2.0"`, dependencies
   `kurbo`, `thiserror`, `rten`, `rten-tensor`, `sha2`, no `curvyo-*`. Workspace members, the
   `profile.dev.package` entry, `deny.toml` (drop the two AGPL exceptions), CI wasm32 lists and
   the `rayon` ban-list check (`rten` only) follow the new name.
3. Fix the callers: `ui-core` and `editor-wasm` depend on `vectrace`; D4 (no re-exports). Delete
   the empty `curvyo-ocr-core` directory.
4. Next commit: the renames of ADR 0048 §5 (`TraceWire` to `Output`, and so on). Next: the
   splits of §7. Then make every item outside the ADR's API `pub(crate)`.
5. Full gate (`CLAUDE.md` §7 with `vectrace` in the wasm32 list), goldens, CPU-time and memory
   tests at the same budgets; report the editor wasm size before and after.

Out of scope here and listed for slice 5: the dev-dependencies `curvyo-image-core` (for PNG/JPEG
decoding; replacing it with `image` may change pixels, so check against the reference tables
first) and `curvyo-svg-core` (Inkscape baselines in `tests/quality.rs`), and the OCR record test
path into `frontend/`.
