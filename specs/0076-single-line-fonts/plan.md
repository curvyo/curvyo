# Plan for single-line fonts

Implementer, 2026-10-20, on `story/single-line-fonts`, stacked on `story/font-library` (`0075`,
PR #168, merged 2026-10-23; rebased onto `main` with Weld v2, #169, on 2026-10-23). Built with the customer's answers of 2026-10-20 (SL-DEF A, SL-LICENCE A,
SL-STICK A, SL-WASM 2 MiB, SL-KIND A) and the default A of the open Frage 23 SL-BACK.

## Affected crates/modules

- `curvyo-text-core`: new modules `svg_font` (faces, glyphs, advances and kerning of an SVG font
  from bytes, hostile-input checks before parsing) and `path_data` (SVG path data to a cubic
  `BezPath`); `face` gets two sources (`OpenType`, `Svg`) and `Face::kind()`; `shape` dispatches;
  `font_names::listable_faces` reads both kinds; new limits in `limits`; `roxmltree` `=0.20.0`
  (workspace dependency, already in `Cargo.lock`).
- `curvyo-document-core`: `FontRef.kind` (`FontKind`), read and written in `text_font` as the
  optional key `kind`; `format_version` 18; Object to path of a single-line text writes Fill off.
- `curvyo-ui-core`: `text_layout::lay_out` (stroke only, open contours), `font_match` by kind,
  `text_convert::area_refusal`, the area commands call it; `cuttable_pieces` is 0 for a
  single-line face; built-in tag `SingleLine`; font library file `format = 2`; the stroke rule
  of criterion 9; Fill skips single-line texts.
- `curvyo-storage-io`: `fonts` lists `.svg` files of the Curvyo folder with `roxmltree`; Add
  font accepts `.svg`.
- `curvyo-app`: the dialog filter offers `.svg`; the listing carries `kind`.
- `curvyo-editor-wasm`: listed faces with `kind`, the fallback EMS Readability for missing
  single-line faces, `face_preview_path`, refusal reason `no_area`, the Style view hides Fill.
- `frontend`: `.svg` in Add font, single-line rows drawn as inline SVG, refusal words, Fill hidden,
  the four shipped fonts in `public/fonts/` with `font.toml`, `index.json` with `kind`.

## Tasks (milestones)

Milestone 0, docs
- [x] 0. Spec, adrs.md and UX notes in `specs/0076-single-line-fonts/`, criterion 23 changed as
  the architect asks, design-system rows, README row (next free 0077), `0062` pointer, ADR 0020
  dated note and index row, Ready.

Milestone 1, core
- [x] 1. `path_data`: all commands, implicit repetition, packed flags, arcs to cubics, open
  unless Z (AC 16)
- [x] 2. `svg_font`: faces, names, metrics, glyphs by single character, missing glyph, advances,
  `hkern` by `u1/u2` and `g1/g2`; limits and DOCTYPE rule (AC 1, 12, 14, 15, 23, 24)
- [x] 3. `Face` with two sources, `FaceStore::add` sniffs, `listable_faces` for SVG; golden
  files in `curvyo-text-core/tests/fixtures/svg_fonts/`; property test on mutated SVG fonts
  (AC 1, 3, 23, 24)
- [x] 4. Shipped fonts, `font.toml`, `NOTICE.txt`, licence test with the Hershey arm (AC 2, 4,
  25)
- [x] 5. `FontRef.kind`, codec, `format_version` 18, golden file `text_single_line_v18.curvyo`
  (AC 21)
- [x] 6. `lay_out` stroke only and open contours; Object to path Fill off; kind-aware
  matching (AC 7, 10, 12, 13, 17, 18, 20, 22)
- [x] 7. `area_refusal` in the Boolean operations, Fracture, Flatten, Offset (AC 19, 22)
- [x] 7a. `area_refusal` in Weld (`weld_plan`), after the rebase onto `main` with Weld v2
  (AC 19, 22)
- [x] 8. Tag Single line, library file format 2, cuttable 0 for single-line (AC 4, 5)
- [x] 9. Criterion 9 stroke rule on Add text and Change font; Fill skips single-line texts
  (AC 8, 9, 10)

Milestone 2, platform, wasm and frontend
- [x] 10. `storage-io` lists `.svg`, Add font accepts it; same names as text-core (AC 1, 3)
- [x] 11. `editor-wasm`: kind in the lists, the EMS Readability fallback, `face_preview_path`,
  refusal `no_area`, Fill hidden in the Style view (AC 6, 8, 19, 22)
- [x] 12. Frontend: `.svg` in Add font, single-line rows, refusal words, Fill hidden, cuttable
  checks skip single-line faces (AC 3, 5, 6, 8, 19)

## Validation

One test per criterion (named `acN_...`), golden files for the SVG font reader, a time-boxed
property test on mutated SVG fonts, the full gate of `CLAUDE.md` §7, `npm run build` and
`npm test` in `frontend/`, and the release wasm size against the 2 MiB budget.

