# Plan for Gradient fill

Implementer, 2026-11-18, branch `story/gradient-fill` (from `main` at format 48). Built under
ADR 0066 (document model) and ADR 0067 (drawing, SVG), the UX notes and the drafts in
`docs/design-drafts/2026-11-gradient/`. The format takes **49** (`main` is at 48; renumbered at
merge if another format PR lands first). The slice is built as five Parts, each a set of green
commits on the one branch; the PR is opened after Part E.

## Affected crates/modules

- **`curvyo-document-core`** (Part A, A2): `gradient_model.rs` (new: `GradientFill`,
  `GradientGeometry`, `GradientKind`, `BoxPoint`, `BoxFraction`, `AxisRatio`, `StopOffset`,
  `GradientStop`, `GradientStops`), `gradient_codec.rs` (new: the two registers, lenient read,
  change write, strict validation), `gradient_eval.rs` (new: `GradientBox`, `PlacedGradient`,
  `colour_at`, `PremultipliedRgba8`, `over_white_grey`), `gradient_map.rs` (new: the image of a
  gradient under an affine map, `rebased`), `gradient_stops.rs` (new: stop list edits: add, remove,
  move, recolour, reverse); `pattern_model.rs` (`FillPaint::Gradient`), `pattern_codec.rs` (the
  paint value), `style_model.rs` (`Fill::gradient`, `paints`), `style_codec.rs` (keys in
  `ALL_STYLE_KEYS`, read, write), `style_validation.rs`, `styles.rs` (`StyleEdit` gradient edits and
  labels), `gradient_commands.rs` (new: per-object geometry in one commit), `document.rs` (format 49).
- **`curvyo-ui-core`** (A, A2, B, C): `history_names.rs` (the six names), transform and operation
  results carry the mapped gradient (A2), SVG mapping both ways (B), interior hit (C), mixed panel
  view (C).
- **`curvyo-svg-core`** (B): `paint.rs` (`PaintValue::Gradient`, `gradient_middle` removed),
  `gradient.rs` (new reader: href chains, units, transform, stops), a gradient writer module, export
  model `SvgGradient`, import-outcome counts; goldens; `docs/svg-import.md`.
- **`curvyo-render-core`** (C): `gradient_fill.rs` (new: `GradientSpan`, stop table,
  `gradient_attributes`), laser-page grey stops.
- **`curvyo-editor-wasm`** (C): `gpu_pipeline.rs` (vertex buffer 1, stop texture, fragment),
  `session/gradient*.rs` (panel view, commands, handles, entry boxes data, hit priority, drags with
  preview and Escape), `wasm_gradient.rs` (bindings), `tests/cpu_time/` budgets.
- **`frontend`** (D): Fill switcher Gradient item and icons, `GradientBlock`, stop rows, handles
  overlay, entry boxes, inline picker under the active stop, import notice.

## Tasks

### Part A: core model (Opus) — `curvyo-document-core`, history names in `curvyo-ui-core`

- [x] A1. Typed model with validating constructors and unit tests: fractions finite, ratio above 0,
  offsets in [0, 1], at least 2 stops, first 0, last 1, non-decreasing; defaults (Linear
  (0, 0.5) to (1, 0.5); Radial centre (0.5, 0.5), radius (1, 0.5), ratio 1) (AC 2, 4, 22, 25).
- [x] A2. Codec: `fill_paint = "gradient"`, `fill_gradient`, `fill_gradient_stops`; lenient read,
  write only the register that changed, hex read in either case and written upper case; keys in
  `ALL_STYLE_KEYS` so copy, duplicate, Paste style, Object to path and clones carry them;
  `legacy_fill` still ignores `fill_kind` / `fill_stops` (AC 16, 17, 22, 23).
- [x] A3. Strict open validation and the refusal reasons of ADR 0066 §7, one unit case each
  (AC 22).
- [x] A4. Commands and labels: `StyleEdit::FillPaint(Gradient)` (first choice writes the defaults
  from the Colour, switching away keeps both registers), Kind switch, stop colour / position / add /
  remove / reverse per shape, `Document::set_gradient_geometries` (Angle, Radius, handle drags, one
  commit for many shapes); labels `fill_gradient`, `gradient_linear`, `gradient_radial`,
  `edit_gradient`, `add_gradient_stop`, `remove_gradient_stop`, `reverse_gradient` and their
  History names; undo restores exactly; a drag on one peer and a stop edit on another both survive
  a merge (AC 2, 3, 6, 7, 8, 18, 22).
