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

# Plan for the text layout controls, slice 4 (kerning)

Implementer, 2026-10-24, on `story/text-kerning` (Part E, criteria 26 to 31, and Part G for its
values). Built for `adrs.md` decision 14 and
[ADR 0021](../../docs/adr/0021-per-character-values-on-a-text.md); UX per `ux-notes.md` §10.3.
Format 21 (20 after slice 3).

## Affected crates/modules

- `curvyo-document-core`: new `text_chars.rs` (`CharValues`, `TextChars`, the Loro marks
  `kern_x`/`kern_y`, the style config); the three plain-string readers carry marks: `objects.rs`
  `copy_map`, `history/node_value.rs` `Node::Text { string, chars }`, `history/merge.rs` (write
  and three-way restore per character); `TextSnapshot::chars`; `Document::edit_text` (replaces
  `edit_text_string`), `create_text_with_chars`, `set_text_chars` ("Change kerning", "Clear
  kerning"); the config on `Document::new`, `from_loro_snapshot` and the history wipe; format 21,
  golden `text_layout_v21.curvyo`.
- `curvyo-text-core`: new `char_offsets.rs` (`CharOffset`, the byte lookup);
  `LayoutOptions::char_offsets` applied in `layout.rs` `place` (along before the cluster, across
  on its glyphs).
- `curvyo-ui-core`: `layout_options` maps the factors; `text_edit.rs` keeps one `CharValues` per
  character with the string (typed characters none, deleted ones gone, undo inside the session),
  `kern` for Alt+arrows, the character subject at the caret or range; `text_layout_fields.rs`
  two more fields (`HorizontalKerning`, `VerticalKerning`), subjects per character; history
  names.
- `curvyo-editor-wasm`: kerning subjects, preview, commit and reset in `session/text_layout.rs`
  (new `session/text_kerning.rs` for the character subjects and writes), `text_kern` binding.
- `frontend`: two more fields in `TextLayoutBlock` (a third group), names, Alt+arrows in the edit
  sink, the readout "Kern +0.3, 0.0 mm".

## Tasks

- [x] 1. document-core, test first: style config on both constructors (and after a wipe), marks
  survive `apply_string_diff` before and after them, `copy_map` copies them, undo restores a
  kern per character, two peers' kerns on different characters both survive (AC 29 to 31, 37)
- [x] 2. document-core: `TextChars` read rules (non-finite 0, clamp), `TextSnapshot::chars`,
  `edit_text`, `set_text_chars` labels, one commit; format 21, golden file with one text with
  `kern_x`, one with `kern_y`, one with both on several characters, one without; a format-20 file
  opens with no mark (AC 26, 28, 37)
- [x] 3. text-core: `char_offsets` along (pen and the rest of the line move, caret in the middle
  of the gap, justify keeps the width) and across (only that cluster), horizontal and vertical;
  a value on a later character of a cluster ignored (AC 26, 27, 38)
- [x] 4. ui-core: `layout_options` factor x Size; session values per character through insert,
  delete, undo; Alt steps 0.1 mm, Shift 1 mm, clamped to the range; field subjects (caret, range,
  end of line, whole texts), Mixed; Object to path equals drawing (AC 26 to 30, 35)
- [x] 5. Session and bindings: the two fields in the block, preview, release one step "Change
  kerning", reset outside edit mode "Clear kerning", in edit mode the session and the node; Alt+
  arrows in the edit session (AC 26 to 29)
- [x] 6. Frontend: the third group in the block, Alt+arrows in the sink, the readout; node tests
  (AC 26, 29)
- [x] 7. Full gate, frontend tsc, npm test, oxlint; headless Chromium screenshots
- [x] 8. Customer feedback on PR #189 (2026-10-24): text-core cumulative `across` (a running rise
  per line) and `CharOffset.rotation_rad` / `PlacedGlyph.rotation_rad` added to the text's turn
  (AC 27, 27b); document-core mark key `rot` in degrees, wrapped, own labels, golden v21 with a
  rotated text (AC 27b, 37); ui-core field Per-character rotation, `chars_label`; session
  "All characters" and per-key resets (AC 27b, 27c, 28); frontend third group with the buttons
  (AC 26, 27c)

## Validation

- `curvyo-document-core/src/text_chars.rs` unit tests and `tests/text_kerning_format.rs`
  (golden, peers, undo, copy); `curvyo-text-core/tests/kerning.rs`; ui-core unit tests of the
  session values and the fields; session tests `text_kerning_tests.rs`; node tests.

## Dated notes

- 2026-10-24: the lead's brief put the two kerning fields into the Text bar's row 2 (0084); the
  spec (criterion 26), UX notes §10.3 and 0084 ("Per-character kerning adds no bar control") put
  them in the text layout block of the Fonts tab. Built per the spec.
- 2026-10-24: tester-owned files adapted mechanically only: `LayoutOptions` is no longer `Copy`
  (it holds `char_offsets`), so `..o` became `..o.clone()` and full literals got
  `..LayoutOptions::default()`; `layout_options` takes the characters' values; `TextSnapshot`
  and `LayoutSubject` literals got the new field; the block's field count 4 became 6 (criterion
  26) and the label test compares the first four; slice 3's `CURRENT_FORMAT_VERSION == 20`
  became `>= 20` (format 21 here). No other assertion changed.
