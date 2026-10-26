# Text layout controls: alignment, spacing, character rotation, taper, vertical text and kerning

Status: Slice 1 built (format 19); slice 3 built (format 20); slice 4 built (format 21); slice 2 built on `story/text-taper` (format 24)
Priority: Should
Origin: Customer. Request of 2026-10-23 (translated), with a screenshot of Inkscape's Text toolbar: "Text alignment right, centred, left, justified goes into the tool bar; likewise horizontal text, vertical left-to-right and right-to-left, auto glyph orientation, upright glyph orientation and sideways glyph orientation. We need a letter 'cone' distortion (Kegelverzerrung, a taper/keystone of each glyph) to compensate a little for fonts laid in a circle. Spacing between letters, spacing between baselines, spacing between words, horizontal kerning, vertical kerning, character rotation go into the properties panel; we need nice sliders there." Placement of each control, units, ranges, defaults, the exact meaning of the taper and of kerning are the product owner's and marked **(Proposal)** where the customer did not state them.
Depends on: `0074-text-objects` (text node, Text bar, Select bar Text group, line layout, Size = height of the capital H, Object to path), `0076-single-line-fonts`, `0078-font-style-buttons` (Style group; the greyed-button exception), `0047` (value fields and sliders), `0020` (one step per interaction), `0043`/`0051` (panel tabs).
Companion: `0080-text-on-path` (text on a path or circle, written in parallel). The taper's main use is text on a circle; it is defined here for every text and `0080` uses it unchanged.

## Slices

The request is four features of different size and risk. One spec, four slices, each its own branch and PR (`CLAUDE.md` §4), built in this order (Proposal):

| Slice | Branch | Parts | Why this cut |
|---|---|---|---|
| 1 | `story/text-layout-controls` | A, B, G | Alignment, letter, word and line spacing, character rotation: per text object, horizontal only; the most used controls and the "nice sliders" the customer asked for. Frage 31 and 32 decided 2026-10-23. |
| 2 | `story/text-taper` | C, G | The taper needs its one-line meaning confirmed (Frage 29) before it is built; best demoed together with or after `0080`. |
| 3 | `story/text-vertical` | D, G | Vertical writing modes and glyph orientation change the line layout in the text engine (vertical metrics, `vert` features, caret movement); the largest and riskiest part. |
| 4 | `story/text-kerning` | E, G | Per-character values need a per-character register in the document model (architect, `needs-customer`); Frage 30 decided A (per character), 2026-10-23. |

Each slice is Ready on its own when its questions are answered, `adrs.md` covers it and the UX notes cover its controls.

## User value

As a maker who lasers names, house numbers and sign lettering, I want to align my lines left, centred, right or justified, open or close the spacing of letters, words and lines with sliders, turn the letters, set text vertically, kern a single pair and taper each letter for lettering on a ring, so that a sign comes out the way I designed it without converting the text to paths and moving letters by hand, and the text stays editable.

## Field reference

