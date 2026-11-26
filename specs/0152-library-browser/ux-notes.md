# 0152 Library browser: UX notes

ux-engineer, 2026-11-25. Written from `specification.md` (Draft, criteria 1 to 25) for Fragen 371
A, 372 A and 373 A, their defaults; section 13 says what differs for B and C. Checked against
`docs/design-system.md` ("Font preview" with its Glyphs-mode detail bar, "Library tab",
"Interaction conventions": "No popups in the properties panel", "Bar overflow: expandable second
row, never a dropdown icon", "Choice strip", "Hidden, not disabled", "Armed action", "No notice
after a success", no operating tips), `specs/0137-document-library/ux-notes.md`,
`specs/0138-template-library/ux-notes.md`, and the code on `story/library-browser` (`88a42156`):
`CanvasArea.tsx`, `SplitDivider.tsx`, `fontPreview/PreviewHeader.tsx`, `PreviewTile.tsx`,
`SizeSlider.tsx`, `GlyphDetailBar.tsx`, `panel/LibraryTab.tsx`. **(Proposal)** items are vetoable at
the demo; section 14 lists them as U1 to U48 (U, so they never collide with P of `0137` or Q of
`0138`), section 15 the spec deltas G1 to G13 and H1 to H14 (the rework). Draft: `docs/design-drafts/2026-11-library-browser/`.
"Muted" is `--panel-muted-fg`. No new colour token.

**Rework after the customer test of PR #483 (2026-11-25).** The customer asked for five changes
(translated): two views, one with the icons and one with the sets; a way back to the old view; a
press on a category replaces, Shift+click adds; no Load button, double-click or drag; how to host
your own library, possibly in a private Git repository with a token. Sections 0.1 to 0.7 are the
new design; the sections after them are amended in place where they disagree (each amended place
names its 0.x section). New proposals are U31 to U48 (section 14), the spec deltas for the product
owner H1 to H14 (section 15.1), the customer question Frage 376 (LIB-TOKEN, section 16; answered A, built in `0157`). The
search UI is unchanged except where the views and the history need it (the architect looks at
search quality separately).

## 0. Rework (customer test, 2026-11-25)

### 0.1 Two views: Items and Sets

- **View switch** (U31): the header's first control in both layouts, a radio strip **Items** \|
  **Sets** in the font preview's mode-switch look (28 px, 1 px `--toolbar-icon` at 60 %,
  `rounded-[5px]`, cells 1 px apart, 14 px, 6 px side padding, about 104 px; `role="radiogroup"`
  "Library view", one Tab stop on the pressed cell, Left and Right move and choose, no wrap). A
  mouse press keeps the focus in the pane. The view is kept with the view state
  (`curvyo.library-browser` field `view`, default Items).
- **Why "Items", not "Icons"** (U32): the view also holds templates and the maker's own pages; a
  cell named "Icons" would hide them. If the customer prefers "Icons", it is a one-word change.
