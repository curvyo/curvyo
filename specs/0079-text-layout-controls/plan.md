# Plan for the text layout controls, slice 1

Implementer, 2026-10-24, on `story/text-layout-controls` (slice 1: Parts A, B, G). Built for the
architect's slice-1 decisions 1 to 6 of [`adrs.md`](adrs.md); UX per [`ux-notes.md`](ux-notes.md)
§1 to §9 with its §0 changes 1 to 4 taken as Proposal (reset stays the reset slot and
Ctrl+Backspace, double-click types, §0.6). Slices 2 to 4 are not built here.

## Affected crates/modules

- `curvyo-text-core`: `layout(face, text, size, &LayoutOptions)`; new `align.rs` (per-line shift
  and justify widths) and `glyph_map.rs` (em outline into text space: scale, rotation about the
  cluster centre); `shape.rs` turns `liga`, `clig`, `calt` off with letter spacing;
  `PlacedGlyph::centre_x`, `LineLayout::extent` and `start`, `TextLayout::natural_line_height`.
- `curvyo-document-core`: `TextAlign`, `TextLayoutValues` in `text_model.rs`; the five optional
  keys in `text_codec.rs` (absent = default, defaults deleted, lenient read); `TextSnapshot::layout`;
  `Document::set_text_layout` (one commit, labels of decision 3); `create_text` takes the values;
  `CURRENT_FORMAT_VERSION` 19, golden `text_layout_v19.curvyo`.
- `curvyo-ui-core`: `text_layout::layout_options` (factor x size) used by every caller of
  `layout`; `realigned_placement` (anchor shift of criterion 4); new module `text_layout_fields`
  (the four fields' scales relative to Size, readouts, Mixed, typed clamps per text, the 1-2-5
  drag grid); `TextEdit::layout`; history names.
