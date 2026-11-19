# Plan for Raster engraving

Implementer, 2026-11-18, branch `story/raster-engraving` (from `main` at format 49, after `0124`).
Built under ADR 0068 (dot values, dithering) and ADR 0069 (rows, output, preview, checks, keys),
the UX notes and the drafts in `docs/design-drafts/2026-11-raster/`. Frage 237 runs with its
default **A** (recipe `dither`, material test record format 3; closed as built A 2026-11-19). The project format takes
**50** (`main` is at 49; renumbered at merge if another format PR lands first). The slice is built
as five Parts, each a set of green commits on the one branch; the PR is opened after Part E.

## Affected crates/modules

- **`curvyo-laser-core`** (A, B): new `job_raster_grid.rs`, `job_raster_sample.rs`,
  `job_raster_tone.rs`, `job_raster_dither.rs`, `job_raster_rows.rs`, `job_raster_plan.rs`,
  `operation_image_options.rs`; extended `job_input.rs` (`LeafGeometry::Image { frame, asset,
  pixels, reduced }`, `LeafGeometry::Gradient`), `job_hatch.rs` (scan lines shared with the
  grid), `job_plan.rs` (raster items between the vector paths and the outlines, grey pyramids in
  the `PlanCache`), `job_moves.rs` (`MoveKind::Sweep`, `Job::sweeps`), `job_timeline.rs`,
  `job_write_grbl.rs`, `job_gate_cause.rs`, `job_gate.rs`, `job_gate_text.rs`,
  `job_fingerprint.rs`, `operation_options.rs`, `operation_kind_run.rs` (gradient leaves);
  B: `test_recipe.rs`, `test_recipe_json.rs`, `test_grid_ops.rs`, `material_test_file.rs`,
  `material_record_file.rs` (record format 3). `Cargo.toml`: `curvyo-image-core`, `libm`
  (ADR 0068 §1; both already in `Cargo.lock`).
- **`curvyo-document-core`** (B): `document.rs` (format 50 and its note), golden projects.
- **`curvyo-ui-core`** (B): history names of the Image settings edits and the grid's Fill/Dither.
- **`curvyo-editor-wasm`** (A: compile glue only; C): `session/output/*` (job input with pixels
  and gradients, Toolpath view of sweeps, checks text), `session/operations/*` (Image settings
  view JSON and edits, Operations-view dots), test grid bar, bindings, contract fixtures,
  `tests/cpu_time/` budgets.
- **`frontend`** (D): Image settings group, grid bar Fill/Dither.

## Tasks

### Part A: laser-core pipeline (Opus)

- [x] A1. `operation_image_options.rs`: `Dither` (seven modes, `floyd_steinberg` default),
  `ImageSettings` (threshold, brightness, contrast, gamma, invert, lowest power) with exact stored
  reads and `to_value`; `OperationOptions.image`; the seven keys read in `read_options`, Engrave
  kinds only, every damage case (AC 13; the typed field rules and edits are B1).
- [x] A2. `job_raster_tone.rs`: one `[u8; 256]` table, Invert, Brightness, Contrast, Gamma in that
  order, `libm::pow` for the 256 gamma entries only (AC 8, 9).
- [x] A3. `job_raster_dither.rs`: Threshold, Ordered 8 x 8, Floyd-Steinberg, Jarvis, Atkinson,
  Stucki in integer error units, columns increasing, no error outside the frame; Grey levels
  (AC 8, 10; white never burns, black always burns, ramp monotonic).
- [x] A4. `job_hatch.rs` scan lines exposed in the scan frame; `job_raster_grid.rs`: rows at the
  hatch's positions, columns half an Interval inside, a dot inside when its centre is inside the
  frame (shared inside test), dot centres in page µm (AC 5, 16 scan angle, 26).
- [x] A5. `job_raster_sample.rs`: grey plane over white with the `over_white_grey` weights, 2 x 2
  box pyramid, level by pixel pitch vs Interval, bilinear in 1/256 steps, edge clamp; gradient
  grey by `colour_at` + `over_white_grey` (AC 4, 6, 7, 24, 27).
