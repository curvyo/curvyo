# Plan for SVG import (first slice of 0024)

Implementer, 2026-10-26, branch `story/svg-import`, stacked on `story/stroke-to-path` (not merged;
customer allowed a stack of up to 3). Built under `adrs.md` (dated note 2026-10-26) and
[ADR 0018](../../docs/adr/0018-svg-import-crate-parser-and-limits.md). Import only; the export is
the later slice. No push, no PR in this run.

## Affected crates/modules

- **`curvyo-svg-core` (new, ADR 0018 §1):** a pure SVG reader that depends on no Curvyo crate.
  `xml_guard` (moved from `text-core/svg_font.rs`: size, gzip, UTF-8, prolog/DOCTYPE scan with the
  rule as an argument, depth scan, element cap, then `roxmltree`), `path_data` (moved from
  `text-core`, arcs split by tolerance for fonts or at most 45° for the import), `units`
  (lengths, viewBox and `preserveAspectRatio`, generator pixel sizes), `css` (the `<style>` subset
  on `simplecss`), `style` (cascade per leaf, colours, paints, gradients' middle colour),
  `reader` (tree walk, transforms baked, layers, groups, primitives with their matrix, `use` as
  clone or copy, limits), `drawing` (the output types `SvgDrawing`, `SvgNode`, `SvgLeaf`).
- **`curvyo-text-core`:** `svg_font.rs` and `limits.rs` use `curvyo_svg_core::{xml_guard,
  path_data}`; `path_data.rs` leaves. Font behaviour and fixtures unchanged (`0076` criterion 23).
- **`curvyo-document-core`:** `drawing_insert.rs` (new): the `Drawing` tree type (groups, layers,
  paths with outlines, primitives, clones by leaf index), `Document::insert_drawing` (one commit,
  one new top layer) and `Document::from_drawing` (a new document whose setup change holds the
  page, display unit, background and the content, so history and undo are empty);
  `DocumentVersion::never()` so an opened SVG is dirty.
- **`curvyo-ui-core`:** `svg_import.rs` (new): `SvgDrawing` to `Drawing`: primitives under the
  `0056` rules, node kinds (criterion 25), Inkscape stars (criterion 28), the even-odd re-winding
  (criteria 50, 51) with `outline_nesting`/`touching_outlines`, the placement translation of Import
  (criterion 3, clone matrices T·M·T⁻¹). `svg_import_style.rs` maps styles. `history_names.rs`:
  "Import SVG".
- **`curvyo-editor-wasm`:** `session/svg_import.rs` (Open of SVG as a new dirty session, Import
  SVG… as one step into a new top layer, selected, active), `wasm_svg_import.rs` (bindings
  `open_file`, `import_svg`), the refusal texts of the UX table.
- **`curvyo-app`:** `pick_project` takes the dialog kind (`open`: projects and SVG drawings plus
  All files; `svg`: SVG only; `project`: Template); `save_project_bytes` takes the suggested name.
- **`frontend`:** `useProject` (Open decides by content in the session, SVG opens untitled and
  dirty with the suggested save name, Import SVG… flow, busy notice from 1 MiB), `FileTab`
  (Import SVG… row below the block, outside the inline check), `host.ts` (browser picker accepts),
  `lib/svgImportText.ts` (notice and name cut).
- **Docs:** `docs/design-system.md` rows (File tab, notice, busy, refusal titles),
  `docs/svg-import.md` (the supported subset, criterion 76), `docs/technical-debt.md` "SVG
  round-trip is lossy", `specs/0054` criterion 27 note (EVENODD A), `deny.toml` if needed.

**Bytes across the Tauri boundary:** as the project open does today (`pick_project` returns
`bytes: Vec<u8>` in the JSON answer; the page passes a `Uint8Array` to the session). The 50 MiB
limit is checked in the session before the parse (criterion 6c); the host reads the file whole.

## Tasks

- [x] 1. Docs commit: spec Ready, UX amendments, ADR 0018, index, ADR 0011 note.
- [x] 2. Budget probe: an `#[ignore]` benchmark of `insert_drawing` over 20,000 paths with 400,000
  nodes in one commit (risk of `adrs.md`; AC 71). Restate the budget from the measurement if missed.
- [x] 3. New crate `curvyo-svg-core`; move `xml_guard` and `path_data` from `text-core` (font tests
  and `0076` hostile fixtures as the net); the literal-entity DOCTYPE rule and the element cap
  (AC 6, 68, 69).
- [x] 4. Reader, tests first: page and units (AC 15 to 20), path grammar and arcs (AC 21 to 24, 30),
  shapes (AC 26, 27, 28 data), transforms (AC 31 to 35), styles and CSS (AC 36 to 43), structure,
  layers, `use` (AC 44 to 49), ignored content (AC 13), limits (AC 69).
- [x] 5. Hostile input: refusals per case (AC 6), entity rules (AC 68), truncation and byte-change
  sweep of every fixture and a proptest over random bytes (AC 70 a), nothing fetched (AC 67).
- [x] 6. `document-core`: `Drawing`, `insert_drawing` one commit, `from_drawing` with empty history
  (AC 2, 3, 77).
- [x] 7. `ui-core::svg_import`: primitives and transforms (AC 26, 27, 33), node kinds (AC 25), star
  match (AC 28), even-odd (AC 50, 51), placement (AC 3), styles (AC 34, 38 to 42); goldens in
  `tests/fixtures/svg/` with `README.md` (AC 74, 75), determinism (AC 72), the 0046 call (AC 73).
- [x] 8. Session and bindings: Open by content (AC 1, 2), Import SVG… (AC 3), the notice (AC 8),
  refusals (AC 6), busy (AC 7), history name.
- [x] 9. Host and frontend: dialogs and filters, File tab row (AC 5), save name after Open (AC 2).
- [x] 10. Docs (AC 76), full gate, browser check with a screenshot.

## Validation

- Golden files: every fixture's imported `Drawing` as JSON, compared within 1e-9 mm (AC 72, 74).
  Inkscape-made and Illustrator samples are written by hand in the layout those programs write
  (no Inkscape on the build VM); the README says so per file. The tester replaces them with files
  saved by Inkscape 1.x when available (AC 75).
- Hostile fixtures and the truncation/byte-change sweep (AC 70 a); a proptest over arbitrary bytes.
  The 60 s nightly fuzz target (AC 70 b) is not built here: the sweep plus the proptest stand in,
  noted as an open point.
- `#[ignore]` budget tests (AC 71) run once in release; numbers in the report.
- Browser check: Import SVG… of a sample file in the browser build, screenshot.

## Notes for the lead (2026-10-26)

- **Permit over refuse, two readings.** A root `<svg>` without `xmlns` is read as SVG (criterion
  6 (a) refuses it as written); `<switch>` takes its first child that needs no
  `requiredExtensions`, so older Illustrator files (whose first child is a `foreignObject` with
  `requiredExtensions`) import (criterion 44 says "the first child element").
- **Budget 71** restated from the measurement (`adrs.md`, dated notes 2026-10-26), timed in thread CPU time by `curvyo-editor-wasm/tests/svg_import_budget.rs`.
- **Not built here:** the 60 s nightly fuzz target of criterion 70 (b) (the truncation and
  byte-change sweep and two proptests stand in); the WASM timings and the `editor-wasm` size delta
  for the PR description; Inkscape-saved fixtures (the files are hand-written in Inkscape's
  layout, `tests/fixtures/svg/README.md`).
- **Existing tests touched:** the button lists of `frontend/tests/fileTab.test.ts` and
  `acceptance_0051_tester.test.ts` gain "Import SVG…" (criterion 5), as the Template button's
  commit did.

# Slice 2: text and images (criteria 78 to 110)

Implementer, 2026-10-27, branch `story/svg-text-images`, stacked on `story/bitmap-import` (not
merged). Built under the `adrs.md` dated note 2026-10-27 and Frage 48 default A (unanswered). No
push, no PR in this run.

## Affected crates/modules

- **`curvyo-svg-core`** (font-free, ADR 0018): `switch` tests (`systemLanguage`,
  `requiredFeatures`, bare `foreignObject`, AC 99); `text_style.rs` (inherited text properties in
  the cascade: font list, size, weight with bolder/lighter, style, stretch, spacing, anchor,
  writing mode, orientation, line height, white space, baselines, text-transform); `text.rs` and
  `text_chars.rs` (a `<text>` into an `SvgText`: characters after white-space processing, one
  style run per character, position lists, line starts, `textPath` pieces, flowed text boxes);
  `data_uri.rs` (base64 and percent decoding straight from the attribute, AC 100, 105);
  `image.rs` (`<image>` viewport, aspect rule and matrix, AC 101). New `SvgNode::Text` and
  `SvgNode::Image`; `NodeState::guide` for carriers (AC 94); carrier links resolved after the walk
  like clones.
- **`curvyo-document-core`:** `DrawingNode::{Text, TextOnPath, Image}`, `DrawingRegisters::guide`,
  `insert_drawing`/`from_drawing` take the asset bytes and write them in the same commit (AC 104);
  `MISSING_FACE_HASH` in `text_model.rs` (adrs note 2).
- **`curvyo-ui-core`:** `svg_font_match.rs` (AC 80 to 82 over `ListedFace`, full and PostScript
  names derived from family and style); `svg_import.rs` two-phase (`svg_faces_needed`, then
  `svg_import` with the face store); `svg_text.rs`, `svg_text_runs.rs`, `svg_text_kerns.rs`
  (Size by cap H, positions to kerns, rotation correction, splits of AC 92, alignment, writing
  modes, line height, text on path, flowed text); `svg_import_images.rs` (decode, frame, budget).
- **`curvyo-editor-wasm`:** Open, Import SVG and the Import button wait for the faces: the drawing
  is kept, the faces asked for, and the host calls again once they are loaded.
- **`frontend`:** the open and import calls repeat while the session answers "waiting for fonts".
- **Docs:** `docs/svg-import.md` (Text, Images), `docs/user/` importing images line,
  `docs/technical-debt.md`, fixtures README.

## Tasks

- [x] 1. Docs commit: criterion 99 reworded, format line, slice 2 Ready.
- [x] 2. svg-core: switch (AC 99), text properties and `<text>` reading (AC 78, 79, 85 to 91, 93,
  95 to 98 data), images and data URIs (AC 100 to 103, 105), carrier roles (AC 94); tests first.
- [x] 3. document-core: drawing variants, guide register, assets in the one commit,
  `MISSING_FACE_HASH` (AC 82, 104, 109).
- [x] 4. ui-core: font matching (AC 80 to 82), Size and transform (AC 83, 84), positions to kerns
  and rotation (AC 86, 87), spacing (AC 88), alignment (AC 89), baselines (AC 90), writing modes
  (AC 91), lines (AC 85), splits (AC 92), text on path (AC 93 to 95), flowed text (AC 96),
  left-out features (AC 97), images (AC 100 to 104), bounds with texts and images (AC 106).
- [x] 5. Session and frontend: two-phase Open, Import SVG and Import button (AC 106).
- [x] 6. Fixtures and goldens (AC 107), hostile text fixtures (AC 98), budgets (AC 108),
  determinism and fuzz seeds (AC 109), docs (AC 110); full gate; browser check.
- [x] 7. Tester findings (2026-10-27), test-first on the tester's cases: an uneven line gap kerned
  on the moving line (AC 85: kerns are stored at the string index, line breaks included); a
  turned character shaped alone, so it has no kerning pair, as Inkscape draws it (AC 87);
  `textLength` built (AC 88); super and sub from OS/2 `ySuperscriptYOffset`/`ySubscriptYOffset`
  (AC 90, `FaceMetrics` gains the two offsets); upright vertical text placed as Inkscape 1.4
  draws it (AC 91); `side="right"` Position counted from the carrier's end (AC 93, spec
  reworded); face matching memoised per style, a piece without kerns keeps its first layout,
  the letters' box linear (AC 98: 100,000 texts 3.2 s, was 5.2 s); the Inkscape fixtures and
  their `name.paths.svg` cut from the tester's real Inkscape 1.4.4 files (AC 107); the SVG
  fuzz target `curvyo-svg-core/fuzz` with the text and image fixtures as seeds (AC 70, 109).

