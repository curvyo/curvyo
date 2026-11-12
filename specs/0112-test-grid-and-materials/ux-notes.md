# 0112 Test grid and materials: UX notes

ux-engineer, 2026-11-12. Written from `specification.md` (criteria 1 to 33), the accepted drafts `docs/design-drafts/2026-11-laser-redesign/` (README P4, P5, P6, P7, P8, P12, P13; screens 02, 03, 04, 05: "die screens sehen gut aus"), the customer decisions of 2026-11-12 (Fragen 192 A, 193 B, 194 amended, 200, 201 B, 203, 204 A, 206 A; test ids, the burned label, materials shared by lasers), `docs/design-system.md` (rows "Trace bar", "Trace Preset button and menu", "Measure tool", "Tool rail architecture", "Toolbox card", "Mode switch" (the preset strip look), "Add and Edit form", "Pairing block" (mono code), "Action notice", "Entered group" (`--context-dim`); "Interaction conventions") and the built notes `specs/0028-material-test-library/ux-notes.md` (superseded where section 15 says), `specs/0111-laser-operation-tools/ux-notes.md` (the unfolded row, the kind buttons), `specs/0101-laser-operations/ux-notes.md`. Customer rules applied: plain fields and no sliders, no captions for kind, test, material and size, the id always burned, no "remove operations" button, no tips, no success notice, no popups in the panel (the material dropdown of the tool bar is the one bar popup, P4), no invented colours. Items marked (Proposal) are vetoable at the demo; section 13 lists them as G1 to G16.

Terminology: "grid" is a test grid object, "cell" one of its squares, "cell row" one of its generated operations, "label row" the operation of its text, "test" the record of a grid in a material, "set" power, speed, passes (and interval) of one kind for one laser. "Ink" is `--toolbar-icon`.

## 0. What is reused

| Part | Reused from (code) | Change in this slice |
|---|---|---|
| Rail button | `ToolRail.tsx`, `railCard.ts` (the "More tools" card of Offset and Measure); `TestGridGlyph` (`OperationGlyphs.tsx`, the 3 x 3 squares of `0028`) | third button of the card |
| Tool bar | `TraceBar.tsx` (replaces the Select bar in place, `role="toolbar"`, whole-group wrapping) | the Test grid bar |
| Material dropdown | the Trace Preset button and menu (row "Trace Preset button and menu") | "New material" first, thickness at the right |
| Segments | the preset strip look of row "Mode switch" (28px, 1px ink at 60 %, `rounded-[5px]`) | Kind, the axis parameter |
| Grid state, regeneration | `useTestGrid` | object with derived operations; no Grid block |
| Unfolded row | `0111` section 6 (`OperationSettingsBlock`) | the source line of a generated row |
| Judge table | `TestGridResult.tsx` (`role="grid"`, roving focus, 24px rows, `0028` 7.1) | moves into the test detail; one tap marks the best cell |
| Materials list | `MaterialsSection.tsx`, `MaterialRow.tsx`, `MaterialFields.tsx`, `useMaterials`, `materialRows.ts`, `materialsView.ts`, `materialWrites.ts`, `useMaterialDrag` | own tab, flat list, row unfolds, per-laser sets, tests |
| New material form | the "Add and Edit form" look of `0045` (`FormatForm.tsx`) | Name, Thickness, Cancel, Add |
| Armed trash | `useDeleteArm`, `useArm` (`0081`) | material, set, test, photo |
| Tab strip | `PanelStrip.tsx` (`GLYPHS`) | `materials: SwatchBook` |
| Notices | `CanvasNotice.tsx`, `useActionNotice` | section 10 |

Gone (superseded, see section 15): `SaveAsMaterialForm.tsx`, `TestGridBlock.tsx`, the Test grid row and the Worked chip of `0028`, the Materials section of the Operations tab, the Test grid button of the Operations tab.

## 1. Principle

A test is made where the material lies: one tool places a grid, its bar sets what varies, and the grid becomes plain operations the maker can read and change in the list. The sheet carries its 5-character id, so the result is found again by what is burned into it. Judging is one tap on the best cell, in the Materials tab, with a note and a photo beside it. A material is one record for all the maker's lasers; each laser has its own numbers under it, typed or tested.

## 2. Tokens and symbols (no new colour)

