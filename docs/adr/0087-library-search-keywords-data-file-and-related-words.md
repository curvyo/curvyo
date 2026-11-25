# ADR 0087: Library search keywords: a generated gzip data file fetched by the page on the first search, and one-hop related words in the matcher

**Status:** Proposed 2026-11-25 by the architect. Not `needs-customer`: no new crate, no new crate in `Cargo.lock` (`flate2` becomes a direct dependency of `curvyo-library-core`; it is already in both builds through `zip`), no format change, no request. The licences of the shipped data (ISC, MIT) were accepted by the customer with Frage 375 A; see "Licences" for why this is no new licence decision. Spec: [`0153`](../../specs/0153-library-keywords/specification.md). Amends [ADR 0086](0086-library-browser-items-matcher-and-icon-tiles.md) decision 2 (field weights, a related-word strength).

**In plain words (for the customer).** The app ships one small file (about 233 KiB) with the search words that Lucide, Tabler and Material Design Icons publish for their icons, and a list of words that mean nearly the same ("trash", "bin", "delete"). The file is read the first time you type in the browser's search and stays on your computer. A script in the repository builds the file from exact, recorded versions of the three projects; nobody edits it by hand.

## Context

What the three npm packages hold (fetched from `registry.npmjs.org` on 2026-11-25; Iconify name lists from `api.iconify.design/collection?prefix=` the same day):

