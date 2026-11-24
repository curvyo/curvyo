# 0137 Document library: UX notes

ux-engineer, 2026-11-24. Written from `specification.md` (criteria 1 to 13; Fragen 268 B+C, 269 A,
270 A) and `docs/design-system.md`: "Properties panel: tabs", "Properties panel: File tab, Workspace
section" (the tree, the document row), "Font preview" (the canvas area's side pane), "Page kinds"
(kind chip), "Bring content in" and "Sketch pages" (Drop badge, Drag ghost), "List row", "Inline
delete confirm", "Armed action", "Canvas notice slot", "Action notice", "No operating tips by
default", "Hidden, not disabled", "Print and cut" (the column measures 244 to 256 px). The surface
is shared with `0138`: its "Online" section is in `specs/0138-template-library/ux-notes.md` and
builds on every section here. No new colour token, no new component kind: the tab is the
workspace tree of `0095` with page rows, the preview is the side pane of `0077`. **(Proposal)**
items are vetoable at the demo; section 14 lists them as P1 to P28, section 15 the spec deltas
E1 to E10, section 16 the answered customer question (Frage 341 C, 2026-11-24: the browser build
has the tab, section 2.1). "Muted" is `--panel-muted-fg`; "column" is the
panel's 244 px content width. Drafts: `docs/design-drafts/2026-11-library/`.

## 1. Principle

Every document the maker has is already in the library; nothing has to be saved into it. The
tab reads like the workspace tree he knows, with one more level: pages, each with its
thumbnail. A page goes in by dragging it onto the canvas or by one button; a few elements go in
by picking them in a large read-only preview beside the drawing and dragging them across. The
result is an ordinary copy that remembers, in one muted line, where it came from. Browsing never
changes, locks or opens a source.

## 2. Where it lives (criterion 1)

- **Its own tab "Library" in the Properties strip** (spec Proposal, confirmed; P1): it stays
  open while the maker draws, and the File tab already holds the workspace tree, export and
  Quit; a second tree there would push Export SVG below the fold at 800 x 600.
- **Place in the strip** (P2): after Fonts: File, Document, Style, Layers, History, Fonts,
  **Library**, Snapping, Grid, Sketches, Parameters, Share. Fonts and Library are the two
  libraries of reusable things; neither is active by itself. `PanelTab::Library`, name
  `"library"`, label and tooltip "Library". On a laser page's strip as well (pages and elements
  can go onto any page, section 7.4).
- **Glyph** Lucide `LibraryBig`, 16 px at 1.5 px absolute stroke (P3).
- **No key** in this slice (P4): the strip's arrows reach it; Shift+Ctrl+B (bookmarks) and
  Shift+Ctrl+Y (Firefox Library) are taken in the browser, and the frequency is not known yet.
- **Never active by itself** (row "Active tab": with History, Layers, File, Fonts, Share).
- **Both builds** (Frage 341 C, answered 2026-11-24): the desktop lists workspaces and added
  folders (sections 3 to 4), the browser the documents it keeps in its own storage (section 2.1).

### 2.1 The browser build (Frage 341 C; ADR 0084 decision 10)

The tab's place, glyph, header row, search, row types, page preview pane, drags, imports, Came
from line and keys are those of the desktop. What differs:

- **The one source: this browser's storage.** The browser keeps documents only in the workspaces
  it holds (synced from a desktop, `0096` criteria 35 and 38), so the tree groups them by those
  (P5, the ADR's Proposal): one group with the heading **"In this browser"** (the section 4.1
  look), then one source row per held workspace, the open one first (semibold, P8), then the
  others in the order of the browser's workspace list; each expands to the `0095` workspace tree
  of its folders, documents and pages. Accessible name "Box job, workspace in this browser, 14
  documents". A workspace row has no Remove: nothing was added to the library, the Share tab
  owns the workspace.
- **Not there** (removed from the tree, nothing greyed; "Hidden, not disabled"): the Workspaces
  and Folders headings, the recent list, added folders, **Add folder...** and its refusals,
  Remove. A file opened from the disk is not stored by the browser and is not listed; the
  library never reads the disk. Tooltips show the whole name and its place inside the workspace
  ("Box job › Sides › Lid"), not a disk path.
