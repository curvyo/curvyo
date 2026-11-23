# Plan for Print and cut, slice 1

Implementer, 2026-11-23, branch `story/print-and-cut` (rebased 2026-11-23 on `main` at format 54, after the docs
commits of the spec, ADR 0081 and the UX notes). Built under ADR 0081 (Proposed, `needs-customer`,
accepted with the PR), Fragen 328 to 333 at their default A, the UX notes and the drafts in
`docs/design-drafts/2026-11-print-and-cut/`. The project format takes **55** (`main` is at 54;
renumbered at merge if another format PR lands first). Criterion numbers refer to
`specification.md`. Proposals of the UX notes (P1 to P22) are built as written unless a task says
otherwise.

## Affected crates/modules

- **`curvyo-geometry-core`**: new `point_fit.rs` (`fit_similarity`, `PointPair`, `SimilarityFit`,
  `FitError`); `+ - * /` only for the matrix, `libm` for the display angle and scale.
- **`curvyo-document-core`**: new `registration_mark_model.rs` (snapshot, limits, labels),
  `registration_mark_codec.rs` (the leaf's registers, the marks group tag, validation),
  `registration_mark_lines.rs` (the drawn lines: circle, crosshair, stroke digit),
  `registration_mark_commands.rs` (insert, add mark, size, the delete rule, the placement rule of
  criterion 1), `print_and_cut_codec.rs` (the `x.curvyo.laser.print_and_cut` register),
  `print_and_cut_commands.rs` (measure, tolerance, Apply, Reset); the `RegistrationMark` arm in
  `ObjectSnapshot`, `RawLeaf`, the reads and the transforms (centre only); `document.rs` format
  55; goldens `print_and_cut_v55.curvyo`, `print_and_cut_damaged_v55.curvyo`.
- **`curvyo-laser-core`**: new `print_and_cut.rs` (the reading of decision 5: rows, causes, fit,
  warnings, Apply state, work position to page mm) and `print_and_cut_text.rs` (the words); GRBL
  golden `tests/fixtures/output/grbl/print_and_cut_cut_line.gcode`.
- **`curvyo-ui-core`**: the new arm in every exhaustive match (bounds and hit from the drawn
  lines, move only, no style panel, no conversions), SVG export of a mark as
  `<g data-curvyo-kind="registration-mark" data-curvyo-mark="n">`, history names.
- **`curvyo-render-core`**: the mark's lines as artwork on a drawing page (PNG export reads the
  same), nothing in operation looks.
- **`curvyo-svg-core`**: golden `tests/fixtures/export/registration_marks.svg` (the export test
  lives where the other export goldens run).
- **`curvyo-editor-wasm`**: `leaf_geometry` arm returning `None`; session commands and views
  (`session/print_and_cut*.rs`, `session/registration_marks*.rs`), `wasm_print_and_cut.rs`;
  laser canvas overlay data (faint crosses, rings, fit outline).
- **`frontend`**: Document tab button, Registration marks Style block, Print and cut group in the
  laser page's Operations tab, canvas overlay.

## Tasks

### Part A: the fit (geometry-core, test-first)

- [x] A1. `point_fit.rs`: closed-form similarity (Umeyama without reflection), residuals, largest,
  RMS, mirrored RMS for 3+ pairs, `TooFewPairs`, `TooClose` by `min_spread`. Tests: 2, 3, 4 pairs;
  known rotation, scale, offset without noise (< 1e-6 mm) and with fixed 0.2 mm noise (exact
  residuals); coincident marks and coincident measures; a repeated measure; reversed order
  (mirrored); collinear (not mirrored); pinned bits of one fit; proptest against random exact
  similarities (AC 11, 12, 15).

### Part B: document model and format (document-core, test-first)

- [x] B1. Mark leaf: model, codec, validator (number 1 to 4, finite centre, diameter 3 to 20,
  line 0.05 to 1), drawn lines, the `ObjectSnapshot::RegistrationMark` arm; transforms move the
  centre only (AC 2, 15).
- [x] B2. Marks object: group tagged `registration_marks`; `insert_registration_marks` with the
  placement rule (page corners when clear by 10 mm, else the artwork box grown by 10 mm clamped
  10 mm inside the page; empty page: page corners), one commit; Add mark (lowest free number at
  its nominal corner); Size and Line width for every mark in one commit; Delete refused below 2
  marks (`MarksBelowTwo`), the whole object deletes; Ungroup refused (AC 1, 2, 3, 6).
- [x] B3. Register `x.curvyo.laser.print_and_cut` on a placement: `marks` µm, `tolerance_um`,
  `reset`; read as `Absent | Unreadable | Measures`; `set_print_and_cut_measure`,
  `set_print_and_cut_tolerance` (one commit each, an unreadable register replaced), Apply (matrix
  and `reset` in one commit, off-page refusal, drawing page untouched), Reset; Duplicate of a
  placement copies no register (AC 9, 16, 18, 19).
- [x] B4. Format 55, the version note, the older-build refusal; goldens `print_and_cut_v55.curvyo`
  and `print_and_cut_damaged_v55.curvyo` open and save back unchanged; a project without the
  register opens as before (AC 19).

### Part C: laser side (laser-core, test-first)