| Source | Package, version | File | Entries | Names on Iconify | Licence of the file |
|---|---|---|---|---|---|
| Lucide | `lucide-static` 1.80.0 | `tags.json`: name → tags | 2,119 | 2,104 match `lucide:` names; 15 unknown to Iconify | ISC (`LICENSE`, with Feather's MIT text for derived icons) |
| Tabler | `@tabler/icons` 3.52.0 (Iconify's `tabler` version; npm latest 3.53.0) | `icons.json`: name → `{category, tags, styles}` | 5,509 | all 5,509; 1,054 of them have style `filled`, which Iconify names `<name>-filled` | MIT |
| MDI | `@mdi/svg` 7.4.47 | `meta.json`: list of `{name, aliases, tags}`; `tags` are categories such as "Developer / Languages" | 7,447 | all 7,447 | Pictogrammers Free License: icons Apache-2.0, "all non-font and non-icon files" MIT, so `meta.json` is MIT |

No package has a `NOTICE` file. Tabler has 69 tags that are JSON numbers (`ambulance` → `911`, `circle-dashed-number-0` → `0`).

A prototype of the format below on these files (a throwaway script, not in the repository): 8,185 distinct keyword words; the related-words rule of spec criterion 8 gives 2,406 head words and 15,228 pairs, 416 heads with more than 8. Payload 487 KB, **233 KiB gzipped** at level 9. Samples: magnifier → search, zoom, ...; trash → delete, rubbish, garbage, bin, remove, ...; bin → trash, delete; cog → settings, gear, preferences, options, ...

The repository's offline generators are Python 3 standard library scripts in `tools/<name>/` (`tools/library-fold`, `tools/unicode-data`). The Unicode names (`specs/0082`) are a binary file in `frontend/public/unicode/`, fetched by the page on first use and decoded by a core crate; that file is not compressed (362 KiB).

## Options

**Where the data lives**

| Option | Why it won or lost |
|---|---|
| **A. A file in `frontend/public/library/`, fetched by the page on the first search text and handed to `WasmLibraryBrowser` (chosen)** | Both builds get it from their own assets (Tauri's asset protocol, the Pages site); nothing at start-up (criterion 4). Same path as the Unicode names. |
| B. `include_bytes!` in `curvyo-library-core` | 233 KiB more wasm at start-up in both builds, against criterion 4. |
| C. Loaded by the desktop host | Two paths; the browser build has no host. |

**Compression**

| Option | Why it won or lost |
|---|---|
| **A. gzip (level 9, `mtime` 0), inflated in Rust by `flate2` (chosen)** | 233 KiB on any server. `flate2` with the `zlib-rs` backend is already compiled into both builds through `zip` (`curvyo-document-core` reads project containers), so it adds no crate and almost no wasm. |
| B. The plain binary, transfer compression left to the server | 476 KiB if the Pages CDN does not compress `.bin` (not checked; it compresses by content type). |
| C. The browser's `DecompressionStream` | Rejected for the Unicode names already (support in older WebKitGTK builds uncertain). |

**Generator**

| Option | Why it won or lost |
|---|---|
| **A. Python 3 standard library (`json`, `gzip`, `tomllib`), `tools/library-keywords/generate.py` (chosen)** | Repository convention for generators; no build. The gate runs it on samples from a Rust test that spawns `python3`, as `curvyo-connector-ctl/tests/acceptance_0128_tester.rs` does. |
| B. Rust (an example of `curvyo-library-core`) | Builder code next to the decoder, but goldens would need a test-only path into an example, and the derivation of the related words is offline tooling, not product code. |

**Related words in the matcher**

| Option | Why it won or lost |
|---|---|
| **A. Per query word, the related words become a second strength array over the index's dictionary; a posting scores `max(weight × strength, RELATED × related strength)` (chosen)** | One pass, as today; a related word counts as a match of the typed word (AND unchanged); `scores` and `rank` keep their signatures. |
| B. Rewrite the query into extra query words | Breaks AND (an extra word would have to match too) and costs a full dictionary pass per related word. |

## Decision

1. **Generator** `tools/library-keywords/generate.py <upstream folder> [--pins <file>] [--out <folder>]`, Python 3 standard library only, run with `python3 -I`. It never fetches. `tools/library-keywords/README.md` gives the commands that fetch each pinned tarball by hand from `https://registry.npmjs.org/<package>/-/<name>-<version>.tgz` and unpack it, as `tools/unicode-data/README.md` does.
   - **Pin file** `tools/library-keywords/pins.toml`: per source the Iconify prefix, project name, npm package, exact version, file inside the package, its SHA-256, the SPDX licence of that file, the project URL and the licence file. The generator refuses a file with another checksum, so a re-run gives the committed bytes. Raising a version is an edit of this file and a re-run, in a `chore/` PR that quotes the generator's printed counts (icons, keywords, related pairs, gained and lost) and the size.
   - **Reading.** Lucide: name → tags. Tabler: name → tags and `category`; style `filled` sets the variant bit of decision 2. MDI: `aliases` are keywords, `tags` are categories. A tag that is a JSON integer is taken as its decimal text; any other non-text value, a missing or empty name, or a file that is not JSON stops the generator with the file and the key named, and nothing is written.
   - **Words.** Each text is folded as the matcher folds it: lower case, then the table the generator reads from `curvyo-library-core/src/fold_table.rs` (one source of truth), then split at whitespace, `-`, `_` and `.`. Duplicates per icon are dropped.
   - **Related words** (spec criterion 8, numbers Proposal): K is related to N when at least 3 distinct icons whose names contain the word N carry K as a keyword, in at least 2 projects or in at least 6 icons; symmetric; words under 3 letters and the stop list `icon outline filled solid alt variant` are dropped; per head at most 8, strongest first (icon count, then the word).
   - **Output**: `frontend/public/library/keywords.bin.gz` (shipped), `frontend/public/library/keywords-LICENSES.txt` (shipped: per source its name, version, licence and the package's `LICENSE` verbatim) and `tools/library-keywords/related-words.txt` (not shipped; one line per head word, for the demo review and for readable diffs). Sorted keys, gzip level 9, `mtime` 0, no file name in the gzip header: two runs on one machine give identical bytes. Goldens compare the inflated payload, because zlib builds may deflate differently.
2. **Format 1 of the payload** (inside the gzip; integers are unsigned LEB128, strings a length and UTF-8; documented in the README):
   1. `CVYK`, the format byte `1`.
   2. Sources: a count; per source its Iconify prefix, project name, package, version, SPDX licence, URL and its variant suffixes (a count and texts; Tabler `filled`).
   3. Words: a count, then each folded word, most frequent first. Every word of sections 4 and 5 is an index here.
   4. Icons, per source in section 2's order: a count; per icon, sorted by name, the name front-coded (shared length with the previous name, then the rest), a variant mask (bit *i*: the icon also exists as `<name>-<suffix i>` with the same words), the keyword indices and the category indices.
   5. Related words: a count; per head word (by index) up to 8 related indices.
3. **Decoder** `curvyo-library-core/src/keywords.rs` (one responsibility: decoding the shipped keyword file). `Keywords::decode(&[u8]) -> Result<Keywords, KeywordsError>` accepts the gzip or the plain payload (a dev server may already have removed the gzip layer), inflates at most 4 MiB, and checks every count, index, length and the sort order; a damaged file is an error, never a panic. It re-folds every word (idempotent), so the matcher's word rule holds whatever the generator did. `Keywords::sources()`, `Keywords::of(prefix, name)` (keyword and category words of an Iconify icon, the variant names included; binary search per source, no map) and `Keywords::related()`. The prefix is the Iconify set prefix, whichever Iconify-compatible source served the set.
4. **Matcher** (`search.rs`; amends ADR 0086 decision 2). Weights, doubled to make room (Proposal): name 16, alias 8, template tag 8, **keyword 6**, category 4, **related 3**, source and set name 2. Keyword and category words of `Keywords::of` enter the index as ordinary fields, so every strength of `0152` criterion 20 applies to them.
   - `SearchIndex::set_related(&Related)` maps each related word to this index's dictionary ids once; words the index does not have are dropped.
   - Per query word of at least 3 letters, the head words it matches exactly, as a prefix or as a substring give their related words, exact heads first, then prefix heads, then substring heads, each group in table order, at most 8 distinct (amended 2026-11-25, see Amendments); the related strength of such a dictionary word is the head match's strength. One hop: a related word is never looked up as a head. No typo or subsequence match of a head.
   - A posting counts the related strength only for name, alias, tag and keyword words (weight 6 or more), not for category or source words (Proposal). AND is unchanged.
   - Both strength arrays live in `Scratch`; no allocation per item or keystroke.
5. **Loading** (criterion 4). The browser page fetches `library/keywords.bin.gz` once, on the first non-empty search text in the Items view, from `import.meta.env.BASE_URL`, and calls `WasmLibraryBrowser.set_keywords(bytes) -> bool`. On success `Browser::set_keywords` keeps the data for the session (also across `set_items`), rebuilds the index once (the `set_items` budget of ADR 0086 decision 10) and the page re-runs the current query. Until then, or after a failed fetch or decode, the search runs without keywords and no notice is shown. The Sets view and the Enter search (`0152` criterion 21) do not use keywords.
6. **Licences and notices.** `docs/third-party-assets.md` gains a "Search keywords" section (source, version, file, licence, the changes: words split and folded, related words derived). `keywords-LICENSES.txt` ships beside the data. A test decodes the shipped file and checks that each source's name and version are in both files. The in-app credit line goes in the library browser, not in the File tab's Credits block, which lists only the credits of objects in the document (`0138` criterion 4).
7. **Dependency.** Workspace `flate2 = { version = "1.1", default-features = false, features = ["zlib-rs"] }`, the backend `zip` already selects (MIT OR Apache-2.0; already passes `cargo deny`). `curvyo-library-core` stays `#![forbid(unsafe_code)]` and wasm32: inflating a byte slice is no I/O.
8. **No format change**: no `format_version`, no file written, no setting.

### Amendments

*2026-11-25, implementer (spec `0153`, `ux-notes.md` section 7), Proposal:*

- **An on-demand "why" call** (criterion 14, UX K4 to K7). `SearchIndex::explain(query, item)`
  gives per typed word its best field word (name, keyword, category, related word with its head,
  source); `Browser::why(item, text)` turns it into the detail bar's words and
  `WasmLibraryBrowser::why(item, text)` returns them as JSON. The page asks it for the one item
  the bar shows and for the current tile, when that item or the text changes, never in the
  keystroke pass, so the 100 ms budget is untouched.
- **Decision 5, retry** (UX K3): the file is fetched once per success; after a failed fetch or
  decode it is fetched again when the search field next goes from empty to text.
- **Decision 1, null tags**: Tabler 3.52.0 has two JSON `null` tags (`number-0`,
  `number-0-small`, a YAML `null` upstream); they are read as the word "null", like integers, so
  the generator runs on the pinned data. Other non-text values still stop it.
- **Decision 6**: the licence texts in `keywords-LICENSES.txt` keep LF line ends (MDI's `LICENSE`
  has CRLF; the repository checks out LF).
- **Places**: the Licences view of UX K9 is `PlaceView::Licences` of ADR 0086's history.

*2026-11-25, architect (review of the `0153` commits):*

- **Decision 4, head order.** Prefix heads come before substring heads. With one pass in table
  order after the exact heads, "loc" took the related words of "allocate", "block" and "clock"
  before those of "lock" and "location", and the cap of 8 could drop the prefix heads entirely.
- **Decision 1, null tags confirmed.** Tabler's `number-0` lists `zero`, `null`, `nil`: the
  YAML `null` was the word. Reading it as "null" keeps the author's tag; skipping would drop it.
  The generator prints the number of null tags it read, so a version bump PR shows new ones.

*2026-11-25, lead (tester's review of `0153`), Proposal:*

- **Decision 1, related words.** The stored table is the symmetric union of each word's 8
  strongest candidates: when either word has the other in its top 8, both list it (before,
  `square` listed `1:1` but not the other way round), so a head may store more than 8, strongest
  first. Decision 4's cap of 8 applies per typed word in the matcher (exact, prefix, substring
  heads in that order, each head's list by strength). A mutual top 8 was tried and rejected: it
  dropped `trash` → `delete`, the customer's example. Heads and related words are letters only (no
  digits, no punctuation). The shipped table: 2,399 heads, 11,878 pairs, 232,259 bytes; the
  decoder no longer refuses a head with more than 8.
- **Code**: the related-words step is `search_related.rs`; the decoder caps a reserve by the bytes
  left and refuses a varint past 64 bits; `Browser::keywords()` is the one copy of the file.

## Licences

All three files are under permissive licences that need only the copyright line and the licence text kept with copies, and are compatible with Curvyo's AGPL-3.0-or-later (ADR 0006). `CLAUDE.md` §3 "licence" is read here as Curvyo's own licence (ADR 0006) and third-party terms the customer has not seen; the customer took these three in with Frage 375 A and held back only Material Symbols. The one difference to the spec, MDI's `meta.json` being MIT rather than Apache-2.0, is more permissive. So no new question.

## Consequences

- The wasm grows by the decoder and the related-word step (estimated 5 to 15 KiB raw), shown by the wasm-size workflow; the data is outside the wasm.
- The first keystroke searches without keywords for the time of one fetch and one index rebuild; later keystrokes are unchanged in cost apart from about 8,000 more dictionary words.
- 15 Lucide tags and any tag of an icon a set later removes are carried but never match (criterion 9).
- A new source (Phosphor, Material Symbols) is a pin entry, a reader function in the generator and a re-run; the format already carries a source list.
- `search_tests.rs` asserts exact scores; those numbers double with the weights.
