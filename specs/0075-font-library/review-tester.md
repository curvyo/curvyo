# Tester and architect review: 0075 font library

Tester and architect, 2026-10-20, on `story/font-library` (3 commits on `origin/main`).
Re-review after the rework: groups became tags, the cuttable check uses Union without a gap
(`CUTTABLE_CHECK_VERSION` 2), and the new command `pick_font_files` hands the chosen paths to
the page before `add_font_files`. Acceptance tests for the reworded criteria were written from
`specification.md` and the public names before the implementation was read.

## Verdict

PASS. No MUST-FIX. Should-fix items are listed below. The six should-fix items of the first
pass are done (`format` 0 is broken, names compare by Unicode lower case on both targets, the
browser stamp holds the hash, the faces have their own store, `FontsTab.tsx` is 480 lines).

## Acceptance tests

| AC | Test | Result |
|---|---|---|
| 1, 6 | `curvyo-ui-core/tests/acceptance_0075_tester.rs` `ac1_ac6_strip_order_builtins_first_then_own_a_to_z` | PASS |
| 2, 6 | same file, `ac2_cell_membership_for_own_and_builtin_tags` | PASS |
| 4 | same file, `ac4_chosen_cell_survives_restart` | PASS |
| 6, 4 | same file, `ac6_last_use_removed_deletes_own_tag_chosen_cell_stays_then_falls_back` | PASS |
| 7 | same file, `ac7_add_field_reuses_existing_tag_ignoring_case`, `ac7_add_field_refuses_bad_names_and_changes_nothing`, `ac7_rename` | PASS |
| 8 | same file, `ac8_delete_removes_from_every_family_builtins_cannot_be_deleted` | PASS |
| 10 | same file, `ac10_add_new_name_creates_tag_and_chip_order_is_strip_order`, `ac10_completion_offers_existing_tags_the_family_lacks`, `ac10_chip_sources` | PASS |
| 10, 17 | same file, `ac10_removed_automatic_tag_stays_hidden_and_add_back_clears_override` | PASS |
| 11 | same file, `ac11_unlisted_family_keeps_tags_and_star_through_the_file` | PASS |
| 16 | `curvyo-storage-io/tests/acceptance_0075_tester.rs` `ac16_two_thousand_faces_listed_and_classified_in_two_seconds` (ignored, release) | PASS (first pass) |
| 21, 32 | same file, `ac21_refusals_and_name_collision`, `ac21_page_supplied_paths_are_refused_unless_readable_fonts` (new: a device, a directory, `..` and a relative path) | PASS |
| 25 | same file, `ac25_remove_never_leaves_the_folder` | PASS |
| 27 | same file, `ac27_file_manager_copy_and_delete_still_work` | PASS |
| 26 | `curvyo-editor-wasm/tests/acceptance_0075_tester.rs` (two tests) | PASS |
| 29, 31 | `frontend/tests/acceptance_0075_browser_tester.test.ts` (headless Chromium, `CURVYO_E2E_BROWSER`) | PASS (first pass); Firefox not installed |

The first pass had no group-based tester test, so none had to be deleted. The strip keeps a
deleted own tag while it is chosen in `FontTagStrip.tsx` (checked by reading; no browser test).

## Architect review

- **Cuttable uses Union as described.** `cuttable.rs` builds one operand per glyph with area,
  calls the same `curvyo_geometry_core::boolean` with `BooleanOp::Union` and
  `BOOLEAN_TOLERANCE` that `plan_boolean` uses, and counts the result's outlines of even depth
  (`outline_nesting`, `depth % 2 == 0`). No glyph with area and a kernel refusal both give 0,
  which is not Cuttable. `CUTTABLE_CHECK_VERSION` is 2, so old cache lines are discarded.
- **No Weld code remains.** `weld` appears in no `.rs`, `.ts` or `.tsx` file except the
  version comment in `cuttable.rs`.
- **Library file fixtures cover the new schema:** `valid.toml` (favourites, overrides,
  `[tags]` per family, chosen own tag) with the round trip, and one fixture per rule: newer,
  `format` 0, no `format`, syntax, unknown key, override of a non-built-in, a family added and
  removed, a built-in or Favourites as own tag, empty and too long names, two spellings. The
  limits are generated in `ac12_the_limits`.
- **`pick_font_files` and `add_font_files` (security).** The host does not bind
  `add_font_files` to the paths the dialog or the drop gave; the page may pass any path. What
  the host does with it is safe: `add_font_files` reads only a regular file (`metadata`
  follows a link, so a link to `/dev/zero` is refused) of at most 64 MiB with a font
  extension, keeps it only when the folder listing would list it, and writes it atomically
  under its own file name into the Curvyo folder. The report holds file names only. So the
  page can at most copy a readable font from anywhere into the folder and then read it with
  `read_font_file`; it never reads another kind of file. Acceptable for this slice.

## Should-fix

1. `curvyo-app/src/fonts.rs:173`: `add_font_files` takes any path from the page. Keep the paths
   from `pick_font_files` and from the window drop event in host state and accept only those, so
   a compromised page cannot copy fonts from arbitrary places.
2. `curvyo-storage-io/src/font_files.rs:105`: the size is checked with `metadata`, then
   `std::fs::read` reads the whole file. A file that grows in between is read whole. Read through
   `take(MAX_FONT_FILE_BYTES + 1)` instead.
3. `curvyo-storage-io/src/font_files.rs:112`: every Add reads each folder file of the same length
   in full for the "present" test; fine at 2,000 faces only because lengths rarely match. Not
   required.
4. For the product owner, criterion 15 (from the first pass, still open): PANOSE serif styles
   2 to 10 as Serif.

## Wasm growth (first pass) (raw `cargo build --release` output, name-section diff against `origin/story/weld`)

- The raw module grows from 13,339,606 to 13,569,053 bytes. Code accounts for +176.7 KB, the
  data section for +9.8 KB, and the name and bindgen sections for the rest.
- Code by origin: `font_library_file` 40.6 KB (including the derived `RawFile` deserialiser),
  `font_library` 33.3 KB, `cuttable` 22.9 KB, new `toml` instances 22.9 KB, the glue 22.8 KB,
  `font_names` 4.8 KB. The rest (+29.7 KB) is unchanged code whose mangled names changed.
- TOML and serde for one small file cost about 64 KB (about 0.5 % of the module). This is by
  design: the host moves text only, so the page's wasm parses the file. The `toml` crate was
  already linked. The cheap slack is about 20 to 40 KB, by reading through `toml::Table`
  instead of derived structs. Not required.

## Gate

`fmt`, `clippy -D warnings`, `nextest` (7,799 passed, 80 skipped), wasm32 builds of every core
crate, `cargo deny` and `cargo doc -D warnings` are green. `npm run build` is green. `npm test`
gives 415 passed, 1 skipped (the browser test without `CURVYO_E2E_BROWSER`).
