# ADR 0086: Library browser: one item list in the page, a word-dictionary fuzzy matcher in `curvyo-library-core`, icon tiles as images, and a fourth kind of the one side pane

**Status:** Proposed 2026-11-25 by the architect. Not `needs-customer`: no new crate, no dependency, no format change, no new kind of request. One point touches requests that leave the machine (decision 7, icon bodies one screen ahead), and the customer decides it with spec `0152` criterion 6, which already says it. Spec: [`0152`](../../specs/0152-library-browser/specification.md), Fragen 371, 372 and 373 at their defaults (A). Builds on [ADR 0085](0085-online-template-sources-read-only.md) (requests, session-only data, the Iconify endpoints) and [ADR 0084](0084-document-library-index-read-only-sources-and-copy-import.md) (the local index, page pictures). Amends ADR 0085 decision 2 ("When") and decision 5 (the parsed fields) by note.

**In plain words (for the customer).** The browser shows what the Library tab has already fetched. Nothing new is fetched by opening it or by typing. The search runs inside the app, on your computer, in a few milliseconds even for the largest icon set (31,722 icons). It tolerates typos and parts of words. Icons are drawn as pictures, so nothing in an icon from the internet can run. The browser is the same side pane as the font preview, and only one of them is open at a time.

## Context

The Library tab lists rows built by `curvyo-ui-core::library_tree` and `library_online`. In the desktop build the host (`curvyo-app::library_rows`, `OnlineState`) builds them. In the browser build `WasmLibraryIndex` builds them. Only the desktop build has the Online section (ADR 0085 decision 2). Each icon cell is drawn today from the host's `icon_svg` text as an `<img>` of a `blob:` URL (`OnlineRows.tsx`). The canvas area hosts three pane kinds through chained conditionals in `App.tsx`: the font preview, the library page preview (`libraryStore.useLibraryPane`) and Before/After. Each pair closes the other through effects.

**What the live API answers (three GETs to `api.iconify.design`, 2026-11-25):**
- `/collections`: 280 sets, 558,863 icons in all. Each set has `name`, `total`, `author {name, url}`, `license {title, spdx, url}`, `samples` (up to 6) and `palette` (bool, all 280: 209 false, 71 true). 264 have `category` (17 values such as "Material", "UI 24px", "Emoji", "Logos", "Archive / Unmaintained"). Some also carry `height`, `displayHeight`, `tags` ("Has Padding", "Uses Stroke"), `version` and `hidden: true` (16 sets: the 16 without a category).
- `/collection?prefix=material-symbols`: 745 KB with `prefix`, `total`, `title`, `categories` (17 groups, each name in exactly one group), `hidden` (714 names, none of them in a group), `aliases` as a map `alias → parent name` (8,181; 55 point to a hidden name, none to another alias) and `suffixes` (themes: "" Regular, "rounded", "sharp", "outline", ...). It has no `uncategorized` and no per-icon tags.
- `/noto.json?icons=red-heart,sun-with-face`: `prefix`, `lastModified`, `icons`, `aliases`, `width`, `height` (128). Colour bodies carry fixed fills (`fill="#f44336"`) and `<defs><radialGradient id="SVG…">` with `url(#SVG…)` references.

The largest sets are iconmind (31,722), coolicons (28,770), uix (23,322), fluent (20,015) and material-symbols (15,806). One loaded set can therefore exceed the spec's 10,000-item budget three times over.

## Options

**Where the item list and the matcher run**

| Option | Why it won or lost |
|---|---|
| **A. In the page: a `WasmLibraryBrowser` holds the items and the index; the item list crosses the IPC once per fetch (chosen)** | One code path for both builds. A keystroke is a synchronous wasm call with no IPC. The CPU-time test sits where `CLAUDE.md` §5 puts it (`curvyo-editor-wasm/tests/cpu_time/`). Cost: about 1 to 3 MB of JSON per fetch for a large set, parsed once. |
| B. In the desktop host, a query command per keystroke; in wasm for the browser build | Two call paths. An IPC round trip and a result list on every keystroke, then a second call for the visible tiles' words. |

**Fuzzy matching**