| Use | Token |
|---|---|
| Cut glyphs, Cut cell lines | `--cut` (4.30:1 on `--toolbar-bg`, 4.81:1 on `--canvas-bg`) |
| Engrave hatch of cells and of the burned text | `--engrave-hatch` (3.17:1 on `--canvas-bg`) |
| Text, glyphs, Score lines, the test id | `--toolbar-icon` |
| Line 2, units, axis lines, sources, "This page" chip border | `--panel-muted-fg`; chip border `--toolbar-icon` at 25 % |
| Selected segment and selected or unfolded row | `--value-fill` (segment ground; row ground with the 3px `--accent` bar) |
| Best cell frame | 2px `--accent` with the `Check` glyph in ink |
| Grid box on the canvas | 1px dashed `--accent` (4 on 3), the rectangle's box |
| Pressed Crosshatch toggle | `--toolbar-icon-active-bg` fill, `--toolbar-icon-active-fg` glyph |
| Primary buttons (Add, Crop, Take photo) | `--toolbar-icon-active-bg`, white label (`0045`) |
| Crop: image outside the frame | 35 % strength (`--context-dim`); frame 1.5px `--accent` with casing; handles the rectangle's squircles |
| Armed rows and squares | `--destructive-soft`, `--destructive` |
| Invalid field | `--field-invalid`, `aria-invalid`, the validation chip |

Symbols (Lucide at 16px, 1.5px absolute stroke, unless noted):

