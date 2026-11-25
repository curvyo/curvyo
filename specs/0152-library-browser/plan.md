# Plan for "Library browser: a big pane with previews, filters, sort and fuzzy search"

One slice, built to ADR 0086 (decisions 1 to 11), `adrs.md` and `ux-notes.md`, Fragen 371 to 374 at
their defaults (A). No `format_version` change, no new crate, no new dependency. Live requests made
while building: only the three GETs ADR 0086 names, to record the golden fixtures.

## Affected crates/modules

- `curvyo-library-core`: `iconify.rs` (`IconSet.category`, `IconSet.palette`), new
  `iconify_listing.rs` (`SetListing`, `parse_set_listing`), new `iconify_tile.rs`
  (`has_fixed_colour`, `icon_tile_svg`, `Rgb8`), new `fold.rs` (diacritic folding, table generated
  by `tools/library/fold_table.py` from Python's `unicodedata`), new `search.rs` (`SearchIndex`, the
  word-dictionary matcher); fixtures `tests/fixtures/iconify_live_*.json`.
- `curvyo-ui-core`: new `library_browser.rs` (the item model), `library_browser_items.rs` (items from
  the local index and the online views), `library_browser_query.rs` (filters, facets, sorts, levels),
  `library_browser_text.rs` (every word); `library_online*` take a `SetListing` per opened set.
- `curvyo-app`: `online_state.rs` keeps the `SetListing` and a per-set failure; new command
  `library_browser_items(revision)` in `library_browse.rs`.
- `curvyo-editor-wasm`: new `wasm_library_browser.rs` (`WasmLibraryBrowser`,
  `WasmLibraryIndex::browse_items_json`); CPU-time budget test `tests/library_browser_budget.rs`.
