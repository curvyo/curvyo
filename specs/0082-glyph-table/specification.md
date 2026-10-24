# Glyph table: every character of the text's font, a click inserts it

Status: Ready (2026-10-24; customer accepted the feature; Frage 39 and 40 unanswered, built with defaults A; UX notes P1 to P7 applied as Proposal)
Priority: Should (R-FONT-007 neighbourhood; no own requirement yet, see Links)
Origin: Customer. Accepted 2026-10-23: the glyph grid of `0077` Frage 25 B ("every character the font has, a click inserts it at the text cursor: ornaments, special characters in script fonts"), listed in `0077` Out of scope as "a glyph grid of one family with click-to-insert". Items marked **(Proposal)** are the product owner's; they are safe defaults the customer can veto at the demo.
Depends on: `0074-text-objects` (text node, edit mode, caret, Text bar, next-text face), `0076-single-line-fonts` (SVG font glyphs, `<missing-glyph>`), `0077-font-preview` (the pane in the canvas area, split and full view, divider, Size slider, engine-drawn glyphs, `Face::missing_chars`).

## User value

As a maker who lasers a wedding sign in a script font or plots a label with a degree sign, a heart or a dingbat ornament, I want to see every character the font of my text really has and insert one with a click at the caret, so that I use the font's ornaments and special characters without hunting in the operating system's character map and without pasting a character the font does not have (which would cut as an empty box).

## Field reference

