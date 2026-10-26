# ADRs for font info and fonts from zip archives

Architect draft 2026-10-26, written against `story/font-info-zip` (from `main` at `a1ccf09`), for
Frage 41 default **A** (one font folder per archive in the Curvyo folder; subfolders one level deep
are listed). **No new ADR, no new crate, no project `format_version` change, no change to the font
library file.** The archive reader uses `zip`, already a workspace dependency (`curvyo-document-core`'s
`.curvyo` container), so nothing new is downloaded for it. One new desktop dependency:
`tauri-plugin-opener` (decision 7). One dated amendment to ADR 0020 §3 (decision 6, text below).

**needs-customer:** only Frage 41 (where archive contents are stored: a persistence decision,
`CLAUDE.md` §3). Every decision below holds for A; under B or C decisions 3 to 5 and 8 change.

## Depends on

- [ADR 0020](../../docs/adr/0020-text-nodes-text-engine-and-font-sources.md) §2: hostile bytes
  read with `Result` and named limits in `curvyo-text-core::limits`, wasm cannot catch a panic,
  fuzzed. §3: `curvyo-storage-io` lists the Curvyo folder; the page holds host ids. §4: the
  included fonts' `font.toml` licence record.
- [ADR 0004](../../docs/adr/0004-persistence-and-cross-machine-sync.md) §7: the Curvyo folder lives
  in the data directory, atomic writes, folder sync carries it.
- [ADR 0001](../../docs/adr/0001-ui-framework-and-canvas-rendering.md) §6: the host does I/O and
  calls core functions; it decides nothing about fonts or archives itself.
- [ADR 0011](../../docs/adr/0011-workspace-and-crate-layout.md): dependency direction unchanged
  (`storage-io` -> `text-core` exists since `0076`).
- [ADR 0006](../../docs/adr/0006-license.md) item 5: a font's own licence governs; the app shows it,
  never judges it.
- [`0074` adrs.md](../0074-text-objects/adrs.md) decision 5 (limits, fuzz run in the PR);
  [`0075` adrs.md](../0075-font-library/adrs.md) decisions 6 and 7 (Add font, Remove, the IndexedDB
  store); [`0076` adrs.md](../0076-single-line-fonts/adrs.md) decisions 2 and 5 (SVG fonts);
  [`0082` adrs.md](../0082-glyph-table/adrs.md) decision 5 (the pane mode key).

## Feature-local decisions (2026-10-26)

1. **Zip reader: the `zip` crate already in the workspace** (`=9.0.4` via `version = "9"`,
   `default-features = false, features = ["deflate"]`; MIT; deflate through `flate2`/`zlib-rs`, pure
   Rust, builds for wasm32, already linked into `curvyo-editor-wasm` and passed by `cargo deny`).
   `curvyo-text-core` takes it as a workspace dependency. Cost: no new crate in `Cargo.lock`, wasm
   size growth expected under 10 KiB (the same code is already in the module; the plan measures it).
   Stored and deflate only: every other method, and encryption, is reported by the crate as
   unsupported and the entry is left out (criterion 16).
   Rejected: **`rawzip`** (MIT, small, zero-copy, no `unsafe`): a new crate plus our own wiring of a
   deflate crate, CRC and name decoding, for a reader we already ship. **Own reader over
   `miniz_oxide`**: ~400 lines of hostile-input parsing we would own and fuzz alone. **The browser's
   `DecompressionStream` in TypeScript**: it only inflates; the central directory, names, CRC and
   every limit would be a second implementation in JS, against criterion 19's one set of limits.
