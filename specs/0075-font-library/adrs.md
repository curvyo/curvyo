# ADRs for the font library: tags, favourites, Add font and Cuttable

Architect draft 2026-10-20, written against `origin/story/weld` at `7bb9ec7` (with `0074` and
`0068`) without editing the repository; rebased onto `main` without `0068`, and groups replaced by
tags, the same day (dated notes). **No new ADR, no new crate, no new external dependency, no
project `format_version` change.** The slice adds one new file format we write (the font library
file, `format = 1`) and one rebuildable cache. Everything else is a module in an existing crate.

## Depends on

- [ADR 0020](../../docs/adr/0020-text-nodes-text-engine-and-font-sources.md) §2: `curvyo-text-core`
  reads hostile bytes with `Result` and the 64 MiB limit; the classification and the family name
  of a face read in the browser are new functions there. §3: `fontdb` lists the Curvyo folder and
  the system fonts in `curvyo-storage-io`, the page never sees a path, `font_bytes(id)` hands bytes
  over raw. §1: font matching (b) compares families with the family key of decision 2.
- [ADR 0004](../../docs/adr/0004-persistence-and-cross-machine-sync.md) §7: the library file and
  the Curvyo folder live in the data directory, atomic writes, no lock file, so folder sync carries
  both. §9: a newer `format` is refused, never partly read. §12: the Curvyo folder is the font
  collection; embedding stays later. §6 (one file per record) is not used; see decision 1.
- [ADR 0001](../../docs/adr/0001-ui-framework-and-canvas-rendering.md) §3 and §6: logic runs as
  wasm; the Tauri host owns platform I/O only, so it copies, deletes and lists files and decides
  nothing about fonts. §5: bytes cross as typed arrays.
- [ADR 0011](../../docs/adr/0011-workspace-and-crate-layout.md) §3: `ui-core` depends on
  `text-core` and `geometry-core` (already), so it is the crate that joins them for the check.
- [`0045` adrs.md](../0045-document-formats-library/adrs.md) decisions 1 to 5: the pattern for a
  per-user TOML file (data directory, canonical writer, `deny_unknown_fields`, golden fixtures,
  host moves text only, `localStorage` in the browser, `u-` ids).
- [`0016` boolean operations](../0016-boolean-operations/): the Union kernel, integer on a grid,
  gives the same result on every target; decision 5 counts its pieces. (`0068` Weld is no longer
  a dependency: Weld was redefined, 2026-10-20.)
- [`0074` adrs.md](../0074-text-objects/adrs.md) decision 5: the limits of criterion 32.

## Feature-local decisions (2026-10-20)

1. **The font library file.** One file `font-library.toml` next to `document-formats.toml` in
   `app_data_dir()` (desktop); in the browser its text sits under the `localStorage` key
   `curvyo.font-library`, beside `curvyo.document-formats`. Shape (`format = 1`):
   ```toml
   format = 1
   chosen = "Wedding"                      # "favourites", a built-in id or an own tag; absent: All
   favourites = ["great vibes", "lobster"] # family keys, sorted
   [overrides.script]                      # built-in tag id, in chip order
   added = ["lobster"]                     # family keys, sorted
   removed = []
   [tags]                                  # own tags by family key, keys sorted
   "great vibes" = ["Signs", "Wedding"]    # tag names, sorted
   ```
   Built-in ids are fixed: `cuttable`, `script`, `sans`, `serif`, `initials`; `all` and
   `favourites` are strip cells, not tags. An own tag is its name: 1 to 24 characters, not All,
   Favourites or a built-in name, one spelling per tag ignoring case; it exists while a family
   carries it, so there is no tag table and no `hidden` entry (tags replaced groups, customer
   2026-10-20; the file never shipped, so `format` stays 1 and there is no migration). Limits:
   256 KiB checked on the text before parsing, 100 own tags, 10,000 keys per list. The writer
   emits exactly this order, so read, write, read is the identity (criterion 12's golden fixtures
   in `curvyo-ui-core/tests/fixtures/font_library/`).
   Rejected: one file per record (ADR 0004 §6). It makes two offline edits on different tags
   conflict-free, but for a few stars and tags it costs a directory, a merge and a migration;
   the same trade `0045` made, with last writer wins.