- [x] C1. `print_and_cut.rs`: from marks, register, `Frame`, matrix, Start to rows (filled, half
  filled, gone), the causes in the ADR order, the fit summary (moved = machine position of the
  page origin, turned, scale, largest, RMS, worst mark), warnings (tolerance, scale 98 to 102 %,
  mirrored, same place within 1 mm, Start is Head position), Apply offered (scale 50 to 200 %),
  on-fit within 1 µm at the four corners, the fitted matrix in page mm; origin-corner flips never
  mirror (AC 10 to 14, 18).
- [x] C2. `print_and_cut_text.rs`: every text of the spec and the UX notes (AC 10, 13, 14, 16).
- [x] C3. Job: `leaf_geometry` returns `None` for a mark (editor-wasm); a test counts zero burn
  moves from marks under a Cut over everything, on the page and placed; GRBL golden
  `print_and_cut_cut_line.gcode`: a 40 mm square placed by a 90 degree fit with an offset (AC 4,
  17, 22).

### Part D: views and export (ui-core, render-core, editor-wasm)

- [x] D1. Every exhaustive match gains the arm (bounds and hit from the lines, move only, no
  style, never converted, never cloned on its own).
- [x] D2. SVG export of the marks and its golden `registration_marks.svg`; PNG draws the lines
  (AC 4).
- [x] D3. Session: Add registration marks (page box and artwork box from the existing bounds),
  marks block view and edits, delete refusal notice, Print and cut group view JSON (sheet picker,
  rows, state, fit block, warnings, Apply and Reset state), measure, Clear, tolerance, Take head
  position value (µm rounding), Apply, Reset; laser canvas overlay data (crosses, rings, fit
  outline); Operations row "Registration marks (not cut)"; wasm bindings (AC 1 to 3, 5 to 10, 13,
  16, 18, 19, 20, 21).

### Part E: frontend

- [x] E1. Document tab button, Registration marks Style block, Print and cut group (rows, Take
  head position from the connector's last `Status`, greyed causes), canvas overlay (AC 1 to 3, 5,
  7 to 10, 12 to 14, 16, 18, 20, 21).

## Validation

Affected-crate gate per part: `cargo fmt --all --check`, `cargo clippy` with `-D warnings` for
the touched crates (`--all-targets`) and their wasm32 build, `cargo nextest run` of the touched
crates, frontend `npm run build:wasm && npx tsc -b --noEmit && npm test && npx oxlint`. The
browser check is the next agent's. `CARGO_INCREMENTAL=0`.

## Deviations and decisions (Proposal)

- **D1 Half rows.** The register stores whole `[x, y]` pairs. A row with only X or Y typed is kept
  by the frontend and committed when the other value is typed; emptying one value of a stored row
  clears it (criterion 9: a half row counts as empty).
- **D2 The two-mark rule.** A delete that would leave a marks object with exactly one mark is
  refused ("Registration marks need at least 2 marks."); deleting every mark, or the object,
  deletes it whole. Ungroup of a marks object is refused.
- **D3 A number twice.** A number the sheet holds twice is not counted; its row says "Mark 3 is
  on the sheet twice: not used" (new text, ADR 0081 §5 names the cause only).
- **D4 Turned** is shown counter-clockwise on the screen positive (UX notes §4.6); the fit's own
  angle (Y down) has the other sign.
- **D5 Line width** stays 0.05 to 1 mm (the UX notes say 0.1): criterion 2, ADR 0081 §1 and the
  file validator all allow 0.05, and a stricter field would refuse values a file may hold.
- **D6 The digit** sits with its lower-left corner half a millimetre outside the circle's square,
  up and right (3.5 mm at the 6 mm default, UX notes P5), and scales with the diameter.
- **D7 Canvas.** The fit outline is dashed (6 on 3 px) in `--preview-new`, which is the accent
  colour; it is 1 device pixel wide with no casing, because the overlay is drawn with the
  grid-line builder that only draws hairlines (the UX notes ask for 1.5 px with a white casing).
  Crosses keep their true size with a 12 px floor. The number tags of crosses (guide green, up
  right) and rings (accent, down left) and the "Fit" tag are host text over the canvas from
  `print_and_cut_labels_json`, as the fixture labels are.
- **D8 SVG.** `curvyo-svg-core::ExportState` gains `data` (`data-*` attributes): a public API
  change of a core crate for the architect's review.
- **D9 Tests of other stories** gain only what the new kind needs: an `unreachable!` arm for
  `RegistrationMark` in exhaustive test matches, the `print_and_cut_` fixtures left out of the
  "every older fixture" loops (as `raster_engraving_` is), the 0029 golden count leaving out
  `print_and_cut_cut_line.gcode` (as `raster_*` is), and the format numbers 54 to 55.

- **D12 File sizes.** `curvyo-ui-core/src/svg_export_parts.rs` grows from 498 to 507 lines by
  the two mark arms of its exhaustive leaf matches (the mark's SVG itself lives in
  `svg_export_marks.rs`); `curvyo-document-core/src/transform_objects.rs` was 713 lines before
  this story and grows by 32 for the mark's plan arm and write (the dimension and mark plans are
  one helper). Splitting either is a refactor of its own, outside this story; noted for the
  lead (`docs/technical-debt.md`).

## Open (not in this build, for the lead)

- The Layers rows of the marks are in tree order (number order while nobody reorders them).