| Tool | Where | What is listed | Insert | Search, recent |
|---|---|---|---|---|
| Inkscape 1.x, Text > Unicode Characters (named "Glyphs" until 1.0, [release notes](https://wiki.inkscape.org/wiki/Translations:Release_notes/1.0/673/en)) | Docked dialog | Code points filtered by Script and Range drop-downs; characters the font lacks are drawn from a fallback font ([forum](https://alpha.inkscape.org/vectors/www.inkscapeforum.com/viewtopica077.html?f=6&t=5861)) | An Append button adds to the selected text; a report says it changed the text's font ([#2118](https://gitlab.com/inkscape/inbox/-/issues/2118)) | No name search; no recent list |
| Affinity Glyph Browser ([help](https://affinity.help/photo/en-US.lproj/pages/Panels/glyphPanel.html)) | Studio panel | Every glyph of the font, also unencoded ones | Double-click inserts | Search by glyph id "G+0131", code "U+00b0" or word "degree"; recently used row with Clear |
| Adobe InDesign / InCopy Glyphs panel ([help](https://helpx.adobe.com/uk/incopy/desktop/format-text/character-formatting/glyphs-special-characters.html)) | Panel | All glyphs, alternates per glyph | Double-click | Last 35 glyphs under Recently Used |
| LightBurn | None | - | Staff: extended characters are not supported; makers copy from Windows Character Map and paste, which often fails ([forum](https://forum.lightburnsoftware.com/t/is-there-any-way-to-include-pua-encoded-characters-from-a-font-package/3948), [forum](https://forum.lightburnsoftware.com/t/can-i-access-glyphs-using-the-mac-version-of-lightburn/25200)) | - |
| macOS Font Book | Own app | Repertoire grid per face | Copy only | - |

**What we do better.**
- **Only what the font has.** The table lists the characters the face itself draws; no fallback font fills a gap (Inkscape does), so what is inserted is what the laser cuts. It uses the same check as the preview's missing-glyph marker (`0077` criterion 18), so the two never disagree.
- **Beside the drawing, in the pane makers already know** (split or full view of `0077`), not a separate dialog; one click inserts at the caret and typing goes on.
- **Never changes the text's font** (the Inkscape report): the table shows the text's own face and inserts into it.
- **Single-line fonts too**: their glyphs are drawn as the centre lines the machine follows.
- **Ornaments in the Private Use Area** (where many commercial script and dingbat fonts put their swashes) are listed and insertable; LightBurn cannot insert them at all.

## Terms

- **Glyph table:** the view of this story, a mode of the `0077` preview pane (Part A).
- **Subject face:** the face whose characters the table shows: the face of the text in edit mode, else of the one selected text object, else the next-text face of the Text bar (`0074`). This is the face an inserted character is set in (Frage 39, default A).
- **Listed character:** a Unicode scalar value the subject face maps to a glyph other than glyph 0: for an OpenType face its character map (`cmap`), for an SVG font a `<glyph>` whose `unicode` is exactly that one character (`0076` criterion 14). Control characters (general category Cc) and unpaired surrogates are never listed. A character is listed exactly when `0077`'s missing-glyph check reports it as present.
- **Cell:** one listed character in the table.
- Notices at most 70 characters, in the slots `0074` uses. Budgets on the plan's machine (as `0074`).

## Acceptance criteria

### Part A: where it lives

1. Given the `0077` preview pane, then its header holds a mode switch with two cells, **Fonts** and **Glyphs** (a radio strip, one Tab stop, the look of the Fonts tab's tag strip; the UX notes place it in both header layouts of `0077` UX notes §2). Fonts is the tile grid of `0077`; Glyphs is the table of this story. The split, full view, divider, Close and Escape rules of `0077` criteria 2 to 5 and 27 hold unchanged in both modes.
2. Given the Text bar of `0074` while a text is in edit mode or exactly one text is selected, then it has a **Glyphs** icon button (Proposal), in a group of its own after the Style group; the Select bar's Text group (exactly one text selected) has the same button after its Style group (Proposal, UX notes P1). A press opens the pane in Glyphs mode (or switches an open pane to it). A second press while the pane shows Glyphs closes it. Opened by mouse, the focus returns to the edit sink, so typing goes on.
3. Given the pane is closed and opened again (Preview button or Glyphs button), then it opens in the mode it showed last (Glyphs button: always Glyphs); the mode is kept per user with the other preview state of `0077` criterion 6. (Proposal)
4. Given Glyphs mode, then the header's subject line names the subject face and its count: "Great Vibes Regular · 412 characters"; while the search holds text, "Great Vibes Regular · 3 of 412 characters"; for a missing face "Lobster Regular · missing". A face with no listed character shows "Mylines Regular has no characters to insert." in the grid (Proposal, UX notes P7). The Fonts tab's strip cell and search do not filter the table.

### Part B: what is listed

5. Given a subject face, then the table shows one cell per listed character, in code point order, and nothing else: no cell for a character the face maps to glyph 0 or does not map, no cell drawn from another font. Test: a fixture face with "A", "ä", "♥" (U+2665) and U+E000 mapped and "ü" unmapped lists exactly those four (plus its space if mapped); Source Sans 3 lists its whole `cmap` minus Cc characters.
6. Given the listed characters, then they are grouped by **Unicode block** (Basic Latin, Latin-1 Supplement, General Punctuation, Arrows, Dingbats, Private Use Area, ...), each group under a header row "Latin-1 Supplement · 96" (the name and the count of listed characters in it), blocks in code point order; a block with no listed character has no header. The header of the block at the top of the grid stays at the top while its cells scroll under it (sticky; Proposal, UX notes P6). The block table follows one stated Unicode version (architect; the plan names it).
7. Given an SVG font (`0076`), then its glyphs with a single-character `unicode` are listed as in criterion 5 and drawn as centre lines (1 screen-px stroke in the text colour, never filled, as `0077` criterion 15); `<missing-glyph>`, glyphs without `unicode` and glyphs whose `unicode` has several characters are not listed.
8. Given a cell, then its character is drawn from the subject face's own glyph by the text engine (as `0077` criterion 15, not as browser text), at the scale where the face's cap height is the Size of criterion 15, on a common baseline, centred on its advance and clipped at the cell's edge. A character with an empty outline (space, no-break space, zero-width characters) is a cell with a dotted box and a short label in the UI font ("SP", "NBSP", "ZWJ"; the label list is the UX engineer's).
9. Given a subject face with many encoded characters, then the count of criterion 4 equals the number of cells, and the Fonts mode tile of the same face shows no missing-glyph marker for a sample made of any listed characters and shows one for any character that is not listed (one check, `Face::missing_chars` of `0077`). Test: for a fixture face, the sample of all listed characters gives an empty missing list; each unlisted printable ASCII character gives itself.
10. Given the subject face changes (another text selected, a pick in the Fonts list or the Fonts mode, the next-text face set), then the table shows the new face within 300 ms for a face with 3,000 listed characters, scrolled to the top, and the search (Part D) is kept and applied to the new face.
11. Given no subject face can be read (the text's font is Missing, `0074` criterion 29), then the table shows "Lobster is missing. Its characters cannot be shown." in the empty-grid look of `0077`, and nothing can be inserted from it. (Permit over refuse does not apply: there is no glyph to show.)

### Part C: inserting

12. Given a text in edit mode, when the maker clicks a cell (or presses Enter or Space on the focused cell), then the character is inserted at the caret, replacing the text selection if there is one, exactly as if it had been typed (`0074` criteria 2 to 10: one keystroke for undo inside the edit session, the 10,000-character limit). The text's face does not change. Clicked by mouse in split view, the focus returns to the edit sink so typing goes on; inserted by keyboard, or clicked by mouse in full view (the canvas is hidden), the focus stays on the cell so several characters can be inserted in a row (Proposal, full view: lead decision 2026-10-24).
13. Given exactly one text object selected and not in edit mode, when a cell is clicked, then that text enters edit mode with the caret at the end of its last line and the character is inserted there (criterion 12 from then on). (Proposal; the alternative "append without entering edit mode" leaves the maker without a caret to continue.)
14. Given no text object in edit mode and not exactly one text selected (nothing selected, other objects, or several texts), when a cell is clicked, then a new text with that one character is created in the subject face (the next-text face) at the Text bar's Size, on the active layer, centred on the visible part of the drawing (the part the pane leaves free, `0077` criterion 2), and is in edit mode with the caret after the character; one history step "Add text" as the Text tool's (`0074`). In full view the text is created the same way and seen when the maker returns to split view or closes the pane. (Proposal: permit over refuse, a click always does something visible.)
15. Given the Size slider of the pane in Glyphs mode, then it sets the cells' cap height on screen with the range, default, tooltip and slider behaviour of `0077` criterion 12; the value is shared with Fonts mode (one slider, one stored value). A cell is a square of side S = max(40px, 2 x size) with a 4px gap, columns of exactly S laid from the left, as many as fit the pane's width; the space left over stays at the right (Proposal, UX notes P2). The cells re-flow within 100 ms of a change and are not drawn again for it (as `0077` criterion 17).

### Part D: search, recent, detail

16. Given Glyphs mode, then the header's sample field of `0077` is replaced by a **search field** (same look and place, one line, at most 64 characters, its own stored text, not the sample). Typing filters the cells within 100 ms of the keystroke for 3,000 listed characters; a query of one character matches that character only (exact match, not by substring in glyph or block names; Proposal, lead decision 2026-10-24); a query of two or more characters matches a cell whose code point equals the query as `U+00E4`, `u+e4` or 4 to 6 hex digits, whose Unicode character name contains every word of the query ignoring case ("heart" finds U+2665 and U+2764 when listed; "a diaeresis" finds "ä" and "Ä"), whose glyph name in the face (`post` table, SVG `glyph-name`) contains the query, or whose block name contains the query ("arrows" lists the Arrows block). Exact character and code point matches come first, then the rest in code point order, still under their block headers. No match: "No character in Great Vibes matches "xyz"."
17. Given Unicode character names (criterion 16), then they are part of the shipped app and work offline (R-SYS-002); they add at most 1 MiB to what the browser loads, and they are loaded on the first use of Glyphs mode, not at start (architect: lazily fetched asset or in the wasm; the text engine stays inside its 2 MiB budget of `0076` Frage 20).
18. Given characters inserted from the table, then a **Recent** row above the first block shows the last 24 distinct characters inserted from the table, most recent first, drawn in the subject face; a recent character the subject face does not list is not shown in that face's Recent row (it stays in the list). The list is kept per user with the preview state (`0077` criterion 6), not per face, and not in the project. With an empty list, the row is not there. The row shows at most two rows of cells (the rest stays in the list) and is not shown while the search holds text; its order does not change while the pointer is over it, and when it appears or grows the scroll position moves by the same height so the cell under the pointer or the focus does not move. (Proposal; Affinity and InDesign have it; UX notes P3.)
19. Given the pointer on a cell or the keyboard focus on one, then a **detail bar** at the bottom of the pane, always present in Glyphs mode (it never comes and goes), has a fixed height of 97px and shows that character in an 80px box at cap height 40px whatever the Size, its code point "U+00E4", its Unicode name "LATIN SMALL LETTER A WITH DIAERESIS", when the face has one its glyph name "adieresis", its block name, and an **Insert** button that inserts it as a click on the cell does (Proposal, UX notes P4); with neither hover nor focus, the focused cell's details, else the bar is empty. The name and code point are selectable text, so they can be copied.

### Part E: keyboard, accessibility, budgets

20. Given the keyboard, then the table is one Tab stop after the search field, the Size slider and the header buttons (as the `0077` grid); arrows move between cells (Up and Down by a row, across block headers), Home and End go to the first and last cell of the row, Ctrl+Home and Ctrl+End to the first and last cell, Page Up and Page Down by a screen, Ctrl+Up and Ctrl+Down to the first cell of the previous or next block; Down or Enter in the search field moves the focus to the first cell; Backspace in the grid goes to the search field like a printable character (Proposal, UX notes P5); Enter or Space inserts (criteria 12 to 14); a printable character goes to the search field; Escape closes the pane as `0077` criterion 5. Each cell's accessible name is the character name and code point ("Latin small letter a with diaeresis, U+00E4"), block headers are group labels.
21. Given a face with 3,000 listed characters, when Glyphs mode opens, then the first screen of cells is drawn within 500 ms; given a face with 20,000 or more (a CJK face, the 65,535-glyph limit), the first screen within 1 s, the table is virtualised (visible rows plus one screen in the page), and scrolling top to bottom leaves no cell blank longer than 300 ms after scrolling stops. A cell is drawn once per face and character; neither a Size change nor a pane width change draws it again. Drawing the table does not slow the drawing: the canvas drag budget of `0077` criterion 23 holds with Glyphs mode open.
22. Given the browser build, then criteria 1 to 21 hold the same way for the fonts the browser lists (`0075` criterion 28, `0077` criterion 24).
23. Given the core crates, then they read no file: listing the characters of a face (character map, SVG glyphs, glyph names), the block grouping and the search match are pure functions on bytes and strings in a `*-core` crate that builds for `wasm32-unknown-unknown` (`CLAUDE.md` §6). An inserted character is ordinary text: no project `format_version` change.

## Out of scope

- **Unencoded glyphs**: alternates, swashes, stylistic sets and ligatures reachable only through OpenType substitutions (`GSUB`: `salt`, `swsh`, `ss01`..`ss20`, `liga`, `dlig`), and SVG font glyphs with multi-character `unicode`. Inserting one needs the text to store a glyph or a feature per character, which is a document model change (Frage 40, default A: a later slice). Alternates a font puts in the Private Use Area are encoded and are in this slice.
- Alternates per character (InDesign's pop-out next to a cell), OpenType feature switches for a whole text.
- A glyph table of a font that is not the subject face (browsing another family without picking it, Frage 39 B), several faces side by side, a face switcher inside the table (the face rows of the Fonts list pick a face).
- Filtering by script or language (Inkscape's Script drop-down); a block jump list (the search on block names covers it).
- Copying a character to the clipboard from a cell (the detail bar's text can be selected and copied), drag and drop of a cell onto the canvas.
- Clearing or editing the Recent list (Affinity's "Clear Recently Used"); Recent per face.
- Emoji with variation selectors or skin-tone sequences as one cell; colour glyphs (COLR, SVG-in-OpenType, bitmap) are drawn as their outline glyph if any.
- Editing glyphs, a font editor, exporting a glyph as a path (insert and Object to path does it).
- A glyph's kerning or metrics shown in the detail bar.

## What the architect must decide (`adrs.md`)

1. **Listing a face's characters**: a `curvyo-text-core` function over the `cmap` (all subtables `skrifa` exposes, format 4, 12, and the symbol `(3,0)` subtable) and the SVG font glyphs, returning code points, glyph names and the empty-outline flag; how it agrees with `Face::missing_chars` by construction (criterion 9).
2. **Unicode data**: the block table and character names (criteria 6, 16, 17): which crate or generated table, its licence (`cargo deny`; the Unicode data licence), and whether names ship in the wasm or as a lazily fetched asset within 1 MiB.
3. **Drawing cells**: reuse of `preview_tile` and the `preview_faces` budget of `0077` decision 2 for single characters; batching per visible row so 20,000-character faces meet criterion 21.
4. **Insertion commands**: the editor-wasm commands for "insert at caret", "enter edit mode at end and insert" and "new text with this character at the visible centre" (criteria 12 to 14), reusing the Text tool's commit paths, one history step each as stated.
5. **Where the mode, search text and Recent list live**: the `curvyo.font-preview` `localStorage` key of `0077` decision 4 with added fields, or a key of its own.

## UX notes

See [`ux-notes.md`](ux-notes.md) (ux-engineer, 2026-10-24; §0 proposed changes P1 to P7 to criteria 2, 4, 6, 15, 18, 19 and 20; applied above as Proposal on 2026-10-24).

## Questions for the customer

Frage 39 (GLYPH-SUBJECT): Which font does the glyph table show?
- A: The font of the text you are editing or have selected (else the font the next text gets). A click inserts into that text in that font. To see another font's characters, pick it first (a click on a tile or a list row; Ctrl+Z goes back).
- B: Any font: a glyph button on every tile and family row opens that font's table without picking it. A click on a character then also changes the text to that font (or inserts a character the text's font may not have).
- PO recommendation: A. The table answers "what can I type in this font", and inserting never changes the font behind your back (the Inkscape bug). B adds a button to every tile and row and a rule for what happens to the text's font.
- Default if unanswered: A.

Frage 40 (GLYPH-ALT): Script fonts often keep their swashes and alternates as hidden glyphs that only OpenType features reach (no Unicode number). Include them now?
- A: Not in this slice. The table lists every character with a Unicode number, the Private Use Area included (many commercial script fonts put their alternates there for exactly this reason). Hidden alternates come as a later slice that lets a text remember "this letter uses alternate 2" (a project format change).
- B: Yes, in this slice: the table also lists hidden glyphs, and the text stores the chosen glyph per letter (a format change, about ten more criteria, and Object to path, Weld and the preview must honour it).
- PO recommendation: A. It ships the table soon and covers dingbats, special characters and PUA ornaments; B doubles the slice and touches the document model.
- Default if unanswered: A.

## Links

Requirements: none of its own yet; R-FONT-007 (preview page it extends), R-FONT-002 ("symbols/ornaments"). The product owner adds R-FONT-012 "glyph table with click to insert" to `docs/requirements.md` when the folder is registered in `specs/README.md`.
Depends on: `specs/0074-text-objects/`, `specs/0076-single-line-fonts/`, `specs/0077-font-preview/`
Related: `specs/0083-font-info-and-zip/` (its Info mode joins the same mode switch)
ADRs: 0020 (text engine), 0001 (UI and canvas)
PR: -
