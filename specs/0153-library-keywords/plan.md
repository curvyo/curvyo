# Plan for "Library keywords: the browser's search also finds icons by published keywords and related words"

One slice, built to ADR 0087 (decisions 1 to 8), `adrs.md` and `ux-notes.md` (K1 to K9), Frage 375
at A. No `format_version` change, no new crate; `flate2` becomes a direct dependency of
`curvyo-library-core` (already in `Cargo.lock` through `zip`). Downloads while building: the three
pinned npm tarballs, once, by hand, to generate the shipped data file (the generator never fetches).

## Affected crates/modules

- `tools/library-keywords/`: `generate.py` (Python 3 standard library), `pins.toml`, `README.md`
  (manual download commands, format 1), `related-words.txt` (generated, not shipped).
- `curvyo-library-core`: new `keywords.rs` (the decoder, `Keywords`), `error.rs`
  (`KeywordsError`), `search.rs` (weights doubled, `WEIGHT_KEYWORD`, `WEIGHT_RELATED`,
  `SearchIndex::set_related`, the related strengths in `Scratch`, `SearchIndex::explain`);
  `search_tests.rs` scores; fixtures `tests/fixtures/keywords/`; tests `tests/keywords_generator.rs`
  (runs the generator with `python3 -I`, goldens, damaged files, the read-back searches) and
  `tests/keywords_notices.rs` (the shipped file against both notices).
- `curvyo-ui-core`: new `library_browser_index.rs` (the browser's index from the list and the
  keyword file), new `library_browser_why.rs` (the detail bar's "Matches …" words);
  `Browser::set_keywords`, `Browser::why`.
- `curvyo-editor-wasm`: `WasmLibraryBrowser::set_keywords(bytes)`, `WasmLibraryBrowser::why(item,
  text)`; `tests/library_browser_budget.rs` gains the shipped keyword file and 4-word queries.
- `frontend`: `public/library/keywords.bin.gz` and `keywords-LICENSES.txt` (generated),
  `lib/libraryKeywords.ts` (fetch on the first search text, 400 ms label, retry), the detail-bar
  line and `aria-description`, the credit line at the foot of the facets, `KeywordLicencesView`
  (a place of the browser).
- Docs: `docs/third-party-assets.md` ("Search keywords"), ADR 0087 amendment (the on-demand
  `why` call, the retry), spec criterion 14 gains "category" (Proposal).

## Decisions taken here (Proposal)

- Icons without keywords and categories are not written to the file; a related word is only kept
  in an index when the index has it as a name, alias, tag or keyword word (the "counted on" rule of
  decision 4), so the 8 are 8 that can match.
- The budget test with the keyword file goes into `curvyo-editor-wasm/tests/library_browser_budget.rs`
  (the 50,000-item budget of `0152` lives there with the `budget_factor` clock);
  `curvyo-ui-core/tests/library_budget.rs` is the document library's (`0137`) and stays.
- Related heads are written sorted by word; "table order" of decision 4 is that order.
- A JSON `null` tag is read as the word "null" (Tabler 3.52.0 has two); otherwise criterion 7c
  would stop the generator on the pinned data. Licence texts keep LF line ends.
- The current tile's `aria-description` is the match text followed by its usual description, so
  nothing is lost for a screen reader (K7).

## Tasks

M1 generator and data (tests first on the samples):
- [x] 1. Samples of the three upstream files (about 30 icons each) with their `pins.toml`, the
  damaged files; expected `keywords.bin` (inflated) and `related-words.txt` (AC 6, 7a, 7b, 7c, 8)
- [x] 2. `generate.py`: pins and checksums, readers, folding from `fold_table.rs`, related words,
  format 1, gzip level 9 `mtime` 0, licences file; stops with file and key, writes nothing (AC 6 to 8)
- [x] 3. `README.md` with the download commands and format 1; real data generated into
  `frontend/public/library/`, size at most 300 KiB (AC 4, 6, 10b, 10c)

M2 `curvyo-library-core` (tests first):
- [x] 4. `Keywords::decode`: gzip or plain, 4 MiB cap, every count, index and order checked, words
  re-folded; `sources`, `of(prefix, name)` with variant names, related (AC 1, 4, 7d, 9, 10c)
- [x] 5. Matcher: weights 16/8/8/6/4/3/2, related words one hop, at most 8, exact/prefix/substring
  heads only, related only on weight 6 or more, AND unchanged, no allocation per keystroke;
  `explain` for one item (AC 2, 3, 5, 14)
- [x] 6. `search_tests.rs` scores doubled; generator test via `python3 -I` with the goldens and the
  read-back searches; notices test; decode CPU time within 100 ms (AC 1 to 4, 7, 10)

M3 `curvyo-ui-core` and `curvyo-editor-wasm`:
- [x] 7. `Browser::set_keywords` (kept across lists, one rebuild), keyword and category words of
  Iconify icons by prefix; `Browser::why` and its words (K4, K5) (AC 1 to 3, 9, 14)
- [x] 8. `set_keywords`, `why` in `WasmLibraryBrowser`; budget with 50,000 items, the shipped file
  and 4-word queries within 100 ms (AC 4, 5)

M4 frontend:
- [x] 9. Fetch on the first search text, " · Loading keywords…" after 400 ms, quiet failure, retry
  on the next empty-to-text (K2, K3) (AC 4, 11)
- [x] 10. The detail-bar line, the tooltip and the current tile's `aria-description` (K4 to K7)
  (AC 14)
- [x] 11. The credit line and the Licences place (K8, K9) (AC 10a)

M5 docs and gate:
- [x] 12. `docs/third-party-assets.md`, ADR 0087 amendment, spec criterion 14 (AC 10b, 14)
- [x] 13. Full gate of `CLAUDE.md` §7, wasm32 clippy, `cargo deny check`

## Validation

Goldens on the samples compare the inflated payload and the related-words text; the read-back test
runs the searches of criteria 1 to 3 on the sample file; a test decodes the shipped file and checks
size, sources and both notices; CPU-time budgets in release (`cargo test --release -p
curvyo-editor-wasm --test library_browser_budget`). Manual: `npm run dev`, open the browser, load
Lucide, type "magnifier".
