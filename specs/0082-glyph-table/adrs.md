# ADRs for the glyph table

Architect draft 2026-10-24, written against `story/glyph-table` (on top of `0074` to `0078`)
without running code. Written for the defaults of Frage 39 A (the subject face is the text's own
face) and Frage 40 A (encoded characters only, no document model change).

**No new ADR, no new crate, no new external dependency, no project `format_version` change, no
change to the font library file.** One new shipped data file (decision 2) that the app writes
only at build time, never at run time; it is versioned with the app and is not a protected
format. The only run-time state written is the `localStorage` key of `0077` (decision 5).

## Depends on

- [ADR 0020](../../docs/adr/0020-text-nodes-text-engine-and-font-sources.md) §2:
  `curvyo-text-core` reads the face (`skrifa` 0.48.0, `harfrust` 0.14.0, the SVG font reader of
  `0076`); hostile bytes give `Result`, never a panic. The 2 MiB text engine budget (dated note,
  Frage 20) covers this slice's Rust code; the names data is outside the wasm (decision 2).
- [ADR 0001](../../docs/adr/0001-ui-framework-and-canvas-rendering.md) §1, §5: the table is React
  widget code; listing, grouping and search are Rust; per face the list crosses once as typed
  arrays, per visible row only path strings cross.
- [ADR 0011](../../docs/adr/0011-workspace-and-crate-layout.md) §3: `ui-core` → `text-core`
  exists; `editor-wasm` stays a binding layer.
- [`0074` adrs.md](../0074-text-objects/adrs.md): the edit session, `text_input`, the Text tool's
  "Add text" step, the 10,000-character limit.
- [`0076` adrs.md](../0076-single-line-fonts/adrs.md) decision 6: centre-line drawing of SVG font
  glyphs.
- [`0077` adrs.md](../0077-font-preview/adrs.md) decisions 1 (`CanvasArea`, split and full),
  2 (`preview_faces` store, engine-drawn `<svg>` tiles, `fontFaceLoader`), 3
  (`Face::missing_chars`), 4 (`curvyo.font-preview` key).

## Feature-local decisions (2026-10-24)