## Dated notes

- 2026-10-23: rebased onto `main` with Weld v2 (`0068`, #169). `plan_weld` refuses through
  `area_refusal` over the selected leaves (a single-line text inside a group counts too); the
  missing-font refusal still comes first. The frontend Weld button already shows the
  `single_line_texts` refusal and its outline through `textRefusalText` and
  `textRefusalOutlines`, as the other Boolean card buttons.
- 2026-10-20: the stroke rule of criterion 9 sits in `curvyo-document-core` (`create_text`,
  `set_text_font`, `single_line_style`), so the switched-on stroke is written in the same commit
  as the text or the pick without a second API; the rule reads only the stored kind.
- 2026-10-20: `Session::set_font_list` keeps its signature (outline faces) for the existing
  tests; the host lists through `set_font_list_with_kinds`. A face's kind is corrected from the
  engine when its bytes arrive.
- 2026-10-20: the row preview of criterion 6 is the free function `face_preview_path(bytes,
  index, text)` (as `font_file_faces`), called by the row loader that already fetches each face's
  bytes; the session keeps no preview state. Safe default taken: a name wider than the slot is
  clipped, not cut with "…" in its own strokes (UX notes §2 asks for up to 6 shorter calls); for
  the UX review.
- 2026-10-20: the Offset tool refuses a single-line text through `offset_plan` with the
  criterion 19 sentence; the readout line 3 text "Single-line text has no area" of UX notes §5 is
  not built (the refusal sentence shows on release); for the UX review.
- 2026-10-20: the host listing (`curvyo-storage-io::svg_fonts`) names faces without parsing
  glyph paths; a font whose paths the engine refuses (a number that is not finite) is listed and
  its row stays in the UI font, as UX notes §2 "a face that fails to parse" says.
- 2026-10-20: the font library file reads format 1 and 2 and writes 2; an own tag named
  "Single line" in a format-1 file now reads as the built-in name and makes the file broken
  (`BuiltInTag::from_name` also matches names); 0075 is unreleased, so no maker has such a file.
- 2026-10-20: two existing tests were updated for the new built-in tag: the strip order in
  `curvyo-ui-core/tests/acceptance_0075_tester.rs` and the completions in
  `font_library_glue.rs`; the fixtures `valid.toml` (format 2) and `newer.toml` (format 3).

## Criteria to tests

| AC | Test |
|---|---|
| 1 | `curvyo-text-core/tests/svg_fonts.rs` `ac01_faces_of_an_svg_file`; `curvyo-storage-io` `fonts::ac01_svg_fonts_in_the_folder` |
| 2, 25 | `curvyo-text-core/tests/included_fonts.rs` (Hershey arm, kind in `index.json`) |
| 3 | `curvyo-storage-io` `font_files::ac03_svg_fonts_are_added_and_removed` |
| 4, 5 | `curvyo-ui-core` `font_library::ac4_ac5_single_line_tag`; `frontend/tests/singleLineText.test.ts` |
| 6 | `curvyo-editor-wasm` `single_line_tests::ac06_single_line_rows` |
| 7, 8 | `single_line_tests::ac07_ac08_stroke_only_and_fill_kept` |
| 9, 10 | `curvyo-document-core/tests/text_single_line_format.rs` `ac09_ac10_the_stroke_of_a_single_line_pick` |
| 11, 13 | `single_line_tests::ac11_ac13_editing_and_size` |
| 12 | `svg_fonts.rs` `ac12_size_is_the_centre_line_of_h`; `single_line_tests::ac17_ac18_object_to_path_and_combine` |
| 14 | `svg_fonts.rs` `ac14_glyphs_advances_and_kerning` |
| 15 | `svg_fonts.rs` `ac15_line_height` |
| 16 | `path_data::tests::ac16_every_command_and_no_auto_close`; `svg_fonts.rs` `ac16_open_unless_z`, `ac16_golden_centre_lines` |
| 17, 18 | `single_line_tests::ac17_ac18_object_to_path_and_combine` |
| 19, 20 | `single_line_tests::ac19_ac20_area_commands_refuse`; `singleLineText.test.ts` |
| 21 | `text_single_line_format.rs` `ac21_golden_file_and_round_trip`; `text_codec::tests::an_unknown_kind_reads_as_outline` |
| 22 | `single_line_tests::ac22_missing_single_line_face`; `font_match::tests::ac22_matching_is_kind_aware` |
| 23 | `svg_fonts.rs` `ac23_hostile_svg_fonts`, `ac23_mutated_svg_fonts_never_panic` (proptest, 256 cases, 20 s box); `path_data::tests::ac23_hostile_numbers_and_points`; `storage-io` `svg_fonts::tests` |
| 24 | the wasm32 build of `curvyo-text-core` in the gate; no file access in `svg_font`/`path_data` |