- [x] A5. Evaluation: `PlacedGradient` from the fill and the leaf's box in its frame (clone matrix
  included), `colour_at` in integers (pad, later stop wins on a hard edge, premultiplied lerp),
  `over_white_grey`; the three tests of AC 5 and the grey of AC 10; Angle and Radius helpers in mm
  (AC 4, 5, 10, 17).
- [x] A6. Gradient map: exact image under an affine map (linear iso-lines under skew, radial
  through the SVD into centre, radius point and ratio), re-expressed in a new box; `rebased` for
  operation results; degenerate maps keep the gradient (AC 15, 19).
- [x] A7. Format 49 and goldens: `gradient_fill_v49.curvyo` (Linear, Radial, elliptical, 9 stops,
  open path, text, linked clone, own-style clone) and the five damaged files, each refused; older
  goldens open; the format-number assertions of earlier tests move to 49 (AC 22, 23).

### Part A2: transforms and operations follow the gradient (Opus) — `document-core`, `ui-core`

- [x] A2.1 Every command that rewrites a leaf by an affine map (scale, mirror, skew of a path,
  Object to path of a rotated primitive, clone matrix) writes the mapped gradient in the same
  commit; moves and a primitive's rotation write nothing; node edits re-fit (AC 15).
- [x] A2.2 Boolean, Combine, Split, Break apart, Offset, Fracture, Object to path keep the first
  input's gradient re-based so the shade stays where it was; Flatten and Weld keep each piece's;
  Spread colours leaves gradients and says so in its skip line (AC 17, 19).

### Part B: SVG (Opus) — `curvyo-svg-core`, mapping in `curvyo-ui-core`

- [x] B1. Export model `SvgGradient` and writer: `linearGradient` / `radialGradient` in `defs`,
  `gradient-n` ids in document order, `userSpaceOnUse`, stops with `offset`, `stop-color`,
  `stop-opacity`, `gradientTransform` only for an elliptical radial; goldens
  `tests/fixtures/export/gradient_*.svg` (7), each read back (AC 13).
- [x] B2. Import: `PaintValue::Gradient`, `gradient.rs` (href / xlink:href chains of 16, attribute
  and stop inheritance, both units, `gradientTransform`, stop-opacity and style stops, offsets
  clamped and monotonic, ends completed, fill-opacity multiplied, spreadMethod pad, focal points as
  centre, mesh as no fill), the counts in the import outcome; `gradient_middle` removed; goldens
  `tests/fixtures/svg_real/inkscape_*.svg` and `illustrator_gradient_*.svg` (AC 14, 27).
- [x] B3. `docs/svg-import.md`, `docs/requirements.md` R-EDIT-006 (AC 26).

### Part C: drawing, canvas editing, session (Opus) — `render-core`, `editor-wasm`, `ui-core`

- [x] C1. Draw list: `GradientSpan`, stop table, `gradient_attributes`; leaf tessellated as its
  Colour fill (holes, fill rule, open path closed, glyph outlines); grey stops on a laser page;
  thumbnails by the same list (AC 9, 10, 11, 20, 21).
- [x] C2. Host pipeline: vertex buffer 1, `Rgba32Float` stop texture, fragment lookup, re-upload
  only on change, no frame skip (AC 9, 24 b, e).
- [x] C3. Interior hit where `colour_at` has alpha above 0 (AC 9).
- [x] C4. Session: panel view (kind, Angle / Radius, stops, Mixed, skip line), commands, active
  stop; handles with their entry-box data, the 8 px priority rule and the 24 px / 12 px hiding,
  drags as style preview with Shift angle stops, Ctrl whole-line move, Escape, double-click adds,
  Delete removes an inner stop; bindings `wasm_gradient.rs` (AC 3, 6, 7, 8, 18).
- [x] C5. Budgets in `curvyo-editor-wasm/tests/gradient_budget.rs`: 24 a, b, c, e met; 24 d met
  as amended (see Notes) (AC 24).