## Validation

- Unit tests per criterion in svg-core and ui-core with the included faces loaded from
  `frontend/public/fonts` (no system font); glyph origins compared with positions computed from the
  face's advances within 0.01 mm.
- Golden files for the new fixtures, hand-written in Inkscape and Illustrator layout (no Inkscape
  on this machine): the `name.paths.svg` comparison of AC 86 needs Inkscape and is left to the
  tester (open point).
- `#[ignore]` budgets of AC 108 run once in release.

## Proposals and open points (slice 2)

- **(Proposal) `MISSING_FACE_HASH`** = 64 zeros marks an imported missing-font reference (adrs
  note 2). It is a valid hash to every reader, never a real SHA-256, matches no loaded face, so the
  family-and-style rule and then "Font missing" apply.
- **Full and PostScript names** are derived from the listing (`family style`, `FamilyNoSpaces-
  StyleNoSpaces`): `ListedFace` carries no name table, and adding one is a host change.
- **Criterion 83's example** corrected to 6.943 mm (the measured H of 656 units; lead,
  2026-10-27).
- **Not built:** the desktop wait for
  the system font list (criterion 106: the import waits for the faces it finds listed, and after
  10 s takes what is loaded); per-glyph rotation correction in vertical text; x-height for
  `middle` is measured from the face's `x` (no OS/2 read). Super/sub take OS/2's offsets (not its
  sizes: a smaller size would split the text, and Inkscape keeps the size too).
