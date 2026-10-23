ux-engineer, 2026-10-20. Written for the customer answers of 2026-10-20 (SL-DEF A, SL-LICENCE A, SL-STICK A, SL-WASM 2 MiB, SL-KIND A) and the architect's `adrs.md` draft of the same day. Checked against `docs/design-system.md` on `origin/main` and on `story/font-library` (`5d5bcd8`), the `0074` UX notes (sections 2 to 7), the `0075` UX notes and the "UX amendment: tags replace groups", and on `story/font-library` `FontTagStrip.tsx`, `FontTagLine.tsx`, `FontTreeRows.tsx`, `FontSearchRow.tsx`, `fontLibraryText.ts`, `StyleSection.tsx`, `FillSection.tsx`, `clone_overrides.rs` (style skip notice), `offset_plan.rs` and `useOffset.ts` (Offset refusals). Existing rows are cited by name.

This slice adds **no new control, no new key, no popup, no panel section that comes and goes and no hint label**. A single-line font is an ordinary font that draws differently. What the maker sees: one more strip cell and chip, family rows drawn in lines, a Style tab without Fill, and one more refusal sentence.

### 1. The "Single line" tag

| Item | Decision |
|---|---|
| Strip order (criterion 4) | All, Favourites, Cuttable, **Single line**, Script, Sans, Serif, Initials, then own tags A to Z (the `0075` order with Single line after Cuttable). A built-in cell: always in the strip, no Rename or Delete icons, F2 and Delete ignore it. |
| Chip order | The same order everywhere: **Cuttable, Single line, Script, Sans, Serif, Initials**, then own tags A to Z. Single line second, so the two production tags stay out of the "+N" overflow. |
| Widths at 244px | "Single line" is about 84px as a strip cell. Row 1: All, Favourites, Cuttable (about 183px, no room for an 84px cell). Row 2: Single line, Script, Sans, Serif (about 231px). Row 3: Initials and the first own tags. **The built-ins take 3 strip rows** instead of 2 (92px instead of 60). The wrap is the strip's own (`flex: 1 1 auto`, cells grow to fill their row, text never truncated); nothing is shortened. The 4-row cap of the strip stays; with more own tags than fit in rows 3 and 4 the strip scrolls by rows as before. If the build measures Serif in row 3, that is accepted: the row count is the same. |
| Chip | The `0075` tag chip, no special look. Tooltip "Detected: single-line font" (a state). When the maker removed it and adds it back, the override is cleared as for every automatic tag; added by hand to an outline font: "Added by you". Removing the chip changes nothing about how the font draws (the kind is the font's, the tag is a name). |
| Included fonts | Hershey Sans 1-stroke and EMS Readability: chips Single line, Sans. Hershey Script 1-stroke and EMS Allure: Single line, Script. Two chips each, 146px or less, so no "+N" in the 176px tag line. |
| Cuttable | A single-line family never gets the Cuttable chip automatically (criterion 5). The Cuttable progress counts only outline faces: "checking 340/1996" when 4 of 2,000 faces are single-line, so the bar ends when the outline faces are done. |
| Empty list | "No font in Single line." / "No font in Single line matches "vibs"." (the `0075` empty texts; the "yet" form is Cuttable's only). |

### 2. Fonts-tab rows of single-line faces

The rule: **a single-line row is an outline row whose name is drawn in lines.** Same height, slots, star, tag line, face count, current mark, tooltip, keys and loading. Only the name's pixels differ.

| Item | Decision |
|---|---|
| What is drawn (criterion 6, `adrs.md` decision 6) | The name laid out by the engine (`face_preview_path`) as an inline `<svg>` in the row's name slot: `<path fill="none" stroke="currentColor" stroke-width="1" stroke-linecap="round" stroke-linejoin="round" vector-effect="non-scaling-stroke">`. 1 CSS px (2 device px on a 2x screen, as the UI font's stems scale), round caps and joins so the letters read as pen lines, `aria-hidden` (the row's accessible name is the text, as today). |
| Size and baseline | **1 em = 16px**, the CSS size of the outline names, so a single-line name and an outline name are scaled by the same rule (em, not cap height; outline faces differ in cap height too). The `<svg>`'s baseline sits on the name line's text baseline (`vertical-align` by the descent), so the name, the face count and a neighbouring outline row line up. Clipped to the 28px name line like every name. |
| Colour | `currentColor`: the row's text colour in both themes, in the hover, focus and selected states (`--value-fill` ground of the current face row). No accent, no muted variant: a line name is not a different state. |
| Width, ellipsis | When the name is wider than the slot, the row asks again with fewer characters and "…" (U+2026; "..." when the face has no glyph for it) until it fits, at most 6 calls, cached per face and slot width; the tooltip keeps the full name ("Hershey Script 1-stroke, 1 face"), as for outline rows. A hard clip without the ellipsis would be the one row type that ends differently. |
| Face rows | The style name ("Regular") drawn the same way at 1 em = 14px, the face-row size. |
| Loading | As `0074`: after 100 ms in view, visible rows plus 10 held; until then the name in the UI font at 14px, swapped in place, no height change. A face that fails to parse stays in the UI font (it is listed, so the file was read once; the next read decides). |
| Missing row | The stored name in the UI font with `TriangleAlert` and "missing" (`0074`): there is no face to draw it with. Unchanged. |
| Hit and keys | The whole row, as today; the thin lines are not a target. |

### 3. Stroke and fill when a single-line font is picked (criterion 9)

**Decision: no notice, no popup, nothing extra on screen.** The change is shown by the thing that changed:
- **Canvas:** the text appears in lines in the fill colour in the next frame. That is the picture the maker picked the font for; without the switch it would be invisible, which is the only case the rule exists for.
- **Style tab** (when the maker opens it): Stroke reads Solid, the fill colour, 0.25 mm, Cap and Join Round, like any set stroke. The Width field's reset slot stays empty (0.25 mm is the default).
- **History:** one row, "Add text" or "Change font"; one Undo takes face and stroke back together. No second history label and no "stroke added" wording.

A notice was considered and rejected: it would describe what is already visible, and a pick in the Fonts tab is often repeated while browsing faces (each pick of a single-line face on a stroke-off text would post it again). A stroke that is on is never touched, so the maker who set a stroke never sees anything change but the font.

Not decided by this note (criterion 10 as written): switching back to an outline face keeps the stroke that was switched on. On a laser that stroke is a cut line around every letter. Frage 23 (SL-BACK) below; default: keep (criterion 10 unchanged).

### 4. Style tab for a single-line text (criteria 7, 8)

- **Fill is hidden, not disabled** (row "Hidden, not disabled"; no new exception): while every object the Style tab edits is a single-line text, the Fill section (title row with its Paint switch, Color, picker, Opacity) and the 16px rule above it are **not in the tree**, exactly as the Markers block is not shown for a text (`0074` criterion 14). The kept fill stays in the document, unseen. Nothing moves: Fill is the last section, so the Stroke section above it is the anchor. The Style tab is then 460px high (700 less Fill 224 and the gap 16), so at 800 x 600 the whole tab fits without scrolling.
- **Why not disabled:** a dimmed Fill block would be the first dimmed control inside the panel body; the three recorded exceptions are a tab icon and two Select-bar buttons that must not move. Here nothing moves when Fill goes, so the rule applies as written.
- **Mixed selection** (a single-line text and other objects): Fill is shown and acts on the others only (criterion 8). Its values are read from the objects it changes (the single-line texts' kept fill counts for nothing, so "Mixed" appears only when the others differ). An edit that left single-line texts alone posts the **style skip notice** of linked clones (same component, place 4px under the Fill title row, status, same duration): "1 single-line text has no fill and was not changed." / "2 single-line texts have no fill and were not changed." When clones were skipped too, both sentences, clones first. This is the only notice of the slice outside the refusals; it is needed because the edit visibly did something else to part of the selection.
- **Stroke** is shown in full; Markers stay hidden (a text). Stroke None is permitted: the text is then not drawn, like a path with neither fill nor stroke, and is still selected by its line box. No warning (permit over refuse).
- A single-line text whose face is missing follows the same rule (the stored kind decides, `adrs.md` decision 1).

### 5. Area commands refuse (criterion 19)

| Item | Decision |
|---|---|
| Buttons | The Boolean card (Union, Difference, Intersection, Exclusion, Reverse difference, Fracture, Flatten) does **not dim** for a single-line text: the dimming rule stays "fewer than two objects or another tool" (row "Boolean toolbox"). Pressing answers with the refusal, as for groups. Tooltip rule and note lines unchanged (a note about single-line texts would be a tip). |
| Wording | One text: "Single-line text has no area. Nothing was changed." Several: "2 single-line texts have no area. Nothing was changed." (always under 70 characters). Alert, 8 s. |
| Order of reasons | One notice, one reason: a missing or loading face first (the `0074` sentence, criterion 22), then single-line texts, then the command's own refusals (groups, open paths, out of range). All or nothing: nothing in the selection changes. |
| Where | Where each command posts today: beside the rail for the Boolean card, Weld's place for Weld; Offset below. |
| Refusal outline | Every offending text is redrawn by the row "Refusal outline": its glyph centre lines as hollow 2px `--field-invalid` lines with the white casing, open from first to last node (the rule "No overlay draws a closing line" already covers it). Not the line box: the lines are what has no area. At most 200 offenders drawn, the count in the sentence exact. |
| Offset tool | With a single-line text in the selection, the handles are drawn as for an outline text. A corner drag draws **no preview**; readout line 3 reads "Single-line text has no area" in `--field-invalid` (the slot of "Nothing is left"); the release writes nothing and posts the sentence above in the canvas notice slot (alert, 8 s) with the refusal outline. In the Offset entry chip, Enter shows the sentence in the chip's message card and the chip stays open (as the group refusal). |
| After Object to path | Ordinary open paths (criterion 20): the `0054` and Weld behaviour and notices, nothing of this section. |
| Converting commands | Object to path, Combine, Split, Break apart, Cut: no notice of their own beyond today's; they convert first (criteria 17, 18). |

### 6. Missing single-line font (criterion 22)

The `0074` marks, unchanged in look and place: the Missing block row, the Font button (`TriangleAlert`, stored name, "missing"), the canvas tag "Font missing" on every such text, selected or not, and the Layers tooltip line. Only the substitute changes:
- Drawn in **EMS Readability**, stroke only, with the text's stroke (and the criterion 9 switch never runs on drawing: the file is not changed by drawing).
- Font button tooltip, second line: "Missing on this computer. Shown in EMS Readability."
- Commands: the `0074` sentence wins ("Font Hershey Script 1-stroke is missing. Nothing was changed."), for the converting and the area commands alike.
- Until EMS Readability is loaded (browser, first use) the text is not drawn and the tag still shows, as an outline text waits for its sans substitute today.

### 7. Caret, selection and preedit on centre-line glyphs (criterion 11)

Unchanged from `0074` section 2; written down because the glyphs are thin:
- **Caret** from the line's ascender to its descender (`ascent`/`descent` of `font-face`, criterion 15), at least 12 screen px, `--text-caret` with its casing. It never depends on the strokes, so it does not shrink or move on letters with less ink.
- **Selection** band ascender to descender, `--text-selection` (`--accent` at 30 %) over the lines; the lines stay in their stroke colour under it. A selected missing glyph without `d` shows the band over its advance, so the selection has no gaps.
- **Hit testing** (click to place the caret, Text tool hover, Select tool press): by advance and line box, as today. The maker never has to hit a 0.25 mm line; a press between strokes inside the line box hits the text.
- **IME preedit** drawn with the text's stroke (not its fill), with the usual `--text-preedit` underline.
- Typing in a single-line face with Stroke None shows only the caret moving; permitted (section 4).

### 8. Add font accepts `.svg` (criterion 3)

| Item | Decision |
|---|---|
| Button | Unchanged: 28 x 28px, `FilePlus`, "Add font…". |
| File dialog filter | One filter, "Fonts (.ttf, .otf, .ttc, .svg)" (desktop); the browser picker's `accept` adds `.svg` and `image/svg+xml`. No separate "Single-line fonts" filter: the maker should not have to know the format to add a font. |
| Result | The Action notice of `0075`, same words, a file counts as one font as today: "1 font added." / "logo.svg is not a font and was not added." (an SVG without a `<font>` holding a glyph) / "2 files are not fonts and were not added. Not fonts: logo.svg, plate.svg." No word about single-line in the notice: the added family appears expanded with its Single line chip and its name drawn in lines, which says it. |
| After adding | `0075` rule: scrolled into view, expanded, focus by keyboard only. With **Single line** chosen in the strip, an added outline font is outside it, so the strip goes to All (the existing "would be hidden" rule); an added single-line font stays in view. |
| Drop | The same: `.svg` files dropped on the tab body are added like the button. |
| Folder | An `.svg` in the Curvyo folder that is not a font counts in "12 · 2 skipped" and is listed in that tooltip (criterion 1). |

### 9. Fit

The window minimum is 800 x 600 (`tauri.conf.json`); the figures are given for 800 x 640 as asked and for 800 x 600. Panel viewport about 572px at 640, 532px at 600.

| Part | 800 x 640 | 800 x 600 |
|---|---|---|
| Fonts tab, strip 3 rows (the built-ins, section 1) | header 32 + strip 92 + 8 + search 28 + 8: list about **404px**, one block header and 7 family rows | list about **364px**, 6 family rows (was 396px and 7 with 2 rows) |
| Fonts tab, strip at its 4-row cap | list about 372px, 6 family rows | list about 332px, 5 family rows (the 120px minimum holds) |
| Style tab, single-line text only | 460px, no scroll | 460px, no scroll |
| Style tab, mixed selection | as today (700px, Fill reached by scrolling at 600) | as today |
| Text bar, Select bar, rail | unchanged (no control added) | unchanged |

No horizontal scroll anywhere; names are cut by the ellipsis rule of section 2.

### 10. Changes to the criteria (for the product owner)

- 6: the row is drawn at 1 em = 16px with 1 CSS px round-capped lines in the row's text colour, and cut with "…" in its own strokes.
- 8: Fill is hidden (not disabled) while only single-line texts are edited; a mixed edit posts "1 single-line text has no fill and was not changed."
- 9: no notice; the change is shown on the canvas and in the Style tab, one history step.
- 19: the Boolean card does not dim for single-line texts; reason order missing face, single-line, the command's own; Offset refuses on release or chip Enter with no preview and readout line 3 "Single-line text has no area".
- 22: the Font button tooltip names EMS Readability.

### Design-system rows

Section "Properties panel: Fonts tab, font library", amend:

| Item | Value |
|---|---|
| Tag strip (amends) | Built-in order All, Favourites, Cuttable, Single line, Script, Sans, Serif, Initials, then own tags A to Z; at 244px the built-ins take 3 rows |
| Tag chip (amends) | Order Cuttable, Single line, Script, Sans, Serif, Initials, own A to Z; Single line's automatic tooltip "Detected: single-line font" |
| Single-line name (`0076`) | The name in the face's own centre lines: inline `<svg>`, `fill="none"`, `stroke="currentColor"`, 1 CSS px, round caps and joins, `vector-effect: non-scaling-stroke`, `aria-hidden`; 1 em = 16px (family) or 14px (face row), baseline on the text baseline; too wide: fewer characters plus "…" in the same strokes; loading, tooltip, states and keys as outline rows |
| Cuttable progress (amends) | Counts outline faces only; single-line faces are never checked |

Section "Properties panel: Style section", add:

| Item | Value |
|---|---|
| Single-line text (`0076`) | Fill section and its rule not in the tree while every edited object is a single-line text (as Markers for a text); Stroke in full. Mixed: Fill edits the others; style skip notice 4px under the Fill title "1 single-line text has no fill and was not changed." / "2 single-line texts have no fill and were not changed." |

Section "Toolbox" rows, amend "Refusal outline":

| Item | Value |
|---|---|
| Refusal outline (amends) | A refused single-line text is drawn as its glyph centre lines, 2px `--field-invalid` with the white casing, open |
| Single-line refusal (`0076`) | Boolean card, Fracture, Flatten, Weld, Offset: "Single-line text has no area. Nothing was changed." / "2 single-line texts have no area. Nothing was changed."; alert 8 s, in each command's notice place; the missing-face sentence wins; no dimming; Offset: no preview, readout line 3 "Single-line text has no area" |

Row "Missing font tag", amend: a single-line text is drawn in EMS Readability, stroke only; the Font button tooltip reads "Missing on this computer. Shown in EMS Readability."

Keyboard shortcuts: nothing added.

### Question for the customer

Frage 23 (SL-BACK), put to the customer in German by the lead: after a single-line font switched a text's stroke on, does switching back to an outline font keep that stroke?
- A: the stroke stays as it is (visible in the Style tab; one Undo takes font and stroke back together).
- B: Curvyo switches the stroke off again on the way back if the maker has not changed it since; the text then remembers that the stroke came automatically (one more key in the file).
Recommendation A; without an answer A (built).