- `curvyo-editor-wasm`: session `text_layout.rs` (alignment press, the block view, preview,
  step, commit, cancel, reset, typed value; the next text's alignment); the edit session and the
  hit, draw and Object to path paths lay out with the text's values; bindings.
- `frontend`: `AlignmentGroup` in the Text bar and the Select bar; `TextLayoutBlock` at the top
  of the Fonts tab (four `ValueField`s, signed bar and zero mark, left overshoot); pure text rules
  in `lib/textLayoutText.ts` with node tests.
- Docs: ADR 0020 dated note, `docs/design-system.md` (UX notes §11), `docs/requirements.md`
  R-FONT-011, `specs/README.md` row 0079.

## Tasks

Milestone 1, core (test first)
- [x] 1. text-core: `LayoutOptions`; alignment Left/Centre/Right on the line extent, trailing
  spaces excluded (AC 2); Justify by spaces, by cluster gaps, last line and one-cluster lines left
  (AC 3); letter spacing between clusters, ligatures off, kerning kept (AC 8); word spacing on
  U+0020, U+00A0, U+3000 (AC 9); set line spacing and natural line height (AC 10); rotation about
  the cluster centre, "I" at 90° (AC 11); caret boundaries in the middle of the letter spacing and
  shifted with the line, line box over shifted lines (AC 5, 15); 500-character budget (AC 5, 12);
  single-line faces (0076) and preview tiles (0077) unchanged (AC 38)
- [x] 2. document-core: `TextLayoutValues`, five keys, read rules (unknown align, non-finite,
  out of range, rotation wrap), defaults deleted; `set_text_layout` one commit with the labels,
  only changed keys and placement written; `create_text` with values; format 19, golden file with
  one text per key, one with all, one with none; version-18 fixtures open with no key (AC 4, 12,
  13, 34, 36, 37)
- [x] 3. ui-core: `layout_options`, `realigned_placement` (widest line keeps its place), the field
  scales (track and typed range relative to the smallest Size, overshoot, 1-2-5 grid, rotation
  wrap and steps), readouts and Mixed, per-text clamps with the notices, hit box with turned
  glyphs, Object to path equals drawing (AC 4, 7, 13, 15, 34, 35)
- [x] 4. Session: alignment press for selected texts, edit mode and the next text (no step);
  block view only with a subject; drag preview, release one step, Escape nothing, click, typed,
  reset; several texts in one step; edit mode keeps the caret and writes its own step; Undo
  (AC 1, 4 to 7, 10, 12 to 14, 33, 34)

Milestone 2, frontend
- [x] 5. `lib/textLayoutText.ts` with node tests: names, tooltips, notices, aria texts,
  alignment roving keys (AC 1, 7, 13)
- [x] 6. `AlignmentGroup` in the Text bar and the Select bar Text group: glyphs, pressed and
  mixed states, roving focus, manual activation, focus return (AC 1, 4, 33)
- [x] 7. `TextLayoutBlock` at the top of the Fonts tab: four value fields, signed bar and zero
  mark, muted Auto, overshoot on both ends, appears and goes with the subject, list anchoring,
  focus to the canvas when it goes (AC 6, 6a, 7, 10, 12 to 14)
- [x] 8. Design-system amendments (UX notes §11) with the measured rows

Milestone 3, gate
- [x] 9. Full gate (`CLAUDE.md` §7), frontend tsc, npm test, oxlint; headless Chromium
  screenshots of the Text bar and the block.

## Validation

- text-core integration tests with the included Source Sans 3 and Source Serif 4 faces for each
  criterion's test case (AC 2, 3, 8 to 11) and a timing test on 500 characters with every value
  set.
- document-core codec unit tests and the golden-file round trip.
- ui-core unit tests of the scales and the anchor shift; session tests with the included fonts:
  steps and labels, Undo, Mixed, edit mode, the next text.
- Node tests for the frontend text rules; a headless Chromium look at the bar and the block.

## Dated notes

- 2026-10-24: preview tiles (`0077`) stay as they are: they show the family's face for the
  sample, without the subject text's spacing or rotation. 0079 does not name them, a tile is one
  line clipped at its edge (letter spacing would only push the sample off the tile), and the
  tile cache is keyed by face and sample (`0077` criterion 17). Left to the lead.
- 2026-10-24: the tester-owned `curvyo-text-core/tests/acceptance_0076_tester.rs` was adapted
  mechanically only: `layout` takes `&LayoutOptions` (default = the old layout, ADR decision 4);
  no assertion changed. `frontend/tests/sliderNames.test.ts` counts the four new names.
- 2026-10-24: the golden file creates each text a size off and resizes it, as the other text
  goldens do, so no two history rows of the file share a label (`0020` criterion 36 test).
- 2026-10-24: measured at build (headless Chromium): Text bar 686px on one row at 1280 x 800, three
  rows (108px) at 800 x 600; Select bar with one text 6 rows (216px) at 800 x 600 (Alignment
  shares its row with Object to path, Linked clone takes the sixth), 2 rows at 1280 x 800; the
  Fonts tab list 277px at 800 x 600 with the block (minimum 120px).
- 2026-10-24: the block keeps the list's scroll offset when it appears or goes, so the first
  visible family row stays the first one in view (criterion 6a); its screen position moves with
  the list's top. Left as is (UX notes §5 asks for the same screen position).
- 2026-10-24: `R-EDIT-048`, which the spec links, does not exist in `docs/requirements.md`;
  R-FONT-011 is added. Left to the product owner.

# Plan for the text layout controls, slice 3

Implementer, 2026-10-24, on `story/text-vertical` (Part D, criteria 20 to 25, and Part G for its
values). Built for `adrs.md` decisions 11 to 13 and the slice 3 spike note there; UX per
`ux-notes.md` §10.2. Format 20 (main at 19 after slice 1 and `0082`).

## Affected crates/modules

- `curvyo-text-core`: new `writing_mode.rs` (`WritingMode`, `GlyphOrientation`, the frame),
  `vertical_orientation.rs` with the generated `vertical_orientation_data.rs`
  (`tools/unicode-data/vertical_orientation.py`); `shape.rs` upright (top to bottom) and sideways
  runs; `layout.rs` line space, column baselines, Auto from `vhea`; `glyph_map.rs` frame and upright
  glyphs; `FaceMetrics::vertical_line_height`.
- `curvyo-document-core`: `TextWritingMode`, `TextGlyphOrientation`, two optional keys, labels
  "Change writing mode" and "Change glyph orientation", format 20, golden `text_layout_v20.curvyo`.
- `curvyo-ui-core`: `layout_options` maps the two values; `kept_placement`; caret geometry
  through the frame; arrow keys in columns; history names.
- `curvyo-editor-wasm`: session `text_writing.rs` (pressed state, press, next text), bindings.
- `frontend`: `WritingGroups.tsx` (two groups, custom glyphs, greyed orientation),
  `lib/textWritingText.ts` with node tests, Text bar and Select bar wiring.

## Tasks

- [x] 1. Spike: harfrust top-to-bottom shaping, `vmtx`/`VORG`/`vert`, the fallback without
  `vmtx`, the UAX #50 source, the wasm size; dated note in `adrs.md` (AC 22, 23, 25, 38)
- [x] 2. text-core, test first (`tests/vertical.rs`): Auto = Sideways for Latin and = Upright for
  CJK; Sideways = the line turned clockwise; Upright advances by `vmtx` with `vert` forms, else by
  ascender − descender; columns RL and LR one Line spacing apart, Auto from `vhea`; alignment,
  letter spacing and the line box along the column; rotation of an upright glyph about its middle;
  a text-space point finds its caret position; single-line fonts (AC 22, 23, 25, 38)
- [x] 3. document-core: two keys, defaults deleted, lenient read, labels, format 20, golden with
  one text per value, one with both, one with none; a format 19 file opens horizontal (AC 20, 37)
- [x] 4. ui-core and session: layout options, kept placement, crosswise caret, arrow keys; press
  on selected texts in one step and Undo, orientation kept on horizontal text, next text's values
  (AC 20, 21, 24, 33, 35)
- [x] 5. Frontend: both groups in the Text bar and the Select bar, greyed orientation, node tests
  (AC 20, 21, 32)
- [x] 6. Full gate, frontend tsc, npm test, oxlint; headless Chromium screenshots

## Validation

- `curvyo-text-core/tests/vertical.rs` with the Noto Sans JP subset, Source Sans 3 and EMS
  Readability; codec unit tests and `curvyo-document-core/tests/text_vertical_format.rs`;
  `ui-core` unit test of the crosswise caret and the kept start; session tests
  `text_writing_tests.rs`; `frontend/tests/textWritingText.test.ts`.

## Dated notes

- 2026-10-24: the tester-owned 0079 acceptance files of slice 1 got
  `..LayoutOptions::default()` / `..TextLayoutValues::DEFAULT` where they build the structs in
  full (two new fields); no assertion changed.
- 2026-10-24: `curvyo-ui-core/src/text_edit.rs` was 621 lines before this slice (over the ~500
  guide); the 18-line key map is added there because the moves it maps live there. Left to the
  lead.
- 2026-10-24: the Select bar with one vertical text was not measured at 800 x 600; the Text bar
  was (design-system row).