- **Spec conflict (tester's `c88_text_length`):** a `textLength` of 60 mm on four 5 px letters
  needs 5 × Size per gap, beyond Letter spacing (2) plus a kern (1), so criterion 92 (b) splits
  the text into four; the test reads only the first piece. Left ignored for the lead.
- **Splits beyond the spec:** a centred or right-aligned text that starts or ends inside a line
  (a bold word in a centred paragraph) keeps Centre for its whole lines and sets the partial line
  as left-aligned pieces; a run longer than the engine's 100,000-character shaping limit is shaped
  in parts of 10,000.
- **Tests of slice 1 changed by slice 2** (criteria 13, 44, 49 give way to 78, 99, 100): the
  tester's `ac13_ignored_content_leaves_the_rest_alone` and `ac42_ac44_ac46_states_and_structure`
  (svg-core), `ac8_nothing_imported_notice_and_long_name` (editor-wasm), and the implementer's
  `ac8`/`ac13` fixture tests now expect the text and image to come in; each change names the new
  criterion.
- **Budgets (AC 108, release, this VM):** 1,000 texts 94 ms; 12 Mpx JPEG (1.3 MB) 70 ms; 100 KB
  file with 20 texts 23 ms. Hostile texts (AC 98): 4 s release, 48 s debug.

# Slice 3: Inkscape shapes as primitives (criteria 111 to 127)

Implementer, 2026-10-27, branch `story/svg-inkscape-shapes`, stacked on `story/svg-text-images`
(PR #210). Built under the `adrs.md` dated note 2026-10-27 (slice 3). No push, no PR in this run.

## Affected crates/modules

- **`curvyo-svg-core`** (reader only): `shapes.rs` reads `sodipodi:type="arc"` into
  `LeafKind::Arc` (numbers, `arc-type`, `open`, drawn `d`, `d` with fine arcs for the check) and
  `arg2` into `StarData`; `reader.rs` sets `NodeState::inkscape` for a spiral, a 3D box group and
  its `box3dside` faces. Missing or unreadable numbers give a plain path (AC 111, 124).
- **`curvyo-ui-core`:** `svg_import_match.rs` (new: the 0.001 mm outline check, AC 111);
  `svg_import_arcs.rs` (new: arc to `0021` ellipse arc, kinds, whole ellipse, transforms, AC 112
  to 114); `svg_import_shapes.rs` (star and polygon through the new check, twisted stars, AC 116
  to 118); `svg_import.rs` (names "Arc", "Spiral", "3D box", "Face N", AC 119 to 121).
- **Docs:** `docs/svg-import.md` section "Inkscape shapes", `docs/technical-debt.md`, fixtures
  README (AC 127).

## Tasks

- [x] 1. Docs commit: slice 3 Ready, adrs note, this plan.
- [x] 2. svg-core: arc data, `arg2`, Inkscape object marks; tests first (AC 112, 113, 116, 117,
  119, 120, 124).
- [x] 3. ui-core: outline check (AC 111, 123), arcs and transforms (AC 112 to 114), stars and
  polygons (AC 116 to 118), `rx ≠ ry` rectangles unchanged (AC 115), spiral and box names and
  transforms (AC 119 to 122), hostile numbers (AC 124); tests first.
- [x] 4. Fixtures `inkscape_shapes.svg` and `inkscape_shapes_edited.svg` with goldens (AC 125),
  the 1,000-shape `#[ignore]` budget and determinism (AC 126), docs (AC 127); full gate.

## Validation

- Unit tests per criterion in svg-core and ui-core; the check compared on the examples of
  criteria 112, 113, 116, 117 and 123 (0.002 mm moved: path; 0.0005 mm: arc).
- Golden files: no Inkscape on this machine, so `inkscape_shapes.svg` is **hand-written** in
  Inkscape 1.4.4's layout; the tester on navi replaces it with a real Inkscape save (AC 125).
- `#[ignore]` budget of AC 126 runs once in release.

## Notes for the lead (slice 3)

- **A whole ellipse is checked against the true ellipse**, not its four-cubic drawing: that
  drawing (existing `0021` behaviour) is 2.7e-4 of the radius off the ellipse, so for radii above
  about 4 mm the literal "as the document would draw it" would refuse every whole ellipse of
  criterion 113. Arcs are checked against the true arc too (lead decision S3-6, 2026-10-27).
- **The 0.001 mm check is bounded, not sampled** (tester finding, S3-8): same subpath count and
  open/closed state, then both outlines flattened within 1e-5 mm and checked both ways; a piece
  of one polyline is near a line of the other when both its ends are (the distance to a line is
  convex), else it is split at the shared corner. 1,000 real shapes: about 75 ms thread CPU.
- **`d` is checked with its arcs split within 1e-5 mm**: the 45° cubics of criterion 22 are up
  to 4.2e-6 of the radius off, too much for 0.001 mm above 240 mm radius.
- `sodipodi:r2` is optional in `StarData` (criterion 116: a polygon ignores it; a star without
  it is a path, criterion 117).
- The fixtures are hand-written (no Inkscape here); the tester replaces `inkscape_shapes.svg` with a
  real Inkscape 1.4.4 save and regenerates its golden.
- Budget of AC 126 (release, this VM): 1,000 mixed shapes in about 10 ms.

# Slice 4: export (criteria 4, 52 to 66, 71 and 72 export parts, 128 to 157)

Implementer, 2026-11-01, branch `story/svg-export` (`story/0024-svg-export`), one PR for the whole
export (customer). Built under `adrs.md` "Export (slice 4, architect 2026-11-01)",
`ux-notes.md` "SVG export: UX notes" and `export-plan.md`.

## Affected crates/modules

- **`curvyo-svg-core`:** writer: `export.rs` (the `SvgExport` input types), `write.rs` (root,
  namedview, pages, tree, ids), `write_leaf.rs` (paths, primitives, arcs, `use`, styles,
  shadow data, markers), `write_text.rs` (live text, `textPath`, images), `write_number.rs`
  (number text, escaping), `base64.rs`; reader: `shadow.rs` (the `curvyo:` profile v1, read only
  under `curvyo:format="1"`), all `inkscape:page`s, the Guides layer, `curvyo:face`/`curvyo:size`
  on text; `is_isometry` moved out of `uses.rs` and public.
- **`curvyo-document-core`:** `DrawingPage::pages` so `Document::from_drawing` creates every page
  (no format change).
- **`curvyo-ui-core`:** `svg_export.rs` (scope, tree walk, guides, clones), `svg_export_leaf.rs`
  (kinds, styles, shadow, markers, strokes as paths), `svg_export_text.rs` (live text from the
  layout, text as paths, images); import: shadow data applied by criterion 57's agreement rule
  (`svg_import_shadow.rs`), pages on Open, guides layer, face reference from `curvyo:face`.
- **`curvyo-editor-wasm`:** `session/svg_export.rs`, `wasm_svg_export.rs`; tests
  `svg_export_golden.rs`, `svg_export_budget.rs`.
- **`curvyo-app`:** `svg_export.rs` (`pick_svg_export_path`, `write_svg_export`, atomic write).
- **`frontend`:** File tab Export SVG… row and the four switches, stored options, notices, busy.
- **Docs:** ADR 0018 point 9, `docs/svg-import.md` "Export", `docs/design-system.md`,
  `docs/technical-debt.md`, CI `performance-budgets` line, spec status.

## Tasks

- [x] 1. svg-core writer, tests first against hand-built `SvgExport` goldens: skeleton (AC 52),
  numbers and escaping (AC 53, 54, 57, 72), ids and tree (AC 53), paths and node types (AC 54,
  58), primitives, stars, arcs, curved shapes (AC 55, 139, 140), styles (AC 56), shadow (AC 57,
  149), `use` (AC 59), pages (AC 128), markers (AC 141), guides layer (AC 143), text and
  `textPath` (AC 132 to 135), images and base64 (AC 137), determinism (AC 60, 72).
- [x] 2. svg-core reader additions, tests first: shadow data gated by `curvyo:format`, pages list
  (AC 129), guides layer (AC 144), `curvyo:face`/`size` (AC 132, 135), markers (AC 142).
- [x] 3. document-core pages in `from_drawing` (AC 129); ui-core import applies shadow data by
  the agreement rule (AC 57, 66, 140, 142, 149), guides (AC 144), face reference (AC 135).
- [x] 4. ui-core mapping `svg_export` (AC 52 to 59, 128 to 131, 136 to 141, 143, 145, 146),
  round-trip property test 500 cases plus option runs (AC 62, 63, 150 to 152), stripped-namespace
  outline check (AC 61), locale run (AC 72).
- [x] 5. editor-wasm session export and binding, goldens `*.exported.svg` and `export_*` (AC 64,
  153), budget test release-only assert (AC 71, 154), safety (AC 157).
- [x] 6. app commands with atomic write, frontend button, switches, notices, busy (AC 4, 147,
  148); docs (AC 156); full gate.

## Validation

- Writer goldens byte for byte; mapping unit tests per criterion; the round-trip property test is
  the oracle for the reader additions; exported goldens re-imported and re-exported byte-equal.
- Budget: `svg_export_budget.rs` thread CPU, asserted only in release.
- Hand checks 65 and 155 with Inkscape: tester.

## Notes for the lead (slice 4)

- **Spec over ADR where they differ:** the Guides layer is marked `curvyo:guides="1"` (criteria
  143, 144), the text face is `curvyo:face` plus `curvyo:size` (criterion 132), `xmlns:xlink` is
  declared only when needed (criterion 52 amended), and no `curvyo:origin` is written (criterion
  149 lists the attributes exactly); pages come back relative to the first (criterion 129).
- **Writer references by key**, not by leaf number: a `use` or `textPath` names the node it
  points at by a key the mapping sets (Proposal; same concept as the ADR's leaf number).
- **Closing segments:** SVG cannot close an outline with a curve without a node on the start.
  The writer ends a curved closing segment on the first node before `Z`; under
  `curvyo:format="1"` the import treats a last node exactly on the first as that node (ADR 0018
  point 9). Without the profile criterion 24 holds unchanged.
- **Extra frames for byte stability:** a turned `<rect>`/`<ellipse>`, every arc and every Inkscape
  star also carry `curvyo:frame` (and a turned primitive `curvyo:stroke-width`), because the
  numbers read back through a transform are not bit-exact. A `<rect>` frame is checked against
  the rect's numbers (its true corners differ from the model's cubic corners by more than
  0.001 mm above 3.7 mm radius).
- **Sketch:** one `<path>` per outline of the solved sketch in a group (criterion 138 asks for a
  `<circle>` for a circle; the solved outline is written as a path).
- **Options are stored in the page's storage on both builds** (the desktop webview keeps it across
  restarts), not in a host settings file.
- **File tab:** the switches are in one column (the UX fallback); `FileTab` takes the export
  section as a slot (`exportSvg`), so the 0051 acceptance tests, which pin the six buttons of
  the tab rendered alone, are unchanged.
- **Open points (in `docs/technical-debt.md`):** dx after a turned character and the frame of a
  vertical upright text converge over trips instead of being byte-stable after one (the import's
  model of SVG positions), arc angles in degrees in the document. Text-on-path lines beyond the
  first, Justify `dx`, and `font-stretch` round trip were not tested with real files.
- **Not built:** criterion 154's 1,000-text and 12 Mpx budgets (only the 20,000-path one, gated
  in release: 1.03 s, 16 MB); criterion 157's network-denying harness (the export does no I/O by
  construction: pure core plus one atomic write); hand checks 65 and 155 are the tester's.

