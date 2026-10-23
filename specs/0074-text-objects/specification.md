# Text objects: place text with the Text tool, pick a font and a size, convert it to paths

Status: Done (PR #165, merged 2026-10-20). Ready 2026-10-20 with the customer decisions and UX notes applied.
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

1. Given the tool rail, then there is a **Text tool** in the next free slot of the Tools card with the key `T` (a tool letter in every state, gated like every letter, design system "Keyboard concept") and the tooltip "Text tool (T)" in every state.
2. Given the Text tool, when the maker clicks on the canvas where there is no text object, then edit mode starts with the caret at the click point as the anchor; nothing is written to the document until the first character is typed. A drag acts as a click at the press point (no frame text in this slice).
3. Given edit mode, when the maker types, then each character appears at the caret in the current face and size in the next drawn frame, at most 50 ms after the key on a text of 500 characters (budget measured on the machine the plan names). Supported: letters, digits, punctuation, Space, Enter (new line), Backspace, Delete, Left, Right, Up, Down, Home, End, Shift with these to select, Ctrl+A (all characters of this text), Ctrl+C, Ctrl+X, Ctrl+V of plain text, and input methods (dead keys, compose, IME) through a text-input overlay (ADR 0001 §4). A pasted text with tabs keeps them as spaces of the font's space width x 4 (Proposal).
4. Given several lines, then they are left-aligned at the anchor's x, and each next baseline is one line height lower, where the line height is the face's ascender minus descender plus line gap (hhea values, or OS/2 typo values if the face sets USE_TYPO_METRICS), scaled to the size.
5. Given a face with kerning and ligatures, then the text is shaped by the font's own tables (GPOS kerning, GSUB standard ligatures `liga` and contextual alternates `calt`, on by default). Test: with the fixture face of the plan, "AV" is narrower than the advance of "A" plus "V" by the font's kerning value within 0.01 mm, and "fi" in a face with an fi ligature draws the ligature glyph.
6. Given a character that the face does not contain, then the face's own "missing glyph" (.notdef) is drawn in its place and kept; nothing is refused and no other font is substituted (permit over refuse; fallback fonts per character are out of scope).
7. Given edit mode, when the maker presses Escape, or clicks on the canvas outside the text, or switches tool, then edit mode ends and the whole session (from entering to leaving) is **one** history step: "Add text" for a new text, "Edit text" for an existing one (`0020`). A click with the Text tool outside starts a new text at once (criterion 2). Escape is the first step of the cascade of `0010`; the next Escape selects the Select tool with the text selected (Proposal: as the Pen leaves its tool). Escape keeps what was typed: it is the one step of the Escape cascade that writes.
8. Given edit mode ends with an empty string, then for a new text nothing is created and no history step is written; for an existing text the text object is removed, and this is the session's one history step "Edit text" (criterion 7), which Undo takes back with the text's string.
9. Given edit mode, then Ctrl+Z and Ctrl+Shift+Z undo and redo typing inside this session only (as a text field does); after edit mode ends the global undo of `0020` takes the session back in one step.
10. Given the Text tool, when the maker clicks on an existing text object, then edit mode starts on it with the caret at the nearest character boundary to the click. Given the Select tool, a double-click on a text object does the same (Inkscape's behaviour) and switches to the Text tool. Given exactly one text selected and the canvas focused, in the Select or the Text tool, Enter starts edit mode with the caret at the end of the string (the Select tool switches to the Text tool). Given the Text tool with nothing selected, Enter starts a new text at the pointer, or at the viewport centre when the pointer is not over the canvas (`0052`).

### Part B: font and size

11. Given the Text tool, then the **Text bar** (the tool's options bar) shows, in this order; given the Select tool and a selection that holds a text, the Select bar shows the same two controls as a **Text group** after Guide and before Object to path, acting on the texts only: **Font** (a button naming the family and face, for example "Source Sans 3 Bold"; pressing it shows the Fonts tab of criterion 15 with the current face scrolled into view and focused) and **Size** (a bar number field in mm: Up and Down step 1 mm, Shift 10 mm, Ctrl 0.1 mm, typed entry; no drag-scrub, the canvas resize is the drag). No other fields in this slice.
12. Given Size, then it is the height of the capital H (Terms; Question 3). Test: for each included face and for the fixture faces, a one-line text "H" at Size 20 mm converted to paths (criterion 22) has the top of its bounding box 20.00 mm above the baseline within 0.01 mm (for a face whose H stands on the baseline, this is the bounding-box height; Great Vibes' swash H also reaches below it). The range is 0.1 mm to 2,000 mm; a typed value outside it is set to the nearest limit with the notice "Size set to 2000 mm, the largest size." (or "0.1 mm, the smallest size."). A typed unit suffix is read as in `0052` (mm, cm, in, pt, px at 96 per inch).
13. Given a text in edit mode or selected, when Font or Size changes, then the change applies to the **whole** text object (no per-character styling) and is one history step ("Change font", "Change text size"); several selected texts all change in one step. With no text selected and no edit mode, the change sets the face and size for the next new text (tool state for the session; Proposal: also kept per user).
14. Given a new text, then its face and size are the last ones set (criterion 13); on the very first use the included sans face (Question 2) at 10 mm; its fill and stroke are the defaults a new rectangle gets, and the Style tab (`0017`) changes them as for a path. Stroke markers (`0018`) do not apply and their controls are not shown for a text.

### Part C: the Fonts tab

15. Given the sidebar strip (`0051`), then **Fonts** is a tab of its own in the Properties panel (icon tab, tooltip "Fonts"), always present, also with nothing selected (`0062` criterion 1). Its key is Shift+Ctrl+T on the desktop only (as in Inkscape); the browser build has no key, because Chrome reserves Ctrl+Shift+T, and the Font button is the route there.
16. Given the Fonts tab, then it lists the families in up to three blocks, in this order, each sorted A to Z, case-insensitive: **Curvyo fonts**, **Included**, **This computer** (desktop only; the browser build has the first two, Question 1). A block with no fonts is not shown. A family that exists in two blocks is listed in each; a pick refers to the face of that block's file. Each row shows the family name set in that family's Regular face (or its first face), loaded when the row scrolls into view; until it is loaded the name shows in the UI font. The list is virtualised.
17. Given the Fonts tab, then a search field above the list filters the rows by a case-insensitive substring of the family name as the maker types, in under 100 ms for 2,000 families.
18. Given the face of the current text (or the next text), then its family row is marked as current and expanded inline to its faces, one row each, named by the face's style name (Regular, Bold, Italic, Light, ...); a click on a family row selects its Regular (or first) face and expands it; a click on a face row selects that face. Selecting applies by criterion 13. Nothing opens as a popup.
19. Given the Curvyo font folder (desktop: the application's data directory from Tauri's path resolver, `app_data_dir()`, sub-folder `fonts/`, ADR 0020 §3; `0062` criterion 6), then every TTF, OTF and TTC file in it (not in sub-folders) is listed in the Curvyo fonts block; the folder is created on first start if missing; the list is read again each time the Fonts tab is shown, so a file the maker copied in with the file manager appears without a restart. Curvyo writes nothing into the folder in this slice. A file that is not a readable font is skipped and counted in the block header ("12 · 2 skipped"; its tooltip lists up to 10 skipped file names).
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
29. Given a text whose face is not available on this computer (a project from another computer, or a font file removed), then the text is drawn in the included sans face at its size and kept; the Font button and the row of the "Missing" block read the missing name with the marker "missing", and the canvas draws the tag "Font missing" on the text (UX notes §4); the stored reference is not changed by drawing, editing the string, moving or styling; Object to path on it is refused with the notice "Font Source Serif 4 Bold is missing. Nothing was changed." ("2 texts use missing fonts. Nothing was changed." for several; ADR 0009 §5; see Open question 5). Picking another face replaces the reference by the maker's choice, one step.
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

5. **TEXT-MISSING** (default accepted with PR #165): Object to path on a text whose font is missing. Default: refused with a notice (criterion 29), because ADR 0009 §5 (accepted) forbids converting with a substitute; converting the fallback silently would change the letters the maker sees in another person's file. The alternative (convert with the fallback face after a notice) needs a new ADR.
6. **TEXT-EDIT-EXIT** (default accepted with PR #165): whether the second Escape goes to the Select tool (criterion 7, Proposal) or stays in the Text tool as Inkscape does. Default: Select tool, as the other creation tools.

## Decided by the customer 2026-10-20

All six questions answered A:

1. **TEXT-FONTSOURCE A:** the Curvyo folder first, then Included, then system fonts on the desktop; the browser has the first two only.
2. **TEXT-LICENCE A:** ship Source Sans 3, Source Serif 4 and Great Vibes, licences SIL OFL 1.1 or Apache-2.0 only. Spike note: Source Sans 3 has no "fi" ligature, so the ligature test of criterion 5 uses Source Serif 4.
3. **TEXT-SIZE A:** Size is the height of the capital H in mm, exact.
4. **TEXT-TOPATH A:** Object to path gives a group with one path per letter.
5. **TEXT-ADR A:** [ADR 0020](../../docs/adr/0020-text-nodes-text-engine-and-font-sources.md) is Accepted (2026-10-20, customer).
6. **TEXT-WASMSIZE A:** the wasm budget for the text engine is raised to 1.3 MiB uncompressed (the spike measured 1,247 KiB, 433 KiB gzip; ADR 0020 "Spike results" and `docs/technical-debt.md`).

## Needs from the architect (before Ready; resolved by ADR 0020 and `adrs.md`)

- **Text node in the document model** (`needs-customer`, document model and file format): the node kind; the string as a Loro text (per-character merge, ADR 0009 §3); the font reference (family, style name, content hash) and its matching rule (criteria 29, 30); size, anchor and transform (criterion 26: what a uniform scale writes, what stays a matrix); `format_version`; how the Layers name and history names follow.
- **Text engine** (new ADR, `needs-customer` for the dependencies): shaping and outlines from bytes in a `*-core` crate (a new crate such as `curvyo-text-core` needs the ADR, `CLAUDE.md` §5). Candidates: `rustybuzz` with `ttf-parser`, or `harfrust` with `skrifa`/`read-fonts`; all MIT or Apache-2.0, all build for `wasm32`. The ADR states the wasm size cost, the glyph-outline cache (per face and glyph in em units, transformed for drawing) and the hostile-file limits (criterion 32).
- **Font discovery** (platform): the system-font list on Linux (fontconfig), Windows and macOS (candidate `fontdb`, MIT), the Curvyo folder through `directories`, the Tauri command that hands bytes over, and the browser build's included fonts (static assets fetched on first use, not compiled into the wasm).
- **Licence record** for the included fonts (ADR 0006 item 5): where the licence texts live, how `cargo deny` and the npm check do not see them and what checks them instead.
- **Text input** on the WebGL canvas: the DOM overlay for the caret, IME and clipboard (ADR 0001 §4), and how its events reach `ui-core`.

## UX notes

ux-engineer, 2026-10-20. Checked against `docs/design-system.md` on `main` (c55f50a), `ToolRail.tsx`, `App.tsx` (overlay row), `PanelStrip.tsx`, the Layers and formats lists, and the customer decisions of 2026-10-20 (all option A: font sources Curvyo, Included, This computer; Source Sans 3, Source Serif 4, Great Vibes shipped; Size = capital H in mm; Object to path = one group, one path per letter; hidden textarea as input sink, canvas draws everything). Existing rows are cited by name; new numbers are below and in "Design-system rows to add". No hint labels, no popup, no panel section that comes and goes.

### 1. Text tool

| Item | Decision |
|---|---|
| Glyph | Lucide `Type` 20px in the tools card (the Layers kind glyph is the same `Type` at 16px, as `Square` is for the rectangle). |
| Slot | Appended at the bottom of the Tools card in ship order (rule "Left tool panel"): the next free slot. The Pencil (`0032`) is on hold, so Text takes slot 7 and the Pencil, when it ships, the slot after it (lead, 2026-10-20). No built tool moves. |
| Name, tooltip | "Text tool (T)" in every state. **T is a tool letter in every state** (like B, N, E, P, O, `*`): it has no meaning on a selection, so the "(Esc, T)" form of criterion 1 does not apply; only R and S carry it today. Amend criterion 1. No second tooltip line. |
| Cursor | The built-in `text` (I-beam) everywhere in the tool: over empty canvas, over any object, over a text and in edit mode. It does not rotate with a rotated text. Pan (Space held, middle button) overrides it as for every tool. |
| Hover | Over a text object, no button down and not in edit mode: the `--hover-box` on the text's line box (what a click edits). No hover over other objects (a click there starts a new text). |
| Press on a locked layer | No edit mode; the Pen's alert in the canvas notice slot, with the same words: "Layer Cut is locked. Unlock it or choose another layer." |
| Bar | The Text bar (section 3), always shown while the tool is active, with or without edit mode. |

### 2. Edit mode: entry, exit, caret, selection, IME

**Entry.**
- Text tool, click (press and release, or a drag, which counts as a click at the press point) where no text object is hit: edit mode with the caret at the click point, which is the anchor (start of the first baseline). Nothing is written until the first character.
- Text tool, click on a text: edit mode on it, caret at the nearest cluster boundary.
- Select tool, double-click on a text: switch to the Text tool, edit mode, caret at the nearest boundary. In a group the first double-click enters the group (`0023`), the next one edits the text.
- **Keyboard (new, needed for keyboard access):** Enter with exactly one text selected and the canvas focused, in the Select or the Text tool: edit mode with the caret at the end of the string (the Select tool switches to the Text tool, as the double-click does). Enter in the Text tool with nothing selected: a new text at the pointer, or at the viewport centre when the pointer is not over the canvas (the rule of `0052`). Enter with one group selected keeps its `0023` meaning.

**Exit.** Escape (first step of the cascade), a click on the canvas outside the edited text, a tool switch, or a click on another text. A click outside with the Text tool starts the next text at once (criterion 7); a click on another text edits that one. What ends edit mode writes one step ("Add text" / "Edit text"), or nothing for an empty string. **Not exits:** a press on the bars, the rail's non-tool buttons, the panel or the Fonts tab (the maker changes Font or Size and keeps typing), window blur, pan and zoom.
- **Escape is the one cascade step that writes.** The text was on screen as typed; Escape keeps it. Record this as an exception in "Escape cascade". Second Escape: the Select tool with the text selected (Question 6 default); third: clears the selection.
- Saving (Ctrl+S, File tab) while in edit mode commits the session as its step and edit mode continues with a new session; the file holds what is on screen.
- History tab Undo or Redo pressed in edit mode: the session is committed first, then the button acts (so Undo takes the whole session back).

**Key gate while typing.** Focus is in the hidden textarea, so the existing gate ("focus in a text field") already blocks every letter, Delete, the arrows' nudge, Home, End, Page Up and Page Down (z-order), Ctrl+A over objects, Ctrl+G and the canvas undo. In edit mode:
- Letters, digits, punctuation, Space, Enter, Backspace, Delete, arrows, Home, End, Shift with them, Ctrl+A, Ctrl+C, Ctrl+X, Ctrl+V, Ctrl+Z, Ctrl+Shift+Z go to the edit session (criteria 3, 9). Ctrl+Left and Ctrl+Right move by word (Ctrl+Shift extends); the browser gives this through `beforeinput`/selection, nothing extra.
- **Space types a space.** It does not pan while edit mode has focus; pan with the middle button or the wheel. Enter adds a line; it does not finish anything. Tab moves focus to the Text bar (Font button) and does not insert a tab; edit mode stays; Escape or Enter in the bar returns focus to the textarea with the caret where it was ("return focus to the canvas" means the edit sink while edit mode lasts).
- Escape during an IME composition belongs to the IME (`isComposing`): it cancels the composition and does not end edit mode.
- App-global chords still work: Shift+Ctrl+F, D, H, L, T, Ctrl+S, Ctrl+O, Ctrl+N (the replacing ones commit the session first, then ask about unsaved changes as today).
- macOS: the system Edit menu's Cut, Copy, Paste, Select All act on the focused textarea, so they work in edit mode without extra code.

**Look in edit mode** (canvas, WebGL, constant screen sizes; nothing here is a DOM overlay except the invisible sink):
| Item | Decision |
|---|---|
| Text box | The text's line box (criterion 25) as the dashed "Bounding-box selection outline", in the text's own frame. No transform handles, no parameter handles in the Text tool (as the shape tools). A new, empty text draws no box, only the caret. The box is the same one the Select tool shows, so the second Escape changes nothing on screen but the handles. |
| Caret | A 1.5 screen-px line in `--text-caret` (`--accent`) over a 1px white casing each side, from the line's ascender to its descender at the caret's x, turned and skewed with the text's transform; at least 12 screen px long (centred on the line's middle) so it reads on a small text when zoomed out. Blink 530 ms on, 530 ms off; steady while typing and for 500 ms after a key; steady with `prefers-reduced-motion`; hidden while the window has no focus. |
| Selection | Per line, the span between the two boundaries, ascender to descender, filled `--text-selection` (`--accent` at 30 %) over the glyphs; no casing. Line breaks inside the selection show as a 0.3 em wide band at the end of the line. Caret hidden while a range is selected. |
| IME preedit | The preedit string is shaped and drawn inline at the caret in the text's face and fill, as if typed, with a 1 screen-px `--text-preedit` (`--accent`) underline under the whole composition and 2px under the clause the IME marks as target; the caret where the IME puts it. Not stored, not in the history, not in the undo of the session until committed. The sink is placed at the caret's screen point (1 x 1 px, transparent) so the candidate window opens next to the caret. |
| Pointer in edit mode | Drag across the text: select; double-click: select a word; Shift+click: extend. A drag that leaves the text keeps selecting (auto-scroll is not built). |
| Screen reader | The sink is a real textarea named "Text" (`aria-label`), described as "Source Sans 3 Bold, 20 mm"; it mirrors the string and the selection of the session, so screen readers and IMEs that read surrounding text (reconversion, dead keys) see the context. Nothing else is announced per key. |
| Over the limit | "A text holds at most 10000 characters." in the canvas notice slot (status, 3 s). |

### 3. Text bar and the Select bar's Text group

**Text bar** (Text tool; the overlay row slot, centred like the Node and polygon/star bars; pill rows of 36px, 28px controls). One group, in this order:

1. **Font button**, 168 x 28px: the outline text button of "Fit to content" (1px `--toolbar-icon` at 60 %, `rounded-[5px]`, hover `--editor-accent-hover`, focus ring as the bars), 8px padding, the Fonts tab glyph `CaseSensitive` 16px, 6px, the face name in 14px UI font ("Source Sans 3 Bold"; the Regular face is written "Source Sans 3 Regular", never only the family), one line, ellipsis at the end. Not a dropdown: no chevron, nothing opens over the canvas. **Press:** shows the panel body if hidden, activates the Fonts tab, scrolls the current face row into view; keyboard activation puts focus on that row, a mouse press too (the point of pressing it is to pick; criterion 11). Tooltip (Radix, 400 ms, bottom): "Font (Shift+Ctrl+T)" on the desktop, "Font" in the browser build, and a second line with the full face name (it may be cut in the button). Several texts with different faces: "Mixed" in `--field-placeholder`.
2. 12px gap, **Size field**, 120 x 28px: the bar number field of the Curve field (white ground, label "Size" inside the left edge, value right-aligned 14px tabular, "mm" inside the right edge; always mm, whatever the display unit, like the nudge readout). Up to two decimals, trailing zeros trimmed ("20", "12.5", "0.25"). Up and Down step 1 mm, Shift 10 mm, Ctrl 0.1 mm, live on the canvas, one commit "Change text size" on key-up; Enter commits a typed value (units of `0052`); Escape restores. Out-of-range values: clamped with the notice of criterion 12, posted 4px under the field (Action notice, status, 3 s). Invalid text: "Enter a number" (row "Bar number field"). "Mixed" with different sizes.

**No drag-scrub and no fill bar on Size** (amend criterion 11): a fill bar from 0.1 to 2000 mm says nothing, and the drag on the canvas (a corner resize with Ctrl, which is uniform) already scales the size, with the readout of section 6. The keyboard and typed entry are the precise routes.

Width: 8 + 168 + 12 + 120 + 8 = 316px, one row in the 324px overlay row at 800 x 600 (panel open, two rail columns).

**Select bar** (Select tool, the selection holds a text): a kind group "Text" with the same Font button and Size field, after the Guide button and before "Object to path" (the kind-group zone of "Select bar"). It acts on the texts in the selection only. Criterion 11 is amended to say: the Text bar in the Text tool, the Text group in the Select bar; one bar at a time stays true. Stroke markers are removed from the Style tab for a text (criterion 14), as for a compound path.

### 4. Fonts tab

**Strip.** Sixth tab, appended after History (File, Document, Style, Layers, History, Fonts; no existing tab moves). Glyph Lucide `CaseSensitive` ("Aa") 16px, 1.5px absolute stroke. Tooltip "Fonts (Shift+Ctrl+T)" on the desktop; **the browser build has no key**: Chrome reserves Ctrl+Shift+T (reopen closed tab) and the page cannot take it, so the tooltip there reads "Fonts" and the Font button is the route (answers the question in the spec's UX list). Never dimmed (criterion 15). The tab never activates by itself: selecting a text does not switch to it (rule "Active tab", as History, Layers, File). Shift+Ctrl+T does what the Font button does: body shown, Fonts tab, focus on the current face row.

**Layout** (244px content, fills the body height, the list is the one inner scroll area, like Layers):
- Header row (24px + 8px): "Fonts" and the subject line: the current face ("Source Serif 4 Bold"), "Mixed", or, with no text selected and no edit mode, the face for the next text with the prefix "Next text: " (a state, not a hint).
- **Search field**, 244 x 28px, white ground, Lucide `Search` 16px inside the left edge (`aria-hidden`), placeholder "Search fonts" (`--field-placeholder`; it names the field), a 24px clear button (`X`, "Clear search") at the right while it holds text. Filters as the maker types (criterion 17), case-insensitive substring of the family name, over all blocks.
- 8px, then the **list**: virtualised, minimum 120px, stable 8px scrollbar gutter.

**Blocks**, in this order, each sorted A to Z: **Missing** (only while the current face is missing, see below), **Curvyo fonts**, **Included**, **This computer** (desktop only). A block with no fonts is not shown, with one exception: on the desktop the "This computer" header is in the list from the first frame with the muted state "Reading…" at its right until the system list arrives, so the block does not appear from nowhere; its rows then fill in under it. With a search, a block without a match is not shown.

**Block header**: 24px, sticky at the top of the list while its rows scroll under it, 12px semibold `--panel-muted-fg`, name at the left, at the right the row count (tabular, "12"; with a search "3 of 120"), and for Curvyo fonts the skipped files when there are any ("12 · 2 skipped"; tooltip lists up to 10 skipped file names; accepts the Proposal of criterion 19). Not a button; no fold in this slice.

**Family row**: 32px (so a script face's ascenders fit), the List row states. Chevron slot 16px (`ChevronRight` / `ChevronDown` 12px, its own 24 x 32px hit area, `aria-hidden`; expands without picking), 4px, the family name set in its Regular (or first) face at 16px, ellipsis, clipped to the row; until that face is loaded, the same name in the UI font at 14px, swapped in place (fixed row height: nothing jumps). At the right the face count, 12px tabular muted ("4"). Tooltip: "Source Sans 3, 12 faces". Rows load their face only when they have been in view for 100 ms (fast scrolling loads nothing) and at most the visible rows plus 10 are held.

**Face row**: 28px, indented 20px (no chevron), the style name ("Regular", "Bold", "Light Italic") set in that face at 14px, UI font until loaded; loaded when its family is expanded.

**Current mark**: the current face row has the selected look of the List row (`--value-fill` ground, 3px `--accent` bar, `aria-selected`). Its family is expanded (criterion 18). If the maker collapses it, the family row shows the 6px `--accent` diamond after the name (the Layers rule "Collapsed container with a selected descendant"), tooltip adds "Contains the current font". Several different faces selected: no row marked.

**Picking**: a click on a family row picks its Regular (or first) face and expands it; a click on a face row picks that face; both apply by criterion 13 (one step "Change font"; with nothing selected it sets the next text's face). A mouse pick returns focus to the canvas (to the edit sink in edit mode, so the maker keeps typing); a key pick leaves focus in the list. A family in two blocks is two rows; each picks its own block's file.

**Keyboard** (`role="tree"` "Fonts", one Tab stop with roving focus, as the formats list): Up, Down, Home, End, Page Up, Page Down between rows (headers are skipped; they are not focusable); Right expands a family, or moves to its first face; Left collapses, or moves from a face to its family; **Enter or Space picks** (arrows only move the focus; a font is never changed by browsing); a printable character typed in the list goes to the search field and is appended (the field takes focus); Escape returns focus to the canvas. In the search field: Down moves to the first row; Enter picks the first matching family's Regular face (as a click on it); Escape clears the text, a second Escape returns to the canvas. Rows: `aria-level` 1 and 2, `aria-expanded`, `aria-setsize`, `aria-posinset`; the block is in the accessible name of a family row ("Great Vibes, included, 1 face").

**Empty states** (14px, then a 12px muted line, like Layers): no font at all (browser build before the included fonts have loaded): "Loading fonts…"; search without a match: "No font matches "vibs"." No muted second line (it would be an operating tip).

**Missing font**:
- **In the list**: the block "Missing" first, with one face row per missing face of the current text(s): Lucide `TriangleAlert` 16px in `--field-invalid` in the indent, the stored name in the UI font ("Source Serif 4 Bold", there is no face to set it in), and the word "missing" right-aligned, 12px `--field-invalid` (4.8:1; glyph and word, never colour alone). It has the current mark. Picking any other row replaces the reference (criterion 29); the Missing block then goes. Not pickable itself (Enter does nothing; it is already current).
- **On the Font button**: `TriangleAlert` 16px `--field-invalid` instead of `CaseSensitive`, the stored name, and the 12px "missing" tag before the right edge (the name truncates first). Tooltip second line "Missing on this computer. Shown in Source Sans 3." (a state).
- **On the canvas**: the text is drawn in the included sans (Source Sans 3 Regular) at its size, and carries a **tag** whether selected or not, because the drawn letters are not the letters of the file and the maker must see that before a cut: the clone tag's surface (DOM, 20px, `--toolbar-bg`, 1px `--field-invalid` border, 8px radius, 6px side padding, `pointer-events: none`, `aria-hidden`), `TriangleAlert` 12px `--field-invalid`, 4px, "Font missing" 12px `--toolbar-icon`. Placement and limits: the rows "Tag placement" and "Many and small" of the clone marks (above the line box, left-aligned, 10px clear, then the other candidates; at most 12 tags, only from a 24px screen box; dropped when no candidate is free). Hidden during a drag and while a `--preview-new` outline shows, like the clone tags. In the Layers row the kind glyph stays `Type`; the row tooltip gets the muted line "Font missing: Source Serif 4 Bold".
- **Object to path and path commands on such a text** are refused (criterion 29, Question 5): refusal look, the offending texts drawn with the "Refusal outline", posted where the command posts (4px under "Object to path"; beside the rail for a rail card). Texts: "Font Source Serif 4 Bold is missing. Nothing was changed." (one face; a family and face longer than 32 characters are cut with an ellipsis so the sentence stays under 70); "2 texts use missing fonts. Nothing was changed." The command converts nothing when any text of the selection is refused (all or nothing, as the Boolean refusals). The spec's "Install it to convert." is dropped (an operating tip).

### 5. Selected text in the Select tool

- The selection box is the oriented line box (criterion 25), not the tight outline bounds, so it does not jump while letters with and without descenders are typed or an empty line is added; the handles are the usual ones ("Transform handle layout"), no parameter handles. The pale red true box of the move chip (`0047`) still shows the tight outline bounds.
- Resize readout: one text, a corner drag with Ctrl (uniform): a second line "Size 24.0 mm" (the cap height the release will set). Without Ctrl the drag stretches and the line is absent (the size stays as set, criterion 26).
- The S chip keeps W and H of the line box; the exact cap height is the Size field.
- Layers row: `Type` glyph, the name from the first 24 characters with each line break shown as a space.

### 6. Notices (final wording)

| Where | Text |
|---|---|
| 4px under the Size field, status, 3 s | "Size set to 2000 mm, the largest size." / "Size set to 0.1 mm, the smallest size." |
| Canvas notice slot, status, 3 s | "A text holds at most 10000 characters." |
| Under Object to path / beside the rail, alert, 8 s | "Font Source Serif 4 Bold is missing. Nothing was changed." / "2 texts use missing fonts. Nothing was changed." |
| Canvas notice slot, alert, 8 s | "Layer Cut is locked. Unlock it or choose another layer." (Text tool press, the Pen's words) |
| Object to path success | as today for the Select bar button; the History label "Object to path" |

### 7. Fit at 800 x 600

- **Rail, column A** (Tools, Group, Path tools): with the Pencil and Text the Tools card has 8 buttons, 8 x 44 + 4 = 356px; Group 92px, Path tools 48px, two 8px gaps: 512px from the 12px inset, ending 524px below the viewport top. The Tauri viewport at 800 x 600 is about 546px (`ToolRail.tsx`; the rail root keeps 4px), so **22px clear**. The browser build at the minimum window is about 8px short: the "Too short" rule applies (the cards under Tools scroll in their column). With Text only (no Pencil yet) column A ends 480px. **Column A is then full**: the next tool needs the customer's decision on a third column or another place (row "Tool rail architecture" amended).
- **Text bar**: 316px in the 324px overlay row, one row. **Select bar with one text selected**: settings; Flip and Guide; the Text group (300px); Object to path and Linked clone: 4 rows, 144px (a rectangle is 5).
- **Fonts tab**: panel viewport about 532px: header 32px, search 28px + 8px, list about 464px: one header and about 13 family rows, or the current family with 5 faces and 8 more families. No horizontal scroll; names truncate.
- Nothing in this slice changes the canvas size; the panel stays 280px with the 32px strip.

### 8. Changes to the criteria

Applied on 2026-10-20 to criteria 1, 7, 10, 11, 15, 19 and 29 (implementer, from the ux-engineer's list).

## Links

Requirements: R-EDIT-048 (new: text objects and the Text tool), R-EDIT-004 (text half), R-FONT-001 (Curvyo font folder, Must), R-FONT-005 (first part)
Depends on: see the header. Followed by: `0062-fonts` stages 2 and 3, `0068-laserable-lettering`; used by `0070-measure-and-dimension` (Question 2 there)
ADRs: 0001 §4 and §5, 0002 §7, 0004 §12, 0006 item 5, 0009 §3 and §5; new: text node (document model) and text engine, both `needs-customer`
PR: https://github.com/curvyo/curvyo/pull/165