2. **Where it runs: a new module `curvyo-text-core::font_archive`** ("reads the font and document
   entries of a zip archive from its bytes within the archive limits"). Rejected: a module in
   `curvyo-document-core` (it has the container code, but an archive of fonts is no part of a
   document, and `storage-io` and the browser would pull the document model for it); a new crate
   (no second user; `CLAUDE.md` §5). API, all pure, over `&[u8]`:
   - `FontArchive::open(bytes) -> Result<FontArchive, ArchiveError>` (`NotZip`, `Damaged`,
     `OverLimit`): checks the archive limits and returns the entry table, each entry classed
     `Font(kind)`, `Document`, `LeftOut` or `Unsafe(reason)`, with its sanitised base name, deduped
     inside the archive (`OFL.txt`, `OFL-2.txt`; criterion 9's name rules).
   - `FontArchive::read(i) -> Result<Vec<u8>, EntrySkip>`: decompresses one entry under its limit.
     The caller takes one entry at a time, so peak memory is the archive plus one entry.
   - `is_document_name(name) -> bool`: the document rule of the Terms, also used by the listing
     and by Remove (decisions 4, 5).
   Font entries are **not** accepted here: each target runs its existing loose-file check on the
   bytes (criterion 17): `storage-io`'s `fontdb` load plus `is_outline_face` and
   `svg_listable_faces` on the desktop (`0075` decision 6, now from memory), `listable_faces` in
   wasm in the browser (`0075` decision 7). The archive code only decides what is a candidate.
3. **Hostile-archive limits (criteria 15, 16).** Named constants in `limits`:
   `MAX_ARCHIVE_BYTES` 256 MiB (checked by the caller with `take(limit + 1)` and again in `open`),
   `MAX_ARCHIVE_ENTRIES` 10,000, `MAX_ARCHIVE_KEPT_BYTES` 256 MiB, `MAX_ARCHIVE_FONTS` 500,
   `MAX_DOCUMENT_BYTES` 16 MiB, fonts `MAX_FACE_BYTES` (64 MiB). One set for both targets: the
   per-entry `read` keeps the wasm heap at about archive + 64 MiB; lower the browser numbers only if
   the plan's measurement of a 256 MiB archive in Firefox and Chrome fails. In `open`, in order:
   - **Entry count before parsing:** read the end-of-central-directory record (and the zip64
     locator) ourselves, about 40 lines; a declared count over 10,000 is `OverLimit` before
     `ZipArchive::new` allocates a table for it.
   - **Overlap:** from `by_index_raw`, every entry's span `[header_start, data_start +
     compressed_size)` sorted by start must not overlap another and must end before the central
     directory; otherwise `Damaged` (the overlapping-file bomb).
   - **Names:** decoded from `name_raw()`: UTF-8 when valid, else CP437 (`zip`'s own decoding); a
     part `..`, a leading `/` or `\`, a drive letter (`C:`) or a UNC prefix is `Unsafe`. Names are
     split on both `/` and `\`, only the last part is ever used, sanitised (criterion 9). Nothing
     calls `enclosed_name` or builds a path from a name.
   - **Types:** directories skipped silently; `is_symlink()` or any Unix mode other than regular
     file is `Unsafe`; `__MACOSX/`, `.`-prefixed and `._` names `LeftOut` uncounted; `.zip`, `.7z`,
     `.rar`, `.tar`, `.gz` `LeftOut` and never opened.
   - **In `read`:** the reader is wrapped in `take(min(declared size, entry limit) + 1)`; more
     bytes than declared, or more than the limit, is `EntrySkip::Unsafe`; the running sum of kept
     bytes over 256 MiB is `OverLimit` for the whole archive (refused, nothing kept). CRC-32:
     `zip` checks it at end of stream and returns an I/O error, mapped to `Unsafe`; a test with the
     wrong-CRC fixture pins this, so a `zip` update that drops the check fails.
   - **Fixtures** in `curvyo-text-core/tests/fixtures/fonts/zip/`, the list of criterion 16, built
     by a checked-in script under `tools/` so the bombs are reproducible and small (42.zip-style:
     nesting only, a few KiB).
   - **Fuzz target, as for fonts:** `curvyo-text-core/tests/hostile_archive.rs`, a `proptest` over
     every prefix and one-byte change of each fixture and random entry tables, running `open` and
     every `read`; plus a `cargo-fuzz` target `font_archive` with the fixtures as corpus and a
     10-minute run reported in the PR (ADR 0020 §2, `0074` decision 5). No panic, no hang, peak
     allocation under archive + 2 x 64 MiB.
4. **Listing one level of subfolders (criterion 10; amends `0074` and `0075`).**
   `storage-io::list_font_folder` reads the top level and every subfolder directly in it whose name
   does not start with `.`; symlinked subfolders are not followed (no loop, no reach outside the
   data directory). The listing gains per subfolder its `documents` (name and size, by
   `is_document_name`) and per face `size` and `folder` (the subfolder name or none). Host ids stay
   paths on the host side; the `0075` stamp `<length>-<modified>` is unchanged, so the cuttable
   cache and favourites (family keys) are unaffected by which folder a file is in. De-duplication
   (criterion 11) compares against every listed Curvyo file, top level and subfolders. Browser:
   keys in `files` become `<folder>/<file>` (top-level files keep their bare name, so stored data
   needs no migration); a new store `documents` `{key, bytes}`; the database version goes up by one
   with an `onupgradeneeded` that only adds the store.
5. **Writing the font folder atomically (criteria 12, 18).**
   - **Desktop** (`storage-io::font_files::add_font_archive`, Tauri command `add_font_files` gains
     archives): read the archive with `take(MAX_ARCHIVE_BYTES + 1)`, `open`, read and check entry by
     entry, then write into a new folder `.curvyo-add-<unique>` inside the Curvyo folder (same
     volume, so the rename is a rename), each file written and `sync_all`ed, then
     `std::fs::rename(temp, final)`. A directory rename onto a name that does not exist is atomic on
     Linux, macOS and Windows; if the name was taken meanwhile, the next `-N` is tried; a Windows
     sharing error (a sync client holding a file) is retried 3 times at 100 ms, then the temporary
     folder is removed and the add reports failure. At start, `list_font_folder` removes only
     folders named `.curvyo-add-*` (never other dot-folders the maker owns). The archive is never
     copied. Rejected: writing file by file into the final folder (a crash leaves a half folder the
     listing would show).
   - **Browser:** all entries are read and checked first (JS `ArrayBuffer`s outside the wasm heap),
     then **one** `readwrite` transaction over `files`, `faces` and `documents` with no other await
     inside it, so it cannot auto-commit half way; a quota error aborts all of it (`0075`
     criterion 30).
   - The folder name and the `-N` rule: the existing `0075` rule (case-insensitive, `storage-io`
     and its TypeScript twin with the shared test table), applied to the sanitised archive stem.
6. **Font info fields (criteria 2, 5): `curvyo-text-core::font_info`** ("reads the info fields of
   one face from its bytes"), `font_info(bytes, index) -> Result<FontInfo, TextError>`, through
   `skrifa`/`read-fonts`, on the bytes the `FaceStore` already holds (no file read, criterion 5).
   Name IDs 0, 4, 5, 7, 8, 9, 13, 14 with the record choice of `0075` decision 3 (English US, else
   the first Unicode record); OS/2 `fsType` as the raw `u16` and `achVendID`; `maxp` glyph count;
   format from the header and tables (`ttcf` with face index and count, `CFF `, `CFF2`, `glyf`).
   Each value is an `Option`; the words ("Restricted: may not be embedded", "Not stated") are UI
   strings in the frontend, from a pure mapping of bits that a test table covers. **SVG fonts:**
   family and style as listed, glyph count = `<glyph>` elements from `svg_font`, every other field
   `None`; `<metadata>` is free-form (often RDF) and is not read. Included fonts: `index.json`
   carries `licence` and `licence_file` from `font.toml`; the frontend words the id and fetches
   the shipped text. **ADR 0020 §3 dated note (lead adds it):** "2026-10-26 (`0083`): the listing
   also carries a display-only folder path for the Curvyo and This computer blocks (criterion 2);
   the page never passes it back, files are still addressed by host id."
7. **Documents (criterion 4).** Text: Tauri command `read_font_document(folder, name)` returns at
   most the first 1 MiB (`take`) and whether it was cut; the page decodes as UTF-8 with
   replacement and shows it as plain text (no Markdown, no HTML, never `innerHTML`). PDF on the
   desktop: command `open_font_document(folder, name)` checks that the name ends in `.pdf`, that
   the canonical path lies in a direct subfolder of the Curvyo folder and is a regular file, then
   calls **`tauri-plugin-opener`** `open_path` (MIT OR Apache-2.0, the official Tauri plugin; only
   the Rust API is used, no JS permission is granted to the page). Rejected:
   `tauri-plugin-shell` (far wider surface); `std::process::Command` with `xdg-open`/`open`/`start`
   (three platform branches, `cmd /c start` quoting on Windows). PDF in the browser: a `Blob`
   (`application/pdf`) from the stored bytes, `URL.createObjectURL`, `window.open(url, "_blank",
   "noopener")`, revoked after 60 s; no network request (R-SYS-007).
8. **Remove (criterion 14; amends `0075` decision 6).** `remove_font_files` accepts files directly
   in the Curvyo folder **or in a direct, non-dot subfolder of it** (canonical path check). After
   deleting, if that subfolder holds no file the listing would list as a font, its files matching
   `is_document_name` are deleted and the folder is removed with `remove_dir` (only when empty;
   never `remove_dir_all`, so a maker's other files keep the folder alive). The result names the
   number of documents removed for the confirmation; the page asks the host for it first
   (`documents` of decision 4). Browser: the same rule on the `<folder>/` keys, in one transaction.
9. **Pane mode (criterion 1).** The `0082` key `curvyo.font-preview` field `mode` gains `"info"`;
   an unknown value reads as `"fonts"` (already the rule), so no version change.

## For the product owner

- Criterion 4 and 14: "font folder" means any direct subfolder of the Curvyo folder, hand-made ones
  included (decisions 4, 8); a hand-made folder with other files is kept when its last font goes.
- Criterion 12: the leftover cleanup removes only `.curvyo-add-*` folders, not every dot-folder.
- Criterion 16: a name with invalid UTF-8 is decoded as CP437, so the `_` case only arises for a
  name the UTF-8 flag claims; the fixture should set that flag.

## Dated notes

- 2026-10-26: draft. No ADR: the reader reuses a vetted dependency in an existing crate, the
  stored layout (one folder per archive) is the maker's own files, not a format we version, and
  every choice is reversible in one PR. The only `needs-customer` point is Frage 41.