- [x] C7. Compact text form of `fill_gradient` and `fill_gradient_stops` (`gradient_text.rs`):
  round-trip, every damage reason refused, merge of a drag and a stop edit, goldens regenerated
  (AC 22, 24 d).
- [x] C6. Laser jobs unchanged by a Gradient fill (AC 12).

Part C bindings for Part D (`wasm_gradient.rs`; JSON decoders `frontend/src/lib/gradientView.ts`,
fixtures `curvyo-editor-wasm/tests/fixtures/gradient_view/`):
`gradient_view_json()` (the block), `gradient_handles_json()` (handles in screen pixels, line,
ellipse, readout, entry box), `set_fill_kind("gradient")` (existing, `wasm_pattern.rs`),
`set_gradient_kind("linear"|"radial")`, `set_gradient_angle_text`, `set_gradient_radius_text`,
`set_gradient_stop_hex(stop, text)`, `set_gradient_stop_position_text(stop, text)` (each answers
`"committed"`, `"unchanged"` or `"invalid:<code>"`), `preview_gradient_stop_hsv(stop, h, s, v)` (ends
with the existing `commit_style_preview` / `cancel_style_preview`), `add_gradient_stop()`,
`remove_gradient_stop(stop)`, `reverse_gradient()`, `set_gradient_active_stop(stop)`,
`commit_gradient_entry(field_text, hex)`, `close_gradient_entry()`,
`take_gradient_import_notice()` (lines joined by `\n`). Handle presses, drags, Escape, Delete and
double-clicks go through the existing canvas pointer, key and `double_click` calls; after a
double-click the host reads the entry box from `gradient_handles_json()`.

### Part D: frontend (Sonnet) — `frontend`

- [x] D1. Fill switcher Gradient item (`GradientPaintIcon`), Linear / Radial icons (AC 1).
- [x] D2. `GradientBlock`: Kind row with Angle or Radius, stop rows, active stop with the inline
  picker, Add stop, Reverse, Mixed line (AC 3, 8).
- [x] D3. Canvas handles overlay, gestures, the two entry boxes (AC 6, 7).
- [x] D4. Import notice lines (AC 27).

- Part D built 2026-11-18: `useGradient.ts` (one hook: the block, the handles polled per frame,
  the commands; `useEditorSession` unchanged), `GradientBlock.tsx`, `GradientOverlay.tsx`,
  `GradientEntryBox.tsx`, `GradientIcons.tsx`, `gradientText.ts`; `ColourPicker` gains `width` and
  `areaHeight` (the entry box hosts it at 236 x 80), `EntryField` gains `compact` (13 px hex). The
  import notice joins `take_gradient_import_notice` to the Open and Import notices in
  `useProject.ts`. Browser check: `frontend/tests/browser/gradient-fill.cjs`. Deviations
  (Proposal): **no eyedropper per stop** (the session has no pick target for a stop; Part C would
  add `begin_pick("gradient-stop")`); **no row windowing** above 40 stops (all rows render; the
  panel scrolls); the entry box's picker previews and Enter writes (one step), the panel picker
  commits at release.

### Part E: gate, docs, PR (lead)

- [ ] E1. Full gate (`CLAUDE.md` §7), browser check on WebKitGTK, design-system row confirmed,
  spec `Status: Done (PR #n)`, PR description with the review guide (AC 26).

## Validation

- Unit tests per module, golden project files (A7), SVG goldens read back by the reader and an
  independent parser (B), native thread-CPU budgets (C5), and one browser check of the GPU path on
  WebKitGTK (C2). The tester writes the acceptance tests from the specification.

## Debt (size limits, architect review S9)

Split in the review round: `svg-core` `read_gradient` (its shape read is `read_shape`),
`gradient/view.rs` `gradient_view` (`gradient_skips`, `stop_rows`), `gradient/entry.rs`
`commit_gradient_entry` (`entry_field`), `svg_export_leaf.rs` `export_style` (`export_stroke`),
`style_codec.rs` `write_changes` (`write_fill_changes`), `clones.rs`
`unlink_clones_and_texts` (`unlinked_outline`), `clone_unlink.rs` (`unlinked_leaf`);
`YieldedHandles` moved out of `transform_handle_layout.rs` into `yielded_handles.rs`.