| Option | Why it won or lost |
|---|---|
| **A. Own matcher over a dictionary of distinct field words, built when items arrive (chosen)** | Icon names repeat words heavily ("outline", "rounded", "arrow"). Each distinct word is scored once per query word, then each item sums lookups. The cost is deterministic and needs no dependency. |
| B. A fuzzy crate (`nucleo-matcher`, `fuzzy-matcher`, `strsim`) | They score subsequences or edit distance, but not the spec's ordered strengths with the length-dependent typo budget. We would wrap one and still write the word splitting, folding and field weights. That is a dependency for perhaps 40 lines of edit distance. |
| C. Scan every item per keystroke | Simple, but it scores the same word thousands of times. Over budget for 30,000+ items once typo distance runs. |

**Diacritic folding**

| Option | Why it won or lost |
|---|---|
| **A. Own table for Latin (U+00C0 to U+024F, U+1E00 to U+1EFF), generated once from the Unicode data and checked in (chosen)** | No dependency. It also folds what decomposition does not (ø, ł, đ, æ, œ, ß). Iconify names are ASCII. Templates and documents are mostly Latin. |
| B. `unicode-normalization` (NFD, then drop marks) | Covers every script but leaves ø, ł, æ, ß unfolded. It is a new dependency in a `*-core` crate. |
| C. `icu_normalizer` (already in the lock through `url`) | Unicode-3.0 licence and its data tables in the wasm bundle for the same result as B. |

**Drawing an icon tile**

| Option | Why it won or lost |
|---|---|
| **A. An `<img>` of a `blob:` URL from SVG text built in Rust, the colour set on the root (chosen; the Library tab does this already)** | SVG as an image runs no script, loads nothing external and keeps gradient ids per image (no collisions between tiles). The CSP already allows `img-src blob:`. |
| B. Inline SVG in the DOM | Needs a sanitiser for untrusted markup, and gradient ids collide between tiles. |
| C. Raster in wasm (`curvyo-render-core`) | Safe, but it is a second SVG renderer path for previews, and slower per tile. |

**Hosting the pane**

| Option | Why it won or lost |
|---|---|
| **A. One side-pane store with the open kind (`font-preview`, `library-page`, `library-browser`, `before-after`) and a layout record per kind (chosen)** | Opening one closes the others by construction. `CanvasArea` takes one active pane object instead of four chained conditionals. |
| B. A fourth chained conditional and two more close-each-other effects | Works today, but each new kind adds pairwise close effects and another branch in all six `CanvasArea` props. |

## Decision

1. **Parser additions in `curvyo-library-core/src/iconify.rs`** (golden tests on the recorded answers of the appendix, in `curvyo-library-core/tests/fixtures/`):
   - `IconSet` gains `category: Option<String>` (absent means "Uncategorised") and `palette: Option<bool>` (absent means unknown, never guessed). `WireSet` reads `category` and `palette`. Both `/collections` and the `collections` map of `/search` go through `set_of`, so search results get them too. That the search answer carries them is from the Iconify docs, not checked live.
   - `height`, `displayHeight`, `tags`, `version`, `hidden` and `suffixes` are not read: no criterion uses them. Hidden sets stay listed, as in the Library tab.
   - `parse_collection` returns a `SetListing` instead of a flat `Vec<String>`. It holds `names` (listing order: `uncategorized` first, then the groups in key order, as today), `groups: Vec<String>` and, per name, the indices of its groups (a name may sit in several groups; the live set has one each). It also holds `aliases: Vec<(alias, name)>`, kept only when the alias resolves, at most four links deep, to a listed name. Aliases to `hidden` names and to missing names are dropped. `hidden` names are never listed (as today).
   - `fn has_fixed_colour(body: &str) -> bool` is new. It is true when a `fill`, `stroke`, `stop-color` or `color` value (attribute or `style`) is other than `currentColor`, `none`, `inherit` or `transparent`, or the body has an `<image>`. It is pure and used for the refinement of criterion 14 **(Proposal)**.
   - `fn icon_tile_svg(icon: &IconBody, colour: Rgb8) -> String` is new: `icon_svg` with `color="#rrggbb"` on the root, so `currentColor` draws in the UI's text colour inside an `<img>`. The colour is three bytes, never page text.
