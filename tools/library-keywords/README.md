# Search keywords of the library browser

`generate.py` writes the keyword file of `specs/0153-library-keywords` (ADR 0087 decision 1) from
pinned files of three icon projects. It never fetches; the tarballs are fetched by hand below.

| Output | What |
|---|---|
| `frontend/public/library/keywords.bin.gz` | Shipped. Fetched by the page on the first search text, decoded by `curvyo-library-core/src/keywords.rs` |
| `frontend/public/library/keywords-LICENSES.txt` | Shipped. Per source: name, version, licence and the package's `LICENSE` verbatim |
| `tools/library-keywords/related-words.txt` | Not shipped. One line per head word with its related words (icon count), for review and diffs |

Inputs, pinned with their SHA-256 in [`pins.toml`](pins.toml):

| Project | Package, version | File | Licence of the file |
|---|---|---|---|
| Lucide | `lucide-static` 1.80.0 | `tags.json` | ISC |
| Tabler Icons | `@tabler/icons` 3.52.0 | `icons.json` | MIT |
| Material Design Icons | `@mdi/svg` 7.4.47 | `meta.json` | MIT |

The script refuses a file with another checksum, so a re-run gives the committed bytes (gzip level
9, `mtime` 0, no file name in the header; the tests compare the inflated payload, since zlib builds
may deflate differently). A file that is not JSON, a missing or empty name, or a tag that is
neither text, an integer nor null stops it with the file and the key named, and nothing is
written. Integer tags (Tabler `ambulance` 911) and null tags (Tabler `number-0`) are read as text.

```text
mkdir -p upstream && cd upstream
curl -sSfLO https://registry.npmjs.org/lucide-static/-/lucide-static-1.80.0.tgz
curl -sSfLO https://registry.npmjs.org/@tabler/icons/-/icons-3.52.0.tgz
curl -sSfLO https://registry.npmjs.org/@mdi/svg/-/svg-7.4.47.tgz
mkdir lucide-static tabler-icons mdi-svg
tar -xzf lucide-static-1.80.0.tgz -C lucide-static --strip-components=1 package/tags.json package/LICENSE
tar -xzf icons-3.52.0.tgz -C tabler-icons --strip-components=1 package/icons.json package/LICENSE
tar -xzf svg-7.4.47.tgz -C mdi-svg --strip-components=1 package/meta.json package/LICENSE
cd .. && python3 -I tools/library-keywords/generate.py upstream
```

It prints the icons per source, the words, the related heads and pairs, the words gained and lost
against the file it replaces, and the sizes. Raising a version: edit `version`, `sha256` and
`licence_sha256` in `pins.toml`, re-run, and quote the printed lines in a `chore/` PR. The gate runs
the generator only on the samples in `curvyo-library-core/tests/fixtures/keywords/`
(`tests/keywords_generator.rs`).

## What is read

- Lucide `tags.json`: name → tags (keywords).
- Tabler `icons.json`: name → `tags` (keywords), `category`, `styles`; style `filled` sets variant
  bit 0 (Iconify's `<name>-filled` has the same words).
- MDI `meta.json`: `name`, `aliases` (keywords), `tags` (categories such as "Developer / Languages").

Each text is folded as the matcher folds it (lower case, then the table of
`curvyo-library-core/src/fold_table.rs`) and split at whitespace, `-`, `_` and `.`; duplicates per
icon are dropped; an icon with no word is left out.

**Related words** (spec criterion 8): K is related to N when at least 3 distinct icons whose names
contain the word N carry K as a keyword, in at least 2 projects or in at least 6 icons; symmetric;
words under 3 letters, words with anything but letters and `icon outline filled solid alt variant`
are not used; a pair is kept when either word has the other among its 8 strongest, and then both
list it (symmetric union of the top 8), strongest first (icon count, then the word), so a head may
list more than 8; the matcher takes at most 8 per typed word. The summary also prints
the number of null tags read as "null".

## Keyword file format 1

The payload inside the gzip; integers are unsigned LEB128 varints, strings a varint length and
UTF-8.

1. `CVYK`, the format byte `1`.
2. Sources: a count; per source its Iconify prefix, project name, npm package, version, SPDX
   licence, URL, then its variant suffixes (a count and texts).
3. Words: a count, then each folded word, most frequent first. Every word of sections 4 and 5 is
   an index here.
4. Icons, per source in section 2's order: a count; per icon, sorted by name (bytes), the name
   front-coded (the length shared with the previous name, then the rest as a string), the variant
   mask, the keyword indices (a count and indices) and the category indices.
5. Related words: a count; per head word (sorted by word) its index, then a count and the related
   indices, strongest first.
