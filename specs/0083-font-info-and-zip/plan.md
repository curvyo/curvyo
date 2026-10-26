# Plan for font info and fonts from zip archives

Implementer, 2026-10-26, on `story/font-info-zip` from `main` (`a1ccf09`). Built for Frage 41
default A (customer did not answer), the architect's decisions 1 to 9 of [`adrs.md`](adrs.md) and
the UX notes ([`ux-notes.md`](ux-notes.md); criterion 14 in the `0081` form, no Info button in a
bar, both applied to the spec).

## Affected crates/modules

- `curvyo-text-core`:
  - new `font_archive.rs` ("reads the font and document entries of a zip archive from its bytes
    within the archive limits"): `FontArchive::open`, `FontArchive::entries`, `FontArchive::read`,
    `is_document_name`; own end-of-central-directory pre-read for the entry count and the
    UTF-8 name flag; overlap check; name rules and sanitising in `archive_names.rs` ("makes an
    archive entry name safe as a file name").
  - new `font_info.rs` ("reads the info fields of one face"): `font_info(&Face) -> FontInfo`.
  - `limits.rs`: the archive limits; `Cargo.toml`: `zip` (workspace dependency).
  - `tests/fixtures/fonts/zip/` (built by `tools/zip-fixtures/make_fixtures.py`),
    `tests/font_archive.rs`, `tests/hostile_archive.rs` (property test), `fuzz/` (cargo-fuzz
    target `font_archive`, not a workspace member).
- `curvyo-storage-io`:
  - `fonts.rs`: one subfolder level (no dot-folders, no followed symlinks), per face `size`,
    `folder`, per folder documents; start-up cleanup of `.curvyo-add-*` only.
  - new `font_archives.rs` ("adds a zip archive's fonts and documents as one font folder"):
    temporary `.curvyo-add-<n>` folder, `sync_all`, rename, `-N` names, Windows retry.
  - `font_files.rs`: de-duplication over every listed file; Remove widened to one subfolder
    level, deleting the documents and the emptied folder; documents read and PDF path check.
- `curvyo-ui-core`: none (the words are frontend strings, decision 6).
- `curvyo-editor-wasm`: `WasmFontArchive` (open, entries JSON, read) and `font_info` of a
  preview face.
- `curvyo-app`: `add_font_files` takes archives, `read_font_document`, `open_font_document`
  (the `open` crate, see "Results and deviations"), listing fields, the `.zip` dialog filter,
  the `.curvyo-add-*` cleanup at start.
- `frontend`: Info mode (`InfoView.tsx`, `fontInfoText.ts`), the third mode cell, archives in
  `browserFonts.ts` (IndexedDB version 3, `documents` store, `<folder>/<file>` keys), archive
  notices, the documents sentence of the armed Remove.
- Docs: ADR 0020 §3 dated note, `docs/design-system.md` rows (UX notes §10).

## Tasks

### Milestone 1: core (commit `feat(curvyo-text-core)`)
- [x] 1.1 Archive limits in `limits.rs` (AC 15)
- [x] 1.2 `FontArchive::open`: pre-read entry count, `zip` table, overlap check, entry classes,
  name rules, unique sanitised names, font count limit (AC 8, 9, 15, 16, 19)
- [x] 1.3 `FontArchive::read`: bounded reads, CRC and size checks, total kept bytes (AC 15, 16)
- [x] 1.4 Fixture script and fixtures, golden tests per fixture (AC 16)
- [x] 1.5 Property test over prefixes and one-byte changes; cargo-fuzz target and 10-minute run
  (AC 16, ADR 0020 §2)
- [x] 1.6 `font_info`: names, format, fsType, vendor, glyph count, SVG values (AC 2, 5)

### Milestone 2: desktop host (commit `feat(curvyo-storage-io)`)
- [x] 2.1 Listing one level deep with documents, size, folder; cleanup of `.curvyo-add-*`
  (AC 10, 12)
- [x] 2.2 `add_font_archive`: checks, in-archive same face once (TTF, OTF, TTC, SVG), dedupe
  against every listed file, atomic folder (AC 9, 11, 12, 17)
- [x] 2.3 Remove in subfolders, documents and the emptied folder (AC 14)
- [x] 2.4 Tauri commands: archives in Add, documents read/open with `tauri-plugin-opener`,
  listing fields (AC 2, 4, 7)

### Milestone 3: page and browser (commit `feat(frontend)`)
- [x] 3.1 wasm bindings `WasmFontArchive`, `font_info` (AC 18, 19)
- [x] 3.2 Browser store version 3: `documents`, `<folder>/<file>` keys, one transaction per
  archive, Remove per folder (AC 10, 11, 14, 18)
- [x] 3.3 Info mode: switch cell, fact list, Show all, Documents, text in place, PDF open,
  missing and unreadable states (AC 1 to 6)
- [x] 3.4 Archive notices, `.zip` in pickers and drop, the armed Remove's documents sentence
  (AC 7, 13, 14)

### Milestone 4: docs (commit `docs`)
- [x] 4.1 ADR 0020 §3 dated note, design-system rows, this plan's results

### Milestone 5: tester findings (commit `fix(curvyo-text-core)`)
- [x] 5.1 `archive_folder_name`: leading dots and spaces removed, `Fonts` when nothing is left,
  so a folder from `.Lobster.zip` is listed and never matches `.curvyo-add-*` (AC 9, 10, 12)
- [x] 5.2 Entries with the same full name are all kept (`OFL.txt`, `OFL-2.txt`) (AC 9, 16)
- [x] 5.3 Browser: a storage error other than a full storage gives "Lobster.zip could not be
  added." and keeps nothing (AC 13, 18)

## Validation

- Golden fixtures per hostile case (criterion 16) with the expected class or error.
- `proptest` over every prefix and single-byte change of each fixture (no panic, bounded).
- cargo-fuzz target `font_archive`, 10 minutes, result below.
- storage-io tests on temp folders: atomic folder, interrupted add (temporary folder left,
  removed at the next listing), one subfolder level, symlinked folder not followed, Remove.
- Frontend unit tests for the words (fsType bits, formats, notices, armed label) and the
  documents count; browser check with a screenshot of Info mode.
- Budgets (criteria 5, 13) measured on this machine, below.

## Results and deviations

- 2026-10-26, fuzz run (ADR 0020 §2): `RUSTC_BOOTSTRAP=1 cargo fuzz run font_archive` with the
  fixtures as seeds, `-max_total_time=600 -max_len=65536 -timeout=10 -rss_limit_mb=2048`:
  2,289,545 runs in 601 s (about 3,800 per second), coverage 2,389 edges, corpus 1,259 inputs,
  peak RSS 556 MB, **no crash, no timeout, no out-of-memory**. The target lives in
  `curvyo-text-core/fuzz/` (own `[workspace]`, not a member, needs `RUSTC_BOOTSTRAP=1` on the
  pinned stable 1.99.0). The gate runs the property test `tests/hostile_archive.rs` (prefixes
  of every small fixture, 256 one-byte changes, 256 random entry tables).
- 2026-10-26, budgets on this machine (release): criterion 13, an archive of 20 fonts and 10.7 MB
  added in 31 ms (budget 2 s, `curvyo-storage-io/tests/archive_budget.rs`); criterion 5, the
  info fields of Source Sans 3 read in 11 µs (budget 100 ms, `tests/font_info.rs`).
- 2026-10-26, wasm size: the release `curvyo_editor_wasm.wasm` grew by 79,704 bytes (14,514,526
  at `a1ccf09`, 14,594,230 on the branch, both before `wasm-opt`), not the "under 10 KiB"
  adrs.md decision 1 expected for the reader alone; the archive reader with its central
  directory and CRC paths, the info fields and the glue are measured together. Tracked with the
  open entry "Editor wasm size" (no hard budget since 2026-10-24).
- 2026-10-26, PDF opener: `tauri-plugin-opener` 2.7.2 pulls `zbus` and 34 other crates on Linux,
  and `cargo deny` fails on them (RUSTSEC-2026-0370/0371, `zvariant` and `zbus_names`
  unmaintained). The host calls the `open` crate instead (MIT, 5.4.5, with
  `shellexecute-on-windows`), which is what the plugin's `open_path` calls itself: same
  behaviour, 3 new crates (`open`, `is-wsl`, `is-docker`, all MIT), no advisory. The page still
  gets no opener permission; the host checks the path first (decision 7).
- 2026-10-26, `font_info` takes the held `Face` (`font_info(&Face)`), not `(bytes, index)`: the
  preview's face store keeps the parsed face, not the SVG font's bytes, and the face holds the
  OpenType bytes already (criterion 5: no second read).
- 2026-10-26, the `.curvyo-add-*` cleanup runs once at app start (`main.rs` setup), not in every
  listing: a listing while an add runs would otherwise remove the folder being built.
- 2026-10-26, sanitised names are cut at 120 characters **and** 240 UTF-8 bytes: 120 characters
  of a 3- or 4-byte script pass the 255-byte name limit of ext4, APFS and NTFS.
- 2026-10-26, a font entry that is not a font the listing would list (a `.ttf` of other bytes) is
  left out silently, as criterion 13 names no reason for it; a write failure reports the archive
  as "could not be read" (UX notes §4 keep three reasons).
- 2026-10-26, a document that cannot be read or opened shows its line ("OFL.txt could not be
  read.") under the Documents list, not in the `0075` slot of the Fonts tab: the tab may not be
  shown while the pane is.
- 2026-10-26, two existing tests changed for the amended specs: `acceptance_0075_tester.rs`
  (`ac25_remove_never_leaves_the_folder`) refused a file one subfolder deep, which criterion 14
  now removes; its sub-folder case is two levels deep now (still refused). The `0082` tester
  test and `glyphTable.test.ts` used `"info"` as an unknown pane mode; they use `"table"`.
- Browser check (Chromium, 1280 x 800 and 800 x 600): a zip with Great Vibes, OFL.txt,
  README.md, a PDF, a `__MACOSX` entry and a `../` entry gives "GreatVibes.zip: 1 font added,
  3 documents kept. 1 file was skipped as unsafe."; Info lists the fields, OFL.txt opens in
  place and Escape closes it first; the PDF opens a `blob:` tab; the armed trash reads "Press
  again to remove Great Vibes from Curvyo fonts. Its 3 documents go too."; after the removal
  IndexedDB (version 3) holds no file and no document.

## New dependency

- `open` 5.4 (desktop only, `curvyo-app`): licence MIT, `Byron/open-rs`, maintained (5.4.5),
  the crate `tauri-plugin-opener` uses for `open_path`; with `shellexecute-on-windows`. Brings
  `is-wsl` and `is-docker` (MIT). Replaces the `tauri-plugin-opener` of adrs.md decision 7
  (see above).

- 2026-10-26, lead (customer review): `open` with default features only, so `dunce` is not pulled (it came only through `shellexecute-on-windows`); on Windows `open` uses `cmd /c start`. `is-wsl` (with `is-docker`, about 90 lines together) stays for now; the customer suggests an upstream PR to `open` that inlines the WSL check instead of the dependency.

- 2026-10-26, tester findings: the `zip` crate keys its entry table by name, so a second entry
  with the same full name replaced the first. `FontArchive::open` walks the central directory
  itself and gives each repeated name a unique stand-in of the same length in the central
  record before `zip` reads the table (the local header is not compared); the entry list shows
  the original names, and the sanitised names are made unique as before (`OFL-2.txt`; two equal
  fonts are "already there" by the `0075` rule). Two records pointing at the same data are now
  both listed and so caught by the overlap check (unreadable).
- 2026-10-26, follow-up candidate (no change): on Windows `open` without `shellexecute-on-windows`
  runs `cmd /c start`; a document path with `%` or `^` may then be expanded or escaped by `cmd`
  and not open.
