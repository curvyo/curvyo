# Font preview: the maker's lettering in every listed font, beside the drawing

Status: Ready (2026-10-23; Frage 25 to 28 decided A; UX notes and adrs.md done)
Priority: Should (R-FONT-007)
Origin: Customer. A story of the fonts epic `0062` (stage 3, its criteria 14 to 17 and the decision FONT-PREVIEW of 2026-10-17: split view next to the drawing, switchable to a full view, no popup). Request of 2026-10-23 (translated): "I'd like to keep going and finish fonts, with a good-looking font preview panel." Items marked **(Proposal)** are the product owner's; they are in this slice as safe defaults and the customer can veto them at the demo.
Depends on: `0074-text-objects` (text node, Fonts tab, picking a face, "Change font"), `0075-font-library` (tag strip, search, favourites, the font library file), `0076-single-line-fonts` (single-line faces drawn from the engine's outlines). Builds after `0076` is merged.
Replaces: `0062` criteria 14 to 17 (the preview page). The pane host of `0063` is not needed first (Frage 26 A): this story builds the split, `0063` reuses it.

## User value

As a maker who lasers names onto chopping boards or plots a wedding sign, I want to type my lettering once (or take it from the text I selected) and see it set in every font of the tag I chose, next to my drawing, and pick one with a click, so that I choose a font by looking at my own words, umlauts included, instead of trying fonts one at a time on the canvas.

## Field reference

| Tool | Own sample text in many fonts at once | Size | Where it lives | Glyphs, info |
|---|---|---|---|---|
| PewPew Studio (the customer's reference, `0062`) | Yes: the lettering in all fonts as a grid of tiles; click one to use it | - | A page of its own | - |
| Inkscape 1.4, Text and Font dialog; the experimental Unified Font Browser (off by default) | Old dialog: one font at a time. The font browser: previews of all fonts with an own sample text, filter by collection ([1.4 release](https://inkscape.org/news/2024/10/13/inkscape-version-1-4-zhtw)); known bugs with `&` and `<` in the sample and cut-off previews ([#10269](https://gitlab.com/inkscape/inbox/-/issues/10269), [#10792](https://gitlab.com/inkscape/inbox/-/issues/10792)) | Setting | A docked dialog | Glyphs dialog, separate |
| LightBurn 2.1 | No: Font Manager lists names to view, search, favourite and hide ([docs](https://docs.lightburnsoftware.com/2.1/Reference/Text/)); users report the dropdown showing every font in one block face ([forum](https://forum.lightburnsoftware.com/t/fonts-style-don-t-display-correctly-in-drop-down/175267)) | - | Dropdown and manager | - |
| Affinity Designer | Hovering a name in the font list previews the selected text in that font on the canvas ([user report](https://forums.steinberg.net/t/apply-for-font-testing/107609)) | - | Font dropdown | Glyph Browser panel ([help](https://affinity.help/designer2/en-US.lproj/index.html)) |
| Corel Font Playground / Font Sampler | Same sample text in several chosen fonts, size per sample ([help](https://product.corel.com/help/Designer/Documentation-Windows/Corel-DESIGNER-en/Corel-DESIGNER-Choosing-fonts-with-Font-Playground.html)) | Per line | Docker | - |
| macOS Font Book | Grid, Sample and List views; custom text per style in older versions; Waterfall only in print ([Apple](https://support.apple.com/en-us/guide/font-book/fntbk1001/mac)) | Slider | Own app | Repertoire, Info (file, maker, languages) |
| Google Fonts | Type a sample, it is set in every listed family; size slider; a glyph section per family | Slider | Web page | Glyphs, licence per family |
| FontDrop! (web) | One file at a time: glyph grid, waterfall, name and licence info ([listing](https://alternativeto.net/software/fontdrop-/about)) | Waterfall | Web page | Yes |
| Cricut Design Space | No: names only; filters Writing, Single layer, Kerned | - | Font picker | - |

**What we do better.**
- **Your lettering, not a pangram,** taken live from the selected text: type on the canvas and every tile follows. Inkscape's browser needs an own sample and is off by default; LightBurn has none.
- **Beside the drawing, not instead of it.** Split view keeps the design in sight and editable; full view uses a wide screen. No popup, no separate app (Font Book, FontDrop).
- **One filter for list and preview.** The tag strip (Cuttable, Single line, Script, own tags) and the search of the Fonts tab decide what the preview shows, so "show me my lettering in every font that cuts in one piece" is two clicks. No other tool has a measured Cuttable filter.
- **Missing letters are shown, not hidden.** A tile says which of the typed characters the font has no glyph for (an "ä" in a German name), before the maker cuts it.
- **Single-line fonts are previewed as the lines the machine will follow**, not as filled shapes.

## Terms

- **Preview:** the view of this story, a pane in the canvas area (not in the Properties panel, not a popup).
- **Sample:** the line of text the tiles show.
- **Listed families:** the families the Fonts tab list shows at this moment: the chosen strip cell and the search (`0075` criteria 1 to 3), in the list's order (blocks, Curvyo fonts first, A to Z).
- **Tile:** one family in the preview.
- Notices at most 70 characters, in the slots `0074` uses. Budgets are measured on the plan's machine (as `0074`).

## Acceptance criteria

### Part A: opening, split and full

1. Given the Fonts tab, then a **Preview** icon button (toggle, `aria-pressed`) sits in the search row between the search field and Add font…. A press opens the preview; a second press closes it. The button is in the browser build too. (No key in this slice, Proposal.)
2. Given the preview opens, then it takes the **right part of the canvas area** (split view), next to the Properties panel: the drawing keeps the left part with its rulers and bars and stays fully usable (select, edit, type, zoom, pan). The drawing's view is not re-zoomed; what was at the left stays where it was. A 1px border separates the two. The pane lies over the drawing's canvas, which keeps its size: opening, closing, switching views and dragging the divider never resize or redraw the canvas, only a window resize does (customer report 2026-10-23: the canvas flickered on every resize). Fitting and centring the drawing, the rulers, the bars and every "visible part" rule use the part the pane leaves free, and a press on the pane never reaches the drawing.
3. Given split view, then the preview is 35 % of the canvas area wide, or as wide as one full tile column at the current Size (criterion 13) if that is wider, so the pane widens with the Size slider (customer decisions 2026-10-23: replaces "half the canvas area, at least 320px", then 30 % raised to 35 % after the demo). When a split would leave the drawing narrower than 360px (an 800 x 600 window, or a large Size in a small window), the preview opens in full view instead. While that holds, the Full view button of criterion 4 is not shown (not shown, not disabled) and the pane stays in full view; this forced full view does not overwrite the stored split or full choice of criterion 6, and widening the window (or lowering the Size) does not switch the view by itself.
4. Given the preview, then a **Full view** symbol button in its header switches to a full view that fills the whole canvas area (the drawing is hidden, not closed; rulers and the tool bars of the drawing are hidden with it); the same button switches back to split view. Its tooltip names what it does ("Full view", "Split view"). (For a drawing that would keep under 360px see criterion 3.)
5. Given the preview, then a **Close** button in its header, Escape with the focus inside the preview (the sample field included: Escape there closes the preview and keeps the sample, it does not clear the field), the Preview button of criterion 1, and Escape with the focus outside the preview once nothing else has an Escape step to take (the canvas's Escape ladder runs first: close an entry, cancel a drag, clear the tool state or selection, leave text edit or a group; a control with its own Escape keeps it; when none of them acts, that Escape closes the preview, and repeated presses are never swallowed; customer decision 2026-10-23) close it; the drawing gets the whole canvas area back with the same view as before. Closing writes nothing into the project and no history step.
6. Given the app is closed with the preview open, when started again, then the preview is closed; split or full, the sample and the size of criteria 8 and 12 are kept per user, which means per browser profile and per computer (webview storage, `adrs.md` decision 4; not synced between computers, not in the font library file). (Proposal)

### Part B: the sample

7. Given the preview, then its header holds a **sample field** (one line, at most 200 characters, white ground) with the sample, and every tile sets exactly that sample.
8. Given the preview opens, then the sample is: the first line of the selected text object if exactly one text object is selected, else the sample last typed into the field (kept per user), else **"Anna & Jonas 2026"** (Proposal: capitals, lowercase, an ampersand and digits, the shape of a wedding sign).
9. Given the preview is open, when the selection changes to exactly one text object, then the field takes its first line (up to 200 characters). While that text stays selected, editing it on the canvas updates the field and all visible tiles within 150 ms of the keystroke (30 visible tiles).
10. Given the maker types into the sample field, then the visible tiles show the new sample within 100 ms of the keystroke (30 visible tiles). Typing in the field never changes the document. The typed sample stays until the next selection change of criterion 9.
11. Given an empty sample field, then every tile shows its family name in its own face, as the Fonts tab row does, as one line cut off at the tile's edge like a sample (criterion 13; permit over refuse: no empty grid, no refusal).

### Part C: tiles

12. Given the preview, then a **Size** slider in its header sets the sample's cap height on screen from 16 to 160px, default 32px (Proposal; lead decision 2026-10-23 after the UX review, was 48px); click and double-click behave as on every slider (`0047`). Its tooltip is the name and value only, "Size: 32 px", without a usage hint line (customer rule of 2026-10-16, no hint labels). The tiles re-flow within 100 ms of a change.
13. Given the listed families, then the preview shows one tile per family, in the list's order, as a grid of equal-width columns: a column is at least max(240px, 5 x size) wide (a size of 160px gives one column), as many columns as fit. A tile shows exactly **one line**: the sample (its first line; the field is one line anyway) set as far as the tile is wide, the rest **cut off** at the tile's right edge (a hard clip: no wrapping, no ellipsis, no word-break rule). All tiles of one size are one line high and equally high, the footer's two rows included (criterion 14). (Customer decision 2026-10-23 after the demo, replaces "1 to 3 reserved lines, ending with an ellipsis".)
14. Given a tile, then it shows the sample set in the family's Regular (or first) face, below it a row with the family name in the UI font at 12px and a **star** (the favourite of `0075` criterion 9, the same state as the list row), and under that a row of its own with **all the family's tags** (customer request 2026-10-23; the row is kept on every tile, a family without tags shows only Add tag) as the chips of the Fonts tab's tag line (`0075` criterion 10, the same component): each chip's x removes that tag from the family with the list's rules (built-in tags included, no history step, the library file written), "+N" when they do not fit beside the name (it shows every tag over the sample until the pointer leaves the tile or Escape), and **Add tag** (+) with the list's inline completion. A press on a chip, its x, "+N" or Add tag never picks the font (as the star, criterion 20), and no tile moves or leaves the grid while its tags change (the row stability of `0075`; criteria 16 and 20). Keys on a focused tile as on a list row: Shift+F2 enters the tag line, F2 opens Add tag, Escape in the line returns to the tile. (Customer decision 2026-10-23, replaces "Cuttable and Single line only".)
15. Given an outline face, then the tile is drawn from the face's glyphs by the text engine, not as browser text, so what the maker sees is what the cut gets, `.notdef` included (`adrs.md` decision 2; the browser would fill a missing "ü" from another font); given a single-line face (`0076`), then the sample is drawn as the glyphs' centre lines in the tile's text colour and never filled (criterion 6 of `0076`, at the tile's size).
16. Given the strip cell or the search of the Fonts tab changes while the preview is open, then the preview shows the new listed families within 100 ms for 2,000 families (`0075` criterion 2), scrolled to the top. A family that leaves the cell because the maker starred it or removed a tag stays until the cell or the search changes (the row stability rule of `0075`).
17. Given a family whose faces have not been read yet (This computer while "Reading…", a face loading), then its tile shows the name and a quiet muted bar in the sample area, never the sample in another font, and swaps in place when the face is loaded (customer decision 2026-10-23, replaces "the name in the UI font with a muted sample placeholder"); tiles load like rows (after 100 ms in view, visible tiles plus one screen ahead); no tile changes height when it loads. Once the Fonts tab lists families, their faces are read and their tiles drawn ahead while the app is idle, those in view first, so the preview opens with its visible tiles drawn; a tile is drawn once per face and sample, and changing the Size or the pane's width never draws it again.
18. Given a sample with characters the family's face has no glyph for, then the tile draws what the face draws for them (`.notdef` or `<missing-glyph>`, as `0074` criterion 6 and `0076` criterion 14) and shows a quiet marker at the tile's corner (`TriangleAlert` 14px, muted); its tooltip lists the characters, at most 10 ("No glyph for: ä, ß"). The family is not hidden. Test: sample "Jürgen" in a fixture face without "ü" shows the marker with "ü"; Source Sans 3 shows none.

### Part D: picking

19. Given a press on a tile (not on its star), then the family's Regular (or first) face is applied exactly as a family row press in the Fonts tab (`0074` criterion 18): to the selected text objects in one history step "Change font", or to the next text with no text selected. The tile of the family that holds the current face is marked as current (the selected look of the face row: `--value-fill` and the 3px `--accent` bar), and the Fonts tab list marks the same family.
20. Given a press on a tile's star, then the favourite toggles as in the list (no history step, the library file is written), nothing is applied, and the tile does not move.
21. Given a pick by mouse, then the focus returns to the canvas (the edit sink in edit mode), as in the list; the sample field keeps the sample (with a selected text it becomes that text's first line, which is the same).

### Part E: keyboard, size, budgets

22. Given the keyboard, then the tile grid is one Tab stop after the sample field, the Size slider and the header buttons; arrows move between tiles (Up and Down by a row of the grid), Home, End, Page Up, Page Down; Enter or Space picks; Ctrl+D toggles the star; a printable character goes to the sample field; Escape closes (criterion 5, also from the sample field). Each tile's accessible name is "Lobster, favourite, Cuttable, current" (parts only when true), and the sample is its description, followed by the missing characters of criterion 18 when there are any ("Anna & Jonas 2026. No glyph for: ä").
23. Given 500 listed families on the desktop, when the preview opens, then the first screen of tiles is shown (names, and samples of the faces already loaded) within 1 s, and every visible tile has its sample within 2 s; with 2,000 families the grid is virtualised (only visible tiles plus one screen are in the page) and scrolling from top to bottom leaves no tile blank longer than 500 ms after scrolling stops. Opening the preview does not slow the drawing: a drag on the canvas in split view keeps its frame rate within 10 % of the same drag with the preview closed. A divider drag (criterion 27) keeps the display's frame rate: it re-flows the tiles without drawing them again and lays the grid out anew only when a column comes or goes (customer report 2026-10-23).
24. Given the browser build, then criteria 1 to 23 and 26 hold the same way with the fonts the browser lists (`0075` criterion 28).
25. Given the core crates, then they read no file and know no screen: if the missing-glyph check of criterion 18 or the single-line drawing of criterion 15 needs the text engine, it is a pure function on bytes and a string in `curvyo-text-core` (`CLAUDE.md` §6); the preview itself is front-end and session view state. No project `format_version` change.

### Part F: the preview and the Properties panel (belongs with Part A; appended to keep the numbers)

26. Given the preview is open, when the maker switches the Properties panel to another tab (Style, for example) or collapses the panel, then the preview stays open and keeps showing the Fonts tab's listed families (strip cell and search as last set), and a subject line in its header names that filter and the count in the Fonts tab's words ("All · 1,240", "Cuttable · 3 of 42" with a search). Picking a tile (criterion 19) works the same from any tab.
27. Given split view, when the maker drags the divider between the drawing and the preview, then the pane resizes live and the drawing's view is not re-zoomed (criterion 2); the pane stays at least one full tile column at the current Size wide and the drawing at least 360px (criterion 3; when both cannot hold, the forced full view of criterion 3 applies). The width is kept as the pane's share of the canvas area with the state of criterion 6 and used instead of the 35 % default from then on, so it scales with the window. A double-click on the divider returns to the default and forgets the kept share. The divider is a focusable `separator` (vertical) in the Tab order after the drawing and before the sample field; Left and Right move it by 16px. (Customer decision 2026-10-23, replaces "Dragging the split divider" in Out of scope.)

## Out of scope

- A glyph grid of one family with click-to-insert into the text (Frage 25 B; next slice candidate).
- A font info view: licence, file, format, embedding permission (`fsType`, R-FONT-008), version (Frage 25 C).
- A live preview of the hovered tile's font on the selected text in the drawing (Frage 27 B).
- Tiles per face (Bold, Italic): a tile shows the family's Regular face; faces stay in the list.
- A waterfall (one font at several sizes), a size in mm at the drawing's zoom, line height or letter spacing controls, a second sample line.
- Putting the preview on the left or below, a preview window of its own (detached), the `0063` workbench switcher.
- Comparing two or three chosen fonts side by side (beyond the favourites cell, which already does this).
- Printing or exporting a specimen sheet.
- Downloading fonts (Google Fonts) or previewing fonts that are not listed (not added, not installed).
- Choosing several tags at once (`0075` out of scope).

## What the architect must decide (`adrs.md`)

Answered in [`adrs.md`](adrs.md) (architect, 2026-10-23): no new ADR, crate, dependency or format change; 1 a `CanvasArea` component with one side pane (dated note); 2 every tile from the engine's outlines; 3 `Face::missing_chars`; 4 a per-user `localStorage` key; 5 two fields in the Text bar state.

1. **The canvas area split:** how the editor's canvas area hosts a second pane (split and full) now, so that `0063`'s workbench shell can reuse it later instead of building a second one; the drawing keeps its view (criterion 2) and its frame rate (criterion 23). Whether this needs an ADR or a dated note.
2. **Drawing tiles:** outline faces through the browser's `FontFace` as the list rows do, or through the engine's outlines; single-line faces from the engine's outlines (as `0076` rows); how 30 tiles redraw within 100 ms of a keystroke, and whether a tile's layout may differ from the engine's shaping (kerning) by a little (the tile is a look, not the cut).
3. **Missing-glyph check (criterion 18):** a coverage query on the face store per family and sample, cached per face and sample.
4. **Where the sample, size and split/full state live** (criterion 6): keys in the font library file (`format` unchanged if optional keys are additive) or a separate per-user view settings file.
5. **Which session state the preview follows** (criterion 9): the selected text's first line and its changes while editing, without a new document read per keystroke beyond what the Text bar already reads.

## UX notes

See [`ux-notes.md`](ux-notes.md) (ux-engineer, 2026-10-23). Its proposed changes P1 to P6 are applied: P1 in criteria 3 and 4, P2 in 13 (superseded by the one-line rule of 2026-10-23), P3 in 12, P4 in 22, P5 in 5 and 22, P6 as criterion 26.

## Decided

Customer, 2026-10-23, second feedback on the Linux desktop: no flicker when grabbing or releasing the divider (criterion 2: the pane lies over the canvas, which keeps its size), a divider drag at the display's frame rate (criterion 23), tiles never shown in a wrong font while loading and drawn ahead (criterion 17), the tags in a row of their own (criteria 13 and 14).

Customer, 2026-10-23, feedback on the preview (three changes): the drawing must not flicker while the divider is dragged (a fix, no criterion changes); the split's default width is 35 % instead of 30 % (criterion 3); a tile shows one line, cut off at its edge, instead of wrapping into 1 to 3 lines with an ellipsis (criteria 11 and 13).

Customer, 2026-10-23 ("so kann es mit den spec weiter gehen", after seeing Frage 25 to 28 with their defaults): Frage 25 A, Frage 26 A, Frage 27 A, Frage 28 A. The questions as they were put follow for the record. The Proposal defaults under "Not put to the customer" are not decided by this; they stay vetoable at the demo.

Frage 25 (PREVIEW-WHAT): What is the first slice of the "font preview panel"?
- A: Your lettering set in every listed font as a grid of tiles, with size, star and click to use (criteria above). Glyph grid and font info come later.
- B: A, plus a glyph grid of the hovered or chosen font: every character the font has, a click inserts it at the text cursor (ornaments, special characters in script fonts).
- C: A, plus a font info line per font: licence, file, format (TTF, OTF, SVG font), embedding permission.
- PO recommendation: A. It is what PewPew does and what you described on 2026-10-17; B and C are separate slices that each fit in one PR afterwards.
- Decided: A (customer, 2026-10-23).

Frage 26 (PREVIEW-HOST): `0062` says the split and full view live in the pane host of the manufacturing workbenches (`0063`), which is not built. Where does the preview go now?
- A: This story builds a small split and full view of the canvas area now, just enough for the preview; `0063` reuses it later.
- B: Wait: build the workbench shell of `0063` first, then the preview inside it.
- C: No split: the preview replaces the font list inside the Properties panel (narrow, about 244px, one font per row).
- PO recommendation: A. You want to finish fonts now; B pulls manufacturing work forward; C is too narrow to compare lettering.
- Decided: A (customer, 2026-10-23).

Frage 27 (PREVIEW-HOVER): Should resting the pointer on a tile show the selected text in that font in the drawing, before clicking?
- A: No, not in this slice. A click applies the font, Ctrl+Z goes back.
- B: Yes: while the pointer rests on a tile (300 ms), the selected text is drawn in that font in the drawing as the blue preview the app uses elsewhere; leaving the tile restores it; nothing is written until a click. (Affinity does this from its font list.)
- PO recommendation: B, because in split view you see the font in your actual design at its real size. It adds about five criteria and some risk in the drawing code.
- Decided: A (customer, 2026-10-23); the hover preview can follow as its own small slice.

Frage 28 (PREVIEW-SAMPLE): Which text do the tiles show?
- A: The selected text's first line; with no text selected, the line you last typed into the preview's own field (criteria 8 to 10).
- B: Always the preview's own field; selecting a text does not change it.
- C: Always the selected text; no own field (with nothing selected, the font names).
- PO recommendation: A: you can try fonts before you have a text, and once you have one, the preview shows your real words.
- Decided: A (customer, 2026-10-23).

### Not put to the customer

Proposal defaults, vetoable at the demo: the default sample "Anna & Jonas 2026" (criterion 8), the size range and default (criterion 12), only the Cuttable and Single line chips on tiles (criterion 14), no key to open the preview (criterion 1), split/full and the sample kept between starts but the preview starting closed (criterion 6).

## Links

Requirements: R-FONT-007 (preview page), R-FONT-002 and R-FONT-003 (the filter it shares), R-FONT-005
Depends on: `specs/0074-text-objects/`, `specs/0075-font-library/`, `specs/0076-single-line-fonts/`, `specs/0062-fonts/` (stage 3, decision FONT-PREVIEW), `specs/0047-transform-polish/` (slider behaviour)
Related: `specs/0063-manufacturing-workbenches/` (pane host), `specs/0068-laserable-lettering/`
ADRs: 0020 (text engine), 0001 (UI and canvas); the canvas area split is the architect's (decision 1)
PR: -