1. **Listing a face's characters (criteria 5, 7, 9, 23): `curvyo-text-core`, one predicate shared
   with the missing check.**
   - New `Face::glyph_for(c) -> u32` (OpenType: the `harfrust` charmap lookup `missing_chars`
     does today; SVG: `glyph_of`). `missing_chars` is rewritten to call it. This is the one
     predicate both use, so criterion 9 holds by construction.
   - New module `glyph_list.rs` ("lists the characters a face draws"):
     `Face::listed_chars() -> Arc<[ListedChar]>`, `ListedChar { c: char, glyph: u32, name:
     Option<Box<str>> }`, in code point order. Candidates: OpenType, the code points of every
     `cmap` subtable with a Unicode or Windows symbol encoding (read through `skrifa::raw`, formats
     4, 12 and the others `read-fonts` iterates), plus `c - 0xF000` for each symbol code in
     `U+F020..U+F0FF` (the symbol fallback a shaper may apply); SVG, each `<glyph>` whose `unicode`
     is one scalar. A candidate is listed exactly when `glyph_for(c) != 0` and it is not general
     category Cc. Glyph names: `skrifa::GlyphNames` (`post`, CFF charset), SVG `glyph-name`;
     a synthetic name (`gid123`) is dropped.
   - Cached once per `Face` (a `OnceLock` field, as the outline cache), so 20,000 characters are
     listed once per face, not per search keystroke.
   - Test (criterion 9, exhaustive): for each fixture face (outline, symbol-encoded, SVG, the
     spec's four-character fixture), every scalar `U+0000..=U+10FFFF` satisfies
     `listed.contains(c) == (glyph_for(c) != 0 && !c.is_control())`.
   - The empty-outline flag of criterion 8 is not part of the list; it comes with the cell's
     drawing (decision 3), so listing a CJK face does not extract 20,000 outlines.
   Rejected: `skrifa::Charmap::mappings()` alone. It walks only the subtable `skrifa` selects,
   which may differ from the one `harfrust` maps through, so a listed character could still be
   reported missing.

2. **Unicode data (criteria 6, 16, 17): blocks in the wasm, names as a lazily fetched file.
   Unicode 17.0.0.**
   - Blocks: a generated `const` table (`Blocks.txt`, about 340 ranges, about 10 KiB) in
     `curvyo-text-core/src/unicode_blocks.rs` with `block_of(c) -> Option<&'static str>`.
     A code point in no block goes into a last group "No block".
   - Names: a generated file `frontend/public/unicode/names-17.0.0.bin`, fetched by the frontend
     on the first switch to Glyphs mode, handed to wasm once, decoded by
     `curvyo-text-core::UnicodeNames::parse(&[u8]) -> Result<UnicodeNames, TextError>` (module
     `unicode_names.rs`, "decodes the shipped Unicode character names"). Format: magic, Unicode
     version, a word dictionary, then per named code point a varint delta and varint word
     indices. Estimate 450 to 550 KiB for about 36,000 explicit names (the plain
     `code;NAME` text is about 1.2 MB, over criterion 17's 1 MiB). Algorithmic names (CJK
     unified and compatibility ideographs, Tangut, Khitan, Nushu, Egyptian hieroglyph ranges,
     Hangul syllables with the jamo short-name table) are computed, not stored. Until the file
     arrives, search matches by character, code point, glyph name and block name, and the detail
     bar shows no name; then the search runs again. Desktop and browser fetch the same file.
   - Generator: an `#[ignore]` test `curvyo-text-core/tests/generate_unicode_data.rs` that reads
     `UnicodeData.txt` and `Blocks.txt` from a path in an environment variable and writes both
     outputs; a normal test decodes the committed file and checks U+2665 "BLACK HEART SUIT",
     U+2764 "HEAVY BLACK HEART", U+00E4, U+AC00, U+4E00 and a truncated file (error, no panic).
     The plan names the source URL and the SHA-256 of both input files.
   - Licence: Unicode License v3 (`Unicode-3.0`, already on the `deny.toml` allow-list; it is
     data, not a crate, so `cargo deny` does not see it). The notice ships as
     `frontend/public/unicode/LICENSE` next to the file.
   Rejected: the `unicode_names2` crate (MIT OR Apache-2.0). Its tables sit in the wasm, so they
   load at start, which criterion 17 forbids, and they take a large share of the about 262 KiB
   left in the 2 MiB budget after `0076`. Rejected: names inside the wasm through our own table
   (same start-up load). Rejected: plain text compressed with the browser's
   `DecompressionStream` (support in older WebKitGTK builds is uncertain; the dictionary format
   is small enough uncompressed).

3. **Drawing cells (criteria 8, 15, 21): the `0077` drawing path, one call per row batch,
   size-independent.**
   - Faces come from `preview_faces` (`0077` decision 2), the same bytes the tiles use.
   - `curvyo-text-core::glyph_cell(face, c) -> GlyphCell { d, advance, empty }`: the glyph's
     outline in cap-height units (1.0 = cap height), origin on the baseline, `d` as the
     `path_data` writer formats it; `empty` when the outline has no segments. No shaping (one
     character, one glyph).
   - Binding: `glyph_list(id) -> Uint32Array` (code points) and `glyph_cells(id, cps:
     Uint32Array) -> JSON [{d, adv, empty}]`, called once per newly visible row batch, plus
     `glyph_detail(id, cp) -> {name, glyph_name, block}` for the detail bar.
   - A cell is an inline `<svg>` with a fixed `viewBox` in cap units; the Size slider and a width
     change only scale or re-flow it, never call wasm (criteria 15, 21). Single-line faces use the
     non-scaling 1 screen-px stroke of `0076` decision 6.
   - Frontend cache per face: code point → cell, kept until the face changes, capped at 32 MiB of
     path text with the least recently shown cells dropped (a 20,000-glyph CJK face may hit it;
     below the cap a cell is drawn once). Virtualisation: visible rows plus one screen, as
     `useFontTreeScroll`; no new npm dependency.
   - Budgets the plan measures (release wasm, plan's machine): listing 3,000 characters under
     20 ms and 20,000 under 150 ms; one screen (about 200 cells) of `glyph_cells` under 100 ms;
     a search keystroke over 3,000 characters under 30 ms.
   Rejected: drawing cells as browser text with a CSS `FontFace` (`0077` decision 2: fallback
   fonts would show characters the cut does not have). Rejected: one `<canvas>` for the whole
   grid (loses per-cell focus, accessible names and selectable text; `0076` decision 6).

4. **Inserting (criteria 12 to 14): one session command, branching in Rust.**
   `EditorSession::insert_from_glyph_table(ch: &str) -> InsertOutcome` in a new module
   `curvyo-editor-wasm/src/session/glyph_insert.rs`:
   - edit mode: `text_input(TextInput::Insert(ch))`, the typed path (undo, selection
     replacement, 10,000-character limit and its notice unchanged);
   - exactly one text selected: `edit_text_at(id, None)` (caret at the end of the string, the
     Enter entry of `0074`), then the same insert;
   - otherwise: `start_new_text` at `viewport_centre()` shifted by half the glyph's advance and
     cap height so the character is centred, then the insert; the finish writes the Text tool's
     "Add text" step. In split view the canvas is already the free part, so its centre is the
     criterion's centre; in full view it is the last split canvas.
   `InsertOutcome { editing: bool, limit_notice: bool }` tells the frontend to refocus the sink.
   Unit tests in the session for the three branches and the history step count.
   Rejected: three binding calls chosen by the frontend (the branch rule would live in
   TypeScript, untested by the gate).

5. **Mode, search text and Recent (criteria 3, 16, 18): the `curvyo.font-preview` key.**
   Three optional fields: `mode: "fonts" | "glyphs"`, `glyphSearch: string` (at most 64
   characters), `recent: string[]` (at most 24, each exactly one scalar, not Cc, distinct). The
   reader's existing rule holds: each unreadable field falls back to its own default alone. An
   older build that rewrites the key drops the new fields (accepted: view state). Rejected: a
   key of its own (criterion 18 says "with the preview state"; two keys to read and write for one
   pane). Rejected: the font library file (`0077` decision 4's reasons).

6. **Grouping and search (criteria 6, 16): `curvyo-ui-core::glyph_table`** ("groups listed
   characters by block and filters them by a query"): pure functions over the list, `block_of`
   and an optional `&UnicodeNames`; query forms one character, `U+00E4`, `u+e4`, 4 to 6 hex
   digits, words matched against the character name, glyph name and block name, ignoring case.

## For the product owner

- Criterion 17: the names file is about 0.5 MiB, loaded on the first switch to Glyphs mode; until
  it arrives (a few ms on the desktop) a search by name finds nothing and then updates.
- Criterion 21: "drawn once per face and character" holds below 32 MiB of cached cells per face;
  a full scroll through a large CJK face may draw some cells twice.
- Criterion 14 in full view: the new text is centred on the drawing as last seen in split view.

## Open for the customer

None. Nothing here is `needs-customer`: no platform, UI framework, document model, persistence,
plugin, licence or account decision. Frage 40 B would need a document model ADR (glyph per
character) and is out of this slice.

## Dated notes

- 2026-10-24: draft. No ADR: decisions 1, 2, 3 and 6 are pure functions and a const table in
  existing crates, decision 4 is one session command, decision 5 a `localStorage` key. The names
  file is build output shipped with the app, not a format users' files depend on. The plan
  measures the editor wasm growth against the 2 MiB text engine budget
  (`docs/technical-debt.md`).
