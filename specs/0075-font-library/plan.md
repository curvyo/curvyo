# Plan for the font library: tags, favourites, Add font and Cuttable

Implementer, 2026-10-20, on `story/font-library`, first stacked on `story/weld`, rebased onto `main` the same day. Built with
the customer's answers to questions 11 to 13 (A, A, B; spec, "Open questions").

## Affected crates/modules

- `curvyo-text-core`: new module `font_names` (family and style name of a face read from bytes,
  the faces of a file the Curvyo block would list) and `font_class` (`declared_class` over the
  12 OS/2 bytes, reading those bytes from a face). Public functions added, nothing changed.
- `curvyo-ui-core`: new modules `family_key`, `font_library` (tags, favourites, overrides, the
  membership rule), `font_library_file` (the TOML file, canonical writer), `cuttable` (the check
  and its cache's line format). `font_match` compares families by `family_key`. Gains `serde`
  and `toml` (workspace dependencies already in `Cargo.lock`).
- `curvyo-storage-io`: the listing carries `os2` and `stamp` per face; new module `font_files`
  (copy into the Curvyo folder, delete a family's files).
- `curvyo-app`: `fonts.rs` lists `os2` and `stamp`; new `font_library.rs` with the commands
  `add_font_files`, `remove_font_files`, `read_font_library`, `write_font_library`,
  `set_font_library_aside`, `read_font_checks`, `write_font_checks`.
- `curvyo-editor-wasm`: new `wasm_font_library.rs` (the library and the check cache as wasm
  objects, the free functions `cuttable_check`, `font_file_faces`); `Session::texts_using_family`
  for the Remove prompt.
- `frontend`: `FontsTab.tsx` split into the strip, the Groups list and the family extras;
  `lib/fontLibraryText.ts` (words), `lib/fontLibrary.ts` (file text), `lib/browserFonts.ts`
  (IndexedDB), `hooks/useFontLibrary.ts`; `public/fonts/*/font.toml` and `index.json` carry the
  shipped tags and piece counts.

## Tasks (milestones)

Milestone 0, docs
- [x] 0. Spec, adrs.md and UX notes in `specs/0075-font-library/`, criteria changes applied,
  design-system rows, README row, `0062` pointer, Ready.

Milestone 1, core (text-core, ui-core)
- [x] 1. `family_key` and its use in font matching (fulfils AC 9, 11)
- [x] 2. `declared_class` and the OS/2 bytes of a face; fixture faces patched in the test
  (AC 15, 33)
- [x] 3. `family_name`, `style_name`, `listable_faces` from bytes; same names as `fontdb` on the
  fixtures (AC 28, 32)
- [x] 4. `font_library`: tags, favourites, chosen cell with fallback, overrides, own tags,
  name rules, membership by override, shipped, automatic (AC 1, 2, 4, 6 to 11, 13, 14, 17, 23)
- [x] 5. `font_library_file`: read, canonical write, limits, newer and broken files; golden
  fixtures in `curvyo-ui-core/tests/fixtures/font_library/` (AC 12)
- [x] 6. `cuttable`: `cuttable_pieces`, the cache line format (AC 17 to 19)
- [x] 7. Shipped tags and piece counts of the included fonts, test-checked (AC 14, 17)

Milestone 2, platform and wasm
- [x] 8. `storage-io`: `os2` and `stamp` in the listing; `font_files` add and remove with the
  TTC rule, atomic copy, `-2` names, byte compare (AC 16, 21, 25, 27, 32)
- [x] 9. `curvyo-app` commands (AC 12, 20, 21, 24, 25)
- [x] 10. `editor-wasm` bindings and `texts_using_family`; Weld notice equals the check
  (AC 13, 19, 25, 26)

Milestone 3, frontend
- [x] 11. Words and their tests (`fontLibraryText.ts`) (AC 3, 7, 8, 10, 22, 25, 30)
- [x] 12. Group strip, Groups list, star, Groups row with chips and Reset, cuttable mark
  (AC 1 to 10)
- [x] 13. The check scheduler in idle slices with the cache, progress (AC 18)
- [x] 14. Add font, drop, Remove on the desktop (AC 20 to 27)
- [x] 15. Browser: IndexedDB store, file picker, quota refusal, `requestFontStorageKeep` (Frage 13 B)
  (AC 28 to 31)

## Validation

- One focused Rust test per core criterion; golden fixtures for `font-library.toml` (valid file,
  each failing rule, round trip).
- The included fonts' shipped tags and piece counts are checked against the check itself.
- Session test: Weld's piece count of a `cuttable` text equals the check (AC 19).
- `storage-io` tests on a temp folder for copy, present, `-2`, refuse, TTC kept.
- Frontend: `node --test` on the words module; `npm run build` for the TSX.
- Manual: `npm --prefix frontend run dev` for the browser build; the desktop through
  `cargo tauri dev` when a display is available.

## Dated notes

- 2026-10-20: Great Vibes' `cuttable` welds into 2 pieces at 20 mm and Gap 0.1 mm, so it is
  not Cuttable (AC 17); Source Sans 3 and Source Serif 4 give 8. Shipped in `index.json`.
- 2026-10-20: the connected-script fixture of AC 17 is Great Vibes with every advance at 80 %
  (its letters overlap: 1 piece), the face without lowercase letters is Source Sans 3 with its
  `cmap` renamed (8 `.notdef` boxes). Both are made in the test from the included files, so no
  binary fixture is added.
- 2026-10-20: PANOSE serif style 11 to 13 counts as Sans only with family kind 2 (Latin Text),
  where the serif style byte has that meaning (AC 15).
- 2026-10-20: `curvyo-ui-core` also gains `thiserror` (workspace, already used by every core
  crate) for the file's error type.
- 2026-10-20: a chip press that gives back the automatic state removes the override instead of
  recording one, so "Set by you" and Reset only show for a real difference (AC 10).
- 2026-10-20: `remove_font_files` takes the family's faces as `{id, index}`, not ids alone: a
  host id names a file, and the TTC rule ("every face of it is among the family's") needs the
  face index (`adrs.md` decision 6).
- 2026-10-20: a check cache line ends with the host id of the face's file,
  `<hash> <index> <pieces or x> <stamp> <host id>`, so a listed face whose id and stamp match
  needs no bytes (decision 5 names that lookup; its line format had no field for the id).
- 2026-10-20: the browser keeps each added file in IndexedDB as `{name, bytes, faces}`, the
  faces (index, family, style, class bytes) read by wasm when it was added, so listing the
  Curvyo block at start needs no wasm and reads no bytes (decision 7 stored `{name, bytes}`).
- 2026-10-20: the page's glue for the library is plain Rust in `curvyo-editor-wasm`
  (`font_library_glue`, `font_checks_glue`, `font_file_glue`), host-tested; `wasm_font_library`
  only binds it. The library is its own wasm object, not part of the session, so no action on it
  can write a history step or change the project (AC 13 by construction).
- 2026-10-20: the Groups button sits right-aligned on its own row under the strip, not at the
  end of the strip's last row (UX notes §2): the strip is one bordered radio group, and the
  button is a separate control. For the UX review.
- 2026-10-20 (review should-fix 5): `FontsTab.tsx` was 902 lines; it is split into the strip,
  the Groups list, the tree rows, the search row with Add font, the notices, the row model
  (`fontRows.ts`), the keys (`fontTreeKeys.ts`), the scrolling (`useFontTreeScroll.ts`) and the
  check queue with its progress (`useCuttableChecks.ts`); `FontsTab.tsx` is about 450 lines.
- 2026-10-20: the list's height is observed through a callback ref: the list element is replaced
  when an empty state or the Groups list was shown, and the old observer reported 0 px, which
  drew only the first rows (also reachable in `0074` through "No font matches").
- 2026-10-20 (review should-fixes 1 to 4): the browser keeps the bytes and the faces in two
  IndexedDB stores (version 2), so listing reads no bytes; its stamp is `<length>-<SHA-256>`;
  file names collide by Unicode lower case on both targets; a library file with `format` 0 or
  below is broken, not newer.
- 2026-10-20 (UX review): the desktop Add font now asks `pick_font_files` for the chosen paths
  first and then calls `add_font_files` with them, as a drop does, so the busy notice counts the
  real files ("Adding 3 fonts…") and never covers the open dialog. The Groups list is one roving
  Tab stop; the Groups row's height is measured from the tree's width.
- 2026-10-20 (rework): rebased onto `main` without the `0068` commits; the cuttable check counts
  the pieces of a Union (`0016`) of the word's glyphs, one operand per glyph, no gap
  (`adrs.md` decision 5). The criterion-19 test does Object to path, Union and Split on the text
  in the session and counts the objects; all five faces agree with the check.
- 2026-10-20 (rework): tags replace groups (UX amendment in the spec). The family row is 52 px
  with a tag line; the Groups row, Reset, the Show switch, the New group row and the Cuttable
  mark are gone; the Tags button sits in the search row. The shipped data's `groups` key is now
  `tags`. The library file's schema changed without a migration (never released).
- 2026-10-20 (tester re-review): serif style 2 to 10 with family kind 2 is Serif; Add font
  accepts only host-granted paths and reads through a 64 MiB `take`.
- 2026-10-20 (customer change): the Tags button and the Tags list are gone; an own tag's strip
  cell carries Rename and Delete (F2 and Delete on the focused cell). Blur saves a rename; a
  refused name keeps the field open with its chip until Enter or Escape.
- 2026-10-20 (customer bug report): deleting the chosen tag, or removing its last use, chooses
  All at once in the model (`FontLibrary::drop_chosen_if_gone`); the strip has no cell for a
  missing tag any more, and after Delete the focus is on All.