2. **The matcher** is `curvyo-library-core/src/search.rs` (with `fold.rs` for decision 3), pure, `#![forbid(unsafe_code)]`, wasm32. No new crate and no dependency.
   - `SearchIndex::build(items: impl Iterator<Item = Fields>)`. `Fields` is the item's field texts by weight: name 8, aliases 4, tags 4, category 2, source and page name 1 **(Proposal)**.
   - *Amended 2026-11-25 by [ADR 0087](0087-library-search-keywords-data-file-and-related-words.md) decision 4 (spec `0153`):* the weights double (name 16, alias and tag 8, category 4, source 2) to make room for published keywords (6) and related words (3).
   - Each text is folded and split at `-`, `_`, `.` and whitespace into words. Every distinct word goes once into a dictionary with its letter mask (a `u64` bit per `a` to `z`, `0` to `9`, plus one bit for "other"). Each item stores `(word id, weight)` pairs in one arena.
   - `rank(&self, query: &str, out: &mut Vec<(u32 score, u32 item)>)`. The query is folded and split the same way and capped at 100 characters. For each query word, every dictionary word gets its best strength: exact 16, prefix 8, substring 4, one or two typos 2, subsequence 1, otherwise none.
     - **Typo** is optimal string alignment distance (insert, delete, substitute, swap neighbours). It is checked against the whole word and against the word's prefix of the query word's length, so a typo still matches while typing ("wethe" matches "weather"). The budget is 0 edits under 3 letters, a swap only at 3 letters, 1 edit at 4 to 7 and 2 at 8 or more (see "Spec changes" 1).
     - **Subsequence** needs at least 3 letters, all in order, starting at the word's first letter **(Proposal)**.
     - **Pruning:** a dictionary word is skipped when the query word's mask has more letters outside it than the budget allows, and when the lengths differ by more than the budget (typo only).
   - An item's score is the sum, over the query words, of the best `weight × strength` over its words. One query word with no match drops the item (AND).
   - The result is sorted by score, ties by the caller's sort order (a stable sort over the pre-sorted list).
   - **Hot path:** `out` and one score buffer per dictionary word are reused. There is no allocation per item or per word, and the typo distance uses two stack rows of 33 cells (words over 32 letters get only exact, prefix, substring and subsequence).
