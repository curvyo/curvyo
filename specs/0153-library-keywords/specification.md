# Library keywords: the browser's search also finds icons by the keywords their projects publish and by related words

Status: Draft
Priority: Should
Origin: Customer, 2026-11-25: Frage 375 (LIB-SYNONYMS) of `0152` stands at its default A (use the published keywords and a related-words table, shipped with the app). Everything marked **(Proposal)** is the product owner's and vetoable at the demo.
Depends on: `0152` (the matcher `curvyo-library-core/src/search.rs`, the 50,000-item budget), `0138` (Iconify sets, Credits)

## User value

As a maker who looks for an icon by what it shows ("magnifier", "trash", "bin", "cog"), not by the name its set gave it, I want the browser's search to know the keywords the icon projects publish and the words that mean nearly the same, so that "magnifier" finds `search` and "trash" finds `delete`, while typing still sends nothing anywhere.

## Field reference

| Tool | What it does |
|---|---|
| Lucide, Tabler, Material Design Icons (their own sites) | Search their own set by per-icon tags, categories or aliases; one set at a time. |
| Iconify (`/search`) | Server search on name parts only; no per-icon tags (architect's research for `0152`). We do not use it for this. |
| Inkscape, LightBurn | No icon library search. |

**What we do better.** One local search across every set the maker loaded, using the tags of the three projects that publish them, and a small table of related words so a keyword of one project helps the sets that have none. No request, no account, no model.

## Data (decided with Frage 375 A; versions pinned by the generator, criterion 6)

| Source | File | Licence | Icons | What it gives |
|---|---|---|---|---|
| Lucide | `lucide-static/tags.json` | ISC | 2,119 | tags per icon name |
| Tabler | `@tabler/icons/icons.json` | MIT | 5,509 | tags and category per icon |
| Material Design Icons | `@mdi/svg/meta.json` | Apache-2.0 | 7,447 | aliases per icon (renamed and alternative names) |

Material Symbols tags are **not** used until their licence is cleared (customer, 2026-11-25). An on-device AI model was rejected (Frage 375 C).

## Acceptance criteria

1. Given an Iconify icon of set `lucide`, `tabler` or `mdi` and a keyword the project publishes for it, when the maker types that keyword, then the icon matches in the Items grid of `0152`. Tests (from the pinned data): a Lucide tag of `search` finds `lucide:search`; a Tabler tag of `trash` finds `tabler:trash`; an MDI alias finds its icon. A keyword never matches an icon it was not published for (`mdi:heart` has no Lucide tags applied).
2. Given the related-words table, then a typed word is also looked up as a **related word** and the icons of its related words match, in every set, with or without own keywords. Tests: "magnifier" finds `search` in `lucide`, `tabler`, `mdi` and in a set with no keywords (for example `ph`, if loaded); "trash" finds `delete`; "bin" finds `trash`. A related word expands one hop only, at most 8 related words per typed word (Proposal), and is never itself expanded.
3. Given the **ranking**, then the weight of a match is, from heavy to light: name, Iconify alias (`0152` criterion 19), published keyword (Lucide tag, Tabler tag, MDI alias), category, related word, set name. Test: with the typed word "search", the icon named `search` ranks above an icon that only has "search" as a keyword, above an icon that matches only through a related word; typo and prefix strengths of `0152` criterion 20 apply to keywords the same way and to related words only by exact, prefix and substring (no typo expansion of a related word, Proposal). All typed words must still match (AND, `0152` criterion 19); a related word counts as a match of the typed word it came from.
4. Given the data file, then it is part of the app in both builds, loaded on the first search text the maker types (not at start-up, not before the browser opens), decoded in a worker or in one go within 100 ms of thread CPU time on the reference machine of the budgets, and kept for the session. Its size is **at most 300 KiB gzipped** (Frage 375 A; the test fails above 320 KiB). The start-up size of the app does not grow by more than the loader (Proposal: under 5 KiB).
5. Given up to 50,000 fetched items and typed text of up to 100 characters with up to 4 words, each with related words, then a keystroke re-filters and re-ranks within **100 ms of thread CPU time** (the budget of `0152` criterion 22, unchanged; measured as in `curvyo-editor-wasm/tests/cpu_time/`); the lookup of keywords and related words does not allocate per item per keystroke beyond what `0152` allows.
6. Given the **generator**, then the data file is produced by one script in the repository (language and place: architect, ADR **(Proposal)**: `scripts/` plus a file listing the pinned upstream versions: `lucide-static`, `@tabler/icons`, `@mdi/svg`, each an exact npm version). Running it fetches those exact versions, writes the data file and a short `SOURCES` header (name, version, licence per source), and is **reproducible**: running it twice gives byte-identical output (sorted keys, fixed compression level). The gate never fetches: it runs the generator on small checked-in samples, criterion 7. Raising a version is a deliberate edit of the pin file and a re-run, in a `chore/` PR that shows the size and the count of keywords gained and lost.
7. Given the **golden tests** (`tests/fixtures/`): (a) a sample of each of the three upstream files (about 30 icons each, with a tag list, a category, an alias, an empty list, a name with variants such as `tabler` outline and filled, and an icon that appears in one file only) is turned into the expected data file; (b) a sample of 40 keywords with overlapping tags gives the expected related-words table (the threshold rule of criterion 8); (c) a damaged upstream file (not JSON, a tag that is not a text, a missing name) makes the generator stop with the file and the key named, and writes nothing; (d) the data file is read back and the sample searches of criteria 1 to 3 pass on it.
8. Given the related-words table, then it is **derived from the upstream data, not written by hand**: a word K is related to a word N when at least 3 distinct icons whose names contain N carry K as a keyword, in at least 2 of the 3 projects or in at least 6 icons (Proposal; the numbers are tuned once against the samples, then fixed by golden test); the relation is symmetric; words of fewer than 3 letters and a short stop list ("icon", "outline", "filled", "solid", "alt", "variant") are not used; every word is folded as in `0152` criterion 19. The table is listed in a readable form beside the data file in the generator's output directory (not shipped) so the PO can review it at the demo.
9. Given an icon of a set that the data does not know (a set with no published keywords), then it matches by its name and the related words only, exactly as before for the typed word itself; nothing breaks and no notice is shown. Given an icon name that the data knows but the loaded set does not have (an old tag for a removed icon), then it is ignored.
10. Given **credits and notices**, then (a) the Library tab's Credits area of `0138` gains a line "Search keywords: Lucide (ISC), Tabler Icons (MIT), Material Design Icons (Apache-2.0)" with the pinned versions, each a link to the project; (b) the app's third-party notices (About, desktop and browser, and the notices file in the repository) hold the copyright line and the full licence text of the three, and the Apache-2.0 NOTICE if the upstream package has one; (c) the data file's header carries the same names and versions. A build without all three (a test reads the notices) fails.
11. Given the **no-request rule** (`R-SYS-002`, `R-SYS-007`), then typing, loading the data and searching send nothing; the data file is a bundled asset, not fetched at run time; the Enter search of `0152` criterion 21 is unchanged and does not send related words (Proposal: only the words the maker typed).
12. Given a language, then English only: no translation ("kreis" does not find "circle", `0152` Out of scope stays); the related words are English words.
13. Given the format, then nothing changes: no `format_version` change, no file written, no setting (Proposal: no switch to turn keywords off; "speculative option").
14. Given the keyboard and the screen reader, then nothing about the browser's interaction changes; a tile that matched only through a keyword or a related word shows, in the detail bar of `0152` criterion 7, the line "Matches “magnifier” (keyword)" or "Matches “bin” (related to trash)" (Proposal) so the result is explainable.

## Out of scope

- Keywords of Material Symbols and of any other set (Phosphor, Heroicons and so on) until their licence and data are looked at; each is its own small follow-up that adds a source line to the generator.
- Translations, stemming, plural forms, spelling variants of languages other than English, per-maker synonyms, a synonym editor.
- An on-device or online AI model; Iconify's server search.
- Keywords for catalogue templates and local pages (their tags are `0152`'s fields).
- Showing keywords on the tile, filtering by a keyword, a keyword cloud.

## UX notes

No new control. Only the detail-bar line of criterion 14 and the Credits line of criterion 10 are new text; `ux-engineer` reviews both before Ready.

## Open questions (customer)

None. Frage 375 is answered by default A. Thresholds and weights are Proposals for the demo.

## Links

Requirements: R-ASSET-004, R-ASSET-007, R-SYS-002, R-SYS-007 (the row proposed for `0152`, R-ASSET-009)
Depends on: `0152`, `0138`
ADR: to be written (architect): data file format and place, generator language, loading, size budget; no new crate expected.
PR: none yet