2. **Model and file code live in `curvyo-ui-core`**, as two modules: `font_library` ("the maker's
   tags and favourites and the membership rule of the built-in tags") and `font_library_file`
   ("reads and writes the font library file"). Membership of a built-in tag: the maker's
   override, else the shipped list, else the automatic result. `ui-core` gains `serde` and `toml`
   as workspace dependencies (already in `Cargo.lock` through `document-core`; nothing new is
   downloaded). Rejected: `curvyo-document-core`, where the formats file lives: the font library
   is no part of any document.
3. **Family key.** The family name exactly as the family row shows it: typographic family (name
   ID 16), else family (ID 1), English (US) record first, else the first Unicode record; this is
   what `fontdb` gives the desktop list. The key is that name trimmed, inner whitespace runs
   collapsed to one space, and lower-cased with `str::to_lowercase` (full Unicode, no
   dependency). No Unicode normalisation: name tables are NFC in practice, and a
   `unicode-normalization` dependency for that case is not worth it. One function,
   `ui-core::family_key`, used by the library and by `0074`'s matching step (b). For bytes read in
   wasm (browser, Add font), `curvyo-text-core` gets `family_name(bytes, index)` and
   `style_name(bytes, index)` with the same rules as `fontdb` and `storage-io`'s `style_name`; a
   test lists the font fixtures both ways and expects the same names.
4. **Classification (criteria 15, 16).** The decision is a pure function in `curvyo-text-core`,
   `declared_class(os2: [u8; 12]) -> Option<DeclaredClass>` over the OS/2 bytes 30 to 41
   (`sFamilyClass`, then the first ten bytes of PANOSE), with the rules of criterion 15.
   On the desktop, `storage-io` already opens every listed file and reads its table directory
   (`is_outline_face`); in the same open it copies those 12 bytes (one more seek, no decision) and
   the listing carries them as `os2` (absent when the face has no OS/2 table). The page hands them
   to wasm with the list; no font bytes cross for this. In the browser, wasm reads the 12 bytes
   from the face it already holds. A family's class is its representative face's (decision 5).
   The plan measures the listing of 2,000 faces against `0074` criterion 20's 2 s.
   Rejected: passing every system font's bytes into wasm (hundreds of MB per start); classifying
   in the host (ADR 0001 §6: the host decides nothing).
5. **The cuttable check (criteria 17 to 19).**
   - **Function:** `curvyo-ui-core::cuttable` ("counts the pieces a test word in one face
     makes"): `cuttable_pieces(bytes, index) -> Result<usize, TextError>` parses the face, lays
     out `cuttable` at size 20 mm (cap height, ADR 0020 §1), passes **one operand per glyph with
     area** to `geometry-core::boolean(BooleanOp::Union, ..)` at `BOOLEAN_TOLERANCE`, and counts
     the outlines of even depth in `outline_nesting` of the result (an island counts). No glyph
     with area, or a refusal, gives 0. There is no gap (revised 2026-10-20, from the architect's
     Weld rework decision 10: Weld lost its gap and piece count; a gap would also make the check
     optimistic). The test of criterion 19 compares it with Union on the converted text
     (Object to path, Union, Split) on the included fonts and two fixtures. The constants and a
     `CUTTABLE_CHECK_VERSION` (2 since the Union change; bumped when word, size or kernel change)
     sit in that module. `editor-wasm` exports it as a free function (binding only). The bytes
     are parsed, checked and dropped: they never enter the session's `FaceStore`.
   - **Which face:** one per family, the representative face: style "Regular", else the first
     face in the list order. The same rule picks the face for decision 4.
   - **Scheduling:** in the page's wasm, one face per `requestIdleCallback` slice (the fallback
     `useEditorSession` already has), bytes fetched by `font_bytes` before the slice. The queue
     holds the Curvyo fonts when listed (an added font runs next), and the system fonts only after
     Cuttable is chosen for the first time on this computer. Progress counts go to the strip cell.
     A face of about 25 ms native is about 50 ms in wasm, below one long task. **Trigger to move
     the check into a Web Worker** running the same wasm (only the scheduler changes): the plan's
     measurement over 2,000 system faces shows a 95th percentile above 50 ms per face, or a
     single face above 300 ms.
     Rejected: running the check natively in the Tauri host on a thread (the fastest, no bytes over
     IPC, but it puts font and geometry logic into the host against ADR 0001 §6 and links three
     core crates into the binary a second time); a Web Worker now (a second wasm instance and a
     worker build for a cost that is not shown yet).
   - **Included fonts:** their results are shipped, not computed at run time: each `font.toml`
     gets `tags = [...]` (criterion 14's shipped list) and `index.json` carries it plus the
     representative face's `cuttable_pieces`. `included_fonts.rs` checks every family has an entry,
     every entry names an included family, and the shipped count equals `cuttable_pieces`. So
     neither target runs the check for them, and the desktop host never needs their bytes.
   - **Cache:** per computer, rebuildable, never authoritative. A line format produced and
     parsed by `ui-core::cuttable` (no dependency): a header `curvyo-font-checks 1 <check
     version>`, a line `system-requested` once Cuttable was chosen, and per face
     `<face hash hex> <index> <pieces or x> <stamp>`. `x` means unreadable and is not retried.
     The stamp is the host's `<length>-<modified seconds>` for the file, passed in the listing:
     a listed face whose host id and stamp match a line needs neither bytes nor a hash, so a
     second start reads no font file; otherwise the bytes are hashed (`FaceHash`) and looked up
     before the check runs. A header with another version, or any line that does not parse,
     discards the whole cache. Desktop: `font-checks.txt` in `app_cache_dir()` (machine-local,
     never synced, deletable); browser: `localStorage` key `curvyo.font-checks`. Written at most
     once per 50 results and at the end, atomically on the desktop.
6. **Add font and Remove on the desktop (criteria 20 to 27).** New Tauri commands in
   `curvyo-app/src/fonts.rs`, I/O in `curvyo-storage-io/src/fonts.rs`:
   - `add_font_files(paths: Option<Vec<String>>)` (async, work in `spawn_blocking`): `None` opens
     the dialog (several files, filter `ttf`, `otf`, `ttc`); `Some` is a drop. Per file: over
     64 MiB, or no face the folder listing would list (`fontdb` load plus `is_outline_face`), is
     "not a font". So **Add font accepts exactly what the Curvyo block lists**; `text-core`
     remains the guard when a face is drawn (`0074` criterion 32, fuzzed there). Same bytes as a
     file in the folder (equal length, then byte compare; no hash dependency in `-io`) is "already
     there". A taken name, compared without case, becomes `<stem>-2.<ext>`, `-3`, ... The copy is
     `write_atomic` into the folder only. Returns `{added: [file names], present: [...],
     not_fonts: [...]}`; the page then calls `font_folder_list` and words the notice.
   - `remove_font_files(ids: Vec<String>)`: the page passes the ids of the family's faces in the
     Curvyo block. A file is deleted only when it lies directly in the Curvyo folder (checked on
     the canonical path, so a system font id is refused) and every face of it is among the ids;
     otherwise it is kept and named in `kept` (the TTC rule of criterion 25).
   - `read_font_library` / `write_font_library(text)` / `read_font_checks` /
     `write_font_checks(text)`: move text only, as `0045`'s formats commands.
   - `font_folder_list` and `system_font_list` gain `os2` and `stamp` per face.
7. **The browser (criteria 28 to 31).** Font files go to **IndexedDB** (database `curvyo-fonts`,
   store `files`: `{name, bytes}` keyed by name). Each Add is one transaction, so a quota error
   aborts it and nothing is kept (criterion 30). Validation, names and class come from wasm
   (`text-core`, decision 3); the `-2` rule and the byte compare are repeated in TypeScript with
   the same test table as `storage-io`. At start the page reads the store, lists its faces as the
   Curvyo block with host ids `idb:<name>`, and hands bytes to the engine as the desktop does
   through `font_bytes`. At the first Add in a page the browser calls `navigator.storage.persist()` (Frage 13,
   decided B by the customer 2026-10-20); Add goes ahead whatever the answer, and evicted fonts
   show as missing (`0074` criterion 29). Rejected: the origin-private file system (OPFS): no transaction across several
   files, and writing from the page thread came late to Safari.
8. **Damaged library file.** A newer `format` is never written over (criterion 12). A file of
   this format that does not parse is reported once ("The font library could not be read.") and kept;
   at the first change it is renamed to `font-library.toml.broken` (`rename_replacing`, as `0045`)
   and the new file written, so the maker can keep working. For the product owner: add to
   criterion 12.

## For the product owner (spec changes; applied to criteria 12, 18 and 32 on 2026-10-20)

- Criterion 18: the included fonts are not checked at run time; their result is shipped and
  test-checked (decision 5). Criterion 32: "the same reader" means the Curvyo block's listing
  rules (decision 6). Criterion 12: decision 8.

## Dated notes

- 2026-10-20: draft. No ADR: every choice here is reversible in one PR except the library file's
  shape, which is versioned (`format = 1`) and has golden fixtures.
- 2026-10-20: customer decided Frage 11 A, Frage 12 A, Frage 13 B (persistent storage is
  requested at the first browser Add; decision 7 updated).
- 2026-10-20 (review): decision 7's store is split into `files` `{name, bytes}` and `faces`
  `{name, faces, length, sha256}`; the browser's check stamp is `<length>-<SHA-256>`.
- 2026-10-20: Customer decision FONT-CUTTABLE A kept; the check uses Union without a gap since
  Weld was redefined (2026-10-20). The branch is rebased onto `main` without the `0068` commits;
  `CUTTABLE_GAP_MM` is gone, `CUTTABLE_CHECK_VERSION` is 2. The shipped piece counts did not
  change (Great Vibes 2, Source Sans 3 8, Source Serif 4 8).
- 2026-10-20: tags replace groups (customer); decision 1's schema and decision 2's model follow,
  the file keeps `format = 1` (never released). Own tags are derived from the families that carry
  them; Reset and the Show switch are gone.
- 2026-10-20 (tester re-review): PANOSE family kind 2 with serif style 2 to 10 is Serif (decision 4);
  `add_font_files` accepts only paths the host granted through `pick_font_files` or a drop on the
  window, each once (decision 6); Add reads a file through `take(64 MiB + 1)`.
