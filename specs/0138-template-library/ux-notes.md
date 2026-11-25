# 0138 Template library over the network: UX notes

ux-engineer, 2026-11-24. Written from `specification.md` (criteria 1 to 10; Fragen 271 B, 272,
273 A; answered 2026-11-24: 334 A, the Curvyo catalogue from GitHub raw files, and 340 B, Enter
also searches the added Iconify sources) and ADR 0085 on top of `specs/0137-document-library/ux-notes.md`: the same Library tab, tree, page
rows, page preview pane, imports and Came from line; this note adds only the "Online" section
and what differs for it. Design system rows used besides those of `0137`: "Properties panel:
File tab" (Export options), "Add page group" (the inline form and Kind switch), "Inline delete
confirm" and "Armed action", "Action notice", "No notice after a success". No new token.
**(Proposal)** items are vetoable at the demo; section 11 lists them as Q1 to Q22 (Q, so they
never collide with the P numbers of `0137`), section 12 the spec deltas F1 to F8, section 13 the
answered customer questions. Draft: `docs/design-drafts/2026-11-library/03-online-section.html`.

## 1. Principle

Online sources sit in the same tree as the maker's own documents and behave the same, with two
differences he can always see: nothing is fetched until he opens a source or presses Refresh,
and every item says whose it is and under which licence before it comes in. Typing in the search
sends nothing; Enter sends the words to the Iconify sources he added. A source whose request finds
no network says "Offline" and everything else carries on.

## 2. The Online section (criterion 1)

- **Third group of the tree**, after Workspaces and Folders, heading "Online" (the `0137`
  heading look). Its sources in the order they were added (Q1).
