# Plan for the glyph table

Implementer, 2026-10-24, on `story/glyph-table` from `main`. Built for Frage 39 A and Frage 40 A
(defaults, customer has not answered), the architect's decisions 1 to 6 of [`adrs.md`](adrs.md)
and the UX notes ([`ux-notes.md`](ux-notes.md), P1 to P7 applied to the criteria as Proposal).

## Affected crates/modules

- `curvyo-text-core`:
  - `face.rs`: `Face::glyph_for(c) -> u32`, the one presence predicate; `missing_chars` calls it.
  - new `glyph_list.rs` ("lists the characters a face draws"): `Face::listed_chars()` (cached in
    a `OnceLock` per face), `ListedChar { c, glyph, name }`; candidates from every `cmap`
    subtable, the selected charmap's own iterator (Mac Roman, symbol), the symbol aliases
    `c - 0xF000` of `U+F020..U+F0FF`, SVG glyphs with a one-scalar `unicode`; glyph names from
    `skrifa::GlyphNames` (`post`, CFF; synthesized ones dropped) and SVG `glyph-name`.
  - new `glyph_cell.rs` ("draws one character as a table cell"): `glyph_cell(face, c)` with the
    path in cap-height units, the advance and the empty flag.
  - new `unicode_blocks.rs` (generated `const` table, Unicode 17.0.0, `block_of`).
  - new `unicode_names.rs` ("decodes the shipped Unicode character names"): `UnicodeNames::parse`,
    `name_of`, `abbreviation_of`, computed names (Hangul syllables, the prefix ranges such as CJK
    unified ideographs).