Justified (each was over the limit on `main` and grows by the few lines that carry or draw the
gradient in the one place the gesture or command already handles every style; splitting them
is a refactor outside this story, noted for the refactorer): `transform_objects` (133 lines,
was 124), `write_offset` (83, was 71), `replace_with_pieces` (78, was 68), `write_group` (68,
was 62), `resize_by_box` (82, was 74), `svg_import.rs` `map` (139, was 124), `gpu.rs` `render`
(95, was 86), `output_pages_frame` (70, was 63), `double_click_hint` (68, was 63),
`key_down_dispatch`, `Session::new`, `Session::open`, `set_tool`, `read_svg`, `text.rs`
`finish` (1 or 2 lines each). Modules already over 500 lines on `main` grow by 1 to 35 lines
(`document.rs`, `shapes.rs`, `transform_objects.rs`, `glyphs.rs`, `group_transform.rs`,
`hit_test_object.rs`, `history_names.rs`, `select_tool.rs`, `svg_export.rs`, `transform_drag.rs`,
`transform_handle_layout.rs`, `session/mod.rs`, `keys.rs`, `select.rs`, `draw.rs`, `lib.rs`
files). New past the limit: `styles.rs` 569 (was 507): the eight gradient `StyleEdit` variants,
their labels and their `apply_to` arms belong to the one enum every style edit is; their
application lives in `gradient_commands.rs`, and splitting the enum would split the one match
that keeps style edits exhaustive. `gpu.rs` 559 (was 542): vertex buffer 1 and the stop table
binding; the stop table and the attribute buffer are their own modules (`gpu_stops.rs`,
`gpu_gradient_buffer.rs`). `pattern_model.rs` 539 (was 537) and `transform_commit.rs` 545 (was
542): two and three lines.

## Notes and deviations