- **Items view** holds the tiles of everything fetched: local pages and documents, templates of
  listed catalogues, the icons of **loaded** sets and the Enter search results, in the chosen sort,
  with the facets of section 5. **No cards**: the Not loaded group of section 7 leaves this view.
  A set that is loaded mixes its icons into the sort like everything else; nothing is put on top
  (the customer's "they are always on top"). The Size slider belongs to this view.
- **Sets view** holds one **card per set**, loaded or not (section 0.2). No tiles, no Size slider
  (not drawn, as the font preview's Info mode: the subject line takes the room).
- **From a set to its items** (U33): a double-click or Enter on a set card **opens** it: a set not
  loaded is loaded first (the card shows its spinner, then the view changes), then the Items view
  shows that set alone as a new place (section 0.3): **Sources** = only that set, search empty,
  Category none, Colour All, Customizable off, sort Name. The facet column shows the one checked
  Sources row, the narrow header "Filters 1", the subject line "Lucide · 1,540 items". The way
  back is Back (section 0.3), which returns to the Sets view as it was.
- **Empty Items view** (nothing fetched but local pages, or nothing at all): the empty line
  "No items yet." 14 px muted, 12 px below it **Show sets** (outline text button 28 px), which
  presses the Sets cell (U34). Browser build without online sources: "Nothing in the library
  yet." as section 10.

### 0.2 The Sets view

| Part | Content |
|---|---|
| Groups | One group per source, sticky headers (the Glyphs block header look, 28 px, 12 px semibold muted): **My documents** · "3 folders"; **Curvyo catalogue**; each Git catalogue by its name; each Iconify source by its host · "280 sets · 4 loaded". Order: My documents, Curvyo catalogue, Git catalogues A to Z, Iconify sources A to Z. Inside a group the cards are A to Z (Best match while searching); loaded and not loaded are mixed, never loaded first (U35) |
| Cards | The 208 x 112 card of section 7.1, with the button slot at the bottom right turned into a **state slot** (U36): nothing (not loaded), `Check` 14 px and "Loaded" 12 px muted, `LoaderCircle` 14 px while loading, or nothing with the failure reason on row 3 (`WifiOff` "Offline", `CircleAlert` and the reason). No Load set, no Load list (section 0.4) |
| Card kinds | Iconify set (samples, name, "2,500 icons · Material", licence chip and author); unfetched Iconify source (`Globe`, host; opening fetches its set list and its set cards replace it **in place**, the view does not change); Git or Curvyo catalogue (`GitBranch`, name, "12 templates" or the host while unfetched; opening fetches the list if needed and opens the Items view on it); a workspace or folder (`Folder`, name, "14 documents · 38 pages"; opens the Items view on it); the Curvyo **ready entry** (`Globe` at 55 %, name muted, "Not added"; opening adds it and fetches its list, the one write being `online-sources.toml` as Add in the tab) |
| Press | Selects the card (current, detail bar); nothing is fetched (U17 kept for cards) |
| Facets | Sources (the sources, not the sets: My documents, Curvyo catalogue, each Git catalogue, each Iconify source; hides groups), Colour (a set's palette flag; local folders and catalogues have none and follow the section 5.2 rule), Category (the set categories: "Material", "Logos", "Emoji"…; folders and catalogues are "Uncategorised"). No Sort, no Customizable in this view (U37): a set's order is its group and its name, and Colour says the same as Customizable for a set. Each view keeps its own facet values |
| Search | Filters the cards by name, category and author (the fuzzy matcher, locally); **Enter** in the Sets view does not search online, it focuses the first card; the `Globe` button is not drawn (U38). Search results are icons and belong to the Items view |
| Subject line | "196 sets · 4 loaded"; filtered "12 of 196 sets" |
| End of the view | After the last group: one row, 16 px above, **Host your own library…** (text button 12 px `--editor-accent`, 28 px target) (section 0.5) |
| Detail bar | The hovered, else the current card: the box shows the three samples at 32 px (a folder or catalogue its glyph at 32 px); name; place "Iconify › api.iconify.design"; licence link · author; facts "2,500 icons · Material · Monochrome · Loaded" or "· Not loaded". No buttons |
| Keys | The cards are one `listbox` "Sets", groups named by their header, arrows as the grid of section 9.2, Enter opens, Space selects |

### 0.3 Places and Back (customer: "you can't get back to your old view")

- **A place** is what the pane shows, with everything that makes it: the view, the level (top,
  the pages of one item, the Page preview of Pick elements…, the hosting guide), the search text,
  every facet value, the sort, the current item or card, and the scroll position as the identity of
  the first visible tile or card (not pixels, so a re-flow by Size or width keeps it) (U39).
- **History** (U40): a stack of the places the maker left, newest on top, at most 50, for the
  session (it survives closing and reopening the pane; it is not stored). An entry is pushed by
  every **change of place**: a press on the other view cell, opening a set, card or multi-page
  item, Show pages, Pick elements…, Host your own library…. Typing, a facet, the sort, Size,
  selecting and scrolling change the place itself, not the history. There is no Forward.
- **Back** (U41): `ArrowLeft` 24 x 24 at the start of the subject line (where the second level had
  it), drawn while the history has an entry, name and tooltip "Back to Sets", "Back to all items",
  "Back to Lucide", 'Back to "arrow"' (the place's short name: a view's name, the one set or item
  it is narrowed to, else its search text), tooltip line 2 "Alt+Left". The Page preview of Pick
  elements… keeps its header **Back** (section 9.3), the same history.
- **Back restores the place exactly**: view, level, search text, facets, sort, scroll anchor,
  current item; the focus goes to the card or tile that was opened from it (else the current one,
  else the grid). Values that name data no longer fetched are dropped (a Search results row after a
  new Enter keeps naming the new results). Nothing is re-fetched; nothing is pushed.
- **The view strip and the history**: each view shows its **last** place when its cell is pressed
  (the Items view after opening Lucide is "Lucide" until the maker changes it); the older Items
  place ('"arrow"') is reached by Back. Example: Items '"arrow"' → Sets → open Lucide → Items
  "Lucide"; Back → Sets as it was; Back → Items '"arrow"' with its filters and scroll.
- **Keys** (U42): Alt+Left (Cmd+\[ on a Mac) anywhere in the pane and in the Page preview of Pick
  elements…, and the mouse's back button over the pane, go Back. Backspace no longer goes back
  (H6): in the grid it goes to the search field like a printable key, which is what it did at the
  top level.
- **Escape ladder** (U43): while the newest history entry was left by **opening** (a set or card,
  a multi-page item, Show pages, Pick elements…, Host your own library…), Escape goes Back, one
  level per press; otherwise Escape closes the pane (also from the search field, which keeps its
  text). A view switch is never undone by Escape, so Escape after a few switches closes at once.
  Outside the pane the canvas's own Escape steps come first.
- **Close and reopen**: the pane reopens at its current place (criterion 3 keeps the view and the
  per-user values across starts; the history and the session-only selections do not survive a
  start).

### 0.4 Selecting in the facet lists (customer: "only with Shift+click, otherwise replace")

- **Every multi-select facet** (Sources with its workspace and folder rows, Category; in the Sets
  view the same two) follows one rule (U44): a **press** on a row selects **only** that row (replaces
  the selection); a press on the row that is the only selected one clears the section (none = all);
  **Shift+press** adds the row to the selection or takes it out. A press on My documents selects it
  with all its workspace and folder rows; Shift+press on a child adds or removes that child only.
- **Role**: the section is a `listbox` `aria-multiselectable="true"` named "Sources" or "Category"
  with `option` rows and `aria-selected` (no longer checkboxes, since a press does not toggle).
  The `Check` glyph and the `--value-fill` ground stay the selected look.
- **Keys**: Up, Down, Home, End move the focus only (nothing narrows while moving); **Space**
  selects only the focused row (a second Space on it clears, as the press); **Shift+Space** adds or
  removes it. Ctrl+A in the list selects every row (= all, the same as none).
- The single-choice strips (Sort, Colour, the view switch) and the Customizable switch are
  unchanged.

### 0.5 Loading without a Load button (customer: "double-click or drag and drop")

- **Gone** (U45): **Load** and **Import page** in the detail bar, **Load set** and **Load list** on
  the cards; tiles never had one. The ready entry's **Add** goes too (opening adds it, 0.2).
- **Loading an item**: **double-click** or **Enter** loads, exactly as before (an icon at 24 mm,
  "Import icon"; a template, all its pages, "Load template"; a local page "Import page"), and a
  **drag** onto the drawing loads at the drop point (split view only). A **multi-page** template
  loads all its pages on double-click (U46: one gesture, one meaning: a double-click loads
  wherever there is something to load); a multi-page **local document** has nothing to load as a
  whole, so its double-click opens its pages. Space selects (it no longer loads; H4).
- **Opening a set or card**: double-click or Enter (0.2); this is now the deliberate press that
  sends the one request, in place of Load set and Load list.
- **What the detail bar still shows**: the preview box, name, place, licence link · author, facts
  line, and only the two buttons that do not load: **Show pages** (a document or template with
  several pages; the only way to the pages of a template) and **Pick elements…** (a local page;
  kept, U47: there is no gesture for it, and picking a part of a page is not a load). The button
  column is reserved in both layouts so the text does not jump on hover: 112 px wide with text
  buttons in the wide layout; in the narrow one a 28 px column of icon buttons, `Files` "Show
  pages" and `MousePointerClick` "Pick elements…" (name and tooltip). A press on the page-count
  chip of a tile is the same as Show pages (24 x 24 target around the 16 px chip).
- **The keyboard route to the pages** is Show pages in the Tab order of the detail bar; no new key.

### 0.6 Host your own library (customer: "info on how to host your own library")

- **Where the link is** (U48): **Host your own library…** (text button 12 px `--editor-accent`,
  28 px target) in two places: the Add source form of the Library tab, under the "What is sent"
  line while the kind is **Git catalogue** (the form's only link; it is a statement of where the
  format is explained, not an operating tip), and the end of the Sets view (0.2). Desktop build
  only, like Add source.
- **What it opens**: the **hosting guide** as a place of the browser (it opens the browser if it
  is closed, a history entry, Back and Escape return): the grid area becomes one scroll region
  (`role="region"` "Host your own library", focusable, 16 px padding, text at most 640 px wide in
  full view; the font preview's Info mode look). The header keeps the view strip (neither cell
  pressed), Full view and Close; no search, no Size, no facets; the subject line "Host your own
  library". The text ships with the app (no request).
- **Content** (the product owner writes the final text; the outline, H11):
  1. "A library is a Git repository with one file `curvyo-catalogue.json` at its top and one folder
     per template."
  2. **Folder of a template**: `template.curvyo` (the document), `thumbnail.png` (at most 1 MiB),
     `template.toml` (your source for the index; Curvyo does not read it).
  3. **Steps**: create a repository (for example on GitHub, public); add one folder per template;
     run the index script of the example repository, which writes `curvyo-catalogue.json` with the
     hashes; push; in Curvyo, Library tab, Add source…, Git catalogue, the address of the raw files
     (`https://raw.githubusercontent.com/<owner>/<repo>/main/`).
  4. **Example**: a two-template `curvyo-catalogue.json` as a code block with **Copy** (outline
     text button 24 px at the block's top right) and a link "Example repository" to
     `github.com/curvyo/templates` (opens in the system browser, the `0138` link cell).
  5. **Rules**: the fields of ADR 0085 decision 3 in one table (name, author, tags, licence,
     credit, fonts, size, hashes, `format_version`); a CC BY licence needs a credit; the limits
     (index 4 MiB and 10,000 templates, picture 1 MiB, template 64 MiB); `customizable` as a tag
     (Frage 371 A).
  6. **Private repositories**: Frage 376 answered A (2026-11-26): the steps for a read-only token,
     specified in `specs/0157-library-git-token/ux-notes.md` section 7. Until `0157` is built, #483
     ships one line "The repository must be public."
- **No separate help window, no web page as the only copy**: the guide must match the Curvyo that
  shows it (its `catalogue_format`), so it ships with it (U48).

### 0.7 Access token for a private repository (Frage 376 A, built in `0157`)

Frage 376 was answered A on 2026-11-26 and is its own story: the design is
`specs/0157-library-git-token/ux-notes.md` (Access token row of the Add source form, `Lock` mark on
the tab row and the Sets view group header, the `KeyRound` action **Change token…** / **Add token…**
with its inline form and an armed **Remove token**, the refusals, the key-store-unavailable line,
the failure lines "The token was refused." and "The saved token could not be read from the key
store."). That file replaces the sketch that stood here; ADR 0090 decides storage and sending.

## 1. Principle

The Library tab stays the place to manage sources; the browser is the place to **look**. It is the
font preview turned to the library: the same side pane, the same Size slider and tiles on the
canvas ground, a detail bar like the Glyphs mode's, and filters that are always visible strips and
lists, never popups. Everything in it is already fetched or local; the only things that send a
request are opening a card that is not loaded (double-click or Enter, section 0.5; before the
rework the Load list and Load set buttons) and Enter in the search field of the Items view.


## 2. Opening, split and full (criteria 1 to 3)

- **Browse button** (U1): the Library tab's search row becomes search field, 4, **Browse** 28 x 28
  (`LayoutGrid` 16 px at 1.5 px, toggle, `aria-pressed`, the pressed look of the Fonts tab's
  Preview button, tooltip and name "Library browser"). The search field shrinks to 212 px at the
  244 px column (224 at 256); its Globe and Clear buttons keep their places. Same glyph as Font
  preview on purpose: the button opens the big grid of the list beside it. Both builds.
- **The one side pane** of `CanvasArea` (`0077` rows unchanged): `role="region"` "Library
  browser", split right of the drawing at 35 % or one tile column, the `SplitDivider` ("Resize
  library browser"), Full view / Split view, Close ("Close library browser"), forced full under a
  360 px drawing, instant open and close, the drawing's view origin kept, the pane over the canvas.
  Divider name and tooltip follow the pane.
- **One pane at a time** (criterion 2): Font preview, Page preview (`0137`) and Library browser
  replace each other. The pressed state of Preview (Fonts tab) and Browse (Library tab) follows
  the pane that is open. A press on a page row in the tab opens the Page preview in place of the
  browser (the tab's rule); see section 9.3 for the way back.
- **Stays open** on a tab change and with the panel collapsed (`0077` P6, `0137` 6.1).
- **Opened by keyboard** (Enter or Space on Browse): focus to the search field. Closed with focus
  inside: to Browse if the Library tab is shown, else the canvas.
- **Kept per user** (criterion 3; key `curvyo.library-browser`, guarded storage like
  `curvyo.font-preview`): the view (Items or Sets, 0.1), full or split, the dragged share (its own, not the font preview's, U2),
  Size, sort, Colour, Customizable, the filter band's open state and the search text. **Source and
  Category selections live for the session only** (G7, U3): their values name fetched data, which
  the next start does not have, and a kept "Material Symbols" that is not loaded would hide
  everything.

## 3. Layout

Two layouts by the pane's width (U4). The split pane is narrow in practice (35 % of a 968 px
canvas area is 339 px); full view and wide drags get the wide one.

### 3.1 Wide (pane at least 640 px)

| Part | Size | Content |
|---|---|---|
| Header | 28 + 16 | one row (section 4): view switch (0.1), 12, search field 200 to 480, 16, Size (Items view only), 16, subject line (flex, Back at its start, 0.3), 8, Full view, 4, Close |
| Facet column | 216 wide, left, under the header | the filters and sort (section 5), `--panel-bg`, 1 px `--toolbar-icon` at 25 % right line, 12 px padding, its own scroll (thin, `overscroll-behavior: contain`) |
| Grid | the rest, `--canvas-bg` | section 6 |
| Detail bar | 113 high, full pane width, at the bottom | section 8 |

### 3.2 Narrow (under 640 px)

| Part | Size | Content |
|---|---|---|
| Header row 1 | 28 | view switch (0.1, about 104), 8, search field (flex; 148 px in a 340 px pane), 4, Full view, 4, Close |
| Header row 2 | 28, 8 above | **Filters** toggle (section 5.1), 12, Size (slider 96; Items view only) |
| Header row 3 | 24, 8 above | Back and the subject line on a row of their own, so the subject is not cut in a split of about 340 px (lead's decision after the UX re-check, Proposal); the wide layout keeps them in its one header row |
| Filter band | 0, or up to 50 % of the pane's height | the facet sections of section 5 at full width, `--panel-bg`, 1 px bottom line, own scroll; shown while Filters is pressed |
| Grid | the rest | section 6 |
| Detail bar | 89 high | section 8, compact |

The header is `--panel-bg` with 8 px padding and a 1 px bottom line (the font preview's frame).
At 800 x 600 in a 339 px split: header 80, detail bar 89, grid about 370 px, three rows of 96 px
tiles. Nothing pops over anything: the band pushes the grid down, and the grid keeps its first
visible tile when the band opens or closes.

## 4. Header (criteria 4, 18, 21)

- **View switch and Back**: sections 0.1 and 0.3.
- **Search field**: the Library tab's field look (`Search` 16 px inside left, white, 14 px,
  placeholder "Search library", at most 100 characters, "Clear search" `X`), its own text (not
  the tab's, U5: the tab filters by substring, the browser by fuzzy match; one text with two
  results would read as a bug).
- **Enter searches online** (criterion 21, Frage 340 B; Items view only, 0.2): Enter also sends the words to every
  added Iconify source. The pointer route and the statement of what goes where is the `0138`
  **Search online** button inside the field (`Globe` 16 px, 24 x 24, left of Clear, present while
  the field holds text and an Iconify source is added, name and tooltip "Search Iconify sources
  (Enter)", not a Tab stop). **No hint line under the field** (G6): customer rule of 2026-10-16
  and `0138` section 7.1 ("no line under the field").
- **Enter status** joins the subject line after " · " (U6): 'Searching "home"…' (after 150 ms),
  then '"home": 48 icons', '"home": no icons', '"home": Offline' or '"home": The server answered
  500.' (several Iconify sources: the sum, and a failed one by its name). It stays until the
  search text is cleared or the next Enter. Announced politely as in `0138` 7.2.
- **Size**: "Size" 12 px muted, the font preview's `SizeSlider` (120 wide, 96 in the narrow
  header), readout "96 px", range 48 to 256, default 96; Arrows 8, Shift+Arrows 32, Home 48, End
  256, Ctrl+Backspace 96, Enter or F2 or double-click to type ("Enter a size from 48 to 256
  px."); tooltip "Size: 96 px". Re-flow in CSS, the grid re-laid only when the column count
  changes (`0077`).
- **Subject line** (12 px muted, ellipsis, `role="status"` throttled to 2 s): "4,210 items";
  filtered "132 of 4,210 items"; while the local index is read "Reading 120 of 500"; at the second
  level "Box job › Lid · 3 pages" (section 9.4); one opened set "Lucide · 1,540 items"; in the
  Sets view "196 sets · 4 loaded" (0.2).
- **Full view / Split view, Close**: the `0077` 28 x 28 buttons.

## 5. Filters and sort (criteria 11 to 17)

### 5.1 Where they are (G2, U7)

Strips, switches and check lists in a facet column, **no dropdowns, no popovers**: the customer's
rules forbid popups in the panel and dropdowns in the bars, and a pane beside them follows the
same calm. In the wide layout the column is always there. In the narrow one the header's
**Filters** button (outline text button 28 px: `SlidersHorizontal` 16 px, "Filters" 14 px, and
while any filter is active the count 12 px muted tabular, "Filters 2"; `aria-expanded`,
`aria-controls` the band; pressed look while open) shows the same sections as a band above the
grid. Same sections, same order, same keys in both.

### 5.2 Sections, top to bottom

Section heading: 24 px, 12 px semibold muted (the `0137` group heading look), 16 px between
sections. The column's first row is "Filters" 14 px with **Clear all** at its right.

| Section | Control | Notes |
|---|---|---|
| Header row | "Filters" 14 px, **Clear all** text button 12 px `--editor-accent`, 24 px high, at the right | Clear all is present while a filter or the search is active, its slot reserved (G4: it clears filters and search text, so its name says "all"). Not there: nothing moves |
| Sort | Choice strip, 2 x 2 fixed cells: **Best match** \| **Name** / **Source** \| **Newest** (`role="radiogroup"` "Sort") | Best match is `aria-disabled` (40 %) while the field is empty, described "Only while searching". When the field goes from empty to text, Best match is chosen; when it is emptied again, the sort the maker chose last without text comes back (default Name, U8). Name is "A to Z", case-insensitive, digits as numbers (criterion 17); Source is "Source, then name"; Newest is local pages by their document's change time, the rest after by name. Cell tooltips carry the long names |
| Sources | Multi-select list (amended by 0.4: a press replaces, Shift+press adds or removes; `role="listbox"` `aria-multiselectable` "Sources", rows `role="option"`), rows 28 px: `Check` 14 px slot (shown when checked), name 14 px (ellipsis, tooltip whole), count 12 px muted tabular at the right. Checked: `--value-fill` ground. None checked = all | Rows (criterion 12): **My documents** (and, with more than one workspace or folder, one indented row per workspace or folder under it; checking the parent checks all), **Curvyo catalogue**, each added Git catalogue, each **loaded** Iconify set by its name, **Search results** (present after an Enter). Only fetched sources are rows; an unfetched one is a card in the Sets view (0.2). A row whose count is 0 stays, muted, still checkable (rows never jump, U9) |
| Colour | Choice strip **All** \| **Monochrome** \| **Colour** (`role="radiogroup"` "Colour") | Monochrome and Colour are `aria-disabled` while no fetched item carries colour data (only local pages and templates fetched), described "No colour data in the fetched items" (G5). With Monochrome or Colour chosen, items without colour data are not shown (criterion 14); the strip's tooltip says it: "Templates and your own pages have no colour data" |
| Customizable | Row 28 px: "Customizable" 14 px, the `ui/switch` at the right (`role="switch"`) | Tooltip (Frage 371 A, criterion 15): "Icons that take the colour you give the object, and templates tagged customizable". Disabled like Colour when no fetched item qualifies |
| Category | Multi-select list like Sources (0.4), A to Z, "Uncategorised" last | Values per criterion 13 (Iconify set category or the set's own group, merged by name ignoring case; a template tag equal to another category; a local page's workspace or folder). Counts are the items that pass the **other** filters. With no category in the fetched items: one 12 px muted line "No categories in the fetched items." in place of the rows (a state, G5). Long lists stay whole; the column scrolls |

- **Narrowing** is AND across sections, OR inside one (two sets checked show both); live, within
  100 ms; the grid scrolls to the top on every change (criterion 16).
- **Keys**: each strip and each list is one Tab stop; in a list Up, Down, Home, End move the focus,
  Space selects only the row, Shift+Space adds or removes it (0.4); in a strip arrows move and
  choose. A mouse press on a filter keeps the focus in
  the pane (unlike the panel strips, the maker keeps filtering).

## 6. The grid and its tiles (criteria 4 to 6, 23)

### 6.1 Grid

`--canvas-bg`, 12 px padding, 8 px gaps; columns of exactly the tile width from the left, the rest
stays at the right (the Glyphs-mode rule; equal tiles, U10). `role="listbox"` "Library items",
one Tab stop, roving focus; virtualised by rows (in view plus one screen; 2,000+ items). The Items
view holds tiles only; the cards are the Sets view (0.1, 0.2).

### 6.2 Tile

Edge S = Size. Width S + 16. `--tile-bg` card, 1 px `--tile-edge`, 8 px radius, no shadow, 8 px
padding.

| Part | Size at 96 px and up | Under 96 px |
|---|---|---|
| Preview | S x S | S x S |
| Name | 16 px line, 6 px above: 12 px `--toolbar-icon`, one line, ellipsis | not drawn |
| Meta line | 16 px line, 2 px above: the **licence chip** first, 4, the source 11 px muted, ellipsis | not drawn |
| Height | S + 56 | S + 16 |

Under 96 px the tile is the preview alone (U11, G3): at 48 px a grid of 40 icons per row is what
an icon hunt needs, and the name, source and licence are in the detail bar for the hovered or
current tile (section 8) and in the tooltip. This holds with Frage 374 A; with B the tile keeps
both lines at every Size and is never narrower than 112 px.

- **Licence chip**: 16 px high, `rounded-[4px]`, 1 px `--toolbar-icon` at 40 %, 11 px
  `--toolbar-icon`, 4 px side padding, never shrinks: the short name as the Library tab writes it
  ("MIT", "Apache 2.0", "CC BY 4.0") or "Licence unknown". One neutral look for every licence: no
  colour says "free" (criterion 5, `0138` criterion 3). The maker's own pages and documents carry
  no chip (they have no licence; the meta line is the source alone).
- **Source** on the meta line: the set's name for icons, the catalogue's name for templates, the
  document's name for local pages (the workspace in the tooltip).
- **Tooltip** (400 ms, below): line 1 the name; line 2 muted "Material Symbols · Apache 2.0 ·
  Google"; local "Box job › Lid › Front · 210 × 297 mm"; template "Curvyo catalogue · CC BY 4.0 ·
  Jane Doe · 24 KB".
- **Previews** (criterion 6): an icon drawn locally from its body as inline SVG, monochrome
  (`currentColor`) in `--toolbar-icon` (11:1 on white), colour icons in their own colours, scaled
  by its viewBox to fit S with 12.5 % margin (24 px icon at S 32 looks like the tab's cell). A
  template's thumbnail or a local page's picture with `object-fit: contain`, never scaled above
  its stored size (224 x 160), centred, the page edge as stored. Bodies and pictures load for the
  visible tiles after 100 ms in view; until then a `--tile-skeleton` bar, 60 % of S wide and S / 4
  high, centred, never another icon (the font preview's bar).
- **No colour or mono marker on the tile** (U12): the drawing is the marker (ink vs. colours), a
  glyph on 2,000 tiles is noise, and templates and pages have no such data. The fact is in the
  accessible name, the tooltip line 2 ("… · monochrome") and the detail bar.
- **Page count chip** (criterion 8) for a document or template with several pages: bottom right
  inside the preview, 16 px, `--panel-bg` ground, 1 px `--tile-edge`, `Files` 12 px and "3"
  11 px tabular; name "3 pages".
- **States**: hover `--editor-accent-hover`; **current** `--value-fill` with the 3 px
  `--editor-accent` bar at the left, `aria-selected`; focus 2 px `--editor-accent` ring inside; a
  tile that **cannot be loaded** (criterion 7): `TriangleAlert` 16 px muted centred in the preview
  on the white ground, the meta line "Cannot be loaded" 11 px muted (licence chip kept), the reason
  in the tooltip and the detail bar ("Made by a newer Curvyo.", "Offline."); Load is not offered.
- **Accessible name** (criterion 23): "Arrow left, Material Symbols, Apache 2.0, monochrome"
  (parts only when known); description the tooltip's line 2.

## 7. Cards: what is not loaded yet (criteria 9, 10; Frage 373 A)

Amended by 0.2 and 0.5: cards live in the **Sets view** only, grouped by source, loaded sets
included; the button slot is a state slot, and opening a card (double-click or Enter) is the
request. The Not loaded group below no longer exists; its anatomy, loading and failure looks hold.

A card is not an item: it is a source or a set the maker has not loaded. Cards sit in the grid's
last group, **Not loaded** (header 28 px: "Not loaded · 196 sets" 12 px semibold muted, sticky),
in fixed columns of their own: **208 x 112 px**, 8 px gaps, independent of Size (U13, G9). Cards
are A to Z, unfetched sources first.

### 7.1 Card kinds

| Card | Anatomy (12 px padding, `--tile-bg`, 1 px `--tile-edge`, 8 px radius) | Button |
|---|---|---|
| Unfetched source (Curvyo catalogue added, a Git catalogue, an Iconify source never opened this session) | `GitBranch` or `Globe` 16 px, name 14 px; line 2 muted the host "raw.githubusercontent.com" | **Load list** (one request: the catalogue's index or the Iconify set list; the same fetch as expanding the source in the tab) |
| Ready entry (Curvyo catalogue not added, `0138` Q2) | `Globe` at 55 %, name muted, line 2 the host | **Add** (adds it, fetches nothing; the card becomes the unfetched-source card in place) |
| Iconify set, not loaded | row 1 three sample icons 24 px in ink 8 px apart (as the tab's set row draws them); row 2 name 14 px; row 3 12 px muted "2,500 icons · Material"; row 4 the licence chip and the author 11 px muted | **Load set** (one request, `/collection`: icon names, no bodies) |

- Buttons: outline text buttons 24 px high, at the card's bottom right; the card's other text
  ellipsises before them.
- **Loading**: the button shows `LoaderCircle` 14 px in place of its label, `aria-busy`, after
  150 ms; the rest of the grid stays usable. **Done**: the card leaves the group; a set's icons
  join the items in the current sort; a source's sets become set cards (Iconify) or its templates
  items (catalogue). The scroll position and the focused tile stay (identity; if it was the card,
  focus goes to the next card). Announced: "Material Symbols: 2,500 icons loaded".
- **Failed**: row 3 of the card becomes `WifiOff` 14 px "Offline" or `CircleAlert` 14 px
  `--destructive` with the reason, 12 px muted; the button stays and tries again (`0138` Q16).
  Only that card changes.
- **Filtering cards**: they follow the search (set name, category, author; a source's name),
  Category, Colour (the set's palette flag) and Customizable; while a **Source** filter is active
  the group is not drawn (Source picks among what is loaded, U14). The header counts what is left:
  "Not loaded · 12 of 196 sets".
- **Keys**: cards are options of the same listbox, after the items; Enter or Space on a card
  presses its button; arrows move through cards in their own columns.

## 8. Detail bar (Frage 374 A; criteria 5, 7)

The pane's last row, always present (the Glyphs-mode detail bar): `--panel-bg`, 1 px top line,
8 px padding. It shows the **hovered** tile, else the **current** one; with neither, "Nothing
selected" 14 px muted (U15).

| Part | Wide (113 px) | Narrow (89 px) |
|---|---|---|
| Preview box | 96 x 96, the tile look, the item at that size | 72 x 72 |
| Text, 12 px left of it | name 14 px (two lines, then ellipsis); line 2 12 px muted: the place "Iconify › Material Symbols" / "Curvyo catalogue" / "Box job › Lid"; line 3 12 px: the **licence as a link** (the `0138` link cell, opens in the system browser), " · ", author or credit; line 4 12 px muted facts: "Monochrome · takes the object's colour" / "Colour" / "210 × 297 mm · Drawing page" / "3 pages" / "Font not included" / the cannot-be-loaded reason | name 14 px one line; line 2 the place; line 3 licence link · author (facts in the tooltip of the preview box) |
| Buttons, right, stacked 4 px apart | the item's actions (0.5: Show pages, Pick elements…), outline text buttons 28 px, 112 wide, column reserved | 28 px icon buttons (`Files`, `MousePointerClick`), 28 px column reserved |

Actions by item (amended by 0.5: no button loads; double-click, Enter and the drag load):

| Item | Buttons |
|---|---|
| Icon | none |
| Template, one page | none |
| Template, several pages | **Show pages** |
| Local page | **Pick elements…** (section 9.3) |
| Local document, several pages | **Show pages** |
| Cannot be loaded | none; the reason on line 4 |
| A set or card (Sets view) | none (0.2) |

- The bar's content changes on hover without delay. The buttons act on the **current** tile, so
  they are **not drawn** while the bar shows a hovered tile that is not the current one (U16;
  Hidden, not disabled; the slot is reserved, nothing moves).
- Licence and credit are visible here before anything is loaded (`0138` criterion 3).

## 9. Selecting and loading (criteria 7, 8)

### 9.1 Pointer

- **Press** on a tile: it becomes current (detail bar, `aria-selected`), focus in the grid (G1,
  U17): a press must not load by accident, the `0138` icon-cell rule ("a press only focuses").
- **Double-click** (amended by 0.5: no button loads): loads, exactly as the tab's same row
  (icon: a path 24 mm wide centred on the visible drawing, one step "Import icon", selected, Came
  from with licence and credit; template: "Load template", every page; local page: "Import
  page"). A local document with several pages opens its pages (9.4) on double-click; a template's
  pages open with Show pages or a press on its page-count chip (0.5). After a mouse load the focus goes to the canvas; the browser stays open,
  the current tile stays. No notice after a success; refusals as in `0137` and `0138` (canvas
  notice slot, alert, "Nothing was changed.").
- **Drag** (4 px from a tile, split view only): the tab row's drag. Icon: Drag ghost of its outline
  at 24 mm, badge `Copy` "Copy", release at the drop point; template: the thumbnail at 70 %,
  `Import` "New page" / "2 new pages"; local page: "New page"; over a sketch page the `0099`
  refusal for elements. A drag does not change the current tile. In full view there is no drag
  (the drawing is hidden).
- **Coarse pointer**: tap selects, double-tap loads, a 400 ms hold starts the drag; detail-bar
  buttons 36 px high.

### 9.2 Keyboard

Arrows move the current tile with the focus (Up and Down by a row), Home, End, Page Up, Page Down,
Ctrl+Home, Ctrl+End; **Enter** loads (a multi-page local document opens its pages), **Space**
selects (0.5); Alt+Left goes Back (0.3); a printable character or Backspace goes to the search
field; Down or Enter in the search field
focuses the first tile (U18). After a keyboard load the focus stays on the tile and "Loaded
arrow-left" is announced.

### 9.3 Picking elements of a local page (U19, G13)

**Pick elements…** in the detail bar opens the `0137` Page preview for that page in place of the
browser (one side pane). Its header then starts with **Back** (`ArrowLeft` 28 x 28, "Back to
library browser"), which returns to the browser as it was (search, filters, scroll, current
tile). Escape in the Page preview with nothing picked also goes back, a second Escape closes. The
Page preview opened from the tab has no Back. Amended by 0.3: this Back is the browser's history
(Alt+Left too); the Page preview is a place, so Back from the browser after a return does not
reopen it.

### 9.4 Second level: the pages of one item (criterion 8)

- The grid shows the item's pages as tiles (local pages with their pictures, a template's pages
  with its page thumbnails once downloaded); the subject line starts with **Back** (`ArrowLeft`
  24 x 24 inside the line, "Back to all items"; since 0.3 the one Back of the history) then "Box
  job › Lid · 3 pages".
- The facets stay as they are; the pages inherit their document's source and category, so they
  pass the filters their document passed (no special state). The search filters pages as in
  `0137` section 5: a document that matched by its name shows all pages, one that matched by a page
  name shows those pages (G10).
- Escape or Alt+Left goes back (G8 as amended by H6: not Backspace); focus returns to the
  document's tile.

## 10. States (criteria 9 to 11)

| State | Look |
|---|---|
| Nothing at all (no local document, no source) | grid: "Nothing in the library yet." 14 px muted, 24 px from the top, centred (the `0077` empty-grid look); the Items view adds **Show sets** (0.1), the Sets view shows the Curvyo ready entry card and Host your own library…. Browser build: "No documents in this browser." (`0137` 2.1) |
| Nothing loaded yet (Items view, only cards exist) | "No items yet." and **Show sets** (0.1) |
| Local index being read | tiles appear as read, in final order, no tile jumps above the focused one; subject "Reading 120 of 500" |
| Nothing matches | grid: 'Nothing matches "hrat" in Monochrome, Arrows.' 14 px muted (the search text, then the active filters by their values, comma separated, "and 3 more" past three), 12 px below **Clear all** (outline text button 28 px); no blank grid (criterion 11). Sets view: 'No set matches "hrat".' and Clear all |
| Source offline or failed | its card (7.1); a fetched source keeps its items; the Sources row shows "Offline" in place of its count after a failed Refresh in the tab |
| Enter search running, failed, offline | the subject line (section 4) and the Sources row "Search results" |
| Tile body or picture fails | the skeleton bar stays; the tile stays usable (the body is fetched again on the next Load) |
| Storage unreadable (browser build) | "Cannot read this browser's storage." in the grid (`0137` 2.1) |

Offline is a state, never an alert; nothing probes, nothing refetches by itself (`0138` section 8).

## 11. Keyboard and focus (criterion 23)

- Tab order: divider, view switch, search field, (narrow: Filters), Size, Back, Full view, Close, the facet
  sections (Clear all, Sort, Sources, Colour, Customizable, Category), the grid, the detail bar's
  buttons, the licence link. In the narrow layout with the band closed the facets are not in the
  tree.
- Escape ladder in the pane (amended by 0.3): while the newest history entry was left by opening
  something, Escape goes Back; then the pane closes, also from the search field, which keeps its
  text (`0077` P5). Outside the pane the canvas's own Escape steps come first. Alt+Left (Cmd+\[)
  and the mouse's back button go Back.
- One polite region for the pane: Enter search outcomes, "Material Symbols: 2,500 icons loaded",
  "Loaded arrow-left", the subject line at most every 2 s.
- No new shortcut beyond Back's Alt+Left (Cmd+\[) of G8; Browse has no key (`0137` P4).

## 12. Side panel versus the pane; light theme; large libraries

- **The 244 to 256 px column** keeps the tree of `0137` and `0138` unchanged (sources, rows,
  Online, Add source); its only change is the Browse button. The browser shows nothing the tree
  does not have, and loads nothing differently; it is a second view of the same data.
- **Light theme only** (design system: one theme, names ready for a swap): `--tile-bg`,
  `--tile-edge`, `--tile-skeleton` of `0077`; `--canvas-bg` grid; `--panel-bg` header, facets and
  detail bar. Measured on white: `--toolbar-icon` 11.0:1, `--panel-muted-fg` 6.9:1, the licence
  chip's text 11.0:1 (its border is decorative). Monochrome icons follow `--toolbar-icon`, so a
  later dark theme redraws them in its ink with no change here. Colour icons keep their colours on
  `--tile-bg` in any theme.
- **Large** (criterion 22): 10,000 items re-filter and re-rank within 100 ms of thread CPU; facet
  counts are computed in the same pass; the grid re-lays only the visible rows; at most 100 bodies
  per request, for the visible tiles plus one screen.

## 13. What differs for the other answers

| Frage | B | C |
|---|---|---|
| 371 Customizable | Templates may declare `colour` and `customizable` in the index: the Colour strip and the Customizable switch count them too; the Colour strip's tooltip then says only "Your own pages have no colour data"; the detail bar's facts line names it for templates | No Customizable row in the facets (not in the tree); its tooltip text goes; nothing else moves |
| 372 Scope | No "My documents" rows, no Newest cell (the strip is Best match \| Name \| Source, one row), no local second level, no Pick elements…; the empty state is "Nothing fetched yet." above the cards | As B in the first slice; the second slice adds the rows, Newest and Pick elements… as written here |
| 371 Fetch | Each Iconify group header of the Sets view (0.2) gains **Load all sets** (text button 24 px at its right), an **Armed action**: first press arms, "Press again to load 196 sets (196 requests)"; while it runs the header reads "Loading 40 of 196 sets" and each card turns into icons as it arrives; stops on Escape or Close | Set cards without a button (name, samples, category, licence); icons come only from Enter (Sources row "Search results"); Category and Colour filter the result icons by their set's data |
| 372 Detail (section 16) | No detail bar; a press loads (criterion 7 as written); both tile lines at every Size, tiles at least 112 px wide; Pick elements… only from the tab | The bar only in the wide layout; narrow tiles keep both lines and a press still selects |

## 14. Proposals (vetoable at the demo)

| # | Proposal |
|---|---|
| U1 | Browse is a 28 px toggle with `LayoutGrid` in the tab's search row (the Fonts tab's Preview look) |
| U2 | The browser keeps its own split share, not the font preview's |
| U3 | Source and Category selections live for the session; the rest of the view per user |
| U4 | Two layouts: facet column from 640 px of pane width, Filters band below |
| U5 | The browser's search text is its own, not the tab's |
| U6 | The Enter search outcome is part of the subject line |
| U7 | Filters are strips, a switch and check lists, never dropdowns or popovers |
| U8 | Best match is chosen when typing starts; the last own sort comes back when the field is emptied |
| U9 | A filter row with count 0 stays, muted |
| U10 | Tiles of exactly S + 16 px, columns from the left |
| U11 | Under 96 px a tile is its preview alone |
| U12 | No colour or mono marker on the tile |
| U13 | Cards are 208 x 112, independent of Size (since U31 in the Sets view, not in a Not loaded group) |
| U14 | Superseded by U31: there is no Not loaded group; the Sets view's Sources facet hides whole groups |
| U15 | The detail bar shows the hovered tile, else the current one |
| U16 | Its buttons (since U45 only Show pages and Pick elements…) are not drawn while it shows a hovered tile that is not the current one |
| U17 | A press selects; a double-click, Enter or a drag loads (since U45; Space selects) |
| U18 | Down or Enter in the search field focuses the first tile (Enter also searches online) |
| U19 | Pick elements… opens the Page preview with a Back button to the browser (one history, U40) |
| U20 | Licence chip first on the meta line, one neutral look for all licences |
| U21 | Mono icons drawn in `--toolbar-icon`, scaled by their viewBox with a 12.5 % margin |
| U22 | Thumbnails never scaled above their stored size |
| U23 | Superseded by U45: opening a card that is not loaded is the request |
| U24 | The Curvyo ready entry is also a card; opening it adds it (U45) |
| U25 | A failed card is opened again to retry (its reason on row 3) |
| U26 | Category list A to Z, Uncategorised last, merged by name ignoring case |
| U27 | Checking My documents checks its workspace and folder rows |
| U28 | Clear all clears filters and search; also offered in the Nothing matches state |
| U29 | After a mouse load the focus goes to the canvas, after a keyboard load it stays |
| U30 | Escape or Alt+Left leaves the second level (Backspace no longer, U42) |
| U31 | Two views, **Items** \| **Sets**, as the header's first control (the font preview's mode-switch look), kept per user |
| U32 | The cell says "Items", not "Icons", because templates and pages are in it too |
| U33 | Opening a set shows the Items view narrowed to that set alone (search empty, other facets reset) |
| U34 | The empty Items view offers Show sets |
| U35 | In the Sets view loaded and not loaded sets are mixed A to Z per source group, never loaded first |
| U36 | A card's button slot is a state slot: nothing, Loaded, spinner, or the reason on row 3 |
| U37 | The Sets view has Sources, Colour and Category only (no Sort, no Customizable) |
| U38 | Enter searches online only in the Items view; in the Sets view it focuses the first card |
| U39 | A place keeps view, level, search, facets, sort, current item and the first visible tile's identity |
| U40 | A history of places for the session, at most 50, pushed by every change of place, no Forward |
| U41 | One Back button at the start of the subject line, named by the place it returns to |
| U42 | Alt+Left (Cmd+[ on a Mac) and the mouse back button go Back; Backspace does not |
| U43 | Escape goes Back only out of something opened, else it closes; it never undoes a view switch |
| U44 | In every multi-select facet a press replaces, Shift+press adds or removes, a press on the only selected row clears; Space and Shift+Space |
| U45 | No Load, Import page, Load set, Load list or Add button; double-click, Enter or a drag loads or opens |
| U46 | A double-click on a multi-page template loads all its pages; Show pages or the page-count chip opens them |
| U47 | Pick elements… and Show pages stay in the detail bar (no gesture does either) |
| U48 | Host your own library… in the Add source form and at the end of the Sets view opens an in-app guide shipped with the app |

## 15. Spec deltas for the product owner

| # | Criterion | Delta |
|---|---|---|
| G1 | 7, 23 | A press selects (current, detail bar); double-click, Enter, Space, the detail bar's button or a drag loads (with Frage 374 A; B keeps criterion 7) |
| G2 | 4, 11 | The header holds search, Size, subject line, Full view, Close; filters and Sort are the facet column (from 640 px) or the Filters band (section 5) |
| G3 | 5 | Under Size 96 the tile shows no name or meta line; the detail bar and tooltip carry them (with 374 A) |
| G4 | 11 | The button is "Clear all" (filters and search) and also sits in the Nothing matches state |
| G5 | 14 to 16 | A Category section without data shows a muted state line instead of a disabled control; Colour and Customizable are disabled with the reason as description |
| G6 | 21 | No hint line under the field; the Search online `Globe` button of `0138` Q19 and the Enter outcome in the subject line |
| G7 | 3 | Source and Category selections are kept for the session only |
| G8 | 23 | Escape (and Backspace, Alt+Left) at the second level goes back before it closes |
| G9 | 10 | Unfetched sources and unloaded sets are cards (208 x 112) in a "Not loaded" group at the grid's end; Load list for a source, Load set for a set |
| G10 | 8 | With search text, a document matches by its name or a page name; its second level shows the matching pages |
| G11 | 10 | The Curvyo ready entry is a card with Add (fetches nothing) |
| G12 | 17 | Sort is a 2 x 2 strip; Best match disabled without text, chosen when text starts |
| G13 | new | A local page offers Pick elements…: the Page preview with Back to the browser |

### 15.1 Spec deltas of the rework (customer test, 2026-11-25)

| # | Criterion | Delta |
|---|---|---|
| H1 | 4, new | Two views, **Items** \| **Sets** (a radio strip, the header's first control); Items holds the tiles of everything fetched and no cards; Sets holds one card per set or source, loaded or not, grouped by source (0.1, 0.2) |
| H2 | 9, 10 | The Not loaded group leaves the grid; cards live in the Sets view only. No Load list, Load set or Add button: a double-click or Enter on a card opens it, which is the one request (fetching a list or loading a set) or, for the ready entry, adds it. A press selects a card |
| H3 | new | Opening a set or a catalogue or folder card shows the Items view narrowed to it alone: Sources = that set, search empty, Category none, Colour All, Customizable off, sort Name |
| H4 | 7, 23 | No Load or Import page button: double-click, Enter or a drag loads; Space selects; a template with several pages loads all its pages on double-click; a local document with several pages opens its pages; the detail bar keeps only Show pages and Pick elements…; a press on the page-count chip shows the pages |
| H5 | 2, 8, new | A history of places (view, level, search, facets, sort, current item, scroll anchor) for the session, at most 50 entries, pushed by a view switch, opening a set, card or item, Show pages, Pick elements… and the hosting guide; **Back** at the start of the subject line restores the place exactly and returns the focus to what was opened; no Forward |
| H6 | 8, 23 (G8) | Back by Alt+Left (Cmd+[ on a Mac), the mouse back button and the Back buttons; Backspace no longer goes back; Escape goes Back while the newest entry was left by opening something, else closes |
| H7 | 12, 13 | Sources and Category (and their workspace and folder rows): a press selects only that row, a press on the only selected row clears, Shift+press adds or removes; Space and Shift+Space likewise; the lists are multi-select listboxes |
| H8 | 3 | The view (Items or Sets) is kept per user; the history is kept for the session only |
| H9 | 21 | Enter searches online only in the Items view; in the Sets view the search filters the cards by name, category and author and Enter focuses the first card |
| H10 | 11 to 16 | The Sets view's facets are Sources (the sources), Colour (the set's palette flag) and Category (the set category); each view keeps its own facet values and search text |
| H11 | new | **Host your own library…** in the Add source form (kind Git catalogue) and at the end of the Sets view opens an in-app guide (a place, with Back): the folder layout, the steps, an example `curvyo-catalogue.json` with Copy, the field rules and limits of ADR 0085 decision 3, a link to `github.com/curvyo/templates`; the product owner writes its text; it ships with the app, no request |
| H12 | new | The guide names the index script of ADR 0085 decision 3 as the way to write `curvyo-catalogue.json`; that script must exist in `curvyo/templates` first (today the repository is empty), else the guide describes the hand-written index only |
| H13 | new (Frage 376) | Frage 376 answered A (2026-11-26), built in `0157` (criteria 1 to 18, ADR 0090; UX `specs/0157-library-git-token/ux-notes.md`): an optional **Access token** field for a Git catalogue in the Add source form, the `Lock` mark on the source, Change, Add and Remove token, the refusals. Until `0157` is built, #483 says "The repository must be public." in the form and the guide |
| H14 | 11 | Empty states: Items "No items yet." with Show sets; Sets 'No set matches "hrat".' with Clear all |

## 16. Customer question

Frage 374 (LIB-DETAIL): Should the browser get a detail bar for the selected item?
- A: Yes, like the glyph table's: the hovered or selected item at 96 px with name, place, licence
  link, author and facts, and its Load button. A press selects, a double-click or Enter loads. The
  spec's Out of scope "detail view of one icon" means zoom, path preview, copy as SVG; those stay
  out.
- B: No: a press on a tile loads it at once (criterion 7 as written); licence and author only on
  the tile line and in the tooltip; tiles keep two text lines at every size.
- C: As A, but only in full view; the split view behaves like B except that a press selects.
- Recommendation: A (the licence and credit are readable before loading, small sizes stay dense,
  and nothing loads by a stray click). Default if unanswered: A.

Frage 376 (LIB-TOKEN), answered A (2026-11-26), built in `0157`: Should a Git catalogue in a
private repository be reachable with an access token?
- A: Yes: an optional read-only token per Git catalogue in the Add source form, kept in the
  operating system's key store, sent as a request header only to that source's own address (never
  on a redirect to another host); the source shows a lock. Needs an architect's ADR (key store,
  likely one new dependency) before it is built.
- B: No, public repositories only for now; the hosting guide says so. Nothing with credentials is
  stored or sent.
- C: As A, but the token is written into `online-sources.toml` in plain text (no new dependency;
  anyone who can read the file can read the token).
- Recommendation: A (private templates are what a shop or a team wants, and the key store keeps
  the token off disk). Default if unanswered: B (public repositories only in #483; A as its own story after Frage 375).
