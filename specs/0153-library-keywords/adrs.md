# ADRs for library keywords

Architect, 2026-11-25. One new ADR. Frage 375 (LIB-SYNONYMS) stands at A. Checked against the
three pinned npm tarballs (`registry.npmjs.org`) and the Iconify name lists of `lucide`, `tabler`
and `mdi` (`api.iconify.design`), both fetched 2026-11-25, and a throwaway prototype of the
format (not in the repository). Nothing here is `needs-customer` (ADR 0087 "Licences").

- [ADR 0087](../../docs/adr/0087-library-search-keywords-data-file-and-related-words.md) (Proposed; not `needs-customer`):
  - generator `tools/library-keywords/generate.py`, Python 3 standard library, never fetches;
    tarballs fetched by hand per its README; `pins.toml` with exact versions and the SHA-256 of
    each upstream file (criterion 6; decision 1);
  - pinned: `lucide-static` 1.80.0, `@tabler/icons` 3.52.0 (Iconify's `tabler` version), `@mdi/svg`
    7.4.47;
  - names: Lucide and MDI names are Iconify's; Tabler `filled` becomes `<name>-filled` through a
    variant mask; no rename table (criteria 1, 9; decisions 1, 2);
  - format 1: sources, folded word dictionary, front-coded icons per source, related words; gzip
    level 9 in `frontend/public/library/keywords.bin.gz`, prototype 233 KiB (criterion 4;
    decision 2);
  - decoder `curvyo-library-core::keywords`, gzip or plain, 4 MiB cap, every index checked;
    `flate2` made direct (already in both builds through `zip`, no new crate) (decisions 3, 7);
  - matcher: weights doubled, keyword 6, related 3; one-hop related words by exact, prefix or
    substring head match, at most 8, counted on name, alias, tag and keyword words only; AND
    unchanged; no allocation per keystroke (criteria 2, 3, 5; decision 4);
  - loading on the first search text, kept for the session, one index rebuild (criterion 4;
    decision 5);
  - notices in `docs/third-party-assets.md` and `frontend/public/library/keywords-LICENSES.txt`,
    checked by a test (criterion 10; decision 6).
- [ADR 0086](../../docs/adr/0086-library-browser-items-matcher-and-icon-tiles.md): the matcher, folding,
  `WasmLibraryBrowser` and the 50,000-item budget; decision 2 amended by note (weights).
- [ADR 0085](../../docs/adr/0085-online-template-sources-read-only.md): no request added; the Enter
  search sends only the typed words (criterion 11).
- `format_version`: none.

## Tests and goldens that change

- `curvyo-library-core/src/search_tests.rs`: the asserted scores double (lines with `(16, 0)` and
  `(128, 0), (128, 3), (64, 1), (16, 2)`).
- `curvyo-library-core/tests/acceptance_0152_tester.rs`: asserts order, not scores; expected to
  pass unchanged.
- `curvyo-ui-core/tests/library_budget.rs`: gains the keyword file and 4-word queries with related
  words (criterion 5).
- New: `curvyo-library-core/tests/fixtures/keywords/` (three upstream samples of about 30 icons,
  their `pins.toml`, damaged files, the expected inflated `keywords.bin` and `related-words.txt`)
  and `curvyo-library-core/tests/keywords_generator.rs`, which runs the generator with `python3 -I`
  (criterion 7); a notices test (criterion 10). The Iconify goldens of `0138` and `0152` do not
  change.

## Spec changes the product owner should make (2026-11-25)

1. **Criterion 7c would stop on the real data.** Tabler has 69 tags that are JSON numbers
   (`ambulance` 911, `circle-dashed-number-0` 0). ADR 0087 reads an integer as its text; other
   non-text values stop the generator.
2. **MDI's licence.** `@mdi/svg` is under the Pictogrammers Free License: icons Apache-2.0, all
   non-icon files MIT, so `meta.json` is MIT. No package has a `NOTICE`. The credit line should
   read "Material Design Icons (MIT)", and the data table should say so too.
3. **Criterion 10a: the Credits block is the wrong place.** It lists only credits of objects in the
   document and is hidden without them (`0138` criterion 4). Recommendation: a line at the foot of
   the library browser; `ux-engineer` places it.
4. **Criterion 10b: there is no About view.** The notices are `docs/third-party-assets.md` and
   `keywords-LICENSES.txt` shipped beside the data. An About view would be new scope.
5. **Criterion 6: the generator does not fetch.** The tarballs are fetched by hand (README
   commands), as for the Unicode data. The pin file also records each file's SHA-256. The place
   is `tools/library-keywords/` (repository convention), not `scripts/`.
6. **Criterion 4: the wasm grows too.** The decoder and the related-word step add an estimated 5 to
   15 KiB raw. The "under 5 KiB" can only hold for the TypeScript loader.
7. **Data table:** MDI also gives categories (its `tags`), weighted as categories. Tabler's 1,054
   filled icons get the keywords of their outline icon. 15 Lucide names are not on Iconify today
   and are ignored (criterion 9).
