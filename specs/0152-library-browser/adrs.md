# ADRs for the library browser

Architect, 2026-11-25. One new ADR. Fragen 371 (LIB-CUSTOMIZABLE), 372 (LIB-SCOPE) and 373
(LIB-FETCH) are assumed at their defaults, A; ADR 0086 "If a Frage is answered otherwise" says
what B or C changes. Checked against three live GETs to `api.iconify.design` (2026-11-25).

- [ADR 0086](../../docs/adr/0086-library-browser-items-matcher-and-icon-tiles.md) (Proposed; not `needs-customer`):
  - the parser fields, against the live answer (criteria 13, 14, 25): `IconSet.category` and
    `IconSet.palette`; `parse_collection` returns a `SetListing` with groups per name and resolved
    aliases, and drops hidden names; `has_fixed_colour`; `icon_tile_svg` with the root `color`;
    golden fixtures from the recorded answers (decision 1, appendix);
  - the matcher `curvyo-library-core::search`: a dictionary of distinct folded words built when
    items arrive; the strengths exact, prefix, substring, typo (OSA by word length) and ordered
    subsequence; field weights; letter-mask pruning; no allocation per item; no dependency
    (criteria 18 to 20, 22; decisions 2, 3);
  - the item model, filters, facets, sorts and words in `curvyo-ui-core::library_browser`
    (criteria 5, 11 to 17; decision 4);
  - one item list per fetch into `WasmLibraryBrowser` in the page, for both builds, so a keystroke
    is a wasm call and crosses no IPC (criteria 9, 22, 24; decision 5);
  - one side-pane store with four kinds and a second per-user prefs key (criteria 2, 3; decision 6);
  - requests (criteria 6, 9, 10, 21): only the existing ones; Load set is `online_open` `Set`;
    bodies one screen ahead amend ADR 0085 decision 2 (decision 7);
  - bodies only as `<img>` of SVG text, never inline (criterion 6; decision 8);
  - session memory with caps, budget 50,000 items, no format change (criteria 22, 25; decisions 9 to 11).
- [ADR 0085](../../docs/adr/0085-online-template-sources-read-only.md): requests on the maker's
  action only, session-only data, the Iconify endpoints, Enter search unchanged. Amended by
  ADR 0086 decisions 1 and 7 (parsed fields, one screen ahead).
- [ADR 0084](../../docs/adr/0084-document-library-index-read-only-sources-and-copy-import.md): the
  local index and page pictures are the local items; Import page and Came from unchanged.

## Notes from the architect review of the build (2026-11-25)

- `parse_set_listing` sits beside `parse_collection`, which stays as its names so the `0138`
  tests keep their API. Accepted; production code calls only `parse_set_listing`.
- No host body cap (ADR 0086 decision 9, amended): every fetched body is kept for the session.
- A second host command, `online_icon_tiles(key, prefix, names, colour)`, wraps the existing
  `online_icons` fetch and returns the tile SVG and `has_fixed_colour` per name. No new request.
- The side-pane store holds only the open kind and closes the others; each kind keeps its own
  layout state (ADR 0086 decision 6 said a layout record per kind). Accepted.
- Bodies one screen ahead (decision 7) means one screen in the scroll direction; asking one
  screen behind too is not covered by criterion 6.
- [ADR 0018](../../docs/adr/0018-svg-import-crate-parser-and-limits.md): the import of an icon
  still goes through the SVG reader and its limits.
- `format_version`: none. No new file; the prefs key `curvyo.library-browser` lives in the
  webview's storage, like `curvyo.font-preview`.

## Spec changes the product owner should make (2026-11-25)

1. **Criterion 20 contradicts its own test.** "bxo finds box" needs one swap, but words under 4
   letters allow no edit. ADR 0086 allows exactly one swap of neighbours at 3 letters
   **(Proposal)**; the spec should say so, or drop the "bxo" test.
2. **The budget of criteria 11 and 22 (10,000 items) is below one loaded set.** Material Symbols
   has 15,806 icons and iconmind has 31,722. ADR 0086 budgets 50,000 items in 100 ms.
3. **Criterion 7: a local page row in the Library tab opens the page preview** (`0137` criterion 5);
   it does not import. The spec should pick one. Recommendation: the tile imports ("Import page")
   and the browser stays open; the page preview is not opened from the browser.
4. **Criterion 13: an icon has two categories,** its set's ("Material") and its group ("Home").
   Recommendation: it carries both ("and" instead of "or"), so a "Material" filter does not miss
   grouped sets.
5. **Numbers:** Iconify has 280 sets (not about 200), and 16 of them are flagged `hidden` and have
   no category; they are listed as "Uncategorised", as in the Library tab. "Load all sets" (373 B)
   is 280 requests and about 25 MB. `samples` has up to 6 names; the card keeps 3 as in `0138`.
   The set `height` of "What data exists" is not needed and not parsed.
6. **Criterion 14 refinement:** a tile could drop out of a Monochrome filter while it is in view,
   once its body loads. Recommendation: the refined state applies from the next change of the
   filters or the search, not while the maker looks at the tile (UX notes).
