# Text objects: place text with the Text tool, pick a font and a size, convert it to paths

Status: Draft (customer decisions applied 2026-10-20; UX notes by the ux-engineer pending, then Ready)
Priority: Must (R-EDIT-004 is a Customer Must and names text; `0062`, `0068` and `0070` all build on a text object)
Origin: Customer. Stage 1 of `0062-fonts` ("a tool to add", "our own card in the properties", 2026-10-17) and its Question 1 (FONT-TEXT, default A: the text object gets its own spec first). Items marked **(Proposal)** are the product owner's and not accepted until the customer says so.
Depends on: `0003` and `0009` (Object to path, the Select bar), `0005`, `0019`, `0047` (transforms), `0017` (Style tab), `0020` (one step per interaction), `0023` (groups), `0043` and `0051` (panel tabs), `0050` (active layer, Layers tab rows), `0010` (Escape cascade, key gate).

## User value

As a maker who cuts and engraves names, house numbers and short slogans on a laser, I want to click on the canvas, type my text, pick one of my fonts and set the letter height in millimetres, so that I get lettering of the exact size I need and can turn it into paths for the cut, without first setting it in another program and importing it.

**Field reference (what the other tools do).**

- *Inkscape.* Text tool (T, F8): a click starts a text, a drag a flowed text in a frame. Font family, style and size (pt by default, px or mm by preference) are in the tool controls bar, with a font-family dropdown that names each font in its own face; a separate Text and Font dialog (Shift+Ctrl+T) shows a sample text. Path > Object to Path gives a group of paths, one per letter. The size is the em size, so a "20 mm" text has capitals of about 14 mm, depending on the font. A missing font is replaced silently by a fallback and nothing marks it.
- *LightBurn.* Text tool (T): click and type; font, Height, H-spacing, V-spacing, Bold, Italic, alignment and a Welded switch in the text options bar. The Height field is documented as "not exact": "generally the height of a capital letter X", and makers report capitals that are a tenth smaller than the set height ([docs](https://docs.lightburnsoftware.com/2.1/Reference/Text/), [forum](https://forum.lightburnsoftware.com/t/text-height-not-accurate/113545)). The usual workaround is to type the size into the W/H fields after typing. Convert to Path (Ctrl+Shift+C).
- *Illustrator, Affinity.* Point text by a click, area text by a drag; size in pt; Create Outlines gives a group of compound paths per letter.
- *Where we do better.* **The size is the height of a capital H, exactly** (Question 3): a 20 mm text has a 20.00 mm H in every font, so a name plate comes out the size the customer ordered without measuring. The font list has the maker's own Curvyo fonts first (`0062` criterion 7, pulled forward). The canvas draws the same outlines that Object to path produces (one engine), so what you see is what is cut. A missing font is visible and never silently replaced in the file (ADR 0009 §5).

## What exists today (from the specs and ADRs; the code is not read for this draft)

- No text node kind: the document has layers, groups, paths, compound paths, rectangles, ellipses, polygons and stars (and clones and images on their branches). No font code, no shaping library, no Text tool, no Fonts tab. `R-EDIT-004` "converts any shape or text" has no text yet.
- ADR 0002 §7 (accepted): a text node references a font by family, style and a content hash of the face, plus its string and layout; glyph outlines are made on demand and never stored; glyphs are paths that may be open. ADR 0009 §3: the string merges per character. ADR 0009 §5: a peer without the face draws a substitute, marked, and may not write re-measured layout or convert that text to paths. ADR 0004 §12: fonts are referenced from a user-managed collection, embedding is an explicit export option. ADR 0006 item 5: a font's own licence governs redistribution. ADR 0001 §4: the canvas is WebGL; DOM overlays are allowed for text input.
- The key `T` is free (design system, shortcut table). The Select bar has "Object to path". `R` and `S` in the Select tool open the rotate and size entries.

## Terms

- **Text object:** a node that holds a string, a font reference, a size and a position, drawn as the glyph outlines of that font. It is a leaf.
- **Font, face:** a family (for example "Source Sans 3") has faces (Regular, Bold, Italic, ...). A text uses exactly one face.
- **Size:** the height of the capital H of the face in millimetres (Question 3, default A). Taken from the face's measured H outline (top of H minus baseline); a face without an H uses its OS/2 cap height, else 0.7 of the em.
- **Anchor:** the start of the baseline of the first line (left end), in document millimetres. A click with the Text tool sets it.
- **Edit mode:** the state in which a text object shows a caret and takes typed characters.
- **Font sources:** *Curvyo fonts* (files in Curvyo's own font folder in the user's configuration directory, `fonts/`), *Included fonts* (shipped with the app, Question 2), *System fonts* (installed on the computer; desktop only).
- Lengths in millimetres; geometric tolerance 0.01 mm unless a criterion says otherwise. Notices: at most 70 characters, in the canvas notice slot, as in `0056`.

## Acceptance criteria

### Part A: the Text tool and typing

1. Given the tool rail, then there is a **Text tool** in the Tools card with the key `T` (a tool letter gated like every letter, design system "Keyboard concept") and the tooltip "Text tool (T)" ("Text tool (Esc, T)" while the Select tool has a selection, as the other tools).
2. Given the Text tool, when the maker clicks on the canvas where there is no text object, then edit mode starts with the caret at the click point as the anchor; nothing is written to the document until the first character is typed. A drag acts as a click at the press point (no frame text in this slice).
3. Given edit mode, when the maker types, then each character appears at the caret in the current face and size in the next drawn frame, at most 50 ms after the key on a text of 500 characters (budget measured on the machine the plan names). Supported: letters, digits, punctuation, Space, Enter (new line), Backspace, Delete, Left, Right, Up, Down, Home, End, Shift with these to select, Ctrl+A (all characters of this text), Ctrl+C, Ctrl+X, Ctrl+V of plain text, and input methods (dead keys, compose, IME) through a text-input overlay (ADR 0001 §4). A pasted text with tabs keeps them as spaces of the font's space width x 4 (Proposal).
4. Given several lines, then they are left-aligned at the anchor's x, and each next baseline is one line height lower, where the line height is the face's ascender minus descender plus line gap (hhea values, or OS/2 typo values if the face sets USE_TYPO_METRICS), scaled to the size.
5. Given a face with kerning and ligatures, then the text is shaped by the font's own tables (GPOS kerning, GSUB standard ligatures `liga` and contextual alternates `calt`, on by default). Test: with the fixture face of the plan, "AV" is narrower than the advance of "A" plus "V" by the font's kerning value within 0.01 mm, and "fi" in a face with an fi ligature draws the ligature glyph.
6. Given a character that the face does not contain, then the face's own "missing glyph" (.notdef) is drawn in its place and kept; nothing is refused and no other font is substituted (permit over refuse; fallback fonts per character are out of scope).
7. Given edit mode, when the maker presses Escape, or clicks on the canvas outside the text, or switches tool, then edit mode ends and the whole session (from entering to leaving) is **one** history step: "Add text" for a new text, "Edit text" for an existing one (`0020`). A click with the Text tool outside starts a new text at once (criterion 2). Escape is the first step of the cascade of `0010`; the next Escape selects the Select tool with the text selected (Proposal: as the Pen leaves its tool).
8. Given edit mode ends with an empty string, then the text object is removed (or never created) and no history step is written.
9. Given edit mode, then Ctrl+Z and Ctrl+Shift+Z undo and redo typing inside this session only (as a text field does); after edit mode ends the global undo of `0020` takes the session back in one step.
10. Given the Text tool, when the maker clicks on an existing text object, then edit mode starts on it with the caret at the nearest character boundary to the click. Given the Select tool, a double-click on a text object does the same (Inkscape's behaviour) and switches to the Text tool.

### Part B: font and size

11. Given the Text tool or a selected text, then the **Text bar** (the tool's options bar) shows, in this order: **Font** (a button naming the family and face, for example "Source Sans 3 Bold"; pressing it shows the Fonts tab of criterion 15 with the current face scrolled into view and focused) and **Size** (a value field in mm, the field type of `0017`: drag, arrows, typed entry). No other fields in this slice.
12. Given Size, then it is the height of the capital H (Terms; Question 3). Test: for each included face and for the fixture faces, a one-line text "H" at Size 20 mm converted to paths (criterion 22) has a bounding-box height of 20.00 mm within 0.01 mm. The range is 0.1 mm to 2,000 mm; a typed value outside it is set to the nearest limit with the notice "Size set to 2000 mm, the largest size." (or "0.1 mm, the smallest size."). A typed unit suffix is read as in `0052` (mm, cm, in, pt, px at 96 per inch).
13. Given a text in edit mode or selected, when Font or Size changes, then the change applies to the **whole** text object (no per-character styling) and is one history step ("Change font", "Change text size"); several selected texts all change in one step. With no text selected and no edit mode, the change sets the face and size for the next new text (tool state for the session; Proposal: also kept per user).
14. Given a new text, then its face and size are the last ones set (criterion 13); on the very first use the included sans face (Question 2) at 10 mm; its fill and stroke are the defaults a new rectangle gets, and the Style tab (`0017`) changes them as for a path. Stroke markers (`0018`) do not apply and their controls are not shown for a text.

### Part C: the Fonts tab

15. Given the sidebar strip (`0051`), then **Fonts** is a tab of its own in the Properties panel (icon tab, tooltip "Fonts"), always present, also with nothing selected (`0062` criterion 1). Its key is a Proposal for the UX notes (Shift+Ctrl+T as in Inkscape, if the browser build lets the page take it).
16. Given the Fonts tab, then it lists the families in up to three blocks, in this order, each sorted A to Z, case-insensitive: **Curvyo fonts**, **Included**, **This computer** (desktop only; the browser build has the first two, Question 1). A block with no fonts is not shown. A family that exists in two blocks is listed in each; a pick refers to the face of that block's file. Each row shows the family name set in that family's Regular face (or its first face), loaded when the row scrolls into view; until it is loaded the name shows in the UI font. The list is virtualised.
17. Given the Fonts tab, then a search field above the list filters the rows by a case-insensitive substring of the family name as the maker types, in under 100 ms for 2,000 families.
18. Given the face of the current text (or the next text), then its family row is marked as current and expanded inline to its faces, one row each, named by the face's style name (Regular, Bold, Italic, Light, ...); a click on a family row selects its Regular (or first) face and expands it; a click on a face row selects that face. Selecting applies by criterion 13. Nothing opens as a popup.
19. Given the Curvyo font folder (desktop: the application's configuration directory found by the `directories` crate, sub-folder `fonts/`; `0062` criterion 6), then every TTF, OTF and TTC file in it (not in sub-folders) is listed in the Curvyo fonts block; the folder is created on first start if missing; the list is read again each time the Fonts tab is shown, so a file the maker copied in with the file manager appears without a restart. Curvyo writes nothing into the folder in this slice. A file that is not a readable font is skipped silently (Proposal: counted in the block header "2 files skipped").
20. Given the system fonts (desktop), then the installed fonts the operating system's font configuration names are listed, read on first use of the Text tool or the Fonts tab and not at app start; the app starts no slower than before (within 50 ms of today's start time). The Curvyo and Included blocks show at once; the system block appears when it is read, within 2 s for 2,000 installed faces on the plan's machine, without blocking typing or the canvas.
21. Given the font formats, then TrueType and OpenType (TrueType and CFF outlines) in `.ttf`, `.otf` and `.ttc` (every face of a collection) are supported. A variable font is listed and used at its default instance only. WOFF, WOFF2, Type 1, bitmap and colour fonts are not listed.

### Part D: Object to path and other commands

22. Given one or more selected text objects, when the maker presses **Object to path** in the Select bar, then each text becomes the result of Question 4 (default A: one group, named by the first 24 characters of the string, that holds one compound path per drawn glyph, holes as holes, spaces and line breaks giving no path), in the text's fill and stroke, at the place where the text was drawn, within 0.01 mm of the drawn outlines; the result is selected, and the step is one history step "Object to path" (with other kinds in the selection, one step for all). The result needs no font to open.
23. Given a glyph outline, then the paths reproduce it exactly: quadratic TrueType segments become cubic segments of the same curve (exact degree elevation), CFF cubics are kept; a test compares converted nodes with the font's outline points scaled to the size within 0.001 mm.
24. Given a selection that holds text objects, when a command that works on paths is run (the Boolean operations of `0016`, Combine, Split, Break apart, Cut of `0035`, `0048`, `0036`, Fracture and Flatten of `0037`, Offset of `0038`), then each text is converted by criterion 22 first, silently, in the same history step as the command (as `0047` converts primitives; `0068` criterion 9). The Node tool does not edit a text object in this slice (it shows the text's box as it shows an object it cannot edit today).

### Part E: select, transform, save

25. Given the Select tool, then a text object is hit anywhere inside the box of its lines (from the top of the tallest line to the bottom of the descenders, from the anchor's x to the end of the longest line), as an image is hit by its frame (`0058`); marquee and lasso select it when its box is inside (`0014`).
26. Given a selected text, then move, nudge, rotate, flip, skew and stretch work as for any object (`0005`, `0008`, `0019`, `0044`, `0047`, `0056`), and the text stays a text object that can be edited (criterion 10): a uniform resize multiplies Size by the scale factor (Size shows the new height); rotation, flip, skew and a non-uniform stretch are kept as the object's transform, Size stays as set, and editing keeps that transform. Object to path then applies it (criterion 22 within 0.01 mm).
27. Given the Layers tab (`0050`), then a text object is a row with the kind glyph `Text` and the name of its first 24 characters (an ellipsis after), updated when the string changes, unless the maker renamed the row.
28. Given a project with text objects, when it is saved, closed and opened again on the same computer, then every text shows the same string, face, size, position, transform and style, and is editable. The file stores the string, the font reference (family, style name, content hash of the face; ADR 0002 §7) and the layout values, never glyph outlines. A change of the on-disk shape takes the next free `format_version` at merge; golden file `text_v<N>.curvyo` with one Latin line, two lines, a rotated text, a stretched text, and a text whose face is missing.
29. Given a text whose face is not available on this computer (a project from another computer, or a font file removed), then the text is drawn in the included sans face at its size and kept; the Font button and the face row read the missing name with a marker "missing" (wording in the UX notes); the stored reference is not changed by drawing, editing the string, moving or styling; Object to path on it is refused with the notice "Font Source Serif 4 is missing. Install it to convert." (ADR 0009 §5; see Open question 5). Picking another face replaces the reference by the maker's choice, one step.
30. Given a face that is available but whose content hash differs from the stored one (another version of the same family and style), then the text uses the available face, without a notice (Proposal; the architect's matching rule may say otherwise in the ADR).

### Part F: limits and safety

31. Given a text object, then it holds at most 10,000 characters; typing or pasting beyond that stops at the limit with the notice "A text holds at most 10000 characters." The limit is one named constant with a test.
32. Given a font file that is damaged or hostile (truncated tables, offsets past the end, loops in composite glyphs, a glyph with more than 65,535 points), then reading it never crashes or hangs the app: the face is skipped in the list (criterion 19) or, if it was in use, the text is drawn as by criterion 29. Fuzz or property tests on the font reader are part of the plan (fixtures in `tests/fixtures/fonts/`).
33. Given the core crates, then they read no file and enumerate no fonts: a platform crate finds the folders and the system fonts and passes the bytes of a face in when it is needed; shaping, metrics and outlines are computed from bytes in a `*-core` crate that builds for `wasm32-unknown-unknown` (`CLAUDE.md` §6). The bytes of a face cross into the editor once per face per session, not per frame (ADR 0001 §5).

## Out of scope

- Groups, favourites, the cuttable tag, the Add font button, removing fonts (`0062` stage 2).
- The preview page with the lettering in every font (`0062` stage 3, split and full view).
- Weld and stroke to path for lettering (`0068`).
- Single-line (engraving, plotter) fonts with open glyphs, such as Hershey or SVG fonts (ADR 0002 §7 keeps the model open for them; a later story).
- Alignment other than left, letter spacing, word spacing, line-spacing field, baseline shift, per-character fonts, sizes or colours (rich text), upper-case switch, bold and italic synthesis for families without such faces.
- Text on a path, text in a frame or shape (flowed text), vertical text, right-to-left and mixed-direction layout (bidi); other scripts are shaped by the library but not tested.
- Per-character font fallback; variable-font axes; colour fonts; WOFF and WOFF2.
- System fonts in the browser build (the Local Font Access API is Chromium-only and asks for permission); adding fonts in the browser (`0062` criterion 9).
- Embedding fonts in the project or on export (ADR 0004 §12, a later story), SVG text import and export (`0024` follow-up), the numbers of dimensions (`0070` Question 2).
- Node editing of a text without converting it; a clone of a text (`0049` takes paths only).

## Open questions (customer; each has a default)

The four questions put to the customer were answered on 2026-10-20 (section "Decided by the customer 2026-10-20" below, together with TEXT-ADR and TEXT-WASMSIZE). As asked:

1. **TEXT-FONTSOURCE:** which fonts in this slice. A (default): Curvyo fonts + included fonts + system fonts on the desktop; the browser has the first two.
2. **TEXT-LICENCE:** may we ship fonts with the app? A (default): yes, only SIL OFL 1.1 or Apache-2.0, three families (a sans, a serif, a connected script), each with its licence text in the app's licence list.
3. **TEXT-SIZE:** what Size means. A (default): height of the capital H in mm, exact.
4. **TEXT-TOPATH:** result of Object to path. A (default): one group with one compound path per letter.

Not put to the customer (safe default, vetoable at the demo):

5. **TEXT-MISSING:** Object to path on a text whose font is missing. Default: refused with a notice (criterion 29), because ADR 0009 §5 (accepted) forbids converting with a substitute; converting the fallback silently would change the letters the maker sees in another person's file. The alternative (convert with the fallback face after a notice) needs a new ADR.
6. **TEXT-EDIT-EXIT:** whether the second Escape goes to the Select tool (criterion 7, Proposal) or stays in the Text tool as Inkscape does. Default: Select tool, as the other creation tools.

## Decided by the customer 2026-10-20

All six questions answered A:

1. **TEXT-FONTSOURCE A:** the Curvyo folder first, then Included, then system fonts on the desktop; the browser has the first two only.
2. **TEXT-LICENCE A:** ship Source Sans 3, Source Serif 4 and Great Vibes, licences SIL OFL 1.1 or Apache-2.0 only. Spike note: Source Sans 3 has no "fi" ligature, so the ligature test of criterion 5 uses Source Serif 4.
3. **TEXT-SIZE A:** Size is the height of the capital H in mm, exact.
4. **TEXT-TOPATH A:** Object to path gives a group with one path per letter.
5. **TEXT-ADR A:** [ADR 0020](../../docs/adr/0020-text-nodes-text-engine-and-font-sources.md) is Accepted (2026-10-20, customer).
6. **TEXT-WASMSIZE A:** the wasm budget for the text engine is raised to 1.3 MiB uncompressed (the spike measured 1,247 KiB, 433 KiB gzip; ADR 0020 "Spike results" and `docs/technical-debt.md`).

## Needs from the architect (before Ready)

- **Text node in the document model** (`needs-customer`, document model and file format): the node kind; the string as a Loro text (per-character merge, ADR 0009 §3); the font reference (family, style name, content hash) and its matching rule (criteria 29, 30); size, anchor and transform (criterion 26: what a uniform scale writes, what stays a matrix); `format_version`; how the Layers name and history names follow.
- **Text engine** (new ADR, `needs-customer` for the dependencies): shaping and outlines from bytes in a `*-core` crate (a new crate such as `curvyo-text-core` needs the ADR, `CLAUDE.md` §5). Candidates: `rustybuzz` with `ttf-parser`, or `harfrust` with `skrifa`/`read-fonts`; all MIT or Apache-2.0, all build for `wasm32`. The ADR states the wasm size cost, the glyph-outline cache (per face and glyph in em units, transformed for drawing) and the hostile-file limits (criterion 32).
- **Font discovery** (platform): the system-font list on Linux (fontconfig), Windows and macOS (candidate `fontdb`, MIT), the Curvyo folder through `directories`, the Tauri command that hands bytes over, and the browser build's included fonts (static assets fetched on first use, not compiled into the wasm).
- **Licence record** for the included fonts (ADR 0006 item 5): where the licence texts live, how `cargo deny` and the npm check do not see them and what checks them instead.
- **Text input** on the WebGL canvas: the DOM overlay for the caret, IME and clipboard (ADR 0001 §4), and how its events reach `ui-core`.

## UX notes (to be written by the ux-engineer before Ready)

Needed: the Text tool glyph and its place in the Tools card (column A must still fit 800 x 600 with eight tools); the canvas cursor of the tool; caret, text selection highlight and the look of a text in edit mode (box or none); the Text bar layout (Font button width, truncation of long names); the Fonts tab glyph, the block headers, row height, the expanded face rows, the current mark, the "missing" marker, the search field and its keyboard model (one Tab stop, roving focus as the formats list of `0045`); the hit box drawing of a selected text; the notices' final wording; whether Shift+Ctrl+T works in the browser build.

## Links

Requirements: R-EDIT-048 (new: text objects and the Text tool), R-EDIT-004 (text half), R-FONT-001 (Curvyo font folder, Must), R-FONT-005 (first part)
Depends on: see the header. Followed by: `0062-fonts` stages 2 and 3, `0068-laserable-lettering`; used by `0070-measure-and-dimension` (Question 2 there)
ADRs: 0001 §4 and §5, 0002 §7, 0004 §12, 0006 item 5, 0009 §3 and §5; new: text node (document model) and text engine, both `needs-customer`
PR: -