| Control | Symbol |
|---|---|
| Test grid tool (rail, 20px command glyph) | custom `TestGridGlyph` |
| Kind segments | custom `OperationGlyph` (Cut, Score, Engrave) |
| Material button, source lines | `SwatchBook` (12px in source lines) |
| Crosshatch | `Hash` |
| Materials tab | `SwatchBook` |
| New material, + Sets for another laser | `Plus` |
| + <kind> set | `Plus` and the kind's `OperationGlyph` |
| Laser header | `Zap` 12px (the Laser kind chip's glyph), chevron `ChevronRight` / `ChevronDown` 12px |
| Test row | kind glyph, `Image` 14px muted (has a photo), `ChevronRight` 16px |
| Test state in line 2 | `Flame` 12px (Lasered), `CircleDashed` 12px (Not lasered), `MapPin` 12px (on a page) |
| Back from a test | `ChevronLeft` and the material's name |
| Best cell | `Check` 16px, 2px stroke |
| Photo from file, Camera, Crop | `Upload`, `Camera`, `Crop` |
| Place on page, Select on page | `MapPin` |
| Damaged record | `TriangleAlert` (unchanged from `0028`) |

## 3. The Test grid tool (criteria 1, 2, 7, 8)

### 3.1 Rail button

- Third button of the "More tools" card, after Measure: `TestGridGlyph` at 20px, tooltip and name "Test grid tool". No letter key (G1).
- On a page that is not a laser page the button stays in the card, dimmed and `aria-disabled`, tooltip line 2 "Laser pages only." (the rail's "dimmed, not removed" rule for fixed blocks; criterion 1 says "not offered", section 14 M1).
- Pressed: the active tool look (`--toolbar-icon-active-bg`). Escape or another tool leaves it, as every tool.

### 3.2 On the canvas

- With the tool active, a press on empty space of the laser page places a grid with the bar's values: its box's top-left at the press point; if it would not fit there, the first free spot of `0028` criterion 3 (G2, section 14 M2). One undo step "Add test grid". The grid is selected and the bar edits it.
- A press on an existing grid selects it (no new grid). A press on anything else places a new grid over it (the bed is the maker's).
- Hover over empty page: the grid's outline at the pointer as the live preview (`--preview-new`, 1.5px, hollow), so the maker sees its size before the press.
- The selected grid: the rectangle's 1px dashed `--accent` box and the centre move from 48px; no resize squircles, no rotate, no skew, no knobs (criterion 7, Frage 206 A). Drag moves it; arrows nudge; M opens the Move chip. S, R, K do nothing.
- Duplicate and Paste make a new grid with a new id (criterion 8); Delete removes the grid and its rows in one step "Delete" (the app's delete step).
- A grid moved off the laser page (drag onto another page) removes its rows in the same step; no notice (criterion 13).
- **With the Select tool** a selected grid shows the Test grid bar in place of the Select bar (G3), so the maker edits a grid without the tool. The `0111` kind buttons are not shown for a grid selection (G4).

### 3.3 Canvas look

- Cells: the kind look of `0101` 13.2 (Cut solid red, Score dotted ink, Engrave hatch). Gap 2 mm, constant.
- Burned text (always, criterion 10), the Engrave look: the test id at the top-left (3 mm caps), the axis titles ("Power %", "mm/min") and the values over the columns and left of the rows.
- No chips per cell: one chip row on the grid's box with the grid's kinds (the `0028` 11 rule, kept).

## 4. The Test grid bar (criteria 3 to 6, 11, 14)

### 4.1 Layout

`role="toolbar"` named "Test grid <id>" ("Test grid" before a grid exists), left-aligned at `--rail-right` + 12, the pill of "Select bar layout" (36px lines, 8px padding, 1px dividers between groups). Two rows, both always shown (not the overflow row 2 of "Bar overflow"). No captions in row 1 (customer).

**Row 1**, in this order:

| Group | Control | Size |
|---|---|---|
| Kind | `role="radiogroup"` "Kind": three segments, glyph 16px and word 13px: Cut, Score, Engrave | 3 x 68px |
| Test id | the id, 13px mono semibold, letter-spacing 0.08em (the "Pairing block" mono), tooltip "Test id"; static text, not a Tab stop, part of the toolbar's name; absent before a grid is selected | 64px |
| Material | the Trace Preset button look: `SwatchBook` 16px, the material's name (ellipsis), a 10px chevron; "Material" in muted text when none is picked | 200px |
| Shared values | the parameters on no axis, in the order Power, Speed, Passes, Interval: plain fields with label and unit inside ("Passes 1 ×", "Speed 3000 mm/min"); Engrave adds the Crosshatch toggle (28 x 28, `Hash`) after them (P12) | Power 104, Speed 160, Passes 88, Interval 120 |
| Cell size | field "Cell size", unit "mm", 3 to 50, default 10 | 120px |

**Row 2**, two units, Columns then Rows:

| Unit part | Control | Size |
|---|---|---|
| Caption | "Columns" / "Rows" 12px `--panel-muted-fg` (the one caption of a unit) | 56px |
| Parameter | `role="radiogroup"` "Columns parameter": segments Power, Speed, Passes, and for Engrave Interval | 52 to 64px each |
| Min | field "Min", the parameter's unit | 128px |
| Max | field "Max", the parameter's unit | 128px |
| Count | field "Count", 2 to 12, default 5 | 76px |

### 4.2 Wrapping (narrow bar)

- Row 1 wraps by whole groups (Kind, Test id with Material, Shared values, Cell size); the Kind group never leaves the first line.
- Row 2: each unit is one wrap item. When Columns and Rows do not fit side by side, Rows goes whole to its own line under Columns (screen 02 B).
- When a unit is wider than the bar (the smallest window, 800 x 640 with the panel open: a 356px bar row against a unit of about 560px), the unit breaks at one place only: caption and parameter on the first line, Min, Max and Count on the next, indented by the caption's 56px. Nothing else inside a unit breaks (G5, section 14 M6).
- The pill grows downwards; no row moves sideways; the bar overlays the canvas and does not change the view.

### 4.3 Values and validation

- Units inside the fields: Power "%", Speed "mm/min", Passes "×", Interval "mm", sizes "mm". Plain fields; no sliders. Enter or leaving commits, Escape reverts, Up and Down step (Power 1, Speed 100, Passes 1, Interval 0.01, Cell size 1; Shift 10 times), committed on key-up.
- A parameter segment pressed that the other axis holds swaps the two axes with their Min, Max and Count (G6; criterion 4 says "not offered", section 14 M3). The parameter that leaves both axes moves to the shared values of row 1 with its default.
- Defaults (criterion 6): Min 20, Max 100 %; Speed 10 % to 100 % of the machine's maximum feed; Interval 0.05 to 0.25 mm; Passes 1 to 5. A shared value takes the picked material's set for this laser and kind, else Power 50 %, Speed 3000 mm/min (at most the maximum feed), Passes 1, Interval 0.10 mm (G7).
- A value outside its range, or a grid wider or higher than the bed: the typed text stays, 2px `--field-invalid`, `aria-invalid`, an Action notice 4px under the bar at the field (`aria-describedby`), the grid stays as it was, nothing is clamped (criterion 5). Texts in section 10. Min above Max is valid (the axis runs downward).
- A valid commit regenerates the grid and its rows in the same frame and overwrites hand edits of the rows: one undo step "Edit test grid" (criterion 14). A Kind change is a commit like any other.
- After the grid is Lasered, a valid commit gives it a new id at once (criterion 11): the id in row 1 and in the toolbar's name change; no visible notice; the sr-only status says "New test id 8PL4D." (polite).

### 4.4 The material dropdown (criterion 6, P4)

- The Trace Preset menu: 4px under the button (above when it would leave the viewport), left edges aligned, at least 232px wide, `--toolbar-bg`, `--panel-elevation-shadow`, 8px radius, `role="menu"` with `menuitemradio` rows of 32px.
- First row: `Plus` and "New material". Then the materials in the Materials tab's order: name 13px, thickness 12px muted tabular at the right, `Check` 16px at the left of the picked one. Damaged and newer records are not listed. Above 8 materials a "Filter" field heads the menu (as `0028` 8.2).
- Enter, Space or Down opens on the picked row; Up and Down without wrap, Home and End; Enter or a click picks and closes; Escape or a click outside closes; focus returns to the button.
- A pick is one undo step "Use material" on the grid; it regenerates the rows with the material's values where the material gives them (the label row's Engrave set, shared values) and creates the test record in that material (criterion 22, G8).
- "New material" switches the panel to the Materials tab with the new-material form open (criterion 6); Add there writes the record and picks it for this grid in the same step (G9); Cancel returns nothing.

## 5. The burned label and the generated rows (criteria 10, 12, 13)

- Names: cell rows "<column value> · <row value> · <id>" with units only on power ("60 % · 1950 · K7Q2M"), the label row "<id> labels" (P5). The name column holds about 11 characters; the tooltip and the accessible name carry all of it.
- Rows sit in their kind's section like any row, in the order: label row with the Marking rows, then cells column by column within each row of the grid (top-left first). Show, Output, rename, armed trash, arrows and drag as `0101`.
- Line 2 as every row: "3 · Cut · 1 item".
- The armed trash of one cell row removes that operation only; the cell's item becomes Not assigned. There is no "remove operations" button (criterion 13).

## 6. The unfolded generated row (criterion 12)

The `0111` unfolded row with one change in the upper part:

- **Cell row:** no material pick (its numbers are the cell's own, G10). Power and Passes, Speed, Interval (Engrave) as fields; then a source line 16px: `SwatchBook` 12px, "<material> · <id> column 3, row 2" 11px `--panel-muted-fg`, wraps to two lines (screen 03 A). Without a material: "<id> column 3, row 2". Then the hairline and the lower part (`0111` 6.3).
- **Label row:** the ordinary pick, pointing to the material's Engrave set for this laser when there is one, else own numbers 20 % and 3000 mm/min (P12); source line "<material> · <id> labels".
- A hand edit is one step "Edit settings" (and gives a Lasered grid a new id, criterion 11).

## 7. The Materials tab (criteria 16 to 21)

### 7.1 Place and header

- Tab strip: after Operations, `SwatchBook`, name and tooltip "Materials"; only on laser pages (P6). The tab's header: "Materials" and the Laser kind chip at the right (as every tab).
- Top to bottom: "+ New material" (or its form), the list. The Operations tab keeps the material pick of a row unchanged (`0111` 6.2).

### 7.2 New material

- "New material": outline text button 244 x 28px, `Plus` 16px and the word (the "Add slot" look of `0102`).
- Press: the button is replaced in place by the `0045` Add and Edit form (`role="group"` "New material", 1px ink at 25 % border, 5px radius, 8px padding): heading "New material" 11px semibold muted; **Name** (228px, text selected, default "Material <N>", 1 to 60 characters); **Thickness** (112px, "mm", empty, one decimal, 0.1 to 100); **Cancel** (outline) and **Add** (Primary), 112px each.
- Enter in a field presses Add; Escape or Cancel drops the draft; a tab change keeps it ("Drafts"). Add writes the record, closes the form, selects and unfolds the new row; one undo step "Add material" (ADR 0055 §9). Validation texts in section 10; the first invalid field takes focus.

### 7.3 Material row (40px, 48 coarse)

- **Line 1:** name 13px ink (flexible, ellipsis, tooltip), thickness "3.0 mm" 12px muted tabular at the right of the name, Trash 24px, group gap 8px, Up 24px, Down 24px (criterion 17; arrows hidden at the ends, slot kept).
- **Line 2** (11px muted): "<n> lasers · <n> sets · <n> tests", each part left out at 0, singular at 1 ("1 laser · 1 test"); all three 0: "No sets" (screen 04).
- States: hover, selected and unfolded (`--value-fill`, the 3px bar running down the unfolded body), focus ring, armed (`0081`). Rename by double-click or F2 (1 to 60 characters, "Rename material"); order by arrows, drag after 4px and Alt+arrows (a library preference, no step, as built); damaged and newer rows as `0028` 10.4.
- A press on the row selects it and unfolds its body; any other unfolded material folds (one at a time); a press on the unfolded row folds it. Not a step.
- Armed trash, status on arming: "Press again to delete Birch 3 mm. Its 3 tests go with it." Second press: one step "Delete material"; its tests' ids stay reserved (criterion 17).

### 7.4 The unfolded material (criteria 18 to 21)

Body indented 8px, 228px wide, in this order:

1. **Thickness** field (112px, "mm"); one step "Edit material".
2. **One block per laser.** Header 28px: chevron 12px, `Zap` 12px, laser name 13px semibold, at the right "This page" (the kind chip look: 11px, 1px ink at 25 % border, `rounded-md`) for the page's laser, or the count "2 sets" 11px muted when folded. The page's laser first and open; other lasers folded, in the order they were added (P7). The header is a disclosure button (`aria-expanded`); folding is session state, not a step.
3. **Inside a laser:** one set per kind in the order Cut, Score, Engrave. Set heading 24px: kind glyph 16px, kind word 12px semibold, at the right the source 11px muted ("Test K7Q2M" or "Typed") and the set's armed trash 24px (G11). Fields: Power (112px, "%") and Passes (112px, "×") in one row, Speed (228px, "mm/min"), and for Engrave Interval (228px, "mm"). Nothing else (criterion 19).
4. **Missing kinds:** one outline button per missing kind, 228 x 28px, `Plus`, the kind glyph and "<Kind> set" ("+ Score set"). A press adds the set with empty fields and the source "Typed", focus in Power; one step "Add set". Engrave + Outline and Cut-out use the Engrave, Score and Cut sets; they have no button.
5. **"+ Sets for another laser":** outline button 228 x 28px, `Plus` and the words. A press unfolds in place the machine profiles of the library that are not yet in the material (`role="radiogroup"`, 28px rows, profile name), with Cancel; a pick adds the laser's block, unfolded, with its "+ <kind> set" buttons and no set yet (the laser enters the record with its first set, criterion 21). No tab switch (G12, section 14 M7).
6. **"Tests (n)"** heading 24px 12px semibold, then the test rows (section 8.1).

- Set field commits: one step "Edit material" each, empty fields allowed while typing a new set (a set with empty Power or Speed is not offered in any pick); the source becomes "Typed" on a hand edit of a tested set (G13). Ranges and texts as section 10.
- A typed set of a laser that is not in the material adds it (criterion 21).

## 8. Tests (criteria 22 to 28)

### 8.1 Test row (40px, 48 coarse)

- Line 1: kind glyph 16px, the id 13px mono semibold, at the right `Image` 14px muted when the test has a photo, then `ChevronRight` 16px.
- Line 2 (11px muted): `Flame` 12px "Lasered 12 Nov · Diode 20 W", or `CircleDashed` 12px "Not lasered · " and `MapPin` 12px "Birch sheet" (the page holding the grid) or "not on a page".
- Newest first. A press or Enter opens the test (8.2). Accessible name "Test K7Q2M, Cut, Lasered 12 Nov, Diode 20 W, with photo".

### 8.2 Test detail (P8)

Replaces the material's body in place (the material row stays above it, selected):

1. **Back:** a text button 28px, `ChevronLeft` 16px and the material's name; Escape in the detail does the same. Focus returns to the test's row.
2. **Head:** the id 16px mono semibold, "Cut · Diode 20 W" 13px; at the right the test's armed trash 24px.
3. **State line** 12px muted: "Lasered 12 Nov 2026 · not on a page" or "Not lasered · on Birch sheet".
4. **Axes line** 11px muted tabular, wraps: "Power 20–100 % × Speed 600–6000 mm/min · 1 × · cell 10 mm · 12 Nov 2026".
5. **Best cell** heading 12px semibold, then the table (8.3).
6. **Set line** under the table, 13px ink tabular, wraps: "Cut set of Diode 20 W: 60 %, 1950 mm/min, 1 ×"; no best cell yet: nothing (the heading is the cue, no tip).
7. **Note** (8.4), **Photo** (8.5), then the place button (8.6).

### 8.3 Judge table (criterion 24)

- `TestGridResult` (`role="grid"` "Best cell of K7Q2M", one Tab stop, roving focus, `0028` 7.1 sizes): axis line, column headers, row headers, 24px cells; from 9 columns the cells share the width (17px at 12, the `0028` M7 rule).
- Cell look: unmarked a 12px square outline in `--lane-line`; hover `--editor-accent-hover`; best cell: 2px `--accent` frame around the cell and `Check` 16px ink in it; focus 2px `--editor-accent` ring.
- A press, Space or Enter on a cell makes it the best cell and moves any earlier mark: one undo step "Choose best cell". The set of this kind for the test's laser becomes the cell's two axis values plus the test's shared values (power, speed, passes, interval only, P13), source "Test <id>". A press on the best cell does nothing (no unmark: the set stays, G14).
- Possible whether Lasered or not. No stepper, no Judge screen.
- Cell names: "<column value>, <row value>" with units, toggle `aria-pressed`, ", best cell" on the marked one.

### 8.4 Note (criterion 26)

- Label "Note" 12px semibold, a 228px text area, 3 lines high, grows to 8, 13px, plain text, at most 500 characters (the field stops at 500; no counter, G15). Saved on leaving the field: one step "Edit note". Escape reverts.

### 8.5 Photo (criteria 27, 31)

- Label "Photo" 12px semibold. With a photo: the image 228px wide (height by its ratio, at most 228px; letterboxed on `--canvas-bg`), its trash armed at its top right (22px square on a `--toolbar-bg` ground). Then a button row: **File** (`Upload`), **Camera** (`Camera`, only with a camera and a permission that is not refused), **Crop** (`Crop`, only with a photo); outline text buttons, 28px.
- **File:** the system file dialog (JPEG or PNG); the image is scaled to 600px on its long side and stored; Crop opens at once (G16). One step "Add photo" (replacing an older photo is the same step).
- **Camera:** the live view takes the photo's place (228px wide), with **Cancel** and **Take photo** (Primary). Take photo stores the frame (600px max) and opens Crop. Escape cancels; the camera stream stops on cancel, take, tab change and test change.
- **Crop:** the image with the frame and eight squircles (the rectangle's handles), outside at 35 %; the size readout "600 × 470 px" 11px muted under it; **Cancel** and **Crop** (Primary). Arrow keys move the frame by 1px (Shift 10) when it has focus; Shift+arrows on a focused squircle resize it. Crop writes the cut image: one step "Crop photo". Cancel or Escape keeps the old one.
- Photo trash: one step "Remove photo".
- Missing or damaged file: the image's place shows `TriangleAlert` 16px and "Photo not found" 12px muted (criterion 31); File and Camera stay.

### 8.6 Place on page (criterion 28)

- An outline text button 228 x 28px, `MapPin` and "Place on <page>" (the active laser page's name), when the grid is on no page; "Select on page" when it is (selects the grid there and makes its page active).
- Place: the grid is recreated on the active laser page with the same id and fields at the first free spot, its rows regenerated; one step "Place test grid". It is selected; the panel stays on Materials.
- Not offered on a non-laser page: the panel has no Materials tab there.

### 8.7 Delete a test (criterion 25)

- The armed trash in the head. Status on arming: "Press again to delete test K7Q2M. Its photo goes with it." Second press: one step "Delete test"; the detail closes back to the material; the id stays reserved. A grid of that test still on a page keeps its id (it is the same test until it changes).

## 9. Keyboard routes

| Action | Route |
|---|---|
| Test grid tool | the rail's toolbar keys (one Tab stop, arrows, Enter) |
| Place a grid by keyboard | with the tool active and the canvas focused, Enter places a grid at the first free spot (G2) |
| Bar | Tab through the bar in visual order (Kind, Material, shared values, Crosshatch, Cell size, Columns parameter, Min, Max, Count, Rows ...); arrows inside a radiogroup; Escape in a field reverts, a second Escape returns to the canvas |
| Material dropdown | Enter, Space or Down opens; Up, Down, Home, End; Enter picks; Escape closes |
| Move a grid | arrows nudge, M Move chip |
| Materials list | one Tab stop; Up, Down, Home, End; Enter or Space unfolds or folds; F2 renames; Delete arms; Alt+arrows reorder; Tab enters the body |
| Laser header | Enter or Space folds or unfolds |
| Test row | Enter opens; Escape in the detail goes back |
| Judge table | one Tab stop; arrows, Home, End, Ctrl+Home, Ctrl+End; Space or Enter chooses |
| Crop | Tab to the frame and each squircle; arrows move, Shift+arrows resize; Enter crops, Escape cancels |

## 10. Notices, status and validation texts (each at most 70 characters)

| Text | Where | Role, time |
|---|---|---|
| "Grid is 260 mm wide, the bed is 200 mm." / "... high, ..." | Action notice under the bar field | `role="alert"`, until corrected |
| "Columns are 2 to 12." / "Rows are 2 to 12." | under the Count field | |
| "Power is 1 to 100 %." / "Speed is 1 to 6000 mm/min." (the machine's maximum feed) / "Passes are 1 to 20." / "Interval is 0.01 to 2.00 mm." / "Cell size is 3 to 50 mm." | under the field | |
| "Enter a number." / "Enter a whole number." | under the field | |
| "Enter a name." / "Thickness is 0.1 to 100 mm." | New material form | |
| "Material has no Cut set for this laser, using stored numbers" | state line of an operation (`0028` 8.4 (c), criterion 21) | text, no notice |
| "Photo not found" | the photo's place | text |
| "Use a JPEG or PNG image." | Action notice under File | `role="alert"`, 8 s |
| "Camera access was refused." | Action notice under the photo's place; the Camera button leaves | `role="alert"`, 8 s |
| "Could not save Birch 3 mm: <reason>." / "Could not save the photo: <reason>." | Action notice under the field or button | `role="alert"`, 8 s |
| Undo feedback of material steps | the `0028` section 13 texts, unchanged | |
| "Press again to delete Birch 3 mm. Its 3 tests go with it." / "... test K7Q2M. Its photo goes with it." / "Press again to delete the Cut set of Diode 20 W." | sr-only polite status on arming | |
| "New test id 8PL4D." / "Best cell: 60 %, 1950 mm/min." | sr-only polite status | |

No visible notice after placing, editing, picking, judging, adding or cropping (no success notice).

## 11. Undo steps

"Add test grid", "Edit test grid", "Use material" (grid), "Place test grid", "Delete" (the app's step, the grid with its rows), "Edit settings" (a row), "Add material", "Rename material", "Delete material", "Edit material" (thickness, a set field), "Add set", "Delete set", "Choose best cell", "Edit note", "Add photo", "Crop photo", "Remove photo", "Delete test". Not in undo: the material order, folding, opening a test, the dropdown, the camera view.

## 12. Empty, error, locked, mixed and large states

- **No material:** the list holds only "+ New material"; the bar's material button reads "Material" muted; a grid without material has an id and no test.
- **Material without sets or tests:** line 2 "No sets"; unfolded, the page's laser block with three "+ <kind> set" buttons, "Tests (0)" with no rows.
- **Laser without sets in a shared material:** an operation shows the state line of section 10.
- **Grid not on a page:** "not on a page" and "Place on <page>".
- **Lasered grid:** the id changes on the next change (4.3); the old test keeps its record and best cell.
- **Damaged, newer records:** `0028` 10.4, unchanged; format 1 records read as one laser (criterion 30), nothing shown.
- **Locked:** a grid on a locked layer is not hit (Layers lock as built); "Select on page" selects it through the Layers rule. The bar edits a selected locked grid (G4: an id-bearing test is not drawing).
- **Mixed:** several grids selected: the Select bar shows the Test grid bar only for one grid; for several, the Select bar as for a group without the kind buttons (G3).
- **Errors:** sections 4.3, 8.5, 10.
- **Large:** a 12 x 12 grid places within 50 ms and a bar edit within 30 ms (criterion 32); its table renders 144 cells without a window; 200 materials: the list is windowed (fixed 40px rows, the unfolded body one variable item, at most 60 row elements), the dropdown shows the Filter; the Materials tab opens within 50 ms.

## 13. Decisions for veto (Proposal)

- **G1** No letter key for the Test grid tool.
- **G2** A press places the grid's top-left at the press point; first free spot only when it does not fit there; Enter places at the first free spot.
- **G3** With the Select tool, one selected grid shows the Test grid bar in place of the Select bar.
- **G4** The `0111` kind buttons are hidden for a grid selection.
- **G5** A unit wider than the bar breaks after its parameter, once.
- **G6** Both axes show every parameter; pressing the other axis's parameter swaps the axes.
- **G7** Shared-value defaults: the material's set, else 50 %, 3000 mm/min, 1 ×, 0.10 mm.
- **G8** Picking a material on a grid is one step "Use material" and creates the test record.
- **G9** "New material" from the bar picks the new material for the grid when Add is pressed.
- **G10** A cell row has no material pick; a source line instead.
- **G11** A set keeps its armed trash (`0028` 10.3) in its heading.
- **G12** "+ Sets for another laser" unfolds the library's profiles in place; no tab switch.
- **G13** A hand edit of a tested set changes its source to "Typed".
- **G14** A press on the best cell does nothing; there is no unmark.
- **G15** The note field stops at 500 characters with no counter.
- **G16** A photo from a file opens Crop at once, as the camera does.

## 14. Mismatches for the product owner

- **M1** Criterion 1 "the tool is not offered" off laser pages: the rail's fixed cards dim a button instead of removing it (row "Tool rail architecture"); the notes dim it with "Laser pages only." in the tooltip.
- **M2** Criterion 2 places at the first free spot; the notes place at the press (G2), first free spot as the fallback. Recommendation: amend criterion 2.
- **M3** Criterion 4 "a parameter chosen for one axis is not offered for the other": screen 02 shows every parameter in both switches; the notes swap (G6).
- **M4** Resolved: burned text is criterion 10 only (id, axis titles, values); the "<material> · <kind>" line of screen 02 is not burned unless the customer decides otherwise.
- **M5** Criterion 24 "For Engrave the cell sets interval and power, speed and passes come from row 1" holds only for screen 05 D's axes; general rule: the cell's two axis values plus the shared values of row 1.
- **M6** Screen 02's rule "nothing inside a unit breaks" cannot hold in the smallest window; G5.
- **M7** Criterion 18 "+ Sets for another laser opens the machine list of the Document tab": that list picks the page's profile and would change the page; the notes list the library's profiles in place (G12).
- **M8** Screen 04 shows material rows without arrows; criterion 17 asks for arrows and drag; the notes keep the arrows.
- **M9** The spec names no way to remove a set; the notes keep the set trash (G11).
- **M10** Criteria 1 and 2 say nothing about the Select tool and the kind buttons on a grid; G3 and G4.
- **M11** Criterion 12 is silent on the material pick of generated rows; G10 for cells, the ordinary pick for the label row.
- Design system rows to add at acceptance: "Test grid tool and bar (`0112`)", "Material dropdown (`0112`)", "Materials tab (`0112`)", "Test detail, judge table, photo and crop (`0112`)"; remove "Test grid row and Grid block", "Materials section" and the Worked glyph of `0028`. No token.

## 15. What this replaces in `specs/0028-material-test-library/ux-notes.md`

| `0028` section | Status |
|---|---|
| 3 (tab layout items 1, 4, 7, 8) | superseded: no add group (`0111`), no row block, no Materials section, no View switch |
| 4, 5, 6, 7 (Add test grid, the Test grid row, the Grid block, Rebuild, the result table on the grid, the Worked chip) | superseded by sections 3 to 6 and 8.3 here; the table code is reused |
| 8 (Settings block, Material pick, pointing states) | kept, moved into the unfolded row (`0111` 6.2) |
| 9 (Save as material form) | superseded: judging writes the set (8.3); a set is typed in the material (7.4) |
| 10 (Materials section) | superseded by section 7 here; damaged and newer rows (10.4) kept |
| 11 (canvas look), 13 (undo notices of material steps) | kept, except the Worked chip and per-grid "Labels" chip |
| 16 M1 to M9, M12 | superseded; M10, M11, M13 to M17 kept |

## 16. Implementer checklist

- Rail: third button of "More tools", dimmed off laser pages; Escape leaves.
- Canvas: live outline under the pointer, place at the press, select an existing grid, box and centre move only, Move chip, duplicate gives a new id, Delete and moving off the page drop the rows.
- Bar: two rows, no captions in row 1, mono test id with tooltip, material button and dropdown with "New material" first, shared values and the Crosshatch toggle for Engrave, Columns and Rows units with Min, Max, Count, swap of axes, whole-unit wrapping and the one break, invalid values kept with the notice, never clamped; each commit one "Edit test grid" that overwrites hand edits; new id after Lasered with sr status only.
- Rows: names, order, cell rows without pick and with the source line, label row with the pick.
- Materials tab: `SwatchBook` tab after Operations; New material form; rows with thickness, counts, arrows, rename, armed trash; one unfolded at a time; per-laser blocks with "This page", sets of four values at most, "+ <kind> set", "+ Sets for another laser" in place, tests newest first with state glyphs.
- Test detail: back, head with trash, state and axes lines, judge table with one tap and the set line, note at 500, photo from file or camera with crop at 600px, "Photo not found", "Place on <page>" and "Select on page".
- Steps of section 11; texts of section 10 at 70 characters at most; no success notice; no popup in the panel; no new colour; windowed list; focus never to `body`.

## Links

Spec: `specs/0112-test-grid-and-materials/specification.md`. Drafts: `docs/design-drafts/2026-11-laser-redesign/` screens 02 to 05, README P4 to P8, P12, P13. Built notes: `specs/0028-material-test-library/ux-notes.md` (section 15 here), `specs/0101-laser-operations/ux-notes.md`. Before: `specs/0111-laser-operation-tools/ux-notes.md`. Next: `specs/0113-fixture-sets-and-pins/ux-notes.md`.