- `frontend`: `lib/sidePaneStore.ts` (one open pane kind), `lib/libraryBrowserState.ts` (per-user
  prefs `curvyo.library-browser`), `lib/libraryBrowserStore.ts`, `lib/libraryBrowserLayout.ts`,
  `lib/libraryLoad.ts` (the tab's load paths, shared), `components/libraryBrowser/*`, the Browse
  button in `LibraryTab.tsx`, `react/no-danger` in `.oxlintrc.json`.

## Decisions taken here (Proposal)

- `parse_collection` keeps its `Vec<String>` answer (the `0138` tester's tests use it); the new
  `parse_set_listing` returns the `SetListing` of ADR 0086 decision 1, and `parse_collection` is its
  names.
- The side-pane store holds the open kind and closes the others by construction; the font preview,
  page preview and Before/After keep their own layout state (no move of their prefs).
- The pane's narrowest split width is one tile column at the Size, mapped onto `paneLayout`'s size.

## Tasks

M1 `curvyo-library-core` (tests first, golden fixtures recorded once, tests never use the network):
- [x] 1. Record the three GETs into `tests/fixtures/iconify_live_*.json` (AC 25)
- [x] 2. `IconSet.category` and `.palette` from `/collections` and `/search` (AC 13, 14, 25)
- [x] 3. `SetListing`: names in listing order, groups per name, aliases resolved to listed names,
  hidden names and aliases to them dropped (AC 13, 19, 25)
- [x] 4. `has_fixed_colour`, `icon_tile_svg` with the root `color` (AC 6, 14)
- [x] 5. `fold`: lower case, Latin table, ß ss, æ ae, œ oe, ø o, ł l, đ d, þ th (AC 19)
- [x] 6. `search`: words split at `-` `_` `.` space; exact, prefix, substring, typo by length (one
  swap at 3 letters), ordered subsequence of 3+; field weights; all words must match; the tests of
  AC 20 (AC 18 to 20, 22)

M2 `curvyo-ui-core::library_browser` (tests first):
- [x] 7. Items from the local index (documents, pages), catalogues (templates, `customizable` tag),
  loaded Iconify sets (icons with groups and aliases) and Enter results; cards for unfetched
  sources, the ready entry and unloaded sets (AC 5, 8 to 10, 12, 21)
- [x] 8. Filters by AND: Source (My documents and its rows), Category (set category and group, merged
  ignoring case, template tag equal to one, local folder), Colour (palette, refined only on the next
  change), Customizable; facet counts with the other filters; disabled states (AC 11 to 16)
- [x] 9. Sorts: Best match (stable over the own sort), Name (natural), Source then name, Newest
  (AC 17); second level with the matching pages (AC 8)
- [x] 10. Words: subject line, Nothing matches, tile name, meta, licence chip, tooltip, accessible
  name, detail bar lines and actions, cards (AC 4, 5, 7, 11, 23)

M3 hosts:
- [x] 11. `curvyo-app`: listing kept per set, per-set failure, `library_browser_items(revision)`
  (AC 9, 10, 24)
- [x] 12. `curvyo-editor-wasm`: `WasmLibraryBrowser` (`set_items`, `query`, `tiles`, `note_body`),
  `browse_items_json` for the browser build, `icon_tile_svg` text; budget test 50,000 items in
  100 ms of thread CPU (AC 11, 22, 24)

M4 frontend:
- [x] 13. Side-pane store with four kinds; Browse toggle in the Library tab (AC 1, 2)
- [x] 14. Prefs `curvyo.library-browser` (split/full, share, Size, sort, Colour, Customizable, band
  open, search text); Source and Category for the session (AC 3)
- [x] 15. Header (search, Globe, Size 48 to 256, subject line, Full view, Close), facet column from
  640 px or Filters band (AC 4, 11 to 17, 21)
- [x] 16. Virtualised tile grid, icon tiles as `<img>` blob URLs batched for the visible tiles plus
  one screen after 100 ms, thumbnails and page pictures lazily, cards with Load list, Load set, Add
  (AC 5, 6, 9, 10)
- [x] 17. Detail bar, press selects, double-click/Enter/Space loads, drag in split view, second level,
  Pick elements… with Back (AC 7, 8)
- [x] 18. Keyboard and states: arrows, Home/End/Page, printable to search, Escape ladder, empty and
  Nothing matches states (AC 11, 23)
- [x] 19. `react/no-danger: error` lint (ADR 0086 decision 8)

## Left out of this slice (for the lead)

- The host's body cap (10,000, least recently asked first; ADR 0086 decision 9) is not built: the
  host keeps every body it fetched this session, as the Library tab does today.
- Bodies are asked for the rows in view and one screen both ways, not only in the scroll direction.
- The subject line's "Reading 120 of 500" while the local index is read, the coarse-pointer
  gestures (tap, double-tap, hold to drag) and the polite 2 s throttle of the subject line.

## Validation

- Golden tests in `curvyo-library-core/tests/` on recorded answers; matcher unit tests for every
  example of AC 19 and 20; ui-core unit tests per filter, facet and sort; the CPU-time budget test in
  `curvyo-editor-wasm/tests/` (release figure, debug as smoke test, `CURVYO_BUDGET_FACTOR`).
- Frontend: vitest for prefs, layout and the side-pane store; the full gate of `CLAUDE.md` §7.

## Rework after the customer test (Part G, criteria 26 to 33; UX H1 to H14)

- [x] 20. Sets view model in ui-core: `library_browser_cards` (one card per set, catalogue,
  folder or unfetched source, loaded or not, grouped: My documents, Curvyo catalogue, Git
  catalogues, Iconify sources), `library_browser_sets` (search, Sources = groups, Colour,
  Category, counts, subject, empty state), `library_browser_sets_text` (AC 26 to 28)
- [x] 21. Items holds no cards; empty "No items yet." with Show sets (AC 26, 27, H14)
- [x] 22. `library_browser_places`: places, the history (50, no Forward, Escape goes Back only
  after opening), Back names, restore drops what is gone, the narrowed Items place; the
  multi-select rule `select_rows` (AC 27, 30, 31)
- [x] 23. `library_browser_guide`: the hosting guide for this `catalogue_format`, its example
  index parses whole; "The repository must be public." (AC 32, 33)
- [x] 24. No Load / Import page / Load set / Load list / Add: double-click, Enter or drag loads,
  Space selects, the detail bar keeps Show pages and Pick elements…, the page chip shows pages
  (AC 7, 28, 29)
- [x] 25. Frontend: view strip, Sets grid with sticky group headers, Back at the subject line,
  Alt+Left / Cmd+[ / mouse back, facet listboxes, the guide view and its link in the Add source
  form (AC 26 to 33)