- 2026-10-24: Alt+arrows act the same in columns: Left/Right change the horizontal (along) kern,
  Up/Down the vertical (across) one, not mapped to the column's direction. Left to the lead.
- 2026-10-24: a kerning field's change in edit mode is not a step of the session's own undo;
  a later Ctrl+Z inside the session that goes back past it restores the characters' values of
  that earlier state too (the session's states hold the values with the string).
- 2026-10-24: measured at build (headless Chromium): the block is 196px with six fields (UX notes
  §4 said 208px); no sideways scroll at 800 x 600.
- 2026-10-24 (customer feedback on PR #189): tests that asserted "Vertical kerning moves the
  character alone" (implementer's `tests/kerning.rs`, tester's
  `acceptance_0079_slice4_tester.rs` in text-core) were rewritten to the new rule: the following
  characters of the line move too, the earlier ones and the next line stay. The tester's field
  order test now expects Per-character rotation after the kerning fields, the field counts 6
  became 7. The degree fields' notices read "°" and drop "to fit its size" (a rotation is never
  limited by Size), which keeps every notice within 70 characters with the longer label.
- 2026-10-24: text on a path (`0080`) is stacked on this branch and not in it; it needs the
  cumulative `across` along the carrier's normal and `PlacedGlyph.rotation_rad` after its rebase.

# Plan for the text layout controls, slice 2 (taper)

Implementer, 2026-10-26, on `story/text-taper` (Part C, criteria 16 to 19, and Part G for the
taper). Built for `adrs.md` decisions 7 to 10. Format 24 (main at 23). Customer corrections of
2026-10-25: the Taper control is a value field of the text layout block in the Fonts tab (not the
Text bar), and the block folds under a "Text layout" header, closed by default.

## Affected crates/modules

- `curvyo-text-core`: new `taper.rs` (the map of criterion 17 and the refit of criterion 18 with
  `kurbo::fit_to_bezpath`); `LayoutOptions.taper`, `TextLayout.taper`; `glyph_map.rs` applies it
  in the glyph frame before the turn; `TextLayout::glyph_in_line` for text on path.
- `curvyo-document-core`: `TextLayoutValues.taper`, key `text_taper`, label "Change taper",
  format 24, golden `text_layout_v24.curvyo`.
- `curvyo-ui-core`: `layout_options` maps the taper; `LayoutField::Taper` (%, −50 to 50);
  `text_on_path` draws the tapered glyph in its frame; the box covers tapered glyphs.
- `curvyo-editor-wasm`: the eighth block field, its notice outcome.
- `frontend`: Taper field after Character rotation; the block's "Text layout" header (fold,
  stored per user, dot when folded with changed values); node tests.

## Tasks

- [x] 1. text-core, test first (`tests/taper.rs`): H at 20 mm, 20 %: stems 0.8 x / 1.2 x apart
  at the baseline and at the top, top stays 20 mm; H and V lines stay lines; curves within
  0.01 mm of the exact map; the 0.05 clamp; advances unchanged; taper before rotation; the
  circle values outside and inside the ring (AC 17, 18, 19, 38)
- [x] 2. document-core: key, default deleted, lenient read and clamp, label, format 24, golden
  with the taper alone and with all values (AC 16, 37)
- [x] 3. ui-core and session: the field (range, steps 1 / 10 / 0.1 %, Mixed, notice at the
  limit), mapping, text on path in each glyph's frame, Object to path (AC 16, 19, 34, 35, 36)
- [x] 4. Frontend: the Taper field, the folding block header, node tests (AC 16, 6)
- [x] 5. Spec, UX notes and `0084` updated for the customer corrections; full gate; screenshot
  of a tapered text on a circle

## Validation

- `curvyo-text-core/tests/taper.rs`; codec unit tests and
  `curvyo-document-core/tests/text_taper_format.rs`; ui-core unit tests of the field and of text
  on path; session test; `frontend/tests/textLayoutText.test.ts`.

## Dated notes

- 2026-10-26: the glyph frame of criterion 17 is the glyph's own em frame: its baseline is the
  glyph origin's line (for a sideways glyph of a column that is its own baseline, not the
  column's centre line); for an upright glyph of a column the middle is the column's centre line.
  Kerned glyphs taper about their raised baseline.
- 2026-10-26: refit accuracy 0.01 mm in text space divided by the placement's largest singular
  value (`ui-core::text_layout::taper_accuracy`); text on a path uses 0.01 mm (its frames are
  rigid). Measured on host (release): 532 tapered glyphs of Source Serif 4 laid out and refitted
  in about 24 ms; the outline grows by under 2 % in elements.
- 2026-10-26: the tester-owned files were adapted mechanically for the eighth field only: the
  block's field count 7 became 8 (`curvyo-editor-wasm/tests/acceptance_0079_tester.rs`), and the
  slice 4 order test now expects the Taper between Character rotation and the kerning fields
  (`acceptance_0079_slice4_tester.rs`, customer decision 2026-10-25). No other assertion changed.
- 2026-10-26: a typed taper beyond the range on several texts answers with the single-text notice
  ("Taper set to 50 %, the largest taper."), because the range is the same for every text.