- `curvyo-ui-core`: new `glyph_table.rs` ("groups listed characters by block and filters them by
  a query"): query forms (one character, `U+00E4`, `u+e4`, 4 to 6 hex digits, words), ordering
  (exact first), groups with counts.
- `curvyo-editor-wasm`:
  - new `session/glyph_insert.rs`: `insert_from_glyph_table(ch)` (caret, edit at end, new text
    centred in the visible drawing) and `glyph_subject()` (the subject face of the spec's Terms).
  - `font_preview_glue.rs` / `wasm_font_library.rs`: `glyph_list`, `glyph_cells`,
    `glyph_detail`, `set_unicode_names`, `glyph_table` (grouped, filtered) bindings.
- `tools/unicode-data/`: `generate.py` (Python 3 standard library only) reads `UnicodeData.txt`,
  `Blocks.txt` and `NameAliases.txt` of Unicode 17.0.0, checks their SHA-256 and writes
  `frontend/public/unicode/names-17.0.0.bin` and `curvyo-text-core/src/unicode_blocks.rs`;
  `README.md` with the source URLs and checksums. `frontend/public/unicode/LICENSE` (Unicode-3.0).
- `frontend`: Glyphs mode of the `0077` pane (mode switch, search field, subject line, grid with
  sticky block headers and Recent row, cells, detail bar, keyboard, empty states), the Glyphs
  button in the Text bar and the Select bar Text group, `curvyo.font-preview` fields `mode`,
  `glyphSearch`, `recent`; the names file fetched on the first Glyphs open; pure rules in `lib/`
  with node tests.
- Docs: `docs/design-system.md` rows (UX notes §8), `docs/technical-debt.md` wasm measurement,
  `specs/README.md` row status.

## Decisions taken while building (dated notes)

- 2026-10-24: the generator is a Python script under `tools/unicode-data/` instead of the
  `#[ignore]` test of `adrs.md` decision 2 (lead's instruction: "script or xtask under tools/").
  It is reproducible: it refuses inputs whose SHA-256 differs from the recorded one, and its
  output is byte-for-byte the committed file. A normal Rust test decodes the committed file.
- 2026-10-24: a query word matches a character name when it is a prefix of one of the name's
  words, ignoring case ("a diaeresis" finds "ä" and "Ä", not "ö": a plain substring test would
  find the "A" of "LATIN" in every Latin letter). Glyph and block names match by substring as
  criterion 16 says.
- 2026-10-24: the names file also carries the abbreviation aliases of `NameAliases.txt` for the
  labels of empty-outline cells (UX notes §3, "one rule, no hand-kept list"); until it is loaded
  such a cell shows its hex code point.

Sources (Unicode 17.0.0, <https://www.unicode.org/Public/17.0.0/ucd/>, licence Unicode-3.0):

| File | SHA-256 |
|---|---|
| `UnicodeData.txt` | `2e1efc1dcb59c575eedf5ccae60f95229f706ee6d031835247d843c11d96470c` |
| `Blocks.txt` | `c0edefaf1a19771e830a82735472716af6bf3c3975f6c2a23ffbe2580fbbcb15` |
| `NameAliases.txt` | `793f6f1e4d15fd90f05ae66460191dc4d75d1fea90136a25f30dd6a4cb950eac` |

## Tasks

Milestone 1, core (test first)
- [x] 1. `Face::glyph_for`; `missing_chars` through it; `listed_chars` with candidates from all
  subtables, symbol aliases, SVG glyphs, glyph names, cached per face; built test faces (the
  spec's four-character face, a symbol-encoded face with `post` names); exhaustive test over
  every scalar for each fixture (AC 5, 7, 9, 23)
- [x] 2. Generator, `unicode_blocks.rs`, `names-17.0.0.bin`, LICENSE; `UnicodeNames::parse` with
  U+2665, U+2764, U+00E4, U+AC00, U+4E00, abbreviations and a truncated file (AC 6, 16, 17)
- [x] 3. `glyph_cell` in cap units, empty flag, single-line kind (AC 7, 8)
- [x] 4. `ui-core::glyph_table`: blocks with counts, query forms, ordering, no-match (AC 6, 16)
- [x] 5. Session: `glyph_subject`, `insert_from_glyph_table` three branches, history steps,
  limit; bindings `glyph_list`, `glyph_cells`, `glyph_detail`, names (AC 4, 10 to 14)
- [x] 6. Budgets measured in release: listing 3,000 / 20,000, one screen of cells, a search
  keystroke (AC 10, 16, 21)

Milestone 2, frontend
- [x] 7. `lib/glyphTable*.ts` pure rules with node tests: prefs fields, recent list, layout
  (cell side, columns, rows across blocks), keys, texts (AC 3, 4, 11, 15, 18, 20)
- [x] 8. Mode switch, search field and subject line in both header layouts (AC 1, 4, 16, P7)
- [x] 9. Glyph grid: virtualised rows, sticky headers, Recent row, cells drawn in batches,
  roving focus and keys, empty states (AC 5 to 8, 10, 11, 15, 18, 20, 21)
- [x] 10. Detail bar with Insert; names fetched on first Glyphs open (AC 17, 19)
- [x] 11. Glyphs button in the Text bar and the Select bar; insert wiring and focus (AC 2, 12 to 14)
- [x] 12. Design-system rows (UX notes §8); wasm size and budgets in `docs/technical-debt.md`

Milestone 3, gate
- [x] 13. Full gate (`CLAUDE.md` §7), frontend tsc, npm test, oxlint; browser check with
  screenshots at 1280 x 800 and 800 x 600

## Measured (2026-10-24, plan's machine)

| Budget | Measured | Where |
|---|---|---|
| List 3,000 / 20,000 characters (release) | 1.3 ms / 2.5 ms (budgets 20 / 150 ms) | `curvyo-text-core/tests/glyph_budget.rs` |
| 200 cells drawn (release) | 3.0 ms (budget 100 ms) | same |
| Search keystroke over 3,000 (release) | 0.5 ms worst (budget 30 ms) | `curvyo-ui-core/tests/glyph_search_budget.rs` |
| First screen of Source Sans 3 (1,614 characters), dev server | 140 ms (criterion 21: 500 ms) | browser |
| Keystroke to rendered table, production build | 12 to 17 ms (criterion 16: 100 ms) | browser |
| Editor wasm | +84,556 bytes (+29,706 gzip) | `docs/technical-debt.md` |
| Names file | 370,854 bytes (criterion 17: 1 MiB) | `frontend/public/unicode/` |

## Validation

- Exhaustive Rust test (decision 1): for each fixture face every scalar `U+0000..=U+10FFFF` is
  listed exactly when `glyph_for` is not 0 and it is not Cc; the missing check agrees.
- The committed names file is decoded by a normal test; the generator re-run gives the same bytes.
- Release-build timings printed by a test (`--release`, `--nocapture`) for the budgets of
  `adrs.md` decision 3.
- Browser: Playwright on `npm run dev` with Great Vibes, Source Sans 3 and Hershey Script; insert
  in all three branches; screenshots in the scratchpad.
