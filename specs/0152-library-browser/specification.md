# Library browser: a big pane with previews, filters, sort and fuzzy search for everything the Library tab lists

Status: Done (PR #483; revision 2026-11-25 after the customer test; Fragen 371 to 374 built at default A; Fragen 375 and 376 open, default as stated; ADR 0086 Proposed, not `needs-customer`)
Priority: Should
Origin: Customer, 2026-11-25 (translated): "#478 is merged. I created the `curvyo/templates` repo, it is still empty, that's fine, Iconify works well. But we need to go strongly into the UI: we need a library browser analogous to the font preview. In the side panel you just can't see well what you're adding, and we need filters, e.g. by categories, black/white (vs. colour), customizables, to sort. And also full-text search, so you can search roughly (fuzzy)." Everything marked **(Proposal)** is the product owner's and vetoable at the demo.
Depends on: `0137` (Library tab, local sources, thumbnails, import, Came from), `0138` (Online section, Git catalogue, Iconify, Enter search, Credits), `0077` (the side pane of the canvas area: split, full, divider, Escape ladder).

## User value

As a maker who looks for an icon, a template or an old page, I want a large browser beside my drawing that shows big previews of everything the Library tab lists, lets me narrow it by category, black-and-white or colour and "customizable", sort it, and find things by typing roughly (a typo, a part of a word), so that I see what I am adding before I add it, instead of squinting at 24 px rows in a 244 px column.

## Field reference

| Tool | Browser | Filters and search |
|---|---|---|
| Iconify web (icon-sets.iconify.design) | Grid of icons with a size, a set list | Set, category, "palette" (monochrome vs. colour), licence, height; search is server-side per word |
| Inkscape 1.4 | Import dialog, no library browser; extensions load symbol sets in a small docked list | None beyond the set |
| Affinity Assets panel, Illustrator Libraries | Grid of thumbnails per library | Name search, categories per library |
| Our own font preview (`0077`) | Tiles beside the drawing, Size slider, tag strip and search shared with the list | Tags, search, favourites |

**What we do better.** One grid across the maker's own pages, the Curvyo catalogue and every Iconify set he loaded, with the same size slider as the font preview; one search that tolerates typos and runs locally while typing; the licence and the source visible on every tile before anything is loaded (R-FONT-008 and `0138` criterion 3 already require them).

## What data exists (checked in the code, 2026-11-25; the criteria name only this)

| Source | Per item today | Notes |
|---|---|---|
| Iconify, `/collections` (`curvyo-library-core::iconify::IconSet`) | set prefix, name, total, author, licence (SPDX, title, url), up to three sample icon names | The API's answer also carries a set **category** (for example "Material", "Logos") and a **palette** flag (`true` = the set's icons carry their own colours, `false` = monochrome, drawn in the current colour) and a height. **The parser does not read these two fields yet**; they are an additive parser change with a recorded-response golden test (`tests/fixtures/`), confirmed against the live answer in ADR 0086 decision 1. Iconify has 280 sets, 16 of them `hidden` and without category; `samples` has up to 6 names, the card keeps 3 as in `0138`; the set `height` is not needed and not parsed. |
| Iconify, `/collection?prefix=` | icon names; the answer groups them in **categories** (the set's own, for example "Arrows", "Weather") plus `uncategorized`; aliases (alternative names pointing to an icon) | The parser **flattens** the categories today; the browser keeps the category per icon name and the aliases. There are **no per-icon tags** in the API. |
| Iconify, `/<prefix>.json?icons=` | the drawing data of the icons asked for (at most 100 per request) | Bodies come for the visible tiles only (ADR 0085). Whether a body uses `currentColor` is known only after this. |
| Iconify, `/search` | `prefix:name` pairs, at most 64, and their sets | Enter only (Frage 340 B). |
| Git catalogue (`catalogue::TemplateEntry`) | path, name, author, **tags**, licence, credit, fonts, size, thumbnail | No palette field, no "customizable" field, no category (tags carry both ideas by convention only). |
| Local documents (`0137`) | document name, workspace or folder, page name, page size, page kind, 224 x 160 px thumbnail, modified time | No tags, no palette. |

## Terms

- **Browser:** the pane of this story. **Library tab:** the side panel of `0137`/`0138`; it stays and keeps working.
- **Item:** one tile: a page of a local document, a catalogue template, or an Iconify icon.
- **Loaded** (an Iconify set): the maker pressed Load on it, so its icon names (one request, `/collection`) are in memory for the session. Nothing is cached on disk (ADR 0085).
- **Fetched:** everything in memory now: local rows, expanded or listed catalogues, loaded Iconify sets, the last Enter results.

## Acceptance criteria

### Part A: opening, split, full

1. Given the Library tab, then a **Browse** icon button (toggle, `aria-pressed`, tooltip "Library browser") sits in its search row; a press opens the browser, a second press closes it. It exists in the desktop and the browser build.
2. Given the browser opens, then it is the side pane of the canvas area with the same behaviour as the font preview (`0077` criteria 2 to 5 and 26 to 27): split to the right of the drawing (35 % or one tile column wide, whichever is wider), a draggable divider (kept share, double-click resets), forced full view when the drawing would keep under 360 px, a Full view / Split view button, a Close button, Escape closing it by the same ladder, the drawing's view not re-zoomed, the pane lying over the canvas which is not resized. Only one side pane is open at a time: opening the browser closes the font preview and the other way round. Closing writes nothing and adds no history step. (One host component, not a second one.)
3. Given the app is closed and started again, then the browser is closed; split or full, its own share (not the font preview's), the Size, the sort, the Colour and Customizable settings, the filter band's open state and the search text are kept per user (key `curvyo.library-browser`) **(Proposal)**; the **Source and Category selections live for the session only** (they name fetched data the next start does not have, UX G7) and the fetched data is not (nothing but `online-sources.toml` is written, `0138` criterion 2).

### Part B: tiles and previews

4. Given the browser, then its header holds, in this order: the search field, a **Size** slider, a subject line ("132 of 4,210 items"), Full view and Close; the filters (criteria 11 to 16) and the Sort control (criterion 17) are strips, a switch and check lists, never dropdowns or popovers, in a facet column beside the grid when the pane is at least 640 px wide and in a Filters band opened by a **Filters** button (with the count of active filters) under it when narrower (UX G2, U4, U7). The Size slider sets the tile's preview edge from 48 to 256 px, default 96 px **(Proposal)**; tiles re-flow within 100 ms of a change; the tooltip is the name and value only ("Size: 96 px").
5. Given the fetched items that pass the filters, then the browser shows one **tile** per item, as a grid of equal tiles, as many columns as fit, in the sort order of criterion 17. A tile shows: the preview, the name on one line cut at the tile's edge, and a second quiet line with the source (set name, catalogue name or document name) and a licence chip (SPDX short name, or "Licence unknown" never styled as free, `0138` criterion 3; the maker's own pages carry no chip). Under Size 96 px a tile is its preview alone (UX G3, U11: a grid of 40 icons per row is what an icon hunt needs); name, source and licence are then in the tooltip and the **detail bar** (Frage 374 A: the hovered, else the current item at 96 px with name, place, licence link, author, facts and its Load button; the detail bar is the pane's last row). A tooltip adds author, licence name and, for a local page, "document › page" and its size. All tiles of one Size are equally high.
6. Given an Iconify icon, then its tile is drawn locally from the icon's body (the same `iconify_svg` the import uses), monochrome icons in the UI's text colour, icons with their own colours in those colours; bodies are asked for in batches of at most 100 for the visible tiles plus one screen ahead, after 100 ms in view; until a body is there the tile shows a quiet bar, never another icon; the grid is virtualised (2,000+ items: only the visible tiles plus one screen are in the page). The preview of a catalogue template is its thumbnail (`0138`), of a local page its stored picture (`0137`), loaded lazily for visible tiles only.
7. Given a press on a tile, then it becomes the **current** tile (detail bar, `aria-selected`) and nothing is loaded (UX G1, U17: a press must not load by accident, as the icon rows of `0138`; Frage 374 A). A **double-click**, **Enter** or a drag loads the item (Part G, criterion 29: no Load or Import page button, Space only selects) exactly as the same row in the Library tab does it: an icon as an imported path (24 mm wide, centred on the visible part of the drawing, one undo step "Import icon", Came from with licence and credit), a template as new pages ("Load template"), a local page as "Import page" (a local page row in the tab opens the page preview, `0137` criterion 5; the browser's tile imports and the browser stays open, ADR 0086); a local page also offers **Pick elements…**, which opens the `0137` page preview in place of the browser with a Back button to it (UX G13, U19). The browser stays open. A drag from a tile onto the canvas works as the drag of the same row (split view only). A tile that cannot be loaded (a template whose format is newer, a source gone offline) says so on the tile and in the detail bar and does nothing else. The licence and credit rules of `0138` criteria 3 and 4 hold unchanged: **Credits** in the File tab list what the objects carry. With Frage 374 B a press loads at once and there is no detail bar.
8. Given a tile that has an expandable thing (a local document with several pages, a template with several pages), then it shows a page count chip and its press opens a second level in the same pane (the pages as tiles, a Back button in the subject line); a single-page document loads directly. With search text a document matches by its name or by a page name, and its second level shows the matching pages (UX G10); Going back is the history of criterion 30 (UX H5, H6: Backspace no longer goes back). **(Proposal)**

### Part C: what is listed, and fetching

9. Given the browser opens, then (Part G, criteria 26 and 27 amend this) it lists the **fetched** items of every source the Library tab shows, and it fetches nothing by itself: it sends no request on opening, none on typing, none in the background (R-SYS-007, `0138` criterion 2, Frage 273 A). Local documents are read as in `0137` criterion 1 (desktop: workspaces and added folders; browser build: the browser's own documents). Per Frage 372 the scope is as chosen there.
10. Given a source that is not fetched yet (the Curvyo catalogue, an added Git catalogue, an added Iconify source), then the browser shows it as a **card** in the **Sets** view (criterion 26), not in the Items grid and with no Load list, Load set or Add button: a double-click or Enter on the card is the request (criterion 28; UX H2). *Superseded in part by Part G: the "Not loaded" group, 208 x 112 px, leaves the grid.* A failed source shows "Offline" or its reason on its card only, as in `0138` criterion 5, and everything else keeps working.

### Part D: filters and sort

11. Given the filter row, then every filter narrows what the grid shows by AND, live and locally within 100 ms of thread CPU for up to 50,000 fetched items (ADR 0086 decision 9: one loaded set can hold 15,806 icons, Material Symbols, or 31,722), and the subject line names the count ("132 of 4,210 items"). A **Clear all** button (clears filters and the search text, UX G4) is present while any filter or the search is active and also sits in the Nothing matches state. An empty result says "Nothing matches" and names the active filters; it never shows a blank grid.
12. **Source** filter (multi-select chips or list; none selected = all): "My documents" (the local ones, per workspace or folder when more than one), "Curvyo catalogue", each added Git catalogue, each loaded Iconify set by its name (with its tile count), and "Search results" (the last Enter results of criterion 21). Only fetched sources are rows; the selection is kept for the session only (criterion 3).
13. **Category** filter (multi-select, none = all), from the data that exists: for an Iconify icon **both** its set's category (from `/collections`, for example "Material"; the 16 hidden sets have none and are "Uncategorised", as in the Library tab) **and**, when the set's list groups its icons, the group (for example "Arrows"), merged by name ignoring case, so a "Material" filter does not miss grouped sets; for a catalogue template, none (an item without a category is under "Uncategorised"; **Proposal:** a tag of the template counts as a category when it equals one of the others' categories, nothing more); for a local page, its workspace or folder name. The filter lists categories A to Z, "Uncategorised" last, with counts of the fetched items that pass the other filters; with no category in the fetched items it shows one muted line instead of rows (UX G5).
14. **Colour** filter (All | Monochrome | Colour): an Iconify icon is Monochrome when its set's `palette` flag is false and Colour when true (the set's flag, known before any body is loaded); items with no such data (catalogue templates, local pages) are shown under All only and are hidden while Monochrome or Colour is on **(Proposal)**; the filter's tooltip says so. Once an icon's body is loaded the browser may refine its state locally (a monochrome set's icon whose body has fixed colours counts as Colour, ADR 0086 `has_fixed_colour`); the refined state applies from the **next change of the filters or the search**, never while the maker looks at the tile **(Proposal)**.
15. **Customizable** filter (a toggle) means what Frage 371 decides; as proposed (A): an Iconify icon from a monochrome set, which takes the colour the maker gives the object (fill and stroke), and a catalogue template that carries the tag `customizable` in its index; nothing else is called customizable. The chip's tooltip says this in one sentence.
16. Given the filters change, then the grid scrolls to the top; a filter whose data is absent in all fetched items (for example Category with only local documents) is shown disabled with the reason as its description (Colour and Customizable), or, for Category, replaced by a muted state line (UX G5), never hidden **(Proposal)**.
17. Given the **Sort** control, then it offers, as a 2 x 2 choice strip (UX G12), **Best match** (only with search text; default then), **Name A to Z** (default without text; case-insensitive, digits as numbers), **Source, then name**, and **Newest** (local pages by their document's modified time first, everything else after, by name) **(Proposal)**. Best match is disabled while the field is empty and chosen when text starts; emptying the field brings the last own sort back (UX U8). A change re-orders within 100 ms; the order is stable (equal items keep their order).

### Part E: fuzzy search

18. Given the search field (one line, at most 100 characters, the same cut as `0138` criterion 10), then typing filters the fetched items live and **sends nothing**; results are ranked and the grid shows them in Best match order unless the maker chose another sort. An empty field shows everything the filters allow.
19. Given the typed words, then a word matches an item when it matches one of the item's **fields**: name (heaviest), aliases (Iconify), tags (templates), category, then source or set name and the page name for local pages (lightest). The words are separated by spaces and **all** must match (AND, each in any field). Matching ignores case and diacritics ("ae" does not equal "ä"; "a" matches "ä"), splits names at `-`, `_`, `.` and spaces ("arrow-left-circle" has the words arrow, left, circle).
20. Given one search word, then it matches a field word, in this order of strength, by: exact; prefix; substring; one typo (an insertion, a deletion, a substitution or a swap of two neighbouring letters: allowed edits 0 for words under 3 letters; for words of exactly 3 letters one swap of two neighbouring letters and no other edit (so "bxo" finds "box", ADR 0086); 1 for 4 to 7 letters; 2 for 8 and more); and a subsequence of at least 3 letters whose letters stay in order ("hrt" matches "heart"). The tests: "hart" and "haert" find "heart"; "bx" finds nothing, "box" and "bxo" find "box"; "wether" finds "weather"; "cir lef" finds "arrow-left-circle"; "kreis" does not find "circle" (no translation, Out of scope). A match's score is the field weight times the strength; the item's score is the sum over the search words; items with equal scores keep the sort of criterion 17. The matcher is a pure function in `curvyo-library-core` (no I/O, builds for wasm32, no new crate and, per ADR 0086, no dependency: a dictionary of distinct folded words built when items arrive) **(Proposal)**.
21. Given Enter in the search field, then, exactly as `0138` criterion 10 (Frage 340 B), the words are also sent to every Iconify source the maker added (never to a Git catalogue, never anywhere else), once per Enter; the results join the grid as source "Search results" (criterion 12) and are then filtered, sorted and drawn like any item, and there is **no hint line under the field** (customer rule of 2026-10-16, `0138` UX notes section 7.1; UX G6): the Search online **Globe** button inside the field (`0138`, present while the field holds text and an Iconify source is added; name and tooltip "Search Iconify sources (Enter)") says it, and the Enter outcome ("home": 48 icons, no icons, Offline or the server's error) is the count line, the subject line of criterion 4 (UX U6). Nothing is sent while typing or on any other action.
22. Given up to 50,000 fetched items (the budget of ADR 0086 decision 9), then a keystroke re-filters and re-ranks within 100 ms of thread CPU (the timing rule of `CLAUDE.md` §5, measured as in `curvyo-editor-wasm/tests/cpu_time/`); the matcher does not allocate per item on the hot path beyond the result list **(Proposal)**.

### Part F: keyboard, state, scope

23. Given the keyboard, then the grid is one Tab stop after the divider, the search field, the filters, Sort and Size; arrows move between tiles (Up and Down by a row), Home, End, Page Up, Page Down; Enter loads (criterion 7; a multi-page item opens its pages), Space selects; Down or Enter in the search field focuses the first tile (Enter also searches online, UX U18); a printable character goes to the search field; Escape goes back from the second level first, then closes (criterion 2, also from the search field, which keeps its text). A tile's accessible name is "Arrow left, Material Design Icons, MIT, monochrome" (parts only when known), its description the tooltip of criterion 5.
24. Given the browser build, then criteria 1 to 23 hold with the sources of that build (`0137` criterion 1: its own stored documents, the Online section as `0138` provides it).
25. Given the format, then nothing changes: no `format_version` change, no new file; the browser reads what the Library tab holds. Parsers gain the fields of "What data exists" (criteria 13 and 14) with golden tests in `tests/fixtures/`.

### Part G: the rework after the customer test (UX deltas H1 to H14; all **(Proposal)**, vetoable at the demo)

26. Given the browser, then its header's first control is a radio strip **Items | Sets** (H1). **Items** holds the tiles of everything fetched and no cards; **Sets** holds one card per set or source, loaded or not, grouped by source (the Curvyo catalogue, each Git catalogue, each Iconify source, the local workspaces and folders). The view is kept per user (key `curvyo.library-browser`, H8). Each view keeps its own facet values and search text; the Sets view's facets are Sources, Colour (the set's palette flag) and Category (the set category) (H10).
27. Given a set, catalogue or folder card, then a press selects it; opening it (double-click or Enter, criterion 28) switches to the Items view narrowed to it alone: Sources = that set, search empty, Category none, Colour All, Customizable off, sort Name (H3). Empty states: Items "No items yet." with a Show sets button; Sets 'No set matches "hrat".' with Clear all (H14). In the Sets view the search filters the cards by name, category and author, and Enter focuses the first card; Enter searches online only in the Items view (H9, amends criterion 21).
28. Given a card, then there is no Load list, Load set or Add button; a double-click or Enter opens it, which is the one request (fetching a list or loading a set, as criterion 9 and `0138` criterion 2 allow: the maker's action is the request) or, for the ready Curvyo catalogue entry, adds it (H2).
29. Given a tile, then there is no Load or Import page button: a double-click, Enter or a drag loads; Space selects; a template with several pages loads all its pages on a double-click; a local document with several pages opens its pages; the detail bar keeps only **Show pages** and **Pick elements…**; a press on the page-count chip shows the pages (H4).
30. Given the maker moves through places, then the browser keeps a **history of places** for the session only (view, level, search, facets, sort, current item, scroll anchor; at most 50 entries), pushed by a view switch, opening a set, card or item, Show pages, Pick elements… and the hosting guide; **Back** at the start of the subject line restores the place exactly and returns the focus to what was opened; there is no Forward. Back is also Alt+Left (Cmd+[ on a Mac), the mouse back button and the Back buttons; Backspace does not go back; Escape goes Back while the newest entry was left by opening something, else closes (H5, H6).
31. Given the Sources and Category lists (and their workspace and folder rows), then a plain press **replaces** the selection with that row, a press on the only selected row clears it, and **Shift+press adds or removes** a row; Space and Shift+Space do the same from the keyboard; the lists are multi-select listboxes (H7).
32. Given the Add source form (kind Git catalogue) and the end of the Sets view, then **Host your own library…** opens an in-app guide, a place with Back (criterion 30): the folder layout, the steps, an example `curvyo-catalogue.json` with a Copy button, the field rules and limits of ADR 0085 decision 3, and a link to `github.com/curvyo/templates`; it ships with the app and sends no request; its final text is the product owner's (H11). *Note, not a criterion (H12):* the guide may name the index script of ADR 0085 decision 3 only once that script exists in `curvyo/templates` (today the repository has none); until then the guide describes the hand-written index only.
33. Given a Git catalogue in a private repository, then the Add source form offers an optional Access token (criteria 1 to 13 of `0156`); without a token, a private repository is refused at Add with 'No Git catalogue at this address. For a private repository, add an access token.' (404, as GitHub answers) or 'The server refused the request. For a private repository, add an access token.' (401 or 403); the tree's failure line stays without the hint. The line "The repository must be public." is removed from the form and from the guide (changed by `0156`).

## Out of scope

- Search by synonyms or related words ("kreis" finds "circle", "bin" finds "trash"): Frage 375 (LIB-SYNONYMS), default A, **its own follow-up story after #483** (use the keywords icon projects publish, Lucide ISC, Tabler MIT, MDI Apache-2.0, plus a related-words table built from them, shipped with the app, about 300 KiB). The architect's research: Iconify's server search matches name parts only and has no per-icon tags; an on-device AI model is rejected.
- Access tokens for private Git catalogues: Frage 376 (LIB-TOKEN) answered A (2026-11-26), its own story `0156-library-git-token` with an ADR; #483 ships the public-only behaviour until then.

- A translation or synonym dictionary ("kreis" finds "circle"); stemming; search inside object contents or text (`0137` Out of scope).
- Editing, tagging, favouriting, collecting items or custom categories for items (a library of favourites is a later story; the font preview's favourites are not shared with it).
- A licence filter; hiding items by licence (the chip shows the licence on every tile).
- Server-side filtering beyond Enter (no filter parameter is sent), loading every set in the background, a disk cache (ADR 0085).
- Larger previews or a detail view of one icon (zoom, path preview, copy as SVG), a recolour preview of icons.
- New sources (svgrepo.com and the other sites of `0138` stay unconnected), `0138`'s offering of templates.
- Dragging from the pane onto the drawing in full view (the drawing is hidden; a press loads centred).
- Changes to the Library tab itself beyond the Browse button (its tree and rows stay).

## Open questions (customer)

Frage 375 (LIB-SYNONYMS): How should the search find icons by related words, not only by their names?
- A: Use the keywords icon projects publish (Lucide ISC, Tabler MIT, MDI Apache-2.0) and a related-words table built from them, shipped with the app (about 300 KiB). Recommended; its own follow-up story after #483.
- B: Names only, as now.
- C: An on-device AI model (rejected by the architect's research: size, speed, licence).
- Recommendation: A. Default if unanswered: A, as the follow-up story; nothing changes in #483.

Frage 376 (LIB-TOKEN) - answered A (2026-11-26), see `0156`: Should a Git catalogue in a private repository be reachable with an access token?
- A: A token per Git catalogue in the operating system's key store (desktop only; needs an ADR).
- B: No tokens now: public repositories only, plus the hosting guide.
- C: A token in plain text in `online-sources.toml`.
- Recommendation: A, as its own story after Frage 375. Default if unanswered: B now in #483.
- **Answered A (2026-11-26), see `0156`.**

Frage 371 (LIB-CUSTOMIZABLE): What does "customizable" mean in the filter?
- A: What the data says today: an icon from a monochrome set (it takes the colour of the object) and a catalogue template tagged `customizable`. No format or catalogue change.
- B: As A, plus two optional fields in the catalogue index (`colour: mono | colour`, `customizable: true`), so authors of `curvyo/templates` state it explicitly and the Colour filter works for templates too; local documents stay without. Additive to `catalogue_format` 1.
- C: No Customizable filter now; Colour (monochrome vs. colour) only.
- Recommendation: A now, B when `curvyo/templates` has content (it is empty, so nothing is lost by waiting). Default if unanswered: A.

Frage 372 (LIB-SCOPE): Which sources does the first browser cover?
- A: All three in one grid with a Source filter: your own documents (pages), the Curvyo and Git catalogues, and Iconify.
- B: Online only (catalogues and Iconify); your own documents stay in the side panel until a later slice.
- C: Iconify and catalogues first, your own documents as a second slice of this story, same pane.
- Recommendation: A (one place to look; the local data is already there and costs no request). Default if unanswered: A.

Frage 373 (LIB-FETCH): How do Iconify icons get into the browser, given that search and categories work on what is fetched and Iconify has 280 sets?
- A: Per set, on your press: each set appears as a card with its samples, "Load set" fetches its icon names (one request, no bodies), then search, category and colour work on it; bodies only for what you see. Enter still searches all added Iconify sources.
- B: As A, plus one "Load all sets" button (280 requests of icon-name lists, about 25 MB; once per press, never automatic), so the fuzzy search covers every set.
- C: No set loading: only the set list (`/collections`: name, category, palette) and Enter search results.
- Recommendation: A. Default if unanswered: A.

Frage 374 (LIB-DETAIL): Should the browser get a detail bar for the selected item?
- A: Yes, like the glyph table's: the hovered or selected item at 96 px with name, place, licence link, author and facts, and its Load button. A press selects, a double-click or Enter loads. The Out of scope "detail view of one icon" means zoom, path preview, copy as SVG; those stay out.
- B: No: a press on a tile loads it at once; licence and author only on the tile line and in the tooltip; tiles keep two text lines at every size (never narrower than 112 px).
- C: As A, but only in full view; the split view behaves like B except that a press selects.
- Recommendation: A (licence and credit are readable before loading, small sizes stay dense, nothing loads by a stray click). Default if unanswered: A.

## UX notes

See `ux-notes.md` (ux-engineer, 2026-11-25), written for Fragen 371 A, 372 A, 373 A and 374 A (section 13 says what differs for B and C); its deltas G1 to G13 and, after the customer test, H1 to H14 (Part G) are taken into the criteria, its proposals U1 to U30 and U31 to U48 are **(Proposal)**: the Browse button, the one side pane with a wide layout (facet column from 640 px) and a narrow one (Filters band), the header, filters and sort as strips and check lists (no dropdowns), the tile grid with the Size slider (48 to 256, default 96), the Not loaded cards with Load list and Load set, the detail bar, selecting, loading and dragging, the second level, states, keys; proposals U1 to U30 and Frage 374 (LIB-DETAIL, the detail bar; default A). Drafts: `docs/design-drafts/2026-11-library-browser/`; design system section "Library browser".

## Architecture

Decided in `adrs.md` and ADR 0086 (Proposed, not `needs-customer`), taken into the criteria: parser fields and fixtures, the matcher in `curvyo-library-core::search` (no dependency, no new crate), the item model and filters in `curvyo-ui-core::library_browser`, one item list in the page for both builds, one side-pane store with four kinds, bodies as `<img>` one screen ahead (amends ADR 0085 decision 2), budget 50,000 items, no `format_version` change.

## Links

Requirements: R-ASSET-001 to -003, R-ASSET-004, R-ASSET-007, R-SYS-007 (new requirement row proposed: R-ASSET-009 Library browser)
ADRs: ADR 0084, ADR 0085, ADR 0086 (Proposed), `adrs.md`
Depends on: `0137`, `0138`, `0077`
Related: `0075` (tag strip, search, favourites of fonts), `0141` (parametric generators, a later source of "customizable" templates)
PR: none yet