- **No footer** in this slice (P26): the tree takes its 36 px. The Online section of `0138` is
  desktop only in this slice (ADR 0085 decision 2, **Proposal**; the PO aligns the spec's Out of
  scope line); if it comes to the browser, the `0138` notes hold there unchanged and the footer
  returns with Add source... in the left slot.
- **Empty** (the browser holds no workspace): "No documents in this browser." 14 px muted, 24 px
  from the top of the tree, no heading, nothing else (P27: the desktop's "No documents yet."
  would read as if the maker had nothing anywhere). No word on how documents get there (no tips).
- **Storage that cannot be read** (blocked or cleared site data, a private window that refuses
  storage): the same place shows "Cannot read this browser's storage." 14 px muted; no row, no
  retry button; the tab reads again when it is shown or the window gets focus (P28). A single
  stored document that fails is its row, with the section 10 texts.
- **Pages without pictures**: documents stored before this slice or written by sync list their
  pages on the empty thumbnail ground until they are saved in this browser (ADR 0084 note 8);
  no text, the rows stay usable.
- **Reading**: the index is read in a worker when the tab is shown and on window focus; the
  subject line counts ("Reading 12 of 40") as on the desktop, and the page stays responsive.
- **Saved version** (section 4.3): only for the document open in this browser tab; another tab
  of the same browser is another window and shows no label.
- **Open source** (section 8.3): opens the stored workspace document by the File tab's route,
  behind its unsaved-changes check. A copy whose source lives only on a desktop has no Open
  source here (its absence is the state, P24).
- **Tab order**: search field, tree. Touch heights (section 12) apply as on any coarse pointer.

## 3. Layout (column 244 px; draft 01)