| Tool | Alignment, writing mode | Spacing and kerning | Rotation, distortion |
|---|---|---|---|
| Inkscape 1.x | Text toolbar: left, centre, right, justify (justify only does something in flowed text); writing mode horizontal, vertical right-to-left, vertical left-to-right; text orientation auto, upright, sideways (SVG `writing-mode`, `text-orientation`) | Toolbar: line spacing (factor, `line-height`), letter spacing and word spacing (px, `letter-spacing`, `word-spacing`); horizontal and vertical "kerning" are `dx`/`dy` on the character after the cursor, also Alt+arrows; Text > Remove Manual Kerns ([tutorial](https://design.tutsplus.com/tutorials/kerning-spacing-and-other-text-tricks-in-inkscape--cms-20576); the labels are criticised as misleading in [inbox#3494](https://gitlab.com/inkscape/inbox/-/issues/3494); line spacing defaults were buggy, [inbox#9629](https://gitlab.com/inkscape/inbox/-/issues/9629)) | Character rotation (`rotate`) for the selected characters about each glyph's origin on the baseline; no taper (Path Effects > Envelope on paths only) |
| Illustrator | Paragraph panel; vertical type tool | Character panel: tracking (1/1000 em), kerning (Auto/Optical/manual per pair, Alt+arrows), leading, baseline shift per character ([help](https://helpx.adobe.com/illustrator/using/line-character-spacing.html)) | Character rotation per character (hidden behind Show Options); envelope distort for keystone on outlines |
| Affinity Designer 2 | Paragraph panel | Character panel: tracking, kerning, leading override, baseline (positive lowers, the opposite of Illustrator) ([help](https://affinity.help/designer/en-US.lproj/pages/Panels/characterPanel.html)) | Rotation in the Character panel; no taper on live text |
| LightBurn 2.1 | Align X / Align Y in the text options | H-Spacing and V-Spacing per text object ([docs](https://docs.lightburnsoftware.com/2.1/Reference/Text/)) | None per glyph |
| Cricut Design Space | Alignment in the text bar | Letter Space and Line Space per text box ([help](https://help.cricut.com/hc/en-us/articles/5280755261719)) | Curve slider bends one line into a circle; glyphs are not tapered |

**What we do better.**
- **Spacing in millimetres that follows the size** (Frage 32 A): the maker reads "1.5 mm" between letters, and a resize keeps the look, because the value is held relative to the letter height (Inkscape's px letter spacing stays fixed when the text is scaled, so a scaled sign changes its rhythm).
- **Justify works on point text**: every line but the last is spread to the widest line (Inkscape justifies only flowed text).
- **Character rotation turns each letter about its own middle**, so letters do not drift off the line as in Inkscape, where they turn about their baseline origin.
- **A taper of live letters** for lettering on a ring (none of the tools above has one on editable text); it keeps the letter height, so Size stays exact.
- **Kerning is visible and removable**: the slider shows the kern at the cursor and its reset clears it; outside edit mode the reset clears all manual kerns of the text (Inkscape needs a menu command).
- Every value is a GIMP-style value field (drag, click, type), the sliders the app already has (`0047`), not a spin box.

## Terms

- **Subject texts:** as in `0078`: the selected texts, or the text in edit mode, or, with neither, the next new text (tool state, `0074` criterion 13).
- **Size, H:** the text's Size, the height of its capital H in mm (`0074` criterion 12).
- **Cluster:** one shaped unit of the text (a character, or a ligature, or a base with its marks).
- **Line extent:** from the left edge of a line's first cluster to the right edge of its last cluster that is not a space, after letter and word spacing; trailing spaces do not count for alignment.
- **Relative value (Frage 32 A):** a length stored as a multiple of Size and shown and typed in mm (shown mm = stored factor × Size, up to two decimals, trailing zeros trimmed).
- **Natural line spacing:** the line height of `0074` criterion 4 (ascender minus descender plus line gap of the face, scaled to Size).
- Angles in degrees with the sign of the Select tool's rotate entry (`R`). Lengths in mm, tolerance 0.01 mm. Notices at most 70 characters, in the slots `0074` uses.

## Acceptance criteria

### Part A (slice 1): alignment in the Text bar

1. Given the Text bar (Text tool) and the Select bar's Text group (Select tool, a selection that holds a text), then after the Style group of `0078` there is an **Alignment** group of four buttons, one choice at a time, one Tab stop with roving focus, in this order with these accessible names: **Align left**, **Align centre**, **Align right**, **Justify**. The current alignment of the subject texts is pressed (`aria-checked`); with subject texts that differ, none is pressed. A new text is left-aligned (today's behaviour) unless the tool state says otherwise (criterion 18).
2. Given a text with several lines, when the alignment is Left, Centre or Right, then each line's extent starts at, is centred on, or ends at the anchor's x (in text space) within 0.01 mm. Test: three lines "A", "AAA", "AA" in the fixture face, Centre: the three line midpoints are equal within 0.01 mm.
3. Given Justify, then every line except the last is spread to the extent of the widest line, by adding the same extra width to each space of that line; a line without spaces is spread by the same extra width between each pair of clusters; a line of one cluster and the last line stay left-aligned. Lines start at the anchor's x. **(Proposal: point text has no frame, so the widest line is the measure.)**
4. Given a selected text, when the maker changes its alignment, then the text stays where it is on the canvas: the anchor moves along the baseline so that the widest line's extent keeps its position within 0.01 mm, and the other lines move to the new alignment. One history step "Change text alignment"; several selected texts change in one step. **(Proposal)**
5. Given edit mode with Centre or Right, when the maker types, then the line grows to both sides (Centre) or to the left (Right) of the anchor, and the caret stays at the typed position; the `0074` typing budget (50 ms on 500 characters) holds.

### Part B (slice 1): spacing and character rotation as sliders in the Properties panel

6. Given the Fonts tab (Frage 31, decided B), then the controls of this part sit as one **text layout block** at the top of the tab, under its header row and above the tag strip and search row of `0075`; there is no new tab. The block is there **only while a text is selected or in edit mode**; with no text selected and no edit mode it is not in the tab at all (not greyed, not collapsed), and the tag strip and the font list move up into its place. The tab's subject line ("Source Sans 3 Bold", "3 texts") names the block's subject; the block has no subject line of its own. Customer decision 2026-10-23: this is an explicit exception to the rule "no panel section that comes and goes" (design-system row "Hidden, not disabled" and the feedback "Operations are tools, not properties"), limited to this block; the ux-engineer records it in `docs/design-system.md`.
6b. Given the block, then its first row is a header **"Text layout"** with a chevron (the disclosure look of "All formats" in the Document tab: `ChevronRight` folded, `ChevronDown` open, a button with `aria-expanded`), and a press folds or unfolds the fields below it. The block starts **folded** for a new user; open or folded is remembered per user on this computer (the webview's storage, like row 2 of the Text bar in `0084`; unreadable storage means folded). While folded and a subject's value of the block is not its default (or Mixed), the header shows the 6px accent dot of the Text bar's expand button (`0084` criterion 13) and its description reads "Changed: Letter spacing, Taper" (the changed fields in block order). Folding never changes a value. **(Customer decision 2026-10-25)**

6a. Given the block appears or goes (a text is selected, the selection is cleared), then the font list keeps its first fully visible family row in view (the anchoring rule of `0075` UX §3), focus does not move, and the tab never scrolls sideways. At 800 x 600 with every field of the built slices in the block, the font list keeps at least its minimum height of 120 px (`0075` UX §1); the ux-engineer states the block's height and the rows left in the UX notes.
7. Given that block, then it shows, in this order, four value fields (design system, row "Value field", the `0047` behaviour: drag, click to set, double-click or Enter to type, Escape restores, reset slot, two-line tooltip): **Letter spacing** (mm), **Word spacing** (mm), **Line spacing** (mm), **Character rotation** (degrees). Ranges, tracks and defaults:

   | Field | Meaning | Stored | Typed range | Track | Default |
   |---|---|---|---|---|---|
   | Letter spacing | extra width after each cluster except the last cluster of a line | relative | −0.5 × Size to 2 × Size | −0.25 × Size to 1 × Size | 0 |
   | Word spacing | extra width added to each space character (U+0020, U+00A0, U+3000) | relative | −1 × Size to 5 × Size | −0.5 × Size to 2 × Size | 0 |
   | Line spacing | distance from one baseline to the next | relative, or Auto | 0 to 10 × Size | 0 to 3 × Size | Auto (natural line spacing) |
   | Character rotation | turn of every glyph about its own centre | angle | −180° to 180° (typed values outside wrap, 270 becomes −90) | −180° to 180° | 0° |

   A typed value outside the range is set to the nearest limit with the notice "Letter spacing set to 40 mm, the largest for this size." (field name and value vary); nothing is refused (permit over refuse). **(Proposal: ranges and tracks)**
   A drag rounds to the smallest 1-2-5 step (0.01, 0.02, 0.05, 0.1, ... mm) that is at least the track's mm span divided by the field width, so dragged values land on round numbers; arrow keys step 0.1 mm, Shift 1 mm, Ctrl 0.01 mm; Character rotation steps 1° by drag and arrows, Shift 15°, Ctrl 0.1°. A rotation drag stops at −180° and 180°; only typed values wrap. **(Proposal, UX notes §0.1 and §0.2)**
8. Given Letter spacing `l`, then the advance of every cluster except the last of each line grows by `l`; the glyph outlines are unchanged. Test: "HH" with `l` = 2 mm is 2.00 mm wider (line extent) than with 0, within 0.01 mm; "H" alone is unchanged. With `l` ≠ 0, optional ligatures (`liga`, `calt`) are not applied, so "fi" draws two glyphs (as CSS does); kerning from the font stays on. **(Proposal)**
9. Given Word spacing `w`, then every space character's advance grows by `w`; a negative `w` that makes a space narrower than zero lets the words overlap (permit). Test: "H H" with `w` = 3 mm is 3.00 mm wider than with 0.
10. Given Line spacing Auto, then lines are set as today (`0074` criterion 4) and the field shows the natural line spacing in mm with no reset icon; when the face changes, the spacing follows the new face. Given a set Line spacing, then each next baseline is that distance lower, the field shows the reset icon, and a face change keeps the value. Reset returns to Auto. While Auto, the value and unit are drawn muted and read "12.4 millimetres, automatic"; no "Auto" word is drawn **(Proposal, UX notes §0.3)**. Test: two lines, Line spacing 15 mm at Size 10 mm: baselines 15.00 mm apart; after a change of Size to 20 mm, 30.00 mm apart (relative, Frage 32 A).
11. Given Character rotation `a`, then every drawn glyph is turned by `a` about the point at the middle of its advance on the baseline raised by half the Size; advances and line positions are unchanged, so letters may touch or overlap (permit). Test: "I" at 90° in the fixture face draws its stem horizontal, centred on the same point as at 0° within 0.01 mm.
12. Given a drag on any of these fields, then the canvas follows at least every second display frame for a 500-character text on the plan's machine (each frame drawn within 50 ms), and the release writes one history step ("Change letter spacing", "Change word spacing", "Change line spacing", "Change character rotation"); a click, a typed value or the reset is also one step; Escape during a drag writes nothing. Several subject texts change in one step.
13. Given several subject texts, then a field shows the value when all have the same value in mm (Character rotation: in degrees), else "Mixed" with no fill bar; a change sets each text to that value in mm (stored relative to each text's own Size). With texts of different Size, the track and the overshoot use the smallest Size, so a drag never clamps a text; a typed value larger than that is set on each text up to its own limit, and when any text was limited the notice reads "Letter spacing limited on 1 of 3 texts, to fit its size." **(Proposal, UX notes §0.4)**
14. Given edit mode, when the maker changes a field with the mouse, then edit mode stays, the change applies to the whole text, focus returns to the edit sink with the caret where it was, and the change is its own history step (as `0078` criterion 16).
15. Given a text with spacing or rotation, then the hit area and the selection box of `0074` criterion 25 (UX §5) cover the aligned lines with their spacing and the turned glyphs' outlines; the caret and the selection of edit mode follow the spaced clusters (the caret between two clusters sits in the middle of the added letter spacing). **(Proposal: caret position)**

### Part C (slice 2): taper (Kegelverzerrung), definition per Frage 29

16. Given the block of criterion 6, then directly after Character rotation, in its group, there is a **Taper** value field (the row "Value field" of the block's other fields: label "Taper", unit "%", a signed bar with a zero mark, reset slot to 0 %), range −50 % to 50 % (the track is the whole range), default 0 %, Up and Down 1 %, Shift 10 %, Ctrl 0.1 %, typed entry ("20", "20 %"); live on the canvas while dragged, one step "Change taper" on release or commit; "Mixed" for differing subject texts; no "All characters" button (it is a value of the text). A typed value outside the range is set to the nearest limit with the notice "Taper set to 50 %, the largest taper." (or "−50 %, the smallest taper."). **(Customer decision 2026-10-25: in the Properties panel, not in the Text bar; range: Proposal)**
17. Given Taper `t` (Frage 29 A), then each glyph is mapped in its own frame: with `y` its height above the baseline, `h` the Size and `x_c` the middle of its advance, every outline point (x, y) goes to (x_c + (x − x_c) · s(y), y) with s(y) = 1 + t · (2y/h − 1), with s never below 0.05. So the glyph keeps its height, at the baseline it is (1 − t) times as wide, at the height of the capital H (1 + t) times as wide, and at half that height unchanged; positive `t` makes letters wider at the top. Test: "H" at Size 20 mm, Taper 20 %: the outer stem edges are 0.8 × and 1.2 × the untapered width apart at y = 0 and y = 20 mm, within 0.01 mm; the H's top stays 20.00 mm above the baseline.
18. Given a taper, then horizontal and vertical straight segments of the outline stay straight segments; other segments are curved by the map and drawn and converted within 0.01 mm of the exact mapped curve (nodes may be added). Advances and line layout are not changed by the taper.
19. Given a taper together with character rotation, then the taper is applied first in the glyph's frame, then the rotation (criterion 11). Given text on a path (`0080`), then the taper is applied in each glyph's frame before the glyph is placed on the path, so on a circle of baseline radius `R` the value t = h / (2 · (R + h/2)) widens each letter exactly as the ring widens at half the letter height (outside the ring), and for letters inside the ring that point at its centre the value is t = −h / (2 · (R − h/2)) (only for R > h/2; architect, `adrs.md` decision 10). This criterion fixes the definition `0080` uses; an automatic "fit to circle" value is `0080`'s to decide.

### Part D (slice 3): vertical writing and glyph orientation

20. Given the Text bar and the Select bar's Text group, then after the Taper field there is a **Writing mode** group of three buttons (one choice, one Tab stop): **Horizontal**, **Vertical, right to left**, **Vertical, left to right**, and after it a **Glyph orientation** group of three buttons: **Auto orientation**, **Upright**, **Sideways**. Defaults: Horizontal and Auto. A change is one step "Change writing mode" / "Change glyph orientation" for all subject texts, and keeps the text in place as criterion 4 does (the first column's start stays where the first line's start was). **(Proposal: button names)**
21. Given Horizontal, then the three Glyph orientation buttons are greyed (`aria-disabled`, 40 %, keep their Tab stop, the `0078` look) with the tooltip second line "Only for vertical text"; the stored orientation is kept and takes effect when the text is set vertical. This is a further exception to the design-system row "Hidden, not disabled", recorded in the build PR. **(Proposal)**
22. Given a vertical writing mode, then the characters of a line run top to bottom in a column; the anchor is the top of the first column's centre line; next lines are columns to the left (Vertical, right to left; the first line is the rightmost) or to the right (Vertical, left to right), one Line spacing apart (Auto: the face's vertical line height from `vhea`, else the natural line spacing). Alignment acts along the column: Left = top, Centre = middle, Right = bottom, Justify per criterion 3. Letter spacing, Word spacing, Character rotation and Taper act along the column in each glyph's own frame.
23. Given Upright, then every glyph stands upright, centred on the column, advanced by its vertical advance (`vmtx`, else the face's ascender minus descender scaled to Size), with the font's vertical forms (`vert`) applied. Given Sideways, then the whole column is set as horizontal text turned 90° clockwise (Latin reads top to bottom with letters lying on their right side). Given Auto, then characters whose Unicode Vertical_Orientation (UAX #50) is U or Tu are set as by Upright and the others (R, Tr) as by Sideways. Test with the plan's fixture face holding Latin and CJK glyphs: "AB" in Auto equals "AB" in Sideways; a CJK ideograph in Auto equals Upright.
24. Given edit mode in a vertical mode, then the caret is a line across the column, Up and Down move along the column, Left and Right move to the previous or next column in reading order (in Vertical, right to left, Left goes to the next line); Home and End go to the top and bottom of the column. **(Proposal)**
25. Given a single-line font (`0076`), then vertical modes work with its centre lines; without vertical metrics, Upright uses the em height as advance.

### Part E (slice 4): kerning per character (Frage 30 A)

26. Given the block of criterion 6, then after Character rotation and the Taper (criterion 16, customer 2026-10-25) there are three more value fields: **Horizontal kerning** (mm, relative, range −1 × Size to 1 × Size, track −0.25 × Size to 0.25 × Size, default 0), **Vertical kerning** (mm, relative, range −2 × Size to 2 × Size, track −0.5 × Size to 0.5 × Size, default 0) and **Per-character rotation** (criterion 27b), each with an "All characters" button beside it (criterion 27c).
27. Given edit mode with a caret and no range, then the fields show and set the kern of the character after the caret: Horizontal kerning is extra space before that character (it and the rest of the line move, positive to the right in reading direction); Vertical kerning moves that character and the rest of its line across the line, positive up (in vertical text: across the column, positive to the right), cumulative like Horizontal kerning and SVG `dy`; the characters before it and the next line stay (customer decision 2026-10-24, replaces "that character alone"). Given a range, then they show the common value of the range's characters or "Mixed", and a change sets every character of the range. At the end of a line there is no character after the caret: the fields show 0 and a change does nothing. **(Proposal)**
27b. Given edit mode, then **Per-character rotation** (degrees, range and track −180° to 180°, default 0, steps as Character rotation) shows and sets the own rotation of the character after the caret, or of each character of the range (Mixed as in 27), in one step "Change per-character rotation"; outside edit mode it shows and sets every character of the subject texts (criterion 28). The character turns about its cluster's centre as in criterion 11, by Character rotation plus its own rotation; Character rotation still turns every character. Object to path and the Weld of `0068` use the same turn. **(Customer decision 2026-10-24)**
27c. Given a kerning field or Per-character rotation showing one value (not "Mixed"), when the maker presses its **All characters** button (icon button, tooltip and accessible name "Apply to all characters"), then that value is set on every character of the subject texts (every character but line breaks), in edit mode too, in one step ("Change kerning" or "Change per-character rotation"). With cumulative vertical kerning (27) a vertical kern of 1 mm on every character builds a letter staircase, each letter 1 mm above the one before it. While the field shows "Mixed" the button is greyed (`aria-disabled`) and does nothing. **(Customer decision 2026-10-24)**
28. Given no edit mode (one or more texts selected), then the fields show the common kern (or own rotation) of all characters of the subject texts (usually 0, else "Mixed"); a change sets every character; the reset slot of a kerning field clears every manual kern of the texts in one step "Clear kerning" (Inkscape's Remove Manual Kerns), the reset slot of Per-character rotation every character's own rotation in one step "Clear per-character rotation"; each leaves the other's values.
29. Given edit mode, then Alt+Left and Alt+Right change the horizontal kern of the character after the caret (or of each character of the range) by −0.1 mm and +0.1 mm, Shift+Alt by 1 mm; Alt+Up and Alt+Down change the vertical kern likewise. These keys are part of the edit session (typing undo inside the session, `0074` criterion 9); a slider change is its own step "Change kerning". In a vertical mode, Alt+Up and Alt+Down change the horizontal kern (along the column, Alt+Down +0.1 mm) and Alt+Left and Alt+Right the vertical kern (across the column, Alt+Right +0.1 mm), matching the caret keys of criterion 24. **(Proposal: Inkscape's keys; vertical mapping: lead decision 2026-10-24, Proposal)**
30. Given a kerned character, then the kern stays with that character: typing or deleting before it moves it along; deleting the character deletes its kern; a character typed directly before or after it has no kern. Copy and paste of plain text carries no kern. **(Proposal)**
31. Given two people editing the same text (ADR 0009 §3), then a kern merges per character like the string: kerns set on different characters by two peers both survive.

### Part F: all slices, the bar at 800 x 600

32. Given the window at 800 x 600 with the panel open and every group of the slices built, then nothing of the Text bar or the Select bar's Text group is cut off and nothing scrolls sideways: groups wrap to the next row as whole groups (design system, "Overlay row").

### Part G: all slices, what the values do elsewhere

33. Given no text selected and no edit mode, when the maker changes a control of the Text bar from this spec (Alignment, Writing mode, Glyph orientation), then the value is set for the next new text (tool state for the session, `0074` criterion 13), no history step is written, and the next text uses it. The fields of the block (criterion 6) are not there in this state, so a new text starts with their defaults (criteria 7, 16, 26) and gets its own values once it is in edit mode. **(Proposal: as Font and Size)**
34. Given a uniform resize of a text (a corner drag with Ctrl, the S entry with one value, `0074` criterion 26), then Size changes and every relative value keeps its factor, so the drawn text scales as one: the mm readouts grow by the same factor, the rotation and taper stay. A non-uniform stretch or skew keeps every value and goes into the placement matrix as today.
35. Given a text with any layout value of this spec, when Object to path (`0074` criterion 22) or a path command that converts first (`0074` criterion 24, Weld of `0068`) runs, then the result matches the drawn outlines within 0.01 mm, as one group with one path per glyph; a single-line text gives open paths (`0076`).
36. Given a text whose face is missing (`0074` criterion 29), then its layout values are kept and applied to the substitute glyphs, never rewritten by drawing; a change by the maker is written as for any text.
37. Given a project with texts that use these values, when it is saved, closed and opened again, then every text shows the same alignment, spacing, rotation, taper, writing mode, orientation and kerns, editable. Values at their default are not stored, so a file from before this spec opens and draws exactly as before. Each slice that adds stored values takes the next free `format_version` at merge (architect); golden file `text_layout_v<N>.curvyo` with one text per value of that slice and one text with all of them.
38. Given the core crates, then alignment, spacing, rotation, taper, vertical layout and kerning are computed in `curvyo-text-core` from the text's values and the face bytes, with no file, clock or thread access, and build for `wasm32-unknown-unknown`; unit tests cover each criterion's test case above.

## Out of scope

- Frame (flowed) text and justification inside a frame; hyphenation; tab stops.
- Per-character letter spacing, word spacing, font, size or colour (rich text); kerning and the own rotation (27b) are the only per-character values (slice 4).
- Optical kerning (spacing by outline shape) and switching the font's own kerning off.
- Right-to-left and mixed-direction horizontal text (bidi), and the shaping of scripts beyond Latin and the CJK test case.
- A canvas handle for the taper, for letter spacing or for kerning (drag between two letters); an automatic "fit to circle" taper value (`0080`).
- Text on a path, text in a circle (`0080`); envelope, arc, bulge or perspective warps of the whole text.
- Baseline-to-baseline snapping of several texts, paragraph spacing, first-line indent.
- SVG `letter-spacing`, `word-spacing`, `line-height`, `dx`, `dy`, `rotate`, `writing-mode`, `text-orientation` import and export (`0024` follow-up; the values are chosen so that they map).
- Keys for alignment and writing mode.

## Decided by the customer 2026-10-23

- Frage 29 (TAPER-DEF): **A**. Each letter becomes a trapezoid around its middle, heights unchanged (criterion 17 to 19).
- Frage 30 (KERN-SCOPE): **A**. Kerning is per character, built as slice 4 (Part E).
- Frage 31 (LAYOUT-PLACE): **B, with a change**: the sliders sit as a block at the top of the Fonts tab, and **when no text is selected the sliders are not there**. This is an explicit customer exception to the rule "no panel section that comes and goes", limited to this block (criterion 6, 6a, 33). The "Text" tab of the earlier default A is dropped.
- Frage 32 (SPACING-UNIT): **A**. Millimetres on screen, held relative to Size.

## Decided by the customer 2026-10-25

- The Taper control is a value field of the text layout block in the Fonts tab, directly after Character rotation (so the kerning fields of criterion 26 now follow the Taper), not a field of the Text bar's row 2 (criterion 16; `0084` row 2 no longer holds it).
- The text layout block folds under a "Text layout" header, folded by default, remembered per user, with the changed-value dot while folded (criterion 6b).

## Decided by the customer 2026-10-24 (feedback on PR #189)

- Vertical kerning moves the following characters too, cumulative like Horizontal kerning and SVG `dy` (criterion 27).
- A per-character rotation in addition to Character rotation, which stays for all characters (criterion 27b, ADR 0021 mark key `rot`).
- An "All characters" button beside each per-character field (criterion 27c).
- Text on a path (`0080`, stacked on this branch) applies the cumulative vertical kern along the carrier's normal and the own rotation on top of each letter's turn; that is built on the `0080` branch when it is rebased onto this one.

## Open questions

None for the customer. Not put to the customer (Proposal, vetoable at the demo): Justify on point text (criterion 3), text stays in place on an alignment change (4), ligatures off with letter spacing (8), the ranges and tracks (7, 26), the slice order, greyed glyph orientation in horizontal mode (21), caret movement in vertical text (24), Alt+arrow keys (29), tool state for the next text for the bar controls only and defaults for the block's fields (33).

## UX notes

See [`ux-notes.md`](ux-notes.md) (2026-10-24): slice 1 in full, slices 2 to 4 sketched; its §0 lists the proposed changes to the criteria.

## What the architect must decide (`adrs.md`)

1. Where the per-object values live in the text node (one optional `text_layout` map, absent keys = default, or one register each) and the `format_version` per slice (criterion 37).
2. Slice 4: a per-character register that merges with the string (for example Loro rich-text marks on `text_string` that do not expand), `needs-customer` as a document model change.
3. Slice 3: vertical shaping with harfrust (direction TTB, `vert`), vertical metrics from skrifa, and the UAX #50 table (size in the wasm budget of ADR 0020 as amended by `0076`).
4. Slice 2: how the taper map is flattened into cubic segments within 0.01 mm for drawing and Object to path.

## Links

Requirements: R-EDIT-048, R-FONT-011 (new, to be added: text layout controls)
Depends on: `specs/0074-text-objects/`, `specs/0076-single-line-fonts/`, `specs/0078-font-style-buttons/`; companion `specs/0080-text-on-path/`
ADRs: 0020 (text engine), 0009 §3 (per-character merge), 0004 §9 (format version)
PR: (one per slice)