- [x] A6. `job_raster_rows.rs`: `Sweep` (start, end, `Levels::Binary | Grey` in run order),
  split at gaps over 5 mm, empty rows and leading/trailing white dropped, bidirectional
  alternation, `runs()`, `run_power` (lowest capped at the set's power) (AC 14, 15, 16, 32 b).
- [x] A7. Plan and moves: `LeafGeometry::Image { frame, asset, pixels, reduced }` and
  `Gradient { outlines, gradient }`; raster items after the vector paths in stacking order, then
  the outlines; `MoveKind::Sweep`, `Job::sweeps`, overscan run-outs, passes repeat the same sweeps,
  Crosshatch ignored for raster; grey pyramids cached per asset in `PlanCache`; fingerprint gains
  asset, frame, gradient bits, image settings (AC 1 to 4, 14 to 18, 38).
- [x] A8. Timeline: a sweep is one burn move of its length at the burn feed; head inside a sweep
  interpolated (AC 21, 22; the 100 x 10 mm test).
- [x] A9. GRBL writer: M4 one `G1` per run with `S` from `run_power`, `S0` over gaps, same-`S`
  runs merged, `F` only on change; M3 `M3 S` / `G1` / `M5` per run, gaps `G0` (AC 19).
- [x] A10. Gate causes `GreyNeedsM4`, `TooManyDots`, `CoarseImages`, `OverlappingRasterItems`,
  `ReducedImages`, `UnreadableImages`, `LowestPowerAbovePower`, their texts with singular forms;
  raster items in `OutsideBed` by their frame (AC 31 to 36).
- [x] A11. Goldens: dot maps in `tests/fixtures/raster/` (ten names of AC 40) and the sixteen
  `tests/fixtures/output/grbl/raster_*.gcode`; `image_skipped.gcode` removed; each G-code read
  back: burned length = burned dots x Interval, total length = timeline's (AC 39, 40).
- [x] A12. Determinism (two builds byte-identical; wasm32 build) and release budgets in
  `curvyo-laser-core` (37 a, b) by thread CPU time (AC 8, 37).

### Part B: settings, recipe and format (Opus)

- [x] B1. Typed Image settings (range texts, rounding to the step), `OptionEdit` variants, one
  undo step "Edit settings"; material commands never touch them (AC 12, 13).
- [x] B2. Recipe `fill` (`solid` | `gradient`) and `dither` (six binary modes, only with
  gradient), damage rules; gradient cells (`#000000FF` to `#FFFFFFFF`, Angle 0) and derived
  operations with `op_options.dither`; "Edit test grid" (AC 44, 45).
- [x] B3. Material test record format 3 (Frage 237 A), format 2 reads Solid; goldens
  `format3_gradient_test.json`, `format2_reads_solid.json` (AC 41, 45).
- [x] B4. Project format 50 with its note in `document.rs`; goldens `raster_engraving_v50.curvyo`,
  `raster_engraving_damaged_v50.curvyo`; save twice byte-identical (AC 13, 38, 41).

### Part C: editor-wasm session (Opus)

- [x] C1. Job input: images in an Engrave with their working image (`Arc`, from the canvas's
  cache or decoded once, `output/pixels.rs`; only Engrave kinds read pixels, 37 g), unreadable as
  `None`; gradient-filled leaves as `LeafGeometry::Gradient`, the gradient box mapped into page
  mm (AC 1 to 4, 23, 26, 28, 30, 36).
- [x] C2. Row "Image settings" view JSON (`operations/image_view.rs`; shown only while the
  operation holds a raster item; Lowest power in Threshold's row; units and range texts), the
  number and Invert edits through `edit_operation_option`, `set_operation_dither` (AC 11, 12).
- [x] C3. Toolpath view: a sweep draws no line; each raster item's sweeps are one dot picture
  (`output/raster_picture.rs`, one texel per dot, power-of-two blocks over 4,096 texels a side)
  drawn through the image pipeline, so its mip levels are the coverage pyramid and a pan never
  walks dots; one ring, one arrowhead per item; an item the play head has not reached is pale
  (AC 20, 22). (Proposal, deviation from ADR 0069 §5: no per-run polylines and no dashed row
  travel; the dots are the picture at every zoom.)
- [x] C4. Operations view: raster items as dots from a `k·I` preview grid (`k` the power of two
  for at least one CSS pixel; `output/raster_view.rs`), no hatch over them, pictures kept while
  their sweeps are the same (AC 20).
- [x] C5. Checks: item names for the dot-limit line, `focus` (`dither`, `lowest_power`) on the
  operations target; the grid bar's Fill and Dither view and edits; the material test record's
  axes line names a Gradient fill and its Dither (AC 31 to 36, 44, 45).
- [x] C6. Contract fixtures (`operations_view/options_engrave_image.json`,
  `output_view/raster_checks.json`, `test_grid_view/engrave_gradient_ordered.json`) with the
  frontend's checked decoders; budgets 37 c to f in `tests/raster_session_budget.rs`, 37 g as a
  session test (AC 37).

### Part D: frontend (Sonnet)

- [x] D1. Image settings group: Dither pick (seven modes), Threshold / Lowest power in one row,
  Brightness, Contrast, Gamma, Invert icon toggle, shown only with a raster item (AC 11, 12).
- [x] D2. Grid bar Fill radiogroup and Dither menu button (AC 44).
- [x] D3. Check lines' buttons (`focus` on the operations target: Dither, Lowest power); no new colour
  (AC 20, 31 to 36). Browser check `frontend/tests/browser/raster-engraving.cjs` passes.

### Rework: Image operation kind (2026-11-19, customer feedback on PR #394)

ADR 0069 §13, §14 and the ADR 0068 amendment; `adrs.md` decisions of 2026-11-19; customer answers
Frage 270 **B** (a fourth material set Image per laser) and 271 **A, sharpened** (in every kind
but Image an image is its frame rectangle). Criterion numbers follow the PO's reworked spec; R1
first (a bug, also visible in Cut), then R2 to R9 in order, each green on its own.

- [x] R1. **Linked clone of an image burns** (editor-wasm, Opus). Root cause in `adrs.md`
  (2026-11-19): a picture clone of one leaf is indexed as a closed outline with no geometry. In
  `session/operations/index.rs` `build_leaves` expands every picture clone
  (`object.picture().is_some()`), not only group clones: a clone of an image or a text gives one
  leaf with the clone's own chain (`key_chain(&[clone])`, so its assignment and select id stay the
  clone's), `shape_of` its picture leaf, source `LeafSource::CloneMember { clone: at, member:
  root }` (doc: "of a group, text or image"). `placed_leaves` expands a picture clone member the
  same way (chain plus the placement; a new source `PlacedCloneMember { clone, member, placement
  }` read in `output/input.rs` `geometry_of` from the clone's picture through the placement map).
  `leaf_geometry`'s doc drops "a clone of a picture". Tests first (session,
  `output/tests/raster_tests.rs` and `operations/tests.rs`): a linked clone of an image in an Image
  operation burns at the clone's place (sweeps in the job, dots in the Operations view, the Image
  settings group shown); a clone of a clone of an image; a clone of a group holding an image and a
  gradient rectangle burns both (regression); an image and a clone of an image on a placed page burn
  mapped by the placement; a clone of a text cuts (same root cause).
- [x] R2. **The kind** (laser-core, Opus). `OperationKind::Image` (`image`, "Image", Marking;
  `ALL` order per UX notes, default after Engrave + Outline); `LeafShape::Gradient { closed, open
  }`; `kind_run`: Image runs images and gradient shapes as raster, anything else nothing; in
  Cut, Score, Cut-out, Engrave and Engrave + Outline `LeafShape::Image` runs exactly as `Outlines
  { closed: 1, open: 0 }` (Frage 271 A: Score scores the frame, Engrave hatches it, Engrave +
  Outline hatches and traces it) and `Gradient` as `Outlines`; `Lines::Frame` removed.
  `main.interval_mm` allowed on `image`. Every exhaustive `match` gets its arm; the fixture job
  kind pick offers no Image. Unit tests per pair of kind and shape, one asserting
  `kind_run(k, Image) == kind_run(k, one closed outline)` for every vector kind.
- [x] R3. **Options per kind** (laser-core, Opus). `operation_options.rs`: the seven image keys on
  `image` only; `scans` (bidirectional, scan angle, overscan: Engrave kinds and Image) beside
  `hatches` (crosshatch: Engrave kinds); damage tests: `dither` on an Engrave, `crosshatch` on an
  Image, `threshold_pct` on an Engrave + Outline.
- [x] R4. **Dispatch and gate** (laser-core, Opus). `job_plan.rs` / `job_raster_items.rs`: raster
  plan only for Image blocks (raster items only, stacking order, no outline role); in every other
  block `LeafGeometry::Image` becomes its frame ring and `Gradient` its outlines before planning.
  Raster causes for Image blocks only. Goldens: `raster_engrave_outline.gcode` removed; the other
  `raster_*.gcode` and dot maps rebuilt from Image blocks with unchanged bytes (asserted); tests:
  an Engrave of a gradient square equals the hatch of the same solid square; an Engrave, a Score
  and an Engrave + Outline of an image (rotated, so the frame is a parallelogram) equal the same
  kind of a rectangle path on its frame, move for move.
- [x] R5. **Image set** (laser-core, Opus; ADR 0069 §14). `SetKind::Image` (`image`, "Image",
  `ALL` last), `SetKind::has_interval()` replacing every Engrave-only interval test
  (`material_record_file.rs`, `material_record_edits.rs`, `test_param.rs`); record format 3 in
  place: `sets.image` (interval required), history and recipe kind `image`, a format-2 record
  holding any of them damaged; `no_laser_set_text(Image)`. `operation_settings_rule.rs`:
  `main_set(Image) = Image`, `linked_interval` for kinds with an interval, no fallback to Engrave.
  Tests first: resolve Live / Stored `NoSet(Image)` / Own for an Image operation; Use material with
  and without an Image set; `refresh` with and without the guard follows the Image set (material
  edit, save, export path); judging an Image test writes the Image set and a `set` entry of kind
  `image`, never the Engrave set; an Engrave operation on the same material still reads the Engrave
  set.
- [x] R6. **Test grid and material record** (laser-core, Opus). The recipe kind is `SetKind`
  with Image (no separate `TestKind`); `operation_kind()` maps Image to `OperationKind::Image`;
  recipe JSON `kind: "image"` with `dither` (six binary modes, absent Floyd-Steinberg); the
  Interval parameter offered on Image; `fill` removed (kept only if the PO's spec keeps a Fill
  choice on the Image grid); `fill`/`dither` on another kind is a damaged recipe; Image cells are
  the gradient rectangle, derived operations `image` with `op_options.dither`; Crosshatch only
  Engrave. Record format stays 3; golden `format3_gradient_test.json` becomes
  `format3_image_test.json` (an Image test, the Image set sourced from it, a `set` entry of kind
  `image`; round trip byte for byte); new damaged goldens `image_set_no_interval.json`,
  `format2_image_set.json`; `format2_reads_solid.json` kept; `test-grid-choice.json` keeps format 2
  and reads an Image recipe.
- [x] R7. **Project goldens** (document-core, Opus). `raster_engraving_v50.curvyo` regenerated
  (raster items in `image` operations with the seven keys, one of them linked to a material with
  its `op_settings` holding the Image set's numbers and interval, a linked clone of the image, an
  Engrave holding a gradient shape and the image (hatching its frame), an Image test grid); `raster_engraving_damaged_v50.curvyo` adds `dither` on an Engrave and
  `crosshatch` on an Image; save twice byte-identical; the format 50 note in `document.rs` names
  the kind `image`. No document-core type changes (the kind stays opaque).
- [x] R8. **Session** (editor-wasm, Opus). `output/input.rs`: pixels only for Image blocks
  (`engraves` becomes the Image test), `raster_ops` replaced by the kind; the Image settings view
  (`operations/image_view.rs`) on every Image row, never on Engrave; select-bar kind buttons
  (six, `kind_press.rs`), naming "Image 1", history labels; Operations view looks (Image dots,
  Engrave hatch of gradient shapes, vector leaves in Image "runs nothing"); fixture jobs' look arm;
  test grid bar: kind Image, Fill gone, Dither only on Image; contract fixtures renamed
  (`operations_view/options_image.json`, `test_grid_view/image_ordered.json`,
  `output_view/raster_checks.json` regenerated); acceptance and budget tests moved to Image
  operations. Materials (`session/materials/`): `KINDS` and `set_kind` with Image, four set rows
  per laser, Add / type / remove an Image set with its Interval field, history rows "Image",
  judging an Image grid cell; operation settings view: "Material has no Image set for this laser";
  contract fixtures `materials_view/` regenerated with an Image set; session tests: a linked Image
  operation follows an Image set edit (one undo step) and keeps its numbers on export refresh.
  Operations view look of an image in Score, Engrave and Engrave + Outline is its frame rectangle.
  *As built (2026-11-19):* R3 and R4 are one commit (the image keys leave Engrave as Engrave
  stops rastering, so either alone leaves the suite red). Criterion 48's line is raised from the
  gate facts (`GateFacts::not_pictures`, the session's assignment), since such a leaf is in no
  block; a plain leaf in Image takes the muted dashed look with no chip (UX E10's muted Image
  glyph is not drawn: render-core chips have no muted ink yet; for the UX review; *drawn since
  F3, `ChipItem::muted`*). The test grid
  view's `fill` becomes `dither: {value, modes}` (Image only); `crosshatch` in the hatch options
  view is `null` on Image. Frontend fixture names changed: `operations_view/options_image.json`,
  `test_grid_view/image_ordered.json`, new `materials_view/image_set.json`.
- [x] R9. **Frontend** (Sonnet). Kind unions and decoders for `image`; the Image kind button and
  glyph (`OperationGlyphs.tsx`, UX notes); `operationOptionRows.ts`: Image settings group on Image
  rows, Crosshatch not on Image; the **Dither pick as an always-visible segmented control** (seven
  segments in the Image settings group, six in the grid bar; UX notes); grid bar kind with Image,
  Fill removed; Materials tab: the fourth set row Image (kind unions, decoders, the set editor with
  Interval, the UX notes' row and glyph); browser check `raster-engraving.cjs` on an Image
  operation, a linked clone of an image included, an Image test judged into the Image set.
- [ ] R10. Tester: acceptance tests from the reworked spec (Image kind, Image set, linked clone,
  placed page, clone of a group, an image hatched by Engrave and scored by Score), then the full
  gate.

### Part E: gate, docs, PR

- [x] F1. (2026-11-19, tester findings) Criteria 12, 37 d: sweeps kept per raster item and worked in
  steps (`job_raster_work.rs`, `prepare_job`); the views read the settled job, the host ticks
  `advance_job` per animation frame (ADR 0069 §12, UX P35); `raster_session_budget` times the app
  frame; budgets in CI. Spec wording of criteria 1 and 31 corrected; `raster_dot_map` doc link.
- [x] F2. (2026-11-19, architect and UX reviews) S1 the Toolpath view and the output group read
  the same job, a tick syncs once the job is no longer pending (AC 37 d); S2 the preview capped at
  300,000 dots per operation, budget at preview step 1 (AC 37 d); S3 one grey pyramid per asset
  (AC 37 c, 38); S4 unreadable images stay held, the canvas reuses the job's pixels (AC 7, 36);
  S5 picture frame from the longest sweep (AC 20); S6 `job_raster_items`, `job_flatten`,
  `tab_facts` split out, `check_text` and `picture_of` split; S7 1,000-image budget, set and map
  lookups, the overlap sweep, the build in its own tick (AC 37 d); U1 the open Dither list
  previews the focused mode with no step, Escape restores (P23, AC 12); U2 raster items show
  without numbers, Not assigned images dimmed (AC 20); U3 field widths (P34); U4 list height;
  U5 the dot-limit line stands in for Nothing runs, row time "–" (AC 34); U6 segment focus;
  U7 the last numbers for 300 ms (P35); U8 Dither list in view, pointer pick keeps focus, Solid to
  Gradient keeps Dither (AC 44).
- [x] F3. (2026-11-19, second architect and UX reviews) A1 picture leaves are read by their
  place (`CloneChain::picture_members` names each leaf's member; one `picture_leaves` for page
  items, cloned groups and placed pages), so a linked clone inside a cloned group burns at its
  place in both stacking orders (AC 46); A4 sweeps carry their leaf's stacking place, a dot picture
  per raster leaf (AC 20, 46 b); A2/U1 picture clones get the Engrave hatch and their kind chip
  (AC 1, 46 b); U2 a vector shape in Image and an Output-off item keep a muted chip (AC 48, `0101`
  13.2); A3 the Document strips render as accepted, the Dither strips opt in to fixed rows; A9 the
  first read works 100,000 dots (AC 37 d, ADR 0069 §12); U3 to U9 and the nits (`KeyOnKind`,
  `libm` in the workspace, the 300 ms mark armed once per pending job); A7 `look.rs`,
  `input.rs`, `operation_image_options.rs` split; A5, A6, A8 docs.
- [ ] E1. Full gate (`CLAUDE.md` §7); browser check of the demo path.
- [ ] E2. Docs: notes on ADR 0011, 0046, 0054, 0056, 0057, 0058, 0064 (ADR 0069 §13's list; written
  2026-11-19, F3); ADR 0068/0069 accepted with the PR;
  `docs/requirements.md` R-MFG-LASER-001; spec `Status: Done (PR #n)`; `specs/README.md` row and
  format plan.
- [ ] E3. PR with "What changes in the demo" and the review guide.

## Debt (sizes over the `CLAUDE.md` §5 limits, justified)

Measured 2026-11-19 after F3. Split in F3: `look.rs` (574 to 443 lines: `look_overlay.rs`,
`look_picture.rs`), `input.rs` (509 to 452: `gate_facts.rs`), `operation_image_options.rs` (501 to
308: its tests in `operation_image_options_tests.rs`), `build_leaves` (71 to 57),
`operation_block` (71 to 46), `operation_looked_artwork` (76 to 59), `preview_items` (68 to 53).

- Files over 500 lines on `main` already, which this story grows a little:
  `curvyo-document-core/src/clone_picture.rs` (669 to 713: the picture members and their test),
  `curvyo-editor-wasm/src/session/history.rs` (546 to 569: the step summary),
  `curvyo-render-core/src/theme.rs` (535 to 539: `OP_MUTED_INK`),
  `curvyo-document-core/src/primitive_model.rs` (1,066 to 1,076: `picture_members()`),
  `curvyo-editor-wasm/src/gpu.rs` (559 to 596: the dot pictures' texture upload),
  `curvyo-app/src/connector.rs` (577 to 592), `curvyo-editor-wasm/src/session/draw.rs` (615 to
  621), `curvyo-ui-core/src/lib.rs` (684 to 687), `curvyo-editor-wasm/src/session/mod.rs` (2,148
  to 2,149). Each is a list of one responsibility's cases; their splits belong to refactor PRs,
  noted for the lead.
- Functions over 60 lines, each one straight pass that a split would only cut into half-steps:
  `group_list` (clone_picture.rs, 71; 69 on `main`, +2 for the members), `operations_view`
  (operations/view_json.rs, 66), `read_tests` (material_test_file.rs, 63), `fixture_blocks`
  (input.rs, 62). `page_input` (57), `check_text` and `picture_of` were split in F2.

## Validation

Core logic test-first in `curvyo-laser-core` unit tests and integration goldens; the tester writes
the acceptance tests from the spec before reading the code. Budgets by thread CPU time in release
builds, one `--test` at a time. `wasm32-unknown-unknown` build of every touched `*-core` crate.