- **No source added**: the heading and one row, the **Curvyo catalogue** ready entry (Q2, the
  spec's Proposal): 32 px, `Globe` 16 px at 55 % opacity, name 14 px muted, and an **Add** outline
  text button 24 px high, 52 px wide at the right. Nothing is fetched and nothing is sent while
  it is only offered (R-SYS-007). Add makes it an added source in the same place (nothing moves)
  and does not open it. Removing it later brings the ready entry back.
- **Where the Curvyo catalogue is read from** (Frage 334 A): GitHub raw files of the
  `curvyo/templates` repository, address `https://raw.githubusercontent.com/curvyo/templates/main/`
  (ADR 0085 decision 1). The ready entry's tooltip and, once added, its source row's tooltip name
  the host in line 2, muted: "raw.githubusercontent.com" (as every source row names its host), so
  the maker sees who answers before he presses Add. No other wording names a host of our own.
- **Builds**: the Online group is in the desktop build only in this slice (ADR 0085 decision 2,
  **Proposal**); the browser's Library tab (`0137` section 2.1) has no Online group and no Add
  source.... If it comes to the browser later, these notes hold there unchanged.
- **Add source...** is the right footer button of the tab (118 x 28, `Plus` 16 px; `0137` section
  3, row 4), absent while the inline form is open (Q3).

## 3. Adding a source (criteria 1, 6, 7)

The **inline form** of `0045` and the Add page group, in place above the footer, `role="group"`
named "Add source", `--toolbar-bg` ground, 8 px padding; the tree above shrinks, nothing floats.

| Row | Control |
|---|---|
| Kind | the Kind switch look: `role="radiogroup"` "Kind", two equal segments 28 px: **Git catalogue** (`GitBranch` 14 px) and **Iconify** (`Globe` 14 px); Git catalogue selected the first time, then the last kind used (per user, not in a file) |
| Address | text field, full width, 14 px, `inputmode="url"`, `aria-label="Address"` with the visible label "Address" 12 px above; placeholder is the empty field (no example URL: an example is a tip) |
| What is sent | one 12 px muted line under Address, by kind (criteria 1 and 10, F6; a statement of what leaves the machine, not an operating tip): Git catalogue "Add sends one request to this address."; Iconify "Add sends one request to this address. Enter in the library search sends your search words to it." The Add button is described by it (`aria-describedby`) |
| Buttons | **Cancel** and **Add**, the `0045` pair (118 x 28 each, 8 px apart) |

- Add checks the address with **one** request (the catalogue's index or the Iconify API's
  collection list; the source's first fetch, so it is fetched only because the maker pressed
  Add). While checking: Add shows `LoaderCircle` in place of its label, `aria-busy`, the fields
  are read only, Cancel stops the check.
- Done: the form closes, the source is appended to Online, **expanded** (the fetched list is
  already there), focused (keyboard). The source's name is the catalogue's own `name` from its
  index or, for Iconify, the host ("api.iconify.design") (Q4).
- Refusals under the Address field (`role="alert"`, `--field-invalid`, the field `aria-invalid`
  and described by it), the form stays: "Enter an address that starts with https://." / "This
  address is already in the library." / "No Git catalogue at this address." / "No Iconify API at
  this address." / "The server did not answer." / "Offline. Nothing was sent."
- Credentials: none in this slice's form. If the architect's ADR gives a source a credential
  (R-ASSET-004), a "Key" password field joins the form under Address (masked, kept by the OS
  key store, never shown again) (F1).
- Enter in the Address field adds; Escape or Cancel closes and returns focus to Add source....

## 4. Source rows (criteria 2, 5)

| Row | Height (coarse) | Anatomy | Accessible name |
|---|---|---|---|
| Online source | 32 (36) | chevron 16, `GitBranch` (Git catalogue) or `Globe` (Iconify) 16, 4, name 14 px (tooltip: the name, line 2 muted the host), at the right the state 12 px muted tabular ("120" items once fetched, nothing before, "Offline"), then a 56 px action slot: **Refresh** `RefreshCw` 24 x 24 and **Remove** `X` 24 x 24 (armed, `0137` section 4.4), shown on hover, focus within, while armed, and Refresh also while expanded | "Curvyo catalogue, Git catalogue, 120 templates" |
| Fetching | 28 | at the child level, `LoaderCircle` 14 px, "Loading..." 12 px muted, after 150 ms | not in the keyboard order; `aria-busy` on the source row |
| Fetch failed | 28 | at the child level, `CircleAlert` 14 px `--destructive`, the reason 12 px muted ("The server did not answer.", "Not a Git catalogue any more.", "The index does not match its hash.") | in the source row's description |

- **Expanding a source fetches its list** (Frage 273 A: "when the maker opens that source's
  list"), the first time in a session only (Q5). Collapse and expand again shows the list from
  this session without a request. **Refresh** fetches again; while it runs its glyph spins
  (still under `prefers-reduced-motion`), the rows below stay until the new list replaces them
  in one frame (the focused row kept by identity, else its neighbour).
- Nothing is fetched in the background, on start, on search, on a tab change or on hover (Q6).
- The fetched lists live for the session only (Q7, F2): the next start shows the sources
  collapsed and without counts, and opening one fetches.
- **Removing** a source (the armed `X`, "Press again to remove Iconify from the library")
  removes the row and its fetched list; nothing else changes; templates already loaded stay in
  their documents.

## 5. Git catalogue items (criteria 3, 6, 8, 9)

A Git catalogue's items are templates; each is a `.curvyo` file, so it is shown as a document
row of `0137` with a licence line:

| Row | Height (coarse) | Anatomy |
|---|---|---|
| Folder of the catalogue (if the index nests) | 32 (36) | the `0137` folder row |
| Template | 52 (56); 68 (72) with line 3 | chevron 16, the catalogue thumbnail 40 x 28 (loaded for visible rows after 100 ms of rest, like local thumbnails; a thumbnail is part of the list fetch, Q8), 8, line 1 name 14 px; line 2, 12 px muted: the **licence name as a link** (cell of its own, underline, `--toolbar-icon`, Q9), " · ", author, " · ", size ("24 KB"), licence first so a cut line never hides it (as the Came from line, Q23); line 3 only when criterion 9 applies: "Font not included" 12 px muted; action slot 28 px: **Load** `Import` 24 x 24, shown on hover, focus within and on the current row |

- **Licence link**: opens the licence's address in the system browser (the maker's own act,
  Q10); the tooltip names the address. In the accessible name: "Box with lid, template by Jane
  Doe, licence CC BY 4.0, 24 kilobytes, font not included".
- **Expanding a template downloads it** (one request for this file, the maker's act), checks
  its hash against the index, reads it with the ordinary reader (criterion 8) and lists its
  pages as `0137` page rows; from there the page preview pane, picking, the drags and Import
  page and Import elements work exactly as for a local document. While downloading: the
  "Loading..." row; the downloaded file is kept for the session only (F2).
- **Load** (the row's action, Q11, F3): downloads if needed and imports **every page** of the
  template as new pages after the last page, one step "Load template"; the first loaded page
  becomes the active page (`0137` P22). A one-page template is the common case, so Load is
  "Import page" without the expand. Dragging a template row onto the canvas does the same
  (badge `Import` "New page"; "2 new pages" for two or more, Q12).
- **Refused file** (criterion 8): the row's line 2 becomes "Cannot be loaded: does not match
  the catalogue." / "...: damaged." / "...: made by a newer Curvyo." in muted text with
  `TriangleAlert` in place of the thumbnail; nothing is imported. Load pressed and refused: the
  same reason as an alert in the canvas notice slot, 8 s, ending "Nothing was changed."

## 6. Iconify items (criteria 3, 7)

An Iconify API lists icon sets, each with its own licence; an icon is SVG, imported by the SVG
import of `0024`.

| Row | Height (coarse) | Anatomy |
|---|---|---|
| Icon set | 44 (48) | chevron 16, a 28 x 28 sample of three icons from the set's own list data (no extra request), 8, name 14 px over line 2 12 px muted: licence link (as in section 5) " · " author " · " "1,240 icons" |
| Icon grid row | 40 (44) | at the child level, up to **5 cells** of 36 x 36 (40 coarse), 4 px apart; the icon 24 px in `--toolbar-icon` on white, 1 px `--toolbar-icon` at 25 %, `rounded-[5px]` (Q13) |

- **Expanding a set fetches its icon list** (the set's own list, one request; Frage 273 A).
  Icons draw from that list's data; no request per icon.
- Cells are the grid's cells: Left and Right move between them inside a row and across rows
  (Up and Down keep the column); a cell's accessible name "home, icon, Material Symbols, licence
  Apache 2.0"; tooltip line 1 the icon's name, line 2 muted "Material Symbols · Apache 2.0".
- **Import an icon**: Enter or Space on a cell, a double-click, or a drag onto the drawing
  (Drag ghost of its outline at 24 mm wide, Q14; badge `Copy` "Copy"): one step "Import icon",
  the icon as a path at the drop point or centred on the visible drawing, 24 mm wide (Q14),
  selected, into the active layer; Came from with the set, the icon name, the address and the
  licence. A press on a cell only focuses it (a single click must not import by accident; the
  page rows open the preview on a press for the same reason).
- No preview pane for icons: the 24 px cell is the preview.

## 7. Search and online sources (criterion 10, Frage 340 B; ADR 0085 decision 5)

### 7.1 Typing filters, Enter also searches online

- **Typing** filters live what this session has already fetched (names of templates, sets, icons,
  template pages, and the results of an earlier Enter); it **sends nothing**. A source that was
  never opened shows nothing under it while searching and is drawn collapsed with its name; the
  heading "Online" stays while one source is listed.
- **Enter** in the search field also sends the words to **every Iconify source the maker added**
  (expanded or not), once per Enter, never to a Git catalogue or the Curvyo catalogue. The words
  are trimmed, inner spaces collapsed, cut at 100 characters (ADR 0085, **Proposal**). Enter with
  an empty field, or with no Iconify source added, sends nothing and shows nothing. Focus stays in
  the field; the local filter is unchanged.
- **Search online button** (Q19): while the field holds text and at least one Iconify source is
  added, a `Globe` 16 px symbol button 24 x 24 sits inside the field, left of Clear search; name
  and tooltip "Search Iconify sources (Enter)". A press does what Enter does and keeps focus in the
  field. It is a control whose name says what goes where, the pointer route to Enter, not a tip;
  without an Iconify source it is not in the tree. The placeholder stays "Search library".
- **Where it is said**: the Add source form for Iconify (section 3, "What is sent") and this
  button. Nothing else: no line under the field, no first-use notice.

### 7.2 The results under each Iconify source

| Row | Height (coarse) | Anatomy |
|---|---|---|
| Results row | 28 | the first child of the source, `Search` 14 px muted, 4, 12 px muted tabular: '"home" · 48 icons' (the words as sent, in quotes, ellipsis; the count of results, at most 64); not in the keyboard order, the source row is described by it |
| Result icons | 40 (44) | the icon grid rows of section 6 (5 cells, 36 x 36), in the order the source answered; each cell's set and licence as in section 6 ("Licence unknown" when the source names none); Enter, double-click or drag imports, one step "Import icon" |
| Icon sets | | below the results, the source's set rows if its list was fetched, filtered by the typed text |

- On Enter each Iconify source is drawn **expanded** with its results row on top (Q20); a source
  that was never opened gets only the results, its set list is not fetched.
- **Loading**: the results row reads `LoaderCircle` 14 px 'Searching "home"...' after 150 ms (the
  spin also under `prefers-reduced-motion`), `aria-busy` on the source row; the tree, the filter
  and the other sources stay usable. The icon cells draw when their bodies arrive, the empty
  36 x 36 cell before.
- **No match**: '"home": no icons' on the results row, no grid row.
- **Typing after Enter**: the results row keeps the words it was sent with, its icons are filtered
  by the new text like everything fetched; the next Enter sends the new words and its results
  replace the old in one frame (focus kept by identity, else on the field). An Enter while a
  search runs drops the running one.
- **Clearing the search** (Clear search or Escape) takes the results rows out with the rest of the
  search tree (`0137` section 5); the results stay fetched for the session, so typing finds them
  again until the next Enter replaces them (Q21).
- **Failure on one source** (ADR 0085 decision 6): the results row takes the Fetch failed look,
  `CircleAlert` 14 px `--destructive` and the reason ("The server answered 500."); a connection
  failure is **"Offline"**: `WifiOff` 14 px and "Offline" on the results row, "Offline" in the
  source row's state (section 8). Each source shows only its own outcome; no notice, no alert.
- **Announcements** in the tab's polite region: "Searching 2 Iconify sources" on Enter, then per
  source "api.iconify.design: 48 icons for home" / "...: no icons for home" / "...: Offline".

## 8. Offline (criterion 5; ADR 0085 decision 6)

- **Per source**: "Offline" is shown on a source after its own request failed at the connection
  level (DNS, connect, timeout, TLS). There is no app-wide online state and nothing probes (F7, as
  ADR 0085 replaces it); other sources keep their state until their own request.
- The source row shows "Offline" at the right in place of its count; expanding a never-fetched
  source whose request failed shows one 28 px child row "Offline" 12 px muted with `WifiOff`
  14 px. **Refresh stays active** (Q16): it is the way to try again, and a success clears
  "Offline". Already fetched lists of this session stay browsable and their downloaded templates
  stay importable.
- Offline is a state, never an alert: no notice, no colour; local sources, drawing and saving are
  not touched. Nothing refetches by itself; the source's next request (Refresh, expanding a set,
  Load, Enter) clears "Offline" when it succeeds.
- Any other failure is the "Fetch failed" row (section 4) with its reason, not "Offline".

## 9. Came from and credits (criteria 3, 4)

- **Came from line** (`0137` section 8.2) for an online item: "From Curvyo catalogue › Box with
  lid" (source › template; Iconify: "From Iconify › Material Symbols › home"), then under it a
  12 px muted licence line "CC BY 4.0 · Jane Doe" with the licence name as a link. **Open
  source** is not in the tree for online items (Q17): the catalogue is not a document.
- **Credits** (criterion 4, the spec's Proposal of a list in the File tab): a block **Credits**
  in the File tab, 12 px below Export options, present only while the document holds an item
  whose licence needs credit (none: not in the tree). Header 28 px: "Credits" 14 px semibold
  with the count 12 px muted at the right ("2"); then one entry per credited item, 12 px, wraps,
  4 px apart: "Box with lid by Jane Doe, CC BY 4.0" with the licence name as a link and the
  source in the tooltip; identical credits listed once. No remove control: the app never removes
  a credit (criterion 4); deleting the object removes its credit with it (F4).

## 10. Keyboard, accessibility, wording

- Tab order of the tab with `0138`: search, tree, Add folder..., Add source... (or the form's
  Kind, Address, Cancel, Add).
- In the search field, Enter also searches the added Iconify sources (section 7); the Search
  online button inside the field is not a Tab stop (Enter does the same), like Clear search
  (Escape).
- In the tree the `0137` keys hold; Right from a source row's name cell reaches Refresh, then
  Remove; on a template row it reaches the licence link, then Load; Ctrl+Enter on a template row
  loads it (the `0137` P25 key).
- Announcements in the tab's polite region: "Loading Iconify", "Iconify: 220 icon sets",
  "Offline", "Loaded Box with lid".
- Every text is formatted in Rust (`curvyo-ui-core`) from the connector's reason codes.

## 11. Proposals (vetoable at the demo)

| # | Proposal |
|---|---|
| Q1 | Online sources in the order they were added |
| Q2 | The Curvyo catalogue is a ready entry with Add until added (the spec's Proposal, kept) |
| Q3 | Add source... is the tab's second footer button |
| Q4 | A source's name comes from its index, or the host for Iconify |
| Q5 | Expanding a source fetches its list once per session |
| Q6 | Nothing is fetched on start, search, hover or tab change |
| Q7 | Fetched lists and downloaded templates live for the session only |
| Q8 | Catalogue thumbnails come with the list fetch |
| Q9 | The licence name is a link cell in the row |
| Q10 | A licence link opens in the system browser |
| Q11 | Load imports every page of a template as new pages |
| Q12 | A template row drag does the same as Load |
| Q13 | Icons in a grid of 5 cells per row inside the tree |
| Q14 | An imported icon is 24 mm wide |
| Q15 | Typing never sends anything; Enter sends the words to every added Iconify source (the behaviour is Frage 340 B; the wording here is the Proposal) |
| Q16 | Offline per source: fetched lists stay browsable, Refresh stays active to try again |
| Q17 | No Open source for online items |
| Q18 | The Credits block in the File tab, listed once per identical credit |
| Q19 | A Search online `Globe` button inside the search field while it holds text and an Iconify source is added |
| Q20 | Enter expands every Iconify source with a results row on top |
| Q21 | Clearing the search hides the results; they stay fetched for the session |
| Q22 | Tooltips of the ready entry and online source rows name the host (Curvyo catalogue: raw.githubusercontent.com) |
| Q23 | Line 2 of template and icon set rows starts with the licence link, then author and size (UX review 2026-11-25: in the 244 px column real author names pushed the licence out of sight) |

## 12. Spec deltas for the product owner

| # | Criterion | Delta |
|---|---|---|
| F1 | 2 | A credential field joins the Add source form only if the architect's ADR gives sources a credential |
| F2 | 2, 5 | Fetched lists and downloaded templates are kept for the session only; there is no offline cache |
| F3 | 3 | "Load" imports every page of the template as new pages (one step "Load template"); a template also expands to its pages like a local document |
| F4 | 4 | The credit belongs to the object; deleting the object removes its credit from the list |
| F5 | 3 | An Iconify icon is imported as a path through the SVG import of `0024`, 24 mm wide, at the drop point or centred, one step "Import icon" |
| F6 | 1 | Adding a source sends one request to check it; say so in criterion 1 ("by its address on purpose" stays true) |
| F7 | 5 | Replaced by ADR 0085 decision 6 (spec criterion 5): "Offline" is per source after a connection failure of its own request; there is no app-wide online state (section 8) |
| F8 | 9 | "Font not included" is the template row's third line |

## 13. Customer questions (answered 2026-11-24)

- **Frage 334 (TPL-HOST): A.** The Curvyo catalogue is read from GitHub raw files of
  `curvyo/templates` (section 2); GitHub sees those requests, and the tooltips name the host.
- **Frage 340 (TPL-SEARCH): B.** Typing filters only what was fetched; Enter also sends the words
  to the Iconify sources the maker added (section 7). This note's earlier recommendation A is
  dropped.