- **Reverse writes the stops only** (UX notes §4: "the handles stay where they are, so the shade
  flips"); ADR 0066 §2 also lists `fill_gradient` for Reverse, which would undo the flip.
- **Angle and Radius are per-object geometry writes** (`Document::set_gradient_geometries`): both
  are measured in mm in the shape's frame, so their geometry depends on each shape's box, which the
  session knows and a plain `StyleEdit` does not.
- Part A built 2026-11-18: `gradient_model.rs`, `gradient_codec.rs`, `gradient_eval.rs`,
  `gradient_map.rs`, `gradient_measure.rs` (Angle and Radius in mm), `gradient_stops.rs`,
  `gradient_commands.rs`; `styles.rs` grows to 568 lines (was 507 on `main`) by the eight
  `StyleEdit` variants and their labels; their application lives in `gradient_commands.rs`.
- **Paste style does not exist** in the product today (no command or binding); criterion 16's
  Paste style is covered by the keys being in `ALL_STYLE_KEYS` and by duplicate and clone tests.
- Budget 24 d (open 10,000 gradient shapes) is measured with the others in C5.
- Part A2 built 2026-11-18: `gradient_frame.rs` (new: the box a gradient is placed in, the tight
  box of the outlines in the `rotation` frame, a text's line box, a clone's root box under its
  composed matrix; `carry_gradient`, `rebased_for_piece`), so the defaults sit on the oriented
  box's edge handles. The gesture results carry the image (`group_transform::apply`,
  `resize_by_box`, `skewed_unchecked`); `transform_objects` and `Document::write_carried_gradient`
  write only `fill_gradient` in the gesture's step. A rotation and a move need nothing (the frame
  absorbs them), nor do clones (their matrix) and texts (their placement). `replace_with_path`,
  `replace_with_pieces`, `write_offset`, `convert_to_paths` and the text conversion re-base
  (`TextConversion` gains `frame`, the text's line box). A flip of a symmetric shape with a
  gradient no longer counts as "looks the same". Spread colours writes only colour registers, so
  gradients stay; its skip-line wording is Part C/D.
- Part B built 2026-11-18: `curvyo-svg-core` `gradient_paint.rs` (the shared `SvgGradient`,
  `SvgStop`, `GradientLosses`), `gradient.rs` (the reader), `write_gradient.rs` (the writer);
  `PaintValue::Gradient` / `Mesh`, `LeafStyle::fill_gradient`, `ExportStyle::fill_gradient`,
  `SvgDrawing::gradient_losses`; `curvyo-ui-core` `svg_export_gradient.rs`,
  `svg_import_gradient.rs` (with `gradient_loss_lines`), `SvgImport::gradient_losses`. The export
  model holds the map from the unit gradient into the file; the writer re-expresses it in each
  element's own user space (a turned `<rect>` carries `rotate()`), so only an elliptical radial
  writes `gradientTransform`. Each gradient element with a lost feature counts once. Deviations
  (Proposal): a stroke that refers to a gradient takes its colour at the middle (a stroke has no
  gradient); an imported **text** keeps the first stop as its colour (its line box is known only
  after layout); the `inkscape_*` and `illustrator_*` goldens are hand-written in those programs'
  layout (no Inkscape here; `SOURCES.md`). `acceptance_0024_tester.rs` `ac38_paint_servers`
  follows the change of `0024` this spec lists (first stop plus the gradient, not the middle).
- Part A2 is split out of Part A: wiring the map into every transform and operation command touches
  `curvyo-ui-core`'s result builders; the pure map and `rebased` are in Part A.
- Part C built 2026-11-18: `curvyo-render-core` `gradient_fill.rs` (spans, stop table, texture rows
  of 2,048 texels, two per stop; equal stop lists share one range) with its shader's CPU twin
  `gradient_fill/shader_reference.rs`, `build_artwork_greyed`, `fade_fill` (a faded or dimmed leaf
  fades its stops: a gradient ignores `fill_opacity`); `curvyo-editor-wasm` `gpu_shader.rs` (the
  WGSL, compiled on every target so a host test validates it and translates it to GLSL ES 3.00
  with `naga`, a dev-dependency already in the lock through `wgpu`), `gpu_stops.rs`, vertex buffer 1
  in `gpu_pipeline.rs` / `gpu.rs`, `session/gradient/` (state, view, commands, canvas, entry, laser),
  `wasm_gradient.rs`; `curvyo-ui-core` `gradient_handles.rs`, `gradient_drag.rs`,
  `gradient_panel.rs`, `YieldedHandles` on `TransformHandleTolerances` (a public field added to a
  core crate's struct: the yield rule needs the handle hit and draw list to skip covered
  handles), the alpha rule in `hit_test_object.rs`.
- Deviations (Proposal): the span keeps per-vertex gradient coordinates instead of ADR 0067 §1's
  `to_gradient: [f32; 6]`, so a placement's `map_positions` keeps the shade without inverting a
  closure; the stop texels are premultiplied in `f32` without rounding (`colour_at` blends on the
  wide scale; 8-bit stops differed by up to 1.2 steps). The handles are drawn by the frontend
  from `gradient_handles_json()` (Part D3), not by `render-core`. Inner-stop dots and the line
  yield shape handles as the ends do. Shader tolerance: 1/255 per channel plus `f32` rounding;
  2/255 on 256 stops 1/255 apart, where `colour_at`'s `1 / 65,536` grid moves a channel by one
  more step.
- **Criterion 24 d** (release, this machine, best of three): the first stored form (a Loro map
  and a list of lists) made 10,000 gradients of 4 stops 900 kB and 453 ms to open against Colour's
  593 kB and 258 ms; the compact text form (ADR 0066 Amendments) brought it to about 355 ms. The
  architect review (S5) found the files unfair: the gradient file wrote the default stops and
  then the 4 stops (two operations on one register), the Colour file wrote no colour. Rebuilt
  alike (one drawing insert, every style register written once): Colour 619 kB and about 220 ms,
  Gradient 901 kB and about 273 ms, Pattern 749 kB and about 285 ms, so 1.24 to 1.26 times Colour
  and 0.95 to 0.97 times Pattern over three runs. The budget is restored to the stricter rule
  (Proposal): no longer than Pattern and within Colour plus 30 %. 24 a: 2.7 ms worst move; 24 b: 2.5 ms against 1.9 ms; 24 c: 4.2 ms; 24 e: no
  measurable difference.