3. **Folding** (`fold.rs`): lower case by `char::to_lowercase`, then the table. "ä" becomes "a", so "a" finds "ä" and "ae" does not. "ß" becomes "ss", "æ" "ae", "œ" "oe", "ø" "o", "ł" "l", "đ" "d", "þ" "th". Other scripts are only lower-cased. The table is generated once by a script in `tools/` from the decompositions in the Unicode data, plus the eight letters above, and checked in. A unit test pins a list of pairs.
4. **The item model, filters, sort and words** live in `curvyo-ui-core::library_browser` (a new module, with words in `library_browser_text.rs`, as `library_online` and `library_online_text` are split).
   - `BrowseItem` holds a key and a kind (icon, template, page). It also holds the name, aliases, tags, categories (for an icon its set's category and its group; for a page its workspace or folder), the source (key and label), the licence, the palette (`Mono`, `Colour` or unknown), whether it is customizable, the modified time and the page count.
   - `items(local: &LibraryIndex, online: &[OnlineSourceView]) -> Vec<BrowseItem>` builds the list from what the Library tab already holds.
   - `Browser::query(&State) -> Answer` returns the total, the count, the facet counts (each facet counted with the other filters on), the order as item indices, the subject line and the empty text.
   - Sorts: Best match, Name (folded, digits as numbers, by a natural comparator), Source then name, Newest. All are stable.
   - No new edge: `ui-core → library-core` exists.
5. **Wasm and host surface.**
   - `curvyo-editor-wasm` exports `WasmLibraryBrowser` with `set_items(json)` (the index is built here), `query(state_json) -> json` (the order as a `Uint32Array`) and `tiles(start, end) -> json` (the words of the visible tiles only).
   - Desktop: one host command `library_browser_items(revision) -> Option<json>`. It answers `None` while the host's fetched data has not changed since `revision` **(Proposal)**. The page asks again whenever the Library tab asks for its rows.
   - Browser build: `WasmLibraryIndex::browse_items_json()` gives the local items (Frage 372 A).
   - Loading reuses the Library tab's paths unchanged: `importIcon`, Load template and Import page.
6. **The pane.** `frontend/src/lib/sidePaneStore.ts` holds `open: PaneKind | null` and, per kind, `full` and `split`. It replaces `setPreviewOpen` and `closePreview` as the way panes close each other.
   - `CanvasArea` gets one active-pane object. The browser component is `components/libraryBrowser/` and reuses `SizeSlider`, the `fontPreviewLayout` and the virtualised grid pattern of `PreviewGrid`.
   - The per-user view state is a second key, `curvyo.library-browser`, read and written by the same guarded helpers as `curvyo.font-preview`. A filter naming a source or category that is gone is dropped on read. A lint rule `react/no-danger: error` goes into `frontend/.oxlintrc.json`.
7. **Requests** (ADR 0085 decision 2, amended).
   - Opening, typing, filtering, sorting and sizing send nothing.
   - The source card's Open or Load and, per Frage 373 A, a set card's **Load set** are the existing `online_open` (`List`, `Set`): one `GET /collection?prefix=` per press, names only.
   - Enter is ADR 0085 decision 5, unchanged.
   - Bodies come through the existing `online_icons` batcher: names collected after a tile has been 100 ms in view, at most 100 per request, one request per set. The batch includes the tiles one screen beyond the visible ones in the scroll direction. That is the amendment to ADR 0085's "visible rows" (spec criterion 6).
   - A visible set card asks its samples' bodies in one request per set, as a visible set row does today. Thumbnails and page pictures are the existing lazy loaders.
8. **Untrusted bodies.**
   - A body is never put into the DOM as markup. It reaches the page only as the SVG text of decision 1, shown as `<img src="blob:…">`. In image mode no script runs and nothing external loads, and the CSP keeps `object-src 'none'`.
   - The import goes through the SVG reader and ADR 0018's limits as today. The 8 MiB answer limit stays.
9. **Memory for the session** (nothing on disk, ADR 0085 decision 1 for the per-user prefs only).
   - The host keeps what it keeps today: the set lists, plus now the groups and aliases of each loaded set. It also keeps bodies, at most 10,000, dropping the least recently asked **(Proposal)**. A dropped body is asked for again only when its tile is in view again.
   - *Amended 2026-11-25 (review of the build):* the body cap is not built. The host keeps every body it fetched this session, as the Library tab already does; 10,000 bodies are about 10 to 20 MB and need 10,000 tiles scrolled into view. The cap comes back if a measurement shows the memory matters. The app has one theme today, so "a theme change clears those texts" has nothing to clear yet.
   - The page keeps the item list, the index (about 2 MB for 50,000 items) and at most 2,000 tile SVG texts. A theme change clears those texts, and the host redraws them from its bodies without a request.
10. **Budget.** A keystroke re-ranks 50,000 items in at most 100 ms of thread CPU, measured in `curvyo-editor-wasm/tests/cpu_time/` (see "Spec changes" 2). `set_items` for 50,000 items takes at most 300 ms, once per fetch **(Proposal)**.
11. **No format change.** There is no `format_version` and no new file except the prefs key in the webview's storage, which is per-user view state like the font preview's.

## If a Frage is answered otherwise

- **371 B:** `catalogue.rs` reads two optional index fields, `colour` (`mono` or `colour`) and `customizable` (bool), additive to `catalogue_format = 1`, with a golden test. The `curvyo/templates` index script writes them. The Colour filter then covers templates. **371 C:** no customizable flag in `BrowseItem` and no toggle. Nothing else changes.
- **372 B or C:** `items` takes no local index. The browser build then has nothing to show (it has no Online section), so the Browse button is desktop only and criterion 24 changes. C only delays the local items to slice 2.
- **373 B:** "Load all sets" is a queue of 280 `/collection` requests in the host, one at a time, cancellable, with progress. That is about 25 MB of name lists (estimated from 745 KB for 15,806 names) and 559,000 items. The budget of decision 10 would need re-measuring, and the maker's machine holds about 60 MB more. **373 C:** no Load set. The icon items are only search results (64 per source per Enter), and Category and Colour work per set card.

## Consequences

- One set's item list can be 1 to 3 MB of JSON per fetch. That is acceptable because a fetch is a maker action; typing never crosses the IPC.
- The font preview's and the library page preview's open state move into the side-pane store. That is a behaviour-preserving refactor done in the same branch as the browser, before it.
- Without a translation, "kreis" does not find "circle" (spec, Out of scope).
- Themes (`suffixes`) and set tags ("Uses Stroke") stay unread until a story asks for a theme filter.

## Appendix: recorded answers for the golden fixtures (trimmed, 2026-11-25)

Proposed paths: `curvyo-library-core/tests/fixtures/iconify_live_collections.json`, `iconify_live_collection.json` and `iconify_live_noto_icons.json`. The last one is too long for this page (4.7 KB, a radial gradient with ids in `sun-with-face`). The implementer records it with the same single GET, `/noto.json?icons=red-heart,sun-with-face`, and keeps it untrimmed.

`/collections`, four of 280 sets (`samples` cut to 3 here; the answer had 6):

```json
{"material-symbols": {"name": "Material Symbols", "total": 15806, "author": {"name": "Google", "url": "https://github.com/google/material-design-icons"}, "license": {"title": "Apache 2.0", "spdx": "Apache-2.0", "url": "https://github.com/google/material-design-icons/blob/master/LICENSE"}, "samples": ["downloading", "privacy-tip", "filter-drama-outline"], "height": 24, "category": "Material", "tags": ["Has Padding"], "palette": false},
 "noto": {"name": "Noto Emoji", "total": 3729, "author": {"name": "Google Inc", "url": "https://github.com/googlefonts/noto-emoji"}, "license": {"title": "Apache 2.0", "spdx": "Apache-2.0", "url": "https://github.com/googlefonts/noto-emoji/blob/main/svg/LICENSE"}, "samples": ["beaming-face-with-smiling-eyes", "computer-mouse", "red-heart"], "height": 16, "category": "Emoji", "tags": ["Precise Shapes", "Has Padding"], "palette": true},
 "tabler": {"name": "Tabler Icons", "total": 6563, "version": "3.52.0", "author": {"name": "Paweł Kuna", "url": "https://github.com/tabler/tabler-icons"}, "license": {"title": "MIT", "spdx": "MIT", "url": "https://github.com/tabler/tabler-icons/blob/master/LICENSE"}, "samples": ["alien", "device-desktop", "photo"], "height": 24, "category": "UI 24px", "tags": ["Precise Shapes", "Has Padding", "Uses Stroke"], "palette": false},
 "whh": {"name": "WebHostingHub Glyphs", "total": 2125, "author": {"name": "WebHostingHub"}, "license": {"title": "Open Font License", "spdx": "OFL-1.1", "url": "https://scripts.sil.org/cms/scripts/page.php?site_id=nrsi&id=OFL"}, "samples": ["addtags", "brightness", "circlecallincoming"], "palette": false, "hidden": true}}
```

`/collection?prefix=material-symbols`, two of 17 groups with three names each, real aliases (one to a hidden name), real hidden names and suffixes:

```json
{"prefix": "material-symbols", "total": 15806, "title": "Material Symbols",
 "categories": {"Home": ["activity-zone", "activity-zone-outline", "airwave"], "Maps": ["360", "360-rounded", "add-home"]},
 "hidden": ["astrophotography-off", "account-circle-full"],
 "aliases": {"360-outline": "360", "360-outline-rounded": "360-rounded", "360-outline-sharp": "360", "astrophotography-off-sharp": "astrophotography-off"},
 "suffixes": {"": "Regular", "rounded": "Rounded", "sharp": "Sharp", "outline": "Outline", "outline-rounded": "Outline Rounded", "outline-sharp": "Outline Sharp"}}
```
