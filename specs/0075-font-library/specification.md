# Font library: tags, favourites, Add font and the Cuttable tag

Status: Done (PR #168, merged 2026-10-23). Ready 2026-10-20; customer questions 11 to 13 decided 2026-10-20, see "Open questions".
Priority: Should (R-FONT-002, R-FONT-003, R-FONT-006 are Should; R-FONT-001 is a Must and already met by `0074`)
Origin: Customer. Stage 2 of `0062-fonts` (request of 2026-10-17: "groupable like the document sizes, e.g. initials, script, sans, serif, cuttable", "mark with * as favourites", "our own folder ... to add fonts just for Curvyo", "a tool to add"). Items marked **(Proposal)** are the product owner's; they are in this slice as safe defaults and the customer can veto them at the demo (as in `0074`, "Not put to the customer").
Depends on: `0074-text-objects` (Fonts tab, blocks, Curvyo folder, font matching), `0016-boolean-operations` (the Union kernel the cuttable check counts with), `0045-document-formats-library` (the per-user file and the strip pattern), `0051` (panel tabs), `R-SYS-010` (inline confirmation). No longer depends on `0068` (Weld was redefined, 2026-10-20).
Replaces: `0062` criteria 2, 3, 6 to 9, 11 to 13 and its Frage 2 (FONT-CUTTABLE). `0062` keeps stage 3 (preview page, criteria 14 to 17).

## User value

As a maker who cuts names and slogans from wood and acrylic, I want to star the few fonts I use, tag fonts such as Script or my own "Wedding", add a font file I bought without installing it in my system, and see at once which fonts cut as one piece, so that I find a font for a laser job in a few clicks instead of scrolling 2,000 names and test-cutting fonts that fall apart.

## Field reference

| Tool | Groups and favourites | Own fonts without installing | Laser or cutting suitability |
|---|---|---|---|
| LightBurn 2.1 | Font Manager: a star puts a font on top and in a Favorites category; fonts can be hidden (Inactive category); Recent; search; no user groups ([docs](https://docs.lightburnsoftware.com/2.1/Reference/Text/); makers ask for "Cursive" and "Single Line" sections on the [forum](https://forum.lightburnsoftware.com/t/font-organization-feature/169164)) | No: install in the OS, then restart LightBurn. Only SHX single-line fonts come from a folder of their own | A "Welded" switch on text; no group or mark |
| Inkscape 1.4 | Font collections in the Text and Font dialog: user collections (stored in the config folder `fontcollections`), plus "Document fonts" and "Recently used"; an experimental font browser ([GSoC page](https://wiki.inkscape.org/wiki/FontSelection_GSOC_2022), [1.4 notes](https://inkscape.org/news/2024/10/13/inkscape-version-1-4-zhtw)) | A user font folder in its config folder and extra font folders in Preferences; read at start | None |
| Affinity (Designer 2, Affinity 3) | A heart marks a favourite, a favourites view lists them ([tutorial](https://allaboutplanners.com.au/how-to-use-fonts-and-the-text-tool-in-affinity-publisher)); no collections | No on the desktop: it lists the OS's installed fonts ([guide](https://logosbynick.com/add-fonts-to-affinity-designer/)); the iPad app has its own font list | None |
| Cricut Design Space | Filters: Writing (single-line, for the pen), Single layer, Multi-layer, and "Only kerned fonts", on by default; no filter by sans, serif or script ([guide](https://heyletsmakestuff.com/wprm_print/using-fonts-and-text-in-design-space)) | No: system fonts must be installed | "Kerned" is a vendor flag: script letters that touch |
| FontBase, NexusFont | Font managers: fonts in several user collections, favourites; FontBase adds fonts to collections by drag and drop ([FontBase](https://fontba.se/learn)); NexusFont has library folders and groups | Their core: fonts from any folder are activated without installing | None |

**What we do better.**
- **Cuttable is measured, not a vendor flag.** The program joins the glyphs of a test word with the Union kernel of `0016` and counts the pieces; one piece means the font has the Cuttable tag (Question 11). The maker can override it. No other tool tells a maker this before the cut.
- **Tags are there on the first start.** Sans, Serif and Script come from the classification each font file declares (Question 12), the three included fonts are tagged by a shipped list, and the maker changes any of it. Cricut has no such filter; LightBurn has no groups or tags.
- **Own fonts without installing, on the desktop and in the browser**, shown first, available at once (LightBurn needs an install and a restart).
- **Favourites and groups belong to the maker, not to the computer's font list:** keyed by family name, they survive removing and adding a file again and follow the maker through a synced data folder.

## What exists after `0074` (from its spec and ADR 0020; code not read)

- The Fonts tab with a header, a search field and the blocks Missing, Curvyo fonts, Included, This computer (desktop). Family rows (name in its own face, face count, chevron) and face rows; current mark; picking applies to the selection or the next text.
- The Curvyo folder `app_data_dir()/fonts/` (non-recursive, TTF, OTF, TTC), created at first start, read again whenever the Fonts tab is shown; Curvyo writes nothing into it. The browser has no Curvyo block.
- Font matching by face hash, then family and style (Curvyo first), else Missing.
- Union (`0016`): joins overlapping outlines into one region; the pieces of its result are its outlines of even depth.
- The per-user formats file `document-formats.toml` in the data directory (`0045`), last writer wins, read at start.

## Terms

- **Family key:** the family name as shown in the family row, compared without case. Favourites, group membership and tag overrides attach to the family key, not to a file or a block. A family listed in two blocks shares its star and its groups. (Normalisation: architect, see below.)
- **Tag:** a name on a family; a family has any number of tags. All and Favourites are filters, not tags. Built-in tags: Cuttable, Script, Sans, Serif, Initials, always present. Own tags: made by typing a new name on a family, existing while at least one family (listed or not) carries them.
  - **All**: every listed family. Not a stored group.
  - **Favourites**: the starred families.
  - **Built-in groups**: Initials, Script, Sans, Serif, Cuttable. Their members come from three sources, in this order of strength: the maker's override for that family (added or removed), then the shipped list (for the included fonts), then the automatic result (classification for Sans, Serif, Script, Question 12; the cuttable check for Cuttable, Question 11). Initials has no automatic source.
  - **Own groups**: made, renamed and deleted by the maker; members only by the maker.
- **Font library file:** the maker's favourites, tags and overrides and the last chosen strip cell, in one per-user file next to the formats file in the data directory (desktop) or in browser storage (browser). There are no hidden groups. Not part of any project; a collaborator never sees it. No document history step and no `format_version` change.
- **Cuttable check:** set the word `cuttable` (lowercase) in the family's Regular face (or its first face) at Size 20 mm, join its glyphs with Union (`0016`, one operand per glyph, no gap) and count the pieces (an island counts). **Cuttable** means exactly 1 piece. The word has no dotted letters and no capitals, because capitals of script fonts often stand apart and a dot is always a second piece.
- Notices at most 70 characters, in the slots `0074` uses. "Next frame" means the list redraws before the next input is handled.

## Acceptance criteria

### Part A: tags in the Fonts tab

1. Given the Fonts tab, then a **tag strip** sits between the header and the search row: All, Favourites, Cuttable, Script, Sans, Serif, Initials, then the own tags A to Z, ignoring case. One cell is chosen at a time (a radio strip, one Tab stop with arrow roving, the wrapped strip of `0045`, at most 4 rows, then it scrolls). The strip wraps and never truncates a name. Each own tag's cell carries a **pencil** (Rename) and a **bin** (Delete); the built-in tags, All and Favourites carry neither. The search field takes the full width of its row beside Add font… (customer change 2026-10-20: no Tags button, no Tags list).
2. Given a chosen strip cell, then the list shows only its families, still in the blocks of `0074` (Curvyo fonts first) and A to Z inside each; block headers count the families shown ("3 of 120"). Search and cell combine: a row is shown when it is in the cell and matches the search. Switching the cell updates the list in the next frame, under 100 ms for 2,000 families (R-FONT-003).
3. Given a tag with no listed member, then the list shows "No font in Cuttable." (with the tag's name), and, with a search, "No font in Wedding matches "vibs"."
4. Given the chosen strip cell, when the app is closed and started again, then the same cell is chosen. A tag that no longer exists falls back to All.
5. Given an own tag's cell in the strip, then its pencil, or F2 while the cell has the focus, turns the cell into a text field holding the name: Enter or blur saves, Escape cancels. Its bin, or Delete while the cell has the focus, replaces the cell by an inline confirmation (criterion 8). The strip stays one Tab stop. (Replaces the Tags list, customer change 2026-10-20.)
6. Given tags, then the built-in tags always exist and are always in the strip. An own tag exists while at least one family carries it, families not listed on this computer included. Removing its last use, on a row or with Delete, deletes it. When the chosen cell's tag is deleted, or its last use is removed with the x on a row, the strip chooses All at once and the tag leaves the strip (customer bug report 2026-10-20; replaces "stays chosen until the choice changes").
7. Given a new tag name (the add field) or Rename in the strip, then the name is 1 to 24 characters and is not All or Favourites. In the add field, a name equal to an existing tag ignoring case adds that tag. In Rename, a name equal to another tag ignoring case shows "A tag named Script exists." under the field and saves nothing, as a name over 24 characters shows "Use at most 24 characters.". Enter or blur saves, Escape cancels.
8. Given the bin or Delete on an own tag's strip cell, then the inline confirmation of `R-SYS-010` asks "Delete tag Wedding? Its 12 fonts stay in Curvyo." with Cancel and Delete. Confirming removes the tag from every family, never a font file. Built-in tags cannot be renamed or deleted.

### Part B: favourites and membership

9. Given a family row, then a **star** button sits on its name line before the face count, filled for a favourite and outlined otherwise; a press toggles the favourite, the font library file is written, and the row keeps its place under the pointer (only the star changes; with Favourites chosen, an unstarred row stays visible until the group or the search changes). A star press does not pick, does not expand, and with the mouse keeps the focus where it was. The star is on every family row in every block; a family in two blocks shows the same state in both.
10. Given a family row, then a **tag line** under the name shows the family's tags as chips in the strip order. Each chip has an **x** that removes the tag from that family, and the line ends with an **Add tag** button. Chips that do not fit collapse into "+N", a press on which expands the family, and an expanded family shows all its tags. Add tag turns the line into a field with inline completion from the existing tags. Enter adds the tag, and a new name creates it. A press writes the file and changes the strip in the next frame. Removing a built-in tag is an override. Adding an automatic tag back that the maker removed clears the override. A chip whose state comes from the automatic result or the shipped list says so in its tooltip ("Detected: one piece", "Declared by the font: script", "Set for the included fonts"), and a built-in tag added by the maker says "Added by you". A row that leaves the chosen cell by the maker's press stays in place until the cell or the search changes. There is no Reset (dropped 2026-10-20).
11. Given favourites, tags and overrides of a family that is not listed on this computer (a font removed, or the file synced from another computer), then they are kept in the file and the family carries its tags again when it is listed again.
12. Given the font library file, then changes are written at once with a write-temp-then-rename (ADR 0004 §7); a change made by another program while the app runs is read at the next start; the last writer wins; a write that fails shows "The font library could not be saved." and keeps the state in memory. A file whose `format` is newer than the app's is refused with "made by a newer version" and never partly read; the app then works with the defaults and does not overwrite it. A file of this format that does not parse is reported once ("The font library could not be read.") and kept; at the first change it is renamed to `font-library.toml.broken` and a new file is written (`adrs.md` decision 8). Golden fixtures cover a valid file, each failing rule and the round trip (the text is canonical: reading and writing gives the same text).
13. Given any action of Parts A and B, then it writes no history step and does not change the open project (it is a per-user setting, like the formats library of `0045`).

### Part C: built-in tags filled on the first start

14. Given the included fonts, then a shipped data file gives their tags: Source Sans 3 in Sans, Source Serif 4 in Serif, Great Vibes in Script; a test checks that every included family has an entry and every entry names an included family (R-FONT-004).
15. Given a font that declares its class (Question 12, default A), then it is tagged **Serif**, **Sans** or **Script** by its OS/2 `sFamilyClass` (classes 1 to 5 and 7: Serif; 8: Sans; 10: Script), else by its PANOSE family kind (3, Latin Hand Written: Script) and, for family kind 2 (Latin Text), its serif style (2 to 10: Serif; 11 to 13, sans styles: Sans). A font that declares neither is in none of the three until the maker adds it. Test: fixture faces with each of these values and one with zeros. *PO note 2026-10-20, revised the same day after the tester's re-review: PANOSE family kind 2 (Latin Text) with serif style 2 to 10 also counts as Serif, so Source Serif 4 (`sFamilyClass` 0, serif style 4) and most system serif faces get the Serif tag; serif style 11 to 13 stays Sans. A face that declares none of these is not prefilled. The included fonts are covered by the shipped list (criterion 14).*
16. Given the system fonts of the desktop (2,000 faces on the plan's machine), then the classification of criterion 15 is available with the "This computer" block, within the 2 s budget of `0074` criterion 20, and app start is no slower than `0074` allows.
17. Given **Cuttable** (Question 11, A), then a family has the tag when its cuttable check gives 1 piece, unless the maker removed it. The maker can add it to any family. Test: a connected-script fixture face whose word joins into 1 piece is Cuttable; Source Sans 3 and Source Serif 4 are not (their letters stand apart: more than 1 piece); a fixture face without lowercase letters (drawn as `.notdef`) is not. Whether Great Vibes is Cuttable is what the check gives; the plan records the result.
18. Given the check, then it runs, without blocking typing, the canvas or the list, for the Curvyo fonts when they are listed (a newly added font within 1 s of its row appearing), and for the system fonts only when Cuttable is chosen for the first time on this computer: the Cuttable cell then shows a progress bar along its bottom edge, the This computer header reads "checking 340/2000", and members appear as they are found. The included fonts are not checked at run time: their result is shipped with them and a test checks it equals the check (`adrs.md` decision 5). Results are cached by face hash, so a face is checked once per computer, not once per start. A face the check cannot read is not Cuttable and is not retried.
19. Given a cuttable check result, then it equals the number of pieces Union (`0016`) makes of the converted text `cuttable` in that face at 20 mm (one kernel; a test on the included fonts and two fixtures).

### Part D: Add font and Remove on the desktop

20. Given the Fonts tab on the desktop, then an **Add font…** icon button sits right of the search field. A press opens the operating system's file dialog (the one layer, as Open), several files may be chosen, filtered to `.ttf`, `.otf`, `.ttc`.
21. Given chosen files, then each one is read and parsed before it is copied: a readable font is copied into the Curvyo folder; a file with the same bytes as a file already there is not copied again; a file whose name is taken by other bytes is copied as `<name>-2.ttf` (`-3`, ...). Nothing outside the Curvyo folder is written; the original stays where it was. Every copy is atomic (temp file, then rename), so a cancelled or failed copy leaves no partial font in the folder.
22. Given the copy is done, then the new families appear in the Curvyo fonts block in the next frame (10 files of 1 MB within 1 s on the plan's machine), the tag strip switches to All and the search is cleared if they would otherwise be hidden, the first new family is scrolled into view and expanded, and one notice says what happened: "3 fonts added.", "1 font added. 2 were already there.", "2 files are not fonts and were not added." (combined in one notice; each part at most 70 characters; the notice has no tooltip: the names of refused and present files are in its text, "Not fonts: notes.pdf, logo.svg.", the first two and "and 3 more" when more do not fit). A file over 64 MiB is not a font for this rule (ADR 0020 §2).
23. Given an own tag or Initials chosen in the strip when Add font… is pressed, then the added families get that tag (Proposal). The automatic tags are never added this way.
24. Given font files dropped onto the Fonts tab, then they are added as by criteria 21 to 23 (Proposal). Files of other kinds in the drop are counted as "not fonts".
25. Given a family row in the **Curvyo fonts** block, then the expanded family has a **Remove from Curvyo fonts** button row, and Delete on the family row opens the same confirmation (not in other blocks: Curvyo never deletes system or included fonts). It asks inline (`R-SYS-010`): "Remove Lobster from Curvyo fonts? 2 texts here use it." (the count of texts in the open project using a face of that family from this block; the sentence without the second part when there are none). Confirming deletes the family's files from the Curvyo folder (every file whose faces all belong to this family; a TTC that also holds another family is kept and the notice says "Lobster is in a file with other fonts and was kept."). The star and tags of the family are kept (criterion 11).
26. Given texts that used a removed face, then they follow `0074`'s matching: the same family and style from another block is used without a notice; otherwise the text is Missing (`0074` criterion 29). Undo does not bring the file back (removing is not a document change); the inline confirmation is the guard, and Add font… brings it back with its tags and star.
27. Given a file the maker copies into or deletes from the Curvyo folder with the file manager, then `0074` criterion 19 still holds (read again when the tab is shown); Add font… and Remove are additions, not the only way.

### Part E: Add font in the browser (`0062` criterion 9)

28. Given the browser build, then **Add font…** opens the browser's file picker with the same filter and several files, and the Curvyo fonts block is shown as soon as it holds a font. Criteria 21 to 25 hold with "the Curvyo folder" read as the browser's origin-private storage for this site; nothing is uploaded and no network request is made (R-SYS-007).
29. Given fonts added in the browser, when the page is reloaded and when the browser is closed and opened again, then they are listed again, and texts using them draw with them (tested in current Chrome and Firefox with default settings). The font library file of the browser is kept in the same storage and survives the same way.
30. Given the browser's storage is full, then nothing of the file is kept and the notice reads "The browser has no room for this font. Nothing was added."
31. Given the browser build, then its fonts and library are separate from the desktop app's (no sync between them in this slice).

### Part F: safety and the core rule

32. Given a damaged or hostile file in Add font, then it is refused as "not a font" by the same reader and limits as `0074` criterion 32 ("the same reader" means the Curvyo block's listing rules: Add font accepts exactly what the Curvyo block lists, `adrs.md` decision 6); adding never crashes or hangs the app (fixtures in `tests/fixtures/fonts/`).
33. Given the core crates, then they read no file: parsing the library file, the classification of criterion 15 and the cuttable check are pure functions on bytes and values in `*-core` crates that build for `wasm32-unknown-unknown`; finding, copying, deleting and storing files is platform code (`CLAUDE.md` §6).

## Out of scope

- The preview page with the lettering in all fonts of a group (`0062` stage 3).
- Cloud sync of fonts and the font library (follows the library sync of ADR 0004 §6 and §12, not built). Folder sync works because both live in the data directory. No sync between browser and desktop.
- Embedding fonts in a project or on export (ADR 0004 §12).
- Hiding single fonts (LightBurn's "Inactive") and switching off the This computer block.
- Showing a font's embedding permission (`fsType`, R-FONT-008).
- Tags per face, a manual order of tags or favourites, nested tags, hiding tags, merging tags on rename, choosing several tags at once in the strip, import and export of the library file.
- Fonts in sub-folders of the Curvyo folder, WOFF and WOFF2, installing fonts into the operating system, a download service or Google Fonts.
- Single-line (engraving) fonts and their meaning for Cuttable; R-FONT-002's "symbols/ornaments" group (the maker makes an own tag).
- A live "Welded" text (LightBurn).
- Moving removed fonts to the OS trash (they are copies).

## What the architect must decide (`adrs.md`)

Decided in [`adrs.md`](adrs.md) (2026-10-20): no new ADR, crate, dependency or project format change.

- **Font library file:** one TOML file beside `document-formats.toml` (as `0045`) or one file per record (ADR 0004 §6); schema, `format`, canonical order; where it lives in the browser.
- **Family key:** which name (typographic family, name ID 16, else ID 1), Unicode normalisation and case folding.
- **Classification of system fonts:** reading OS/2 and PANOSE for 2,000 faces inside `0074`'s 2 s budget without passing every file's bytes into wasm; classification stays a core function (`CLAUDE.md` §6 names font classification as core logic).
- **Cuttable check:** which crate calls `curvyo-text-core` and the Union kernel together (`ui-core`, as Object to path); scheduling so wasm does not block input (idle slices or a worker); the result cache keyed by face hash: format, place, and that it is rebuildable and not authoritative.
- **Add and Remove:** the Tauri commands, validation before copy (host or wasm), atomic copy, name collisions, deleting a family's files (TTC rule of criterion 25).
- **Browser storage:** origin-private file system or IndexedDB for font bytes; whether to request persistent storage; how the bytes reach the face store.
- Confirm: no `format_version` change and no new crate.

## UX notes

ux-engineer, 2026-10-20. Checked against `docs/design-system.md` and `frontend/src/components/panel/FontsTab.tsx` on `origin/story/weld` (`7bb9ec7`), the `0074` UX notes (section 4), the `0045` rows (preset strip, formats list, inline confirm) and the architect's `adrs.md` draft of today. Existing rows are cited by name. No popup, no context menu, no hint label, no panel section that comes and goes; every operation has a visible trigger in the tab.

### 1. Layout of the tab (top to bottom)

| Part | Decision |
|---|---|
| Header row | Unchanged (`0074`): "Fonts" and the subject line. Nothing is added to it: the subject line ("Next text: Source Sans 3 Regular") needs the width, so **Add font is not in the header row** (amend criterion 20, see section 4). |
| Group strip | Directly under the header row, section 2. |
| 8px, search row | The search field, now **212 x 28px**, 4px, the **Add font** button 28 x 28px (section 4). The browser build has the same row. |
| 8px, list | The tree of `0074` (the one inner scroll area, minimum 120px, 8px gutter). |

Fit at 800 x 600 (panel viewport about 532px): header 32, strip 2 rows 60, 8, search row 28, 8: **list about 396px**, one block header and 11 family rows (`0074` had 464px). With the strip at its cap of 4 rows (124px) the list is about 332px, 9 family rows. No horizontal scroll.

### 2. Group strip and the Groups list

**Strip.** The wrapped preset strip of `0045` (row "Presets (quick selection, Orientation)": `ToggleGroup` look, 28px cells, 1px dividers, `flex: 1 1 auto`, minimum 32px, 6px side padding, text never truncated, further rows 4px below, the last row's cells grow to fill it). One `role="radiogroup"` named "Font group", cells `role="radio"`, no heading line (the cells are names; "Font group" is the accessible name only). Order: All, Favourites, Initials, Script, Sans, Serif, Cuttable, own groups in creation order. Hidden groups are not cells.

- **Selected state**: the pressed `ToggleGroup` cell (`--toolbar-icon-active-bg` / `-fg`); exactly one cell pressed, All when nothing else.
- **Groups button** (amends criterion 1 and 5: it is not a radio cell, it opens something): a 28px outline button (the "Fit to content" look) with the label "Groups" and `ChevronDown` 12px (`ChevronUp` while open), `aria-expanded`, `aria-controls` the Groups list. It sits on a row of its own, 4px under the strip, right-aligned (amended in the UX review of 2026-10-20, `review-ux.md`: outside the strip's scroll box it never scrolls away at the 4-row cap and never moves when groups change; the cost is one 32px row). Its own Tab stop after the strip.
- **Widths at 244px** (14px UI font): All, Favourites, Initials, Script in row 1 (about 232px with dividers); Sans, Serif, Cuttable and the Groups button in row 2. So the built-ins take **2 rows** on every desktop font; each own group adds about one cell, a 24-character name takes a row of its own.
- **Overflow cap**: the strip shows at most **4 rows** (124px). Beyond that it scrolls inside its own box by whole rows (wheel, and the chosen or focused cell is always scrolled into view; no scrollbar gutter, the `overflow-y: auto` thin bar of the list). The Show switch (below) is the maker's tool to keep the strip short; the cap only protects the list's 120px minimum at 800 x 600.
- **Mouse press** chooses the group; focus returns to the canvas (the edit sink in edit mode), as a preset cell does. **Keyboard**: one Tab stop, Tab lands on the pressed cell; Left, Right, Up, Down, Home, End move and choose at once (choosing changes no document, so arrows may choose, as in the preset strip).
- On a group change the list scrolls to its top; the current face row is not forced into view (Shift+Ctrl+T does that).

**Groups list** (criterion 5; amended: it opens below the strip **in place of the search row and the font list**, not above them). The tab then shows header, strip and the Groups list, which is the tab's one scroll area. So it never squeezes the font list and the strip above it shows each change at once (a Show switch moves a cell in or out). Pressing Groups again, or Escape, closes it and restores the search row and the list as they were (search text and scroll kept). Changing tab closes it.

- **Row**, 28px, the format-row look of `0045` (List row states, roving focus): name 14px (ellipsis, tooltip with the full name), member count 12px tabular `--panel-muted-fg` ("12", counted among the listed fonts; Cuttable while checking: "12 · 340/2000"), for own groups the icon buttons **Rename** (`Pencil`) and **Delete** (`Trash2`), 24 x 24px, 16px glyphs, then the **Show** `Switch` (accessible name "Show group Script"). Built-in rows reserve the 52px of the two icon buttons empty, so the switches form one column. Rows: Favourites, Initials, Script, Sans, Serif, Cuttable, own groups; All has no row.
- **New group**: the last row, a full-width 28px outline button "New group". Pressing it turns the row into a 28px text field (white ground, placeholder none, the field is labelled "New group name" for assistive tech) with the focus in it; Enter saves, Escape or blur with an empty field cancels. On save the group is chosen in the strip (criterion 7) and the Groups list stays open with the focus on the new group's row.
- **Rename**: the name turns into the same text field in place, prefilled and selected; Enter saves, Escape restores. F2 on a focused own-group row does the same.
- **Validation**: the validation chip of the formats form, 4px under the field: "A group named Script exists." / "Enter a name." / "Use at most 24 characters." (typing beyond 24 is not blocked; the chip explains). Nothing is saved while it shows; the focus stays in the field.
- **Delete**: the row is replaced by the **Inline confirm** (Cancel focused, the action ignores presses for 400 ms): "Delete group Wedding? Its 12 fonts stay in Curvyo." (with 0 members: "Delete group Wedding?"), buttons **Cancel** and **Delete**. The Delete key on a focused own-group row opens it too. A built-in row has no Rename, no Delete, and F2 and Delete do nothing on it.
- **Keyboard**: one Tab stop (roving): Up, Down, Home, End between rows; Left and Right between a row's controls (Rename, Delete, Show); Enter or Space presses (the switch: Space); Escape steps back: closes the field or the confirm, else closes the Groups list and puts the focus on the Groups button.

### 3. Family row: star, cuttable mark, Groups row, Reset

**Family row** (32px, `0074`), from the left: chevron slot 24px, 4px, name (flex, ellipsis), **cuttable mark slot 16px**, **star 24px**, face count, 8px. The name loses 40px against `0074`; at 244px it keeps about 130px, room for 10 to 14 characters at 16px, the rest ellipsis and tooltip as before.

| Item | Decision |
|---|---|
| Star | A 24 x 32px button, Lucide `Star` 16px, 1.5px stroke. Favourite: filled and stroked `--toolbar-icon`. Not a favourite: outline in `--panel-muted-fg` (visible on every row, as the format rows' stars of `0045`: consistency, no hover reveal). Hover: `--editor-accent-hover` ground on the 24 x 32 box only. `aria-pressed`, constant accessible name "Favourite: Lobster", tooltip "Favourite". **A press toggles only the star**: it does not pick, does not expand, does not move the row or the scroll (criterion 9); with the mouse, focus stays where it was (the canvas or the edit sink keeps it, so typing goes on). Not a separate Tab stop (the tree has one); the key is Ctrl+D (section 6). |
| Rows that leave their group | General rule for criterion 9 and 10: a row that stops being a member of the chosen group by the maker's own press (star off in Favourites, a chip off in that group) **stays in place** until the group or the search changes; it is not greyed. A row that a background result removes or adds (the cuttable check) follows the anchoring rule below. |
| Cuttable mark | In the 16px slot left of the star: Lucide `Scissors` 12px in `--panel-muted-fg`, only when the family is a member of Cuttable (by the check, the shipped list or the maker). A state mark, not a button: `aria-hidden`; the row's accessible name gains ", cuttable"; the row tooltip gains the line "Cuttable: welds into 1 piece" (by the check), "Cuttable: set for the included fonts", "Cuttable: set by you". Not a member, not yet checked, unreadable: the slot is empty. The slot is always reserved, so a mark arriving never moves the name. Hidden group or not, the mark shows (it is the group's whole point at a glance). |
| Anchoring | When background results add or remove rows above the first fully visible row (Cuttable filling in, the This computer block arriving), the list keeps that row at its screen position (the scroll offset is corrected in the same frame). The focused row never changes because of a background result. |

**Expanded family** (`0074` criterion 18): its face rows, then two new rows, both `aria-level` 2 tree items, indented 20px like faces:

1. **Groups row** (criterion 10). The 12px `--panel-muted-fg` label "Groups" on its own 16px line, then the chips wrapping in rows of 24px, 4px apart; at the end of the last chip line the **Reset** button. Height: 16 + 4 + chip rows (2 rows for the built-ins plus a few own groups: 16 + 4 + 52 = 72px, + 8px bottom space).
   - **Chip**: 24px high, 8px side padding, 12px text, `rounded-[5px]`, 1px `--toolbar-icon` at 40 % border; pressed (member): `--toolbar-icon-active-bg` ground, `--toolbar-icon-active-fg` text, plus a `Check` 12px before the name (shape, not only colour). `role="checkbox"`-like toggle buttons with `aria-pressed`, names "Script: Lobster is in it". Order: Initials, Script, Sans, Serif, Cuttable, then own groups; hidden groups included (criterion 10). Favourites is not a chip (the star is its control).
   - **Tooltip** per chip (Radix, 400ms, bottom), the source of the state: "Declared by the font: script", "Detected: welds into 1 piece", "Detected: welds into 7 pieces", "Set for the included fonts", "Set by you". A chip with no automatic source and no override has no tooltip. These are states, not tips.
   - **Reset** (the Proposal of criterion 10; form decided here): a 24px text button "Reset" in the outline look, at the right end of the last chip line. It removes this family's overrides of the built-in groups (own-group membership is not touched: it has no automatic state to return to). It is **always in the tree**, disabled (`aria-disabled`, 40 % opacity, tooltip "No changes by you") when the family has no override, so pressing chips never moves it in or out. No confirm and no notice: the chips change state in place, and every chip can be pressed back. Accessible name "Reset groups of Lobster".
   - A press on a chip or Reset writes the library file at once, changes the strip counts and the cuttable mark in the next frame; the row stays (rule "Rows that leave their group").
2. **Remove row** (criterion 25), only for a family in the **Curvyo fonts** block: a 28px outline button "Remove from Curvyo fonts", full width minus the indent, label `--destructive`. Pressing it replaces the button by the **Inline confirm**: "Remove Lobster from Curvyo fonts? 2 texts here use it." (without the second sentence when no text uses it), buttons **Cancel** and **Remove**. Confirming deletes, collapses nothing (the family row is gone, the focus goes to the next family row, else the previous). TTC kept: the Action notice under where the button was, "Lobster is in a file with other fonts and was kept." (refusal look, 8s). Other blocks have no Remove row (never disabled: absent, because Curvyo never removes those fonts).

### 4. Add font and drop

| Item | Decision |
|---|---|
| Button | 28 x 28px icon button right of the search field (amends criterion 20: the search row, not the header row), the outline look of the Layers toolbar, Lucide `FilePlus` 16px, accessible name and tooltip "Add font…" (the ellipsis marks the OS file dialog, as "Open…"). Same in the browser build (its file picker). Its own Tab stop after the search field. |
| While copying | If adding takes over 150 ms: the button shows the busy state (glyph replaced by `LoaderCircle` 16px, still, no spin with `prefers-reduced-motion`; `aria-busy`) and presses are ignored; the Action notice busy text "Adding 3 fonts…" (`role="status"`). Typing, the canvas and the list stay usable. |
| Result | The **Action notice** under the Add font button (4px below, right-aligned, max-width 244px; criterion 22). It is not an in-flow line, so the list never jumps when it comes or goes. Wording, one notice, parts joined in this order and cut by the 70-character rule per part: "3 fonts added." / "1 font added. 2 were already there." / "Lobster.ttf is not a font and was not added." (one file: its name, cut to 24 characters with an ellipsis) / "2 files are not fonts and were not added." With an own group chosen (criterion 23): "3 fonts added to Wedding." **File names** (amends criterion 22: the notice takes no pointer, so it has no tooltip): when files were refused or already there and the names fit, the notice's second sentence lists them, "Not fonts: notes.pdf, logo.svg." (the 140-character, 8 s duration rule of the Action notice); more than fit: the first two and "and 3 more". Success look when at least one font was added; refusal look when none was. |
| After adding | Criterion 22 as written: strip to All and search cleared only if the new families would be hidden, the first new family scrolled into view (to the top third of the list), expanded, and **keyboard focus on it** when the add was started by keyboard; with the mouse the focus stays where it was. No highlight animation: the expanded row with its Groups row is the mark. |
| Drop (criterion 24) | A drag of files over the Fonts tab's body (strip, search row, list; not the header) shows a **2px dashed `--accent` inset outline around the body** and the Add font button in its pressed look; no text. Leaving the body or Escape removes it. The drop adds as the button does, the notice anchored to the Add font button. A drag that carries no files shows nothing. While the Fonts tab is not shown, nothing in the panel reacts (a drop elsewhere keeps today's behaviour of the window). Desktop: Tauri's window drag-drop event, hit-tested against the tab body's rectangle. |
| Storage full (browser, criterion 30) | The refusal notice under the button: "The browser has no room for this font. Nothing was added." |
| Library file errors (criterion 12) | "Font groups could not be saved." and "Font groups could not be read." as refusal notices under the strip (left-aligned, 4px below its last row), once per session each; "made by a newer version": "Font groups were made by a newer version of Curvyo." once, same place. |

### 5. The Cuttable group

- **Progress** (amends criterion 18: not in the cell text, which would widen the cell on every count and rewrap the strip): while the system fonts are checked, the Cuttable cell carries a **2px determinate bar along its bottom edge inside the cell** (`--accent`; on the pressed cell `--toolbar-icon-active-fg`), and the count in words goes to three places that already have a slot: the cell's accessible description and tooltip "Checking fonts: 340 of 2000", the **This computer block header**, whose right side reads "checking 340/2000" instead of its count (the `0074` "Reading…" slot), and the Groups list row count. The bar is gone when the queue is empty. No cell, row or header changes width while counting.
- **Members appear as found**, under the anchoring rule of section 3. Block header counts update at most every 500 ms during the run (no flicker of digits).
- **Empty states with Cuttable chosen**: while checking and none found yet: "No font in Cuttable yet." (the progress is in the header); after the run: "No font in Cuttable." (criterion 3). The This computer header shows "checking …" even when the block has no member yet (it is in the list from the first frame as in `0074`).
- **Manual add or remove**: the Cuttable chip in the family's Groups row (section 3), as for every built-in group; its tooltip names the piece count when checked. Reset returns to the check's result.
- **The first choice of Cuttable** starts the system run (criterion 18). Choosing it by arrow keys while passing through the strip also starts it (the strip chooses on arrow); this is accepted: the run is idle-time work and is stored, so it never runs twice.

### 6. Keyboard

| Where | Keys |
|---|---|
| Tab order in the tab | strip (one stop), Groups button, search field, Add font, list (one stop). With the Groups list open: strip, Groups button, Groups list (one stop). |
| Strip | Left, Right, Up, Down, Home, End move and choose; Escape returns to the canvas. |
| List (tree, `0074` model kept) | **Ctrl+D** (Cmd+D) on a family row, or on a face, Groups or Remove row of that family: toggles its star (a non-printable chord; printable keys still go to the search field). The browser's bookmark key is taken with `preventDefault` while the list has focus; checked in Chrome and Firefox at build, fallback there: none needed beyond the mouse star, as the strip's Favourites and the chips cover reading and assigning. **Delete** on a Curvyo-block family row opens the Remove confirm (focus to Cancel). |
| Groups row | A tree item. Enter or Space on it (or Right) moves the focus into its chips; inside, Left, Right, Home, End move between chips and Reset, Space or Enter toggles a chip or presses Reset; Up and Down leave to the previous or next tree row; Escape returns the focus to the Groups row. Disabled Reset is skipped. |
| Remove row | Enter or Space opens the confirm; inside it, Tab between Cancel and Remove, Escape cancels and returns to the row. |
| Groups list | Section 2. |
| Add font | Enter or Space opens the file dialog; after the add, the focus goes to the first new family row. |
| Shift+Ctrl+T | Unchanged (`0074`): shows the tab, focus on the current face row; it closes the Groups list first. |

Screen reader: the strip announces "Font group, Cuttable, radio, 7 of 9"; a family row "Lobster, Curvyo fonts, 2 faces, favourite, cuttable"; a star press is announced by its `aria-pressed` change; chips by theirs. Group changes are not announced beyond the radio.

### 7. Changes to the criteria (for the product owner; applied to the criteria above on 2026-10-20)

- 1 and 5: "Groups…" is a disclosure button "Groups" after the strip, not a radio cell; the Groups list takes the place of the search row and the list while open.
- 9 and 10: the "row stays" rule applies to chips as well as the star; a star press keeps the focus where it was.
- 10: Reset is a "Reset" button at the end of the Groups row, always present, disabled without overrides; it does not touch own groups.
- 18: progress is a bar in the Cuttable cell and "checking 340/2000" in the This computer header and the Groups list, not the cell text.
- 20: Add font… is an icon button right of the search field.
- 22: no tooltip on the notice; refused and present file names are in its text.
- 25: Remove is a "Remove from Curvyo fonts" button row in the expanded family; Delete on the family row opens the same confirm.

### Design-system rows (added to `docs/design-system.md`, section "Properties panel: Fonts tab, font library")

| Item | Value |
|---|---|
| Layout (replaces the `0074` row) | Header row (24px + 8), group strip (2 to 4 rows of 28px, 4px apart), 8px, search row (search field 212 x 28, 4px, Add font 28 x 28), 8px, the list (the one scroll area, minimum 120px, 8px gutter). At 800 x 600 the list is about 396px with a 2-row strip. With the Groups list open it replaces the search row and the list |
| Group strip | The `0045` preset strip look, one `role="radiogroup"` "Font group": All, Favourites, Initials, Script, Sans, Serif, Cuttable, own groups; hidden groups absent; at most 4 rows, then the strip scrolls by rows; pressed cell = chosen group; arrows move and choose; mouse press returns focus to the canvas |
| Groups button | 28px outline button "Groups" + `ChevronDown`/`ChevronUp` 12px, `aria-expanded`, on a row of its own 4px under the strip, right-aligned, outside the strip's scroll box (it never scrolls away and never moves when groups are added, hidden or renamed), own Tab stop |
| Groups list | 28px rows: name 14px, count 12px muted, Rename (`Pencil`) and Delete (`Trash2`) 24px for own groups (space reserved on built-ins), Show `Switch`; last row "New group" (turns into a text field); rename and new name in place, validation chip under the field; Delete via Inline confirm; one Tab stop, Up/Down rows, Left/Right controls, F2, Delete, Escape steps back |
| Family row (amends `0074`) | chevron 24, 4, name, cuttable mark slot 16, star 24, face count, 8px |
| Favourite star | 24 x 32 button, `Star` 16px; filled `--toolbar-icon` or outline `--panel-muted-fg`; always visible; `aria-pressed`; toggles only the star, never picks, never moves the row; Ctrl+D in the list |
| Cuttable mark | `Scissors` 12px `--panel-muted-fg` in a reserved 16px slot, members only, `aria-hidden`, state named in the row's accessible name and tooltip |
| Groups row | Tree item in an expanded family: "Groups" 12px muted label, wrapping 24px toggle chips (pressed: active ground plus `Check` 12px), tooltip names the source ("Declared by the font: script", "Detected: welds into 1 piece", "Set for the included fonts", "Set by you"); "Reset" 24px outline button at the end, disabled without overrides; Enter/Right enters the chips, Left/Right inside, Up/Down leave |
| Remove row | Curvyo-block families only: 28px outline button "Remove from Curvyo fonts", `--destructive` label, Inline confirm in place; Delete on the family row opens it |
| Add font | 28 x 28 icon button `FilePlus`, "Add font…", busy state over 150 ms; result in the Action notice under it (names in the text, no tooltip); drop of files on the tab body: 2px dashed `--accent` inset outline, no text |
| Cuttable progress | 2px determinate bar inside the bottom of the Cuttable cell; "checking 340/2000" in the This computer header and the Groups list count; no width change while counting |
| Row stability | A maker's own press never removes a row from view until the group or search changes; background additions above the first visible row keep it in place |
| Empty (adds to `0074`) | "No font in Cuttable." / "No font in Cuttable yet." (while checking) / "No font in Cuttable matches "vibs"." |

Keyboard shortcuts table, add:

| Action | Shortcut | Notes |
|---|---|---|
| Favourite a font | `Ctrl+D` (`Cmd+D`) in the Fonts list | `0075`; on any row of the family; the browser's bookmark key is prevented while the list has focus |
| Choose a font group | Tab to the group strip, arrows, Home, End | `0075`; choosing writes no history step |
| Remove a Curvyo font | `Delete` on its family row, then the Inline confirm | `0075`; Curvyo fonts block only; not undoable (a file, not the document) |
| Rename, delete an own group | `F2`, `Delete` in the Groups list | `0075`; built-in groups ignore both |

## UX amendment: tags replace groups (customer, 2026-10-20)

ux-engineer, 2026-10-20. Customer change of 2026-10-20 ("Tags are better than groups for fonts ... a plus adds a new tag, and an x next to the tag in the listing removes it again"). Read against `specs/0075-font-library/specification.md` (UX notes §1 to §7), `review-ux.md`, and on `story/font-library` (`b5b148a`) `FontsTab.tsx`, `FontGroupStrip.tsx`, `FontGroupsList.tsx`, `FontFamilyRows.tsx`, `FontTreeRows.tsx`, `fontRows.ts`, `fontTreeKeys.ts`.

This amendment **replaces** UX notes §2 (Groups list), the Groups row, Reset and the Cuttable mark of §3, and the matching parts of §5 and §6. Everything else in the UX notes holds (star, Add font, drop, Remove, notices, anchoring, row stability, Cuttable progress bar).

No popup, no panel section that comes and goes, no hint label. The only new things a press shows are the add field (in place of a line the maker pressed on) and the inline confirm, both of which already exist in the tab.

### A1. Model in one paragraph

A **tag** is a name on a family. A family has any number of tags. **Built-in tags** are Cuttable, Script, Sans, Serif and Initials. They always exist, cannot be renamed or deleted, and are always in the strip. **Own tags** exist while at least one family carries them, including families not listed on this computer (criterion 11). Removing the last use deletes the tag. Cuttable, Script, Sans and Serif get **automatic** members from the cuttable check, the classification and the shipped list. Initials has no automatic source. The maker's add or remove on a built-in tag is an **override**. Adding an automatic tag back that the maker had removed clears the override, so the automatic state applies again. Favourites stays the star, and Favourites is not a tag.

### A2. Family row: two lines, fixed height

| Item | Decision |
|---|---|
| Height | **52px for every family row** (was 32): name line 28px, tag line 24px. Fixed, so the list stays virtualised with constant row heights, nothing moves when a background result adds a tag, and 2,000 families make a 104,000px list. An expanded family grows (section 3). Face, header and Remove rows are unchanged. |
| Name line (28px) | chevron slot 24, 4, name (16px in its own face, flex, ellipsis), current mark, star 24 x 28, face count, 8px. **The Cuttable mark slot (`Scissors`) is dropped**: the Cuttable chip says it, and the name gains 16px back (about 146px at 244). |
| Tag line (24px) | Starts at 28px (under the name), ends 8px from the right edge. From the left: tag chips, then the overflow chip if needed, then the **Add tag** button (20 x 20, Lucide `Plus` 14px, outline look, accessible name and tooltip "Add tag"), right-aligned at the end of the line. The add button is on every family row, always visible (the same rule as the star: no hover reveal). |
| Tag chip | 20px high, `rounded-[5px]`, 1px border `--toolbar-icon` at 40 %, no fill, 12px text `--toolbar-icon`, 6px left padding, the name, then the **remove zone** 18 x 20 with Lucide `X` 10px in `--panel-muted-fg` (`--toolbar-icon` on hover, with an `--editor-accent-hover` ground on the zone only). **The x is always shown** (the customer's "x next to the tag", and no hover reveal, as for the star). Only the zone is a button. A press on the chip's name does what a press on the row does (picks), so a press near the x never does something unexpected. Chips are 4px apart. A tag name is never truncated except when a single chip is wider than the whole line: then it gets an ellipsis, with the full name in its tooltip. |
| Chip order | One order everywhere (row and strip): **Cuttable, Script, Sans, Serif, Initials**, then own tags A to Z, ignoring case. Cuttable comes first, so the one tag that matters most for the laser job is never in the overflow. |
| Overflow | Chips are laid out in order. When they do not all fit, as many as fit stay and leave room for an overflow chip "+2" (20px, the chip look without the x, 12px tabular `--panel-muted-fg`). It is a button: a press expands the family (the same as the chevron), and its accessible name is "2 more tags, show all". The row tooltip gets a line "Tags: Cuttable, Script, Wedding" with all of them. Room at 244px: 176px for chips, so two chips plus "+N" in the usual case (Cuttable 70px, Script 56px, Wedding 68px). |
| Built-in, automatic and overridden tags | **Same look as own tags.** A chip whose state comes from an automatic source has the source tooltip as before ("Detected: one piece", "Declared by the font: script", "Set for the included fonts"). A built-in tag the maker added says "Added by you". Own tags have no tooltip. A built-in tag the maker removed is not shown. These are states, not tips. |
| Background results | When the cuttable check finds a member, its Cuttable chip appears at the front of the tag line. The row height does not change, and a chip that no longer fits goes into "+N". This is the only change the maker did not press for, and it is the one the old Scissors mark had. |
| Mouse focus | Pressing x or Add tag does not pick and does not expand. Pressing x keeps the focus where it was (`mousedown` default prevented, as the star does). Pressing Add tag moves the focus into the field; after Enter or Escape the focus goes back to where it was before the press. |
| Accessible name of the row | "Lobster, Curvyo fonts, 2 faces, favourite, tags Cuttable, Script, Wedding" (no ", tags" part when the family has none). |

### A3. Expanded family

The family row's tag line **wraps to show every tag** (no overflow chip), 24px per line, with Add tag at the end of the last line. Then come the faces and, for a Curvyo font, the Remove row. **The Groups row and Reset are gone.** Height of the expanded family row: 28 + 24 x lines. It is computed from the tree's measured `clientWidth` less the 28px indent and the 8px gutter, not from a constant (`review-ux.md` SHOULD-FIX 2). Only expanded rows vary in height, as faces already do.

### A4. Adding a tag

| Item | Decision |
|---|---|
| Field | A press on Add tag (or the keys in section 8) turns **the tag line it sits on** into a text field: it spans from the line's start to the right edge, 20px high, 12px text, white ground, and covers that line's chips until it closes. The row height never changes. In an expanded row it covers the last line. Accessible name "Add tag to Lobster", no placeholder. |
| Completion | **Inline completion, no list**: after the caret, the rest of the first existing tag that starts with the typed text (ignoring case, tags the family does not have yet, in the chip order) is shown selected in `--panel-muted-fg`. Up and Down step through the other matches. Right or End takes the completion into the text. Delete or Backspace drops it. The strip right above the list shows every existing tag, so the maker sees what exists without a dropdown. |
| Enter | Adds the name as shown, completion included, and closes the field. A name that equals an existing tag ignoring case adds that tag in its existing spelling. A new name creates the tag, and it appears in the strip in the next frame, in A to Z order. Adding a built-in tag is an override, or clears a "removed" override. A tag the family already has changes nothing. An empty field closes. |
| Escape, blur | Closes without adding. |
| Validation | 1 to 24 characters, the validation chip of the formats form, 4px under the field, drawn over the row below: "Use at most 24 characters." / "All and Favourites are not tags." Nothing is added while it shows, and the focus stays in the field. |
| After adding | The library file is written at once. The chip shows where its order puts it, or goes into "+N". With a tag chosen in the strip, the row stays even if it would now leave or join (the row stability rule). |

### A5. Removing a tag

A press on the x removes that tag from that family. The library file is written and the chip goes away in the next frame (the chips after it close up; the row does not move). There is no confirm: one tag on one family is put back with Add tag, by typing a letter or two. Removing an automatic tag is a "removed" override, and adding it back clears it. Removing the last use of an own tag deletes the tag. If that tag is the chosen strip cell, the strip chooses All at once and the tag leaves the strip (customer bug report 2026-10-20). The same holds for Delete on its strip cell, after which the keyboard focus is on the All cell.

**Reset is dropped.** Every override can be seen (an added built-in chip says "Added by you") or undone in place (add the tag back). A per-family button with no visible effect on most rows is not worth its tab stop and its 24px.

### A6. Tag strip (the old group strip)

Same component, look, 4-row cap and keys as before (one `role="radiogroup"`, now named "Font tag", **single choice**, arrows choose, a mouse press returns the focus to the canvas). Cells: All, Favourites, Cuttable, Script, Sans, Serif, Initials, then own tags A to Z. Built-ins are always there, own tags only while used. **No Show switch any more**: hiding goes away with the Groups list, and the 4-row cap with scrolling by rows protects the list's 120px minimum. At 244px, row 1 is All, Favourites, Cuttable, Script (about 231px) and row 2 is Sans, Serif, Initials. The Cuttable cell keeps its progress bar.

### A7. Managing tags: in the strip (customer change 2026-10-20)

The Tags button and the Tags list are dropped: the customer found that once open it was not clear how to get out. Renaming and deleting happen on the strip cell of an own tag.

| Item | Decision |
|---|---|
| Cell | An own tag's cell holds its name, then **Rename** (Lucide `Pencil` 12px) and **Delete** (`Trash2` 12px), each a 20 x 20 icon button with the `--editor-accent-hover` ground on hover, not Tab stops. Built-in cells, All and Favourites have no icons. The extra width is accepted (customer: nobody makes 20 tags). |
| Rename | The cell becomes a text field holding the name (selected), 24px high, white ground. Enter or blur saves, Escape cancels. A clash ignoring case ("A tag named Script exists."), "Use at most 24 characters." and "All and Favourites are not tags." show in the validation chip under the field; nothing is saved while it shows. After saving or cancelling, the focus is on the cell. |
| Delete | The cell is replaced by the **Inline confirm** across the strip's width: "Delete tag Wedding? Its 12 fonts stay in Curvyo.", Cancel (focused) and Delete (400 ms guard). Confirming removes the tag from every family, the families not listed here included. A deleted chosen tag switches the strip to All; after confirming, the focus is on the All cell. |
| Search row | The search field takes the full width beside Add font…; there is no Tags button. |

### A8. Keyboard

| Where | Keys |
|---|---|
| Tab order | strip (one stop), search field, Add font, list (one stop). |
| Family row (tree) | Unchanged: Up, Down, Home, End, PageUp, PageDown, Right expands or goes to the first face, Left collapses, Enter picks, Ctrl+D star, Delete Remove (Curvyo block), printable keys go to the search. **New: F2 opens the add field of the focused family**, on any row of that family. |
| Add field | Typing, Up and Down through the matches, Right or End takes the completion, Enter adds, Escape closes. After Enter or Escape **the focus goes to the family's tag line**, on Add tag. |
| Tag line | A focus position inside the family row, entered with F2 then Escape, or with **Shift+F2** directly (it lands on the last chip's x). Left and Right move between the chips' x, the "+N" chip and Add tag; Home and End go to the ends. Enter or Space presses the focused control (removes the tag, expands the family, opens the field). **Delete or Backspace on a chip removes it.** The focus then goes to the next chip, or to Add tag. Escape returns to the family row. Up and Down leave to the previous or next tree row. The focused chip has the 2px `--editor-accent` ring around the whole chip, and its accessible name is "Remove tag Script from Lobster". |
| Tag strip | Left, Right, Up, Down, Home, End move and choose (one Tab stop). On an own tag's cell F2 renames and Delete opens the confirm; built-in cells, All and Favourites ignore both. In the field Enter saves, Escape cancels. |
| Shift+Ctrl+T | Unchanged. |

The mouse can do all of it, and so can the keyboard. The add field, Rename and Delete are reached by a visible button and by a key.

### A9. Fit

| Size | Result |
|---|---|
| 800 x 600 (panel viewport about 532px) | header 32, strip 2 rows 60, 8, search row 28, 8: **list 396px** (385 with the Groups row, now gone). At 52px rows that is one block header and **7 family rows**, against 11 with 32px rows. This is the price of tags on every row, and the customer asked for it. With the strip at its 4-row cap: list 332px, 5 rows and part of a sixth. No horizontal scroll. |
| 1280 x 800 | list about 596px, 11 family rows. |
| Narrow name | 146px of name at 244px (the Scissors slot is dropped), the same as or more than `0074`. |

### A10. Changed criteria (reworded by number; numbering kept)

Title: "Font library: tags, favourites, Add font and the Cuttable tag". In the Terms, **Group** becomes **Tag**: "a name on a family; a family has any number of tags. All and Favourites are filters, not tags. Built-in tags: Cuttable, Script, Sans, Serif, Initials, always present. Own tags: made by typing a new name on a family, existing while at least one family (listed or not) carries them." The source order (override, shipped list, automatic) stays. The font library file holds favourites, tags and overrides, and the last chosen strip cell. There are no hidden groups. Part A becomes "Part A: tags in the Fonts tab".

1. Given the Fonts tab, then a **tag strip** sits between the header and the search row: All, Favourites, Cuttable, Script, Sans, Serif, Initials, then the own tags A to Z, ignoring case. One cell is chosen at a time (a radio strip, one Tab stop, the wrapped strip of `0045`, at most 4 rows, then it scrolls). The strip wraps and never truncates a name. A **Tags** icon button in the search row, between the search field and Add font…, opens the Tags list (criterion 5).
2. Unchanged, with "group" read as "strip cell".
3. Unchanged, with "group" read as "tag" ("No font in Cuttable.", "No font in Wedding matches "vibs".").
4. Given the chosen strip cell, when the app is closed and started again, then the same cell is chosen. A tag that no longer exists falls back to All.
5. Given the **Tags** button, when pressed, then the Tags list takes the place of the font list (not a popup) and the search field is disabled. The list has one row per tag, in strip order, with its name and its count among the listed fonts. Own tags also have Rename and Delete. Pressing Tags again, or Escape, closes it and restores the list as it was.
6. Given tags, then the built-in tags always exist and are always in the strip. An own tag exists while at least one family carries it, families not listed on this computer included. Removing its last use, on a row or with Delete, deletes it. When the chosen cell's tag is deleted, or its last use is removed with the x on a row, the strip chooses All at once and the tag leaves the strip (customer bug report 2026-10-20; replaces "stays chosen until the choice changes").
7. Given a new tag name (the add field) or Rename, then the name is 1 to 24 characters and is not All or Favourites. In the add field, a name equal to an existing tag ignoring case adds that tag. In Rename, a name equal to another tag ignoring case shows "A tag named Script exists." under the field and saves nothing. Enter saves, Escape cancels.
8. Given Delete on an own tag, then the inline confirmation of `R-SYS-010` asks "Delete tag Wedding? Its 12 fonts stay in Curvyo." Confirming removes the tag from every family, never a font file. Built-in tags cannot be renamed or deleted.
9. Unchanged. The star sits on the name line of the 52px family row.
10. Given a family row, then a **tag line** under the name shows the family's tags as chips in the strip order. Each chip has an **x** that removes the tag from that family, and the line ends with an **Add tag** button. Chips that do not fit collapse into "+N", a press on which expands the family, and an expanded family shows all its tags. Add tag turns the line into a field with inline completion from the existing tags. Enter adds the tag, and a new name creates it. A press writes the file and changes the strip in the next frame. Removing a built-in tag is an override. Adding an automatic tag back that the maker removed clears the override. A chip whose state comes from the automatic result or the shipped list says so in its tooltip ("Detected: one piece", "Declared by the font: script", "Set for the included fonts"), and a built-in tag added by the maker says "Added by you". A row that leaves the chosen cell by the maker's press stays in place until the cell or the search changes. There is no Reset (dropped 2026-10-20).
11. Unchanged, with "groups" read as "tags".
12. The notices read "The font library could not be saved." and "The font library could not be read." The rest is unchanged.
13. Unchanged.
14. Given the included fonts, then a shipped data file gives their tags. The rest is unchanged.
15. Unchanged, with "in Serif, Sans or Script" read as "tagged Serif, Sans or Script".
17. Given **Cuttable**, then a family has the tag when its cuttable check gives 1 piece, unless the maker removed it. The maker can add it to any family. The tests are unchanged.
18. Unchanged, except that "the Groups list count" reads "the Tags list count".
22. Unchanged, except that "the group strip switches to All" reads "the tag strip switches to All".
23. Given an own tag or Initials chosen in the strip when Add font… is pressed, then the added families get that tag (Proposal). The automatic tags are never added this way.
25. Unchanged, except that "Favourites and groups of the family are kept" reads "The star and tags of the family are kept".
26. Unchanged, except that "with its groups and star" reads "with its tags and star".

Out of scope: "Groups per face (groups hold families), reordering groups ..., nested groups" becomes "Tags per face, a manual order of tags or favourites, nested tags, hiding tags, merging tags on rename, choosing several tags at once in the strip". The note about the "symbols/ornaments" group now reads "the maker makes an own tag".

For the architect (`adrs.md`): the library file stores tags per family plus built-in overrides (added or removed), with no group table and no `hidden` entry. The file is unreleased (the story is not merged), so its schema changes without a migration and `format` stays 1. Own tags are derived from the families that carry them, and are not stored as a separate list.

### A11. Design-system rows (replace the `0075` rows in "Properties panel: Fonts tab, font library")

| Item | Value |
|---|---|
| Layout (replaces) | Header row (24px + 8), tag strip (2 to 4 rows of 28px), 8px, search row (search field 180 x 28, 4, Tags 28 x 28, 4, Add font 28 x 28), 8px, the list (the one scroll area, minimum 120px, 8px gutter). At 800 x 600 the list is about 396px with a 2-row strip, 7 family rows. With the Tags list open it takes the list's place and the search field is disabled |
| Tag strip (replaces Group strip) | The `0045` preset strip look, one `role="radiogroup"` "Font tag", single choice: All, Favourites, Cuttable, Script, Sans, Serif, Initials, own tags A to Z; at most 4 rows, then it scrolls by rows; arrows move and choose; a mouse press returns the focus to the canvas |
| Tags button (replaces Groups button) | 28 x 28 icon button `Tags` 16px in the search row, `aria-expanded`, its own Tab stop |
| Tags list (replaces Groups list) | 28px rows: name 14px, count 12px muted, Rename (`Pencil`) and Delete (`Trash2`) 24px for own tags (space reserved on built-ins); no switch, no new row; rename in place with the validation chip; Delete via Inline confirm; one Tab stop, Up/Down rows, Left/Right controls, F2, Delete, Escape steps back |
| Family row (replaces) | 52px, fixed: name line 28 (chevron 24, 4, name, current mark, star 24 x 28, face count, 8) and tag line 24 (from 28px in: chips, "+N", Add tag 20 x 20 at the right, 8px gutter). Expanded: the tag line wraps, 24px per line |
| Tag chip | 20px, `rounded-[5px]`, 1px `--toolbar-icon` at 40 %, 12px text, 6px left padding, remove zone 18 x 20 with `X` 10px `--panel-muted-fg`, always shown; only the zone is a button; chips 4px apart; order Cuttable, Script, Sans, Serif, Initials, own A to Z; automatic, overridden and own tags look the same, and the tooltip names the source ("Detected: one piece", "Declared by the font: script", "Set for the included fonts", "Added by you") |
| Tag overflow | "+N" chip, 20px, 12px tabular muted, no x; a button that expands the family; the row tooltip lists all tags |
| Add tag | 20 x 20 outline button `Plus` 14px at the end of the tag line, always shown; it turns the line into a 20px field with inline completion (Up/Down through the matches, Right/End takes the completion), Enter adds or creates, Escape closes; the row height never changes |
| Cuttable mark | Removed: the Cuttable chip replaces it |
| Groups row | Removed, with Reset |
| Cuttable progress (amends) | ..."checking 340/2000" in the This computer header and the Tags list count... |
| Row stability (amends) | It also covers tags: a row stays until the choice or the search changes; a deleted chosen tag switches the strip to All at once (2026-10-20) |

Keyboard shortcuts table: replace "Rename, delete an own group" with:

| Action | Shortcut | Notes |
|---|---|---|
| Add a tag to a font | `F2` in the Fonts list | `0075`; on any row of the family; inline completion, Enter adds |
| Remove a font's tag | `Shift+F2` in the Fonts list, arrows to the chip, `Delete` | `0075`; or the chip's x; not a document change, so there is no undo: add it back |
| Rename, delete an own tag | `F2`, `Delete` in the Tags list | `0075`; built-in tags ignore both |
| Choose a font tag (rewords "Choose a font group") | Tab to the tag strip, arrows, Home, End | `0075`; single choice; writes no history step |

### A12. What the implementer removes

`FontGroupsList.tsx`: the Show switch and the New group row; it becomes the Tags list. `GroupsRow` and Reset in `FontFamilyRows.tsx`. The Scissors slot in `FamilyRow`. The `groups` row kind in `fontRows.ts`: `FAMILY_PX` becomes 52, and an expanded family's height comes from its wrapped tag lines (measured width). The Groups button row in `FontGroupStrip.tsx`. `fontTreeKeys.ts`: Right on an expanded family goes to the first face (it looked for a `groups` row); F2 and Shift+F2 are new.

*The criteria above are reworded by section A10 (applied 2026-10-20), and criteria 1, 5, 7, 8 and 18 again by the customer change of the same day that drops the Tags button and the Tags list for Rename and Delete on the strip cell (A7); the A10 wording of those criteria, the A11 rows for the Tags button and Tags list, and the Tags list key row are superseded. This amendment replaces UX notes §2, the Groups row, Reset and the Cuttable mark of §3, and the matching parts of §5 and §6. Since the check uses Union, the tooltip "Detected: welds into 1 piece" reads "Detected: one piece" everywhere (2026-10-20).*

## Open questions

Decided by the customer 2026-10-20:

- Frage 11 (FONT-CUTTABLE), A: Cuttable is detected by the check and can be overridden by hand. *Customer decision FONT-CUTTABLE A kept; the check uses Union without a gap since Weld was redefined (2026-10-20).* Replaces `0062` Frage 2.
- Frage 12 (FONT-CLASSIFY), A: Sans, Serif and Script are prefilled from the OS/2 class and PANOSE.
- Frage 13 (FONT-BROWSERKEEP), B: at the first browser Add font the page calls `navigator.storage.persist()` (Firefox shows its own prompt). Granted or refused, Add goes ahead. If refused, nothing extra appears; the browser may evict the fonts when storage is tight, and texts using them then show the existing missing-font marking (`0074` criterion 29). The call sits in `requestFontStorageKeep` (`frontend/src/lib/fontStorageKeep.ts`).

## Links

Requirements: R-FONT-002, R-FONT-003, R-FONT-004, R-FONT-005 (groups, favourites), R-FONT-006, R-SYS-007, R-SYS-010
Depends on: `specs/0074-text-objects/`, `specs/0016-boolean-operations/`, `specs/0045-document-formats-library/`, `specs/0062-fonts/` (stage 2 of it)
ADRs: 0020 §2 and §3, 0004 §6, §7 and §12, 0006 item 5
PR: https://github.com/curvyo/curvyo/pull/168