| # | Part | Height | Notes |
|---|---|---|---|
| 1 | Header row: "Library", subject line right | 24 + 8 | subject 12 px muted, `role="status"`: "214 documents" (documents in every readable source), "Reading 120 of 500" while the index is built (section 9) |
| 2 | Search row: the Fonts tab search field, full column | 28 + 8 | `Search` 16 px inside left, placeholder "Search library", "Clear search" `X` while it holds text |
| 3 | The tree (the tab's one scroll area) | fills, at least 176 | stable 8 px scrollbar gutter; rows end 16 px short of it (the `0075` overlay-scrollbar rule) |
| 4 | Footer, 8 px above: **Add folder...** (desktop; the browser has no footer, section 2.1) | 28 | outline text button with `FolderPlus` 16 px, 118 x 28 (36 coarse), left; the right 118 px slot is taken by `0138`'s **Add source...**, empty until `0138` ships (P6) |

The header row, search row and footer never scroll. At 800 x 600 the tree is about 380 px:
eight document rows, or three document rows and five page rows. Nothing in the tab grows a
section: removing, loading and errors happen inside rows (sections 4, 9, 10).

## 4. The tree (criteria 1, 2, 10, 13)

One `role="treegrid"` named "Library", built from the `0095` workspace tree (virtualised, at
most 60 row elements plus the focused one, rows of fixed height per type placed by their
offsets, indent 12 px per level up to 6, natural case-insensitive order inside a folder, folders
first). Order of the sources on the desktop (Frage 268 B and C; the browser: section 2.1):

1. **Workspaces**: the open workspace first, then the other workspaces of the recent list in its
   order. A missing recent workspace is not listed (the File tab still shows it as "Not found").
2. **Folders**: the folders added with Add folder..., in the order they were added (P7: the
   maker's own order is the one he remembers; A to Z was weighed).

### 4.1 Group headings

"Workspaces" and "Folders": 24 px, 12 px semibold muted, 4 px left padding, 8 px above
(none above the first). A heading is not a row in the keyboard order and not a tree level
(the Fonts tab's block headers rule); its group is in each source row's accessible name. A
group with no source is not drawn; with no source at all the tree is the empty state (section
9).

### 4.2 Row types

| Row | Height (coarse) | Anatomy, left to right | Accessible name |
|---|---|---|---|
| Source: workspace or added folder | 32 (36) | chevron 16, `Folder` 16, 4, name 14 px (semibold for the open workspace, P8), document count 12 px muted tabular at the right; added folder only: **Remove** `X` 24 x 24 in a 28 px action slot, shown on hover, focus within and while armed | "Box job, open workspace, 14 documents" / "Logos, folder, 31 documents" |
| Folder inside a source | 32 (36) | the `0095` folder row without its New document button | "Sides, folder, 6 documents" |
| Document | 44 (48) | chevron 16, the 40 x 28 thumbnail (the `0095` document row: `object-fit: contain` on `--canvas-bg`, 1 px `--toolbar-icon` at 25 %, placeholder `FileText`), 8, name 14 px over line 2 12 px muted: "3 pages · 2 Nov 12:03" (the `0095` time format) | "Lid, document, 3 pages, changed 2 November 12:03" |
| Page | 52 (56) | indent of the document plus 12, the 56 x 40 page thumbnail (same ground and border; placeholder the empty ground), 8, line 1 name 14 px then the kind chip (11 px word, the `0098` page row rule: the chip never shrinks, the name truncates), line 2 the size 12 px muted tabular "210 × 297 mm" in the document's display unit; **Import page** `Import` 16 px, 24 x 24 in a 28 px action slot at the right, shown on hover, on focus within and on the current row | "Front, drawing page, 210 by 297 millimetres, in Lid" |
| Source that cannot be read | 44 (48) | chevron slot empty, `TriangleAlert` 16 px, name 14 px over "Cannot be read: <reason>" 12 px muted; added folder: Remove stays | "Old jobs, folder, cannot be read: drive not connected" |
| Document that cannot be read | 44 (48) | `TriangleAlert` in place of the thumbnail, line 2 "Cannot be read: <reason>" | "..., cannot be read: damaged" |
| Reading pages | 28 | at the page level, `LoaderCircle` 14 px (spins, still under `prefers-reduced-motion`), "Reading pages..." 12 px muted; shown only after 150 ms | not a row in the keyboard order; `aria-busy` on the document row |

- **Expand**: a press on a source, folder or document row, its chevron, Right or Enter. A
  document row expands to its pages (criterion 2). A document never opens from the library:
  opening is the File tab's job (P9); the library is read only.
- **Name truncation**: ellipsis, the whole name and the path (desktop) in the tooltip, as in
  `0095`.
- **States** (row "List row"): hover `--editor-accent-hover`; the **current page** (the one the
  preview shows, section 6) `--value-fill` with the 3 px `--accent` bar, `aria-selected`; focus
  2 px `--editor-accent` ring inside the row. No zebra.
- **Thumbnails** load for the rows on screen after 100 ms of rest, never for rows above or below
  (criterion 13); a page thumbnail that cannot be drawn keeps the empty ground, the row stays
  usable.

### 4.3 The open document and "Saved version" (criterion 11)

The document open in this window is listed in its source like any other. While it has unsaved
changes its row's line 2 reads "Saved version · 3 pages · 12:03" (P10) and the preview header
(section 6.2) carries the same words: what the library shows is the file on disk. Clean, the
row has no label. The label is a state, not a tip. For documents open in other windows the
label appears only if the app learns their state without reading their memory (E7, architect).

### 4.4 Removing an added folder (criterion 1)

The **Armed action** on the row's `X` (the `0095` recent-workspace remove, armed): first press
fills it `--destructive` with a white glyph; the armed name and tooltip read "Press again to
remove Logos from the library"; the second press removes the row; nothing on disk changes, so
the name says "from the library" and the glyph is `X`, never `Trash2` (P11). `Delete` on the
focused row arms, `Delete` or `Enter` removes; the 400 ms guard, 4 s disarm, Escape and focus
rules of the convention. No undo (it is not a document change); Add folder... brings it back.

### 4.5 Add folder... (criterion 1)

Opens the OS folder dialog ("Add folder to library"). Done: the folder is appended to Folders,
expanded, scrolled into view and focused (keyboard) or left as is (mouse). Refusals in the
Action notice under the button (`role="alert"`, 8 s, `--field-invalid` text): "Logos is already
in the library." (the row is focused), "Box job is a workspace. It is already listed." A folder
inside an added folder, or one that holds an added folder, is added as asked (P12): both rows
show the same documents; the maker removes the one he does not want.

## 5. Search (criterion 3)

- Filters as typed, case-insensitive substring of document and page names, across every
  source; no Enter needed, no debounce visible (the 100 ms budget is on the index).
- **What stays** (P13): a document whose name matches stays, collapsed; a document with a
  matching page stays, expanded, showing only the matching pages; folders and sources that hold
  a match stay, expanded; everything else is out of the tree; a heading without a source left is
  not drawn. Clearing the search restores the expand state from before it.
- The matched part of a name is semibold (P14); never colour.
- No match: 14 px muted, 24 px from the top of the tree: 'No document or page matches "lid".'
- A printable key typed on the tree goes to the search field (the Fonts tab rule); Down in the
  field focuses the first row; Escape in the field clears it, then returns to the canvas.
- The current page leaves the tree if it no longer matches; the preview keeps showing it.

## 6. Page preview pane: picking elements (criterion 5; draft 02)

### 6.1 Where and when

- **The canvas area's one side pane**, the `0077` Font preview pane mechanics unchanged:
  `role="region"` "Page preview", split right of the drawing at 35 % of the canvas area, the
  `SplitDivider`, Full view and Split view, always full when a split would leave the drawing
  under 360 px; the drawing's canvas keeps its size and view origin. One side pane at a time:
  opening the page preview closes the Font preview and the reverse (P15).
- **Opened** by a press or Enter on a page row: the row becomes the current page and the pane
  shows it (P16). Another page row replaces the page in the open pane; picks are dropped.
- **Closed** by Close, by Escape with nothing picked (section 6.4), by switching to another
  document in this window (New, Open). It stays open on a tab change: the maker drags while the
  Layers tab is shown.

### 6.2 Anatomy

| Part | Notes |
|---|---|
| Header, 28 px row, `--panel-bg`, 8 px padding, 1 px bottom line | page name 14 px, the kind chip, then the subject line 12 px muted, ellipsis: "Box job › Lid" (source › document; "Saved version" appended for section 4.3), then Full view `Maximize2` / `Minimize2` and Close `X`, 28 x 28, 4 px apart (the `0077` buttons) |
| Body, `--pasteboard-bg` | the page at fit (8 % margin), drawn read-only by the same renderer and look as the canvas (page background, pasteboard, no grid, no guides of the active document); wheel zooms toward the pointer, Space-drag pans, double-click on empty pasteboard fits again (P17) |
| Footer, 28 px row, 8 px padding, 1 px top line | the count 12 px muted, `role="status"`: "Nothing picked" / "3 of 42 elements picked"; at the right **Import page** and **Import elements**, outline text buttons 112 x 28, 8 px apart; Import elements is absent while nothing is picked, its slot reserved, nothing moves |

### 6.3 Picking (pointer)

What can be picked: the page's **top-level objects** (children of its layers): a group picks as
a whole, a linked clone as itself. The rules are the Select tool's, so nothing new has to be
learnt (P18, E2):

- click on an object picks only it; Shift+click adds or removes it; click on empty pasteboard
  clears the picks;
- a drag from empty pasteboard draws the Select marquee with its touch and contain modes,
  colours, legend and modifiers (Shift adds, Ctrl removes);
- hover: the `--hover-box` of the object under the pointer; picked: each object's 1 px dashed
  `--shape-handle-stroke` selection box with its casing, **no handles** (nothing in the preview
  can be moved, scaled or edited);
- hidden objects (eye off) are drawn as hidden and cannot be picked; locked objects can (a copy
  is not an edit; P19).

### 6.4 Picking (keyboard)

The body is one Tab stop (`tabindex="0"`, `aria-describedby` the count). With it focused:
Right and Left step a focus ring (the hover box, solid) through the top-level objects in
stacking order, front first; Space picks or unpicks the ringed one; Ctrl+A picks all; Escape
clears the picks, a second Escape closes the pane and returns focus to the page row. Each step
announces "Rectangle 3f9a1c, 2 of 42, picked" in the pane's polite region. Tab order in the
pane: header buttons, body, footer buttons.

### 6.5 Dragging picks onto the drawing (Split view)

- A press on a picked object and a drag of more than 4 px starts the drag (a press on an
  unpicked object picks only it first). While inside the pane: nothing moves, the pointer is
  `grabbing`.
- Over the drawing: the **Drag ghost** outlines of the picked objects at the drawing's zoom
  (one per drawn leaf; past 2,000 leaves the picks' joint box only, the `0100` rule), centred
  on the pointer, with the Select move's snapping; the **Drop badge** `Copy` "Copy" (P20: the
  word of the Ctrl copy badge; "Import" was weighed, but the result is exactly a copy).
- Release over the drawing: **Import elements** at the drop point (the picks' joint box centred
  there). Release over the pane, the panel or a bar, or Escape: nothing happens, no step.
- Over a sketch page: ghost at 40 %, `Ban` "Not on a sketch page", `not-allowed` (the `0099`
  refusal; E4); the release changes nothing.
- In Full view the drawing is hidden: only the footer buttons import.

## 7. Importing (criteria 4 to 8)

### 7.1 A page: drag or Import page (criterion 4)

- **Drag** a page row (4 px on the thumbnail or the name; never on the action slot): the page
  thumbnail at 70 % follows the pointer over the panel; over the canvas area the badge
  `Import` "New page" (P21) on top of it; anywhere on the drawing (pasteboard included) the
  release imports. Over a laser page the word stays "New page": the library never places a
  page (that is `0100`'s Pages-list drag with "Placed page"), and the different word says so.
  Escape or a release over the panel cancels.
- **Import page**: the row's action button or the pane's footer button.
- Result: the page is added after the last page, one step "Import page", and **becomes the
  active page** (P22, E5): the Pages list scrolls it into view, the view fits it. The Library
  tab stays shown, the focus stays where it was (keyboard) or goes to the canvas (mouse).
- Kind fallback (a laser or sketch page whose machine profile cannot be copied): the page comes
  in as a drawing page; status notice in the canvas notice slot, 5 s: "Front came in as a
  drawing page: machine profile Muse is missing." The reason after the colon is the one the
  core gives.
- No notice after a plain success (the customer rule of 2026-10-31).

### 7.2 Elements: drag or Import elements (criterion 5)

- From the footer button: the picks land centred on the visible part of the drawing (spec
  Proposal kept), into the **active layer of the active page** (the `0050` rule for new objects,
  "Initial Layer" if the page has none; E6), one step "Import elements".
- After either route: the copies are the selection, the Select tool is active, the picks in the
  pane stay (a second drop makes a second copy, criterion 6), focus goes to the canvas (mouse)
  or stays on the button (keyboard).
- Linked clones whose original was not picked: status notice 5 s: "2 linked clones became
  paths: their originals were not picked." (one: "1 linked clone became a path: its original
  was not picked.").
- Active page is a sketch page: Import elements refuses with the Action notice under it
  (`role="alert"`, 8 s): "Elements cannot go onto a sketch page." No step.

### 7.3 Resources (criterion 8)

Nothing in the UI: fonts and gradients come along silently. A font that is not installed shows
the `0107` missing-font marks the text already has (Layers tooltip "Font missing: <face>").

### 7.4 Importing from the open document itself

Allowed (E10): its pages read from the saved file like any other, and the row carries "Saved
version" when it differs. The result is a copy with Came from like any import.

## 8. Came from (criteria 6, 7, 9)

### 8.1 Layers row

The row tooltip's second muted line (row "Row" of the Layers tab): "From Box job › Lid,
24 Nov 12:03" (document, page, the `0095` time format; the item name is not repeated, it is the
row). An object of an imported page carries it too (criterion 6: top-level objects).

### 8.2 Style tab: the Came from line (E3)

Called the **Came from line**, not "origin line": `0049` already has an "Origin line" in the same
tab (Select clones), and two lines of one name would mislead every later note.

- **Place**: the last block of the Style tab, after every Style section, 12 px above it a 1 px
  `--toolbar-icon` at 25 % divider (P23: it is information about the object, not a style; the top
  is reserved for the clone block and the origin line).
- **Anatomy**, `role="group"` named "Came from": line 1, `FileInput` 16 px muted, 4 px, 12 px
  muted text "From Box job › Lid", ellipsis, the tooltip whole plus "Imported 24 Nov 2026 12:03";
  then 8 px below, **Open source** (full-column outline text button, 244 x 28, the Select clones
  look) only while the source document is found in a known source (criterion 7). Gone: the line
  stays, the button is not in the tree (no "not found" text: its absence is the state; P24).
- **Several objects selected**: every one with the same document and page: the same line and
  button; different sources: "From 3 documents" without the button; some without Came from: the
  line counts only those that have it, "2 of 5 from Box job › Lid", with the button.
- **Imported page**: the same block, at the end of the page's Document tab (after Background),
  and the Pages list row tooltip line 2 "From Box job › Lid".

### 8.3 Open source

A press opens the source document by the same route as opening it from where it lives (the
workspace tree for a workspace document, the plain Open for a file in an added folder), behind
the File tab's unsaved-changes check (`0051`, its sentence "... before you open another
document?"), then activates the page by its stored identity. If the page is gone in the source:
the document opens on its first page and the notice "Lid has no page Front any more." (status,
5 s). If the file went between the selection and the press: alert in the Action notice "Lid
cannot be found any more." and the button goes.

## 9. Empty, loading, reading

| State | Look |
|---|---|
| No source (no workspace open, none recent, no folder added) | tree area: "No documents yet." 14 px muted, 24 px from its top; Add folder... stays in the footer. No illustration, no explanation (the `0095` empty-state rule). Browser: "No documents in this browser." (section 2.1) |
| A source with no document | the source row's count reads "0"; expanding shows nothing under it (no "Empty" row) |
| First open, index being built | rows appear as their source is read, in final order (no row jumps above the focused one); subject line "Reading 120 of 500"; the search filters what is read so far |
| Expanding a document | "Reading pages..." row after 150 ms (section 4.2); the page rows replace it in one frame |
| Preview pane reading a page | the page thumbnail scaled to fit as the placeholder, `aria-busy` on the body, after 150 ms a `LoaderCircle` 16 px at the body's centre; picking starts when the page is read |

## 10. Errors (criterion 10)

All in the row, never a dialog, never a notice; the other sources keep working. Every text is
formatted in Rust (`curvyo-ui-core`) from the core's reason:

| Where | Text |
|---|---|
| Source row | "Cannot be read: folder not found." / "Cannot be read: no access." / "Cannot be read: drive not connected." |
| Document row | "Cannot be read: not a Curvyo file." / "Cannot be read: damaged." / "Cannot be read: made by a newer Curvyo." (the `0095` texts) |
| Page row, thumbnail fails | no text; the empty thumbnail ground |
| Preview, page fails to read | the body shows "Cannot be read: damaged." 14 px muted centred; Import page and the picks are not in the tree |
| Import fails in the core | alert in the canvas notice slot, 8 s: "Front could not be imported: <reason>. Nothing was changed." |

A document that cannot be read does not expand; Enter on it does nothing more (the reason is
already on the row). A source that comes back (drive reconnected) is re-read when the tab is
shown again or the tree is scrolled to it, not polled (architect: the `0095` 2 s folder scan
may serve the open workspace).

## 11. Keyboard and focus

- Tab order in the tab: search field, tree (one Tab stop, roving focus), Add folder..., Add
  source... (`0138`); in the browser search field and tree only.
- In the tree: Up, Down, Home, End, Page Up, Page Down between rows (headings skipped); Right
  expands, then moves to the row's action cell (Import page, Remove); Left goes back to the
  name cell, collapses, then to the parent; Enter or Space on a source, folder or document
  toggles; **Enter on a page row opens it in the preview** (focus stays in the tree);
  `Ctrl+Enter` on a page row imports it (P25, the only shortcut of the tab, list focus only;
  Cmd on macOS); Delete on an added folder arms Remove. Escape returns focus to the canvas.
- A mouse press on a row keeps focus in the tree (the maker browses with the pointer and may
  continue with the arrows); a press on an import action returns focus to the canvas.

## 12. Accessibility

- Treegrid semantics of `0095`: `aria-level`, `aria-posinset`, `aria-setsize`, `aria-rowindex`,
  `aria-rowcount`, `aria-expanded` on expandable rows, `aria-selected` on the current page.
  Thumbnails `alt=""` (the name carries the meaning).
- One visually hidden polite region for the tab (index progress at most every 2 s, "Imported
  page Front" after a keyboard import, armed and removed states); the pane has its own (section
  6.4).
- Contrast: every text is `--toolbar-icon` or muted on `--panel-bg`, as in `0095`; the dashed
  pick boxes are the selection boxes of the canvas with their casing.
- Touch (`pointer: coarse`): heights in brackets in 4.2; action slots always shown; a row drag
  starts after a 400 ms hold (the Layers rule); in the pane a long press is Shift+click.

## 13. Large libraries and timing (criteria 3, 13)

- 500 documents x 8 pages: the virtualised tree as `0095`; thumbnails for visible rows only;
  the first screen of rows within 200 ms of thread CPU time (criterion 13).
- Search from the index within 100 ms for 1,000 documents (criterion 3); the tree re-lays only
  the rows on screen.
- The preview pane draws one page; past 10,000 objects the hover box follows at most once per
  frame.

## 14. Proposals (vetoable at the demo)

| # | Proposal |
|---|---|
| P1 | The library is its own tab "Library" (the spec's Proposal kept) |
| P2 | Its place in the strip: after Fonts |
| P3 | Glyph `LibraryBig` |
| P4 | No key for the tab in this slice |
| P5 | Browser: the stored documents grouped by the browser's workspaces under "In this browser" (the listing itself is Frage 341 C) |
| P6 | Footer with Add folder... and, from `0138`, Add source... (118 px each) |
| P7 | Added folders in the order they were added |
| P8 | The open workspace's name semibold |
| P9 | A document never opens from the library (the File tab opens) |
| P10 | "Saved version" in the row's line 2 and the preview header |
| P11 | Remove of an added folder is an armed `X`, never a trash |
| P12 | Nested added folders are allowed and both shown |
| P13 | Search keeps matching documents collapsed and documents with matching pages expanded |
| P14 | The matched part of a name in semibold |
| P15 | One side pane at a time (page preview or font preview) |
| P16 | A press on a page row opens the preview pane on it |
| P17 | The preview zooms by wheel, pans by Space-drag, fits by double-click |
| P18 | Picking follows the Select tool: click, Shift+click, marquee with its modes |
| P19 | Locked objects can be picked, hidden ones cannot |
| P20 | Drop badge for elements: `Copy` "Copy" |
| P21 | Drop badge for a page: `Import` "New page" |
| P22 | An imported page becomes the active page |
| P23 | The Came from line is the last block of the Style tab |
| P24 | Open source absent (no text) when the source is gone |
| P25 | Ctrl+Enter on a page row imports it |
| P26 | Browser: no footer in this slice |
| P27 | Browser empty state "No documents in this browser." |
| P28 | Browser storage that cannot be read: one line in the tree area, read again on show or focus |

## 15. Spec deltas for the product owner

| # | Criterion | Delta |
|---|---|---|
| E1 | 1 | Superseded by Frage 341 C (2026-11-24): both builds. For the browser, "there are no workspaces" reads "no recent list and no added folders": its stored documents live in the workspaces it holds and the tree groups them by those (ADR 0084 decision 10, note 7; section 2.1) |
| E2 | 5 | "Ctrl-click" becomes the Select tool's click and Shift+click; the "marquee in the thumbnail preview" is the Select marquee in the page preview pane |
| E3 | 1, 7 | "the Style tab's origin line" is the "Came from line" (the `0049` Origin line exists); it is also shown for an imported page in the Document tab and the Pages list tooltip |
| E4 | 5 | Elements cannot be dropped onto a sketch page (the `0099` refusal); add as a criterion or veto |
| E5 | 4 | The imported page becomes the active page |
| E6 | 5 | Imported elements go into the active layer of the active page |
| E7 | 11 | "Saved version" is guaranteed for the document open in this window; for other windows only if the architect's design knows their dirty state without reading their memory |
| E8 | 2 | Per-page thumbnails: the container holds one `thumbnail.png` today (`0095`); the page rows need a per-page thumbnail or a render on expand (architect, `adrs.md`) |
| E9 | 7 | Open source needs a stable identity of the source document and page in the Came from record (a path alone breaks on a move inside a workspace); architect |
| E10 | 4, 5 | Importing from the document open in this window is allowed (from its saved file) |

## 16. Customer question (answered)

Frage 341 (DOCLIB-WEB), the library in the browser build: **C**, answered 2026-11-24. The browser
build has the Library tab and lists the documents stored in the browser (section 2.1). Options A
(desktop only) and B (Online section only) are dropped.

## 17. Drafts

`docs/design-drafts/2026-11-library/`: `01-library-tab` (the tab: sources, an expanded document
with page rows, the current page, Saved version, an armed Remove, a read error, search with
matches, the empty state, and in figure E the browser build: "In this browser", no footer, its
empty state), `02-page-preview` (the split pane with three picks, the drag over the
drawing with the Copy badge, the page row drag with the New page badge, the Came from line in the
Style tab and the Layers tooltip). `03-online-section` belongs to `0138`.
