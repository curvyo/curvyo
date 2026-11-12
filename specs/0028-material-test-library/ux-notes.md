# 0028 Material test library: UX notes

ux-engineer, 2026-11-11. Checked against `specification.md` (criteria 1 to 31, with the customer answers of 2026-11-11: Frage 183 A, 184 A, 185 B, 186 "as in LightBurn", and the changed answers 187 B+C, 188 B, 189 B; Frage 190 default A), revised 2026-11-11 to the revised ADR 0054 (`op_settings` with `overrides`, Reset to material, the stored copy kept current) and ADR 0055 §9 (material edits are undo steps), `specs/0101-laser-operations/ux-notes.md` (rows, rename, armed trash, arrows and drag, symbols, look, summary), `specs/0102-laser-fixtures/ux-notes.md` (the rectangle model of section 0, block fields of 4.1, the expanded row), `specs/0103-fixture-jobs/ux-notes.md` (the disclosure section, the block under a selected row), `specs/0026-machine-profile/ux-notes.md` (library writes, "Differs from profile", damaged and newer records), `docs/design-system.md` ("Interaction conventions": "No popups in the properties panel", "No operating tips", "No notice after a success", "Hidden, not disabled", "Inline confirm", "Inline delete confirm", "Armed action", "List row", "Canvas notice slot", "Mixed-state display"; "Document formats library": the Add and Edit form and the Primary button; "Properties panel: tabs": Active tab, Drafts). Customer rules applied: no invented colours (symbols and text; ink, `--cut` red and `--engrave-hatch` only), in-place rename by double-click or F2, the armed trash of the Fonts tab, arrows plus drag and drop for order, no machine settings in a row, edit like a rectangle, grid fields in the Properties panel, no popups, no tips, no success notices. Items marked (Proposal) are vetoable at the demo; section 16 lists them as M1 to M17, section 17 the spec changes they need.

**Superseded in part (2026-11-12, laser redesign).** Sections 3 (add group, row block, Materials section, View switch), 4 to 7, 9, 10.1 to 10.3, the Worked chip of 11 and the proposals M1 to M9 and M12 are replaced by `specs/0111-laser-operation-tools/ux-notes.md` and `specs/0112-test-grid-and-materials/ux-notes.md` (its section 15 lists each). Section 8 (settings block, material pick, pointing states) stays and moves into the unfolded operation row; 10.4 and 13 stay.

Terminology: "grid" is a test grid, "cell" one of its squares, "mark" the Worked mark, "set" the power, speed and passes of one kind, "pointing" an operation that stores a material id (Frage 185 B), "own numbers" an operation's own power, speed and passes. "Ink" is `--toolbar-icon`.

## 0. What is reused

| Part | Reused from | Change in this slice |
|---|---|---|
| Add group | `0101` section 2.1 (`role="toolbar"`, 28px icon buttons) | a sixth button, Add test grid |
| Test grid row | `0101` section 4.2 (40px, glyph, name, Show, Output, armed trash, arrows, line 2) | no digit, no tick; line 2 says "test grid" |
| Block under the list | the Parts list of `0103` section 5.1 (what belongs to the selected row) | holds the Settings block or the Grid block |
| Block fields | `0102` section 4.1 (bar number field, 28px, label and unit inside, 244 / 118 / 78px widths) | none |
| Inline form | `0045` Add and Edit form (1px ink at 25 % border, 5px radius, 8px padding, Primary button, Cancel) | Save as material |
| Disclosure section | "Fixture jobs (N)" of `0103` section 3.1 | "Materials (N)" |
| Material rows | `0101` rows, `0081` armed trash, `0101` rename and order | no Show, no Output |
| Canvas | `0101` section 13 (kind looks, chips, budget), `0102` section 0 (the rectangle's box and Move chip) | the Worked tick chip |

## 1. Principle

A test grid is made like a rectangle: one press adds it with sensible values, and its fields then change it in place. The operation list stays a list of what runs; everything about the selected row (its settings, or a grid's fields and result) sits in one block under the list. A material is a record in the maker's library, listed in a collapsed section; an operation that uses it points to it and shows its name, so one edit of the material changes every operation that uses it, and the line says plainly when the material cannot be found. Every state is a symbol and a word; nothing is told by colour. Nothing opens a dialog or a popup.

## 2. Tokens and glyphs (no new colour)

| Use | Token | Contrast |
|---|---|---|
| Cut symbols and Cut cell lines | `--cut` (`0101`) | 4.81:1 on `--canvas-bg`, 4.30:1 on `--toolbar-bg` |
| Engrave hatch of Engrave cells and of the labels | `--engrave-hatch` (`0101`) | 3.17:1 on `--canvas-bg` |
| Every other glyph, the tick, text, Score lines | `--toolbar-icon` | 8.27:1 on the panel, 9.25:1 on `--canvas-bg` |
| Muted text, line 2, unmarked cell box, units | `--panel-muted-fg`, `--lane-line` | 5.0:1; 3.1:1 (non-text, 3:1 met) |
| Selected row, highlighted cell, focus, field border | `--value-fill`, `--accent`, `--editor-accent` | as `0101` |
| Invalid field | `--field-invalid` with the validation chip and `aria-invalid` | 4.8:1 |
| Armed row and square, Replace button | `--destructive-soft`, `--destructive` | as `0081` |
| Save button | Primary button (`--toolbar-icon-active-bg`, white label) | as `0045` |

No value is added. New glyphs (16 x 16px grid, 1.5px stroke, `currentColor`, ink):

| Glyph | Drawing | Use |
|---|---|---|
| Test grid | nine squares 3.5px, 3 x 3, 2px apart, inset 0.75px, outlines | Add test grid button only. Not the fixture Grid `#` of `0102` (separate squares, not lines) |
| Worked | Lucide `Check`, 2px stroke | the marked cell in the table, the Worked line, the canvas chip |
| Not found | Lucide `Link2Off` | settings states (b) and (c) |
| Differs from material | Lucide `Diff` | the settings state with overrides (8.4) |
| Reset to material | Lucide `RotateCcw` (the reset glyph of `0026` "Reset from profile") | the Reset to material button (8.4) |
| Edited by hand | Lucide `PencilLine` | the grid state of criterion 10 |
| Damaged | Lucide `TriangleAlert` | damaged material row |

## 3. The Operations tab layout

Top to bottom, 244px column, the tab scrolls as one column (`0103` J2):

1. Add group (`0101`), now six buttons (section 4).
2. Cuts last.
3. The list (`0101`, windowed, minimum 120px), with Test grid rows (section 5).
4. **Row block** (new): for the selected row only. A selected operation shows the Settings block (section 8), a selected test grid the Grid block (sections 6 and 7), a selected unknown operation or nothing selected: nothing (no empty line). 8px above it.
5. Fixture jobs section (`0103`). A fixture job has no settings, so it never fills the row block.
6. Summary (`0101` section 7).
7. **Materials section** (section 10), collapsed by default (M12).
8. View switch.

## 4. Adding a test grid (criteria 1 to 3)

- **Add test grid**: the sixth button of the add group, after an 8px gap behind Add Cut-out (the group gap of `0101` 4.2). 28 x 28px (36 coarse), Test grid glyph, name and tooltip "Add test grid". With items selected on the canvas it assigns nothing and has no description. Toolbar keys as `0101`.
- **A press adds a grid at once** (M1, replaces the inline make block): the fields of the last grid made for this machine, else the defaults of criterion 1 (Cut, 5 x 5, 20 to 100 %, 10 to 100 % of the maximum feed, cell 10 mm, gap 3 mm, labels on). Placed as criterion 3. One undo step "Add test grid". The grid is the canvas selection, its row is added at the end of its section and selected, the Grid block opens under the list and is scrolled into view. Focus stays on the button (as `0101` 6.1). No notice.
- **Refusals** (nothing added, no step): Action notice 4px under the button, refusal look, `role="alert"`, 8 s: "Test grid is 260 mm wide, the bed is 200 mm." (remembered or default values do not fit) or "No free spot on the bed for a 75 × 62 mm grid." (Proposal: placing it over other items is the alternative).

## 5. The Test grid row (criterion 6)

- One `listitem` in the list, 40px (48 coarse), the line 1 of `0101` 4.2: glyph, name, Show, Output, Trash, group gap, Up, Down.
- **Glyph:** the kind symbol of `0101` section 3 (Cut red; Score and Engrave ink), as the spec says.
- **Name:** "Test grid <N>" (lowest free N on the page). Rename in place by double-click or F2, as `0101` 4.4; undo "Rename test grid".
- **Line 2:** "<Kind> test grid · <C> × <R> cells", or "<Kind> test grid · Output off". Example: "Cut test grid · 5 × 5 cells". **No digit** (nothing is assigned to a grid by digit) and **no tick** while items are selected (a grid takes no assignment): the glyph stays (M2). The digits of the other rows count past it.
- **Section:** Cutting for Cut, Marking for Score and Engrave. Arrows, drag and Alt+arrows as `0101` 4.6, inside its section; undo "Move operation".
- **Armed trash** as `0101` 4.5. Status on arming: "Press again to delete Test grid 1. Its 25 cells and labels become Not assigned." On removal: "Test grid 1 deleted." The items stay on the page (criterion 6). Undo "Delete test grid row".
- **Press** selects the row and opens the Grid block; it does not change the canvas selection (as `0101` decision 6). The block's "Select" button does (6.1).
- Accessible name: "<name>, <kind> test grid, <C> by <R> cells" plus ", Output off".

## 6. The Grid block: fields and rebuild (criteria 7 to 11)

### 6.1 Where it shows

- Under the list (section 3, item 4) while a Test grid row is selected. **Selecting the grid on the canvas** (its top group, or any of its items) selects its row; if the Style or Document tab is active, the panel switches to the Operations tab (M3, an addition to "Active tab": a test grid is edited in the Operations tab, as a fixture is in the Document tab). History, Layers, File and Fonts never switch (as today).
- `role="group"` named "Test grid 1".

### 6.2 Layout, top to bottom

| Row | Content |
|---|---|
| Head, 24px | kind glyph 16px, name 13px semibold ink (ellipsis, tooltip), at the right a text button "Select" (12px, underlined on hover and focus; selects the grid on the canvas) |
| Machine and size, 16px | 11px `--panel-muted-fg`, tabular: "Generic laser · 75 × 62 mm" (profile name of the page's copy; the size with labels) |
| Kind, 28px | `role="radiogroup"` named "Kind", three text segments 78px: Cut, Score, Engrave (M4) |
| "Power, %" heading, 16px | 11px semibold `--panel-muted-fg` |
| Power fields, 28px | three 78px fields: "From", "To", "Columns" |
| "Speed, mm/min" heading, 16px | as above |
| Speed fields, 28px | "From", "To", "Rows" |
| "Cell, mm" heading, 16px | |
| Cell fields, 28px | two 118px fields: "Size", "Gap" |
| Labels, 28px | label "Labels" 12px semibold `--panel-muted-fg` at the left, `PanelSwitch` at the right (`role="switch"`, name "Labels") |
| Label fields (only while Labels is on) | the settings layout of 8.3: "Power" and "Speed" (118px each, units % and mm/min inside), "Passes" (118px) |
| State line (only when edited by hand) | section 6.4 |
| Result | section 7 |

- Gaps: 4px between fields, 8px between groups. Heights: about 300px without the result.
- No X or Y field (M5): the grid is moved like a rectangle (6.5). No Kind-is-fixed rule (M4).
- Field look and keys of `0102` 4.1: Enter or blur commits, Escape reverts, Up and Down by 1 (Shift 10) committed on key-up. Units in the group heading keep the 78px fields readable ("From 12000").

### 6.3 Commit, validation, rebuild

- A valid commit rebuilds cells and labels in place in the same frame: one undo step "Edit test grid" (criterion 9). A Kind change is a commit like any other: it rebuilds, clears the mark, moves the row to its section and keeps its place at that section's end (Proposal).
- **Nothing is clamped** (criterion 2): an invalid value keeps the typed text, the field gets the 2px `--field-invalid` border, `aria-invalid` and the validation chip under it (12px, 4px below, `aria-describedby`). The grid stays as it was. Escape reverts the field; correcting it commits. One chip at a time.
- Texts (at most 70 characters): "Enter a whole number." / "Enter a number." / "Columns are 2 to 12." / "Rows are 2 to 12." / "Power is 1 to 100 %." / "Speed is 1 to 6000 mm/min." (the machine's maximum feed) / "Cell size is 3 to 50 mm." / "Gap is 0 to 20 mm." / "Passes are 1 to 20." / "Grid is 260 mm wide, the bed is 200 mm." (or "high") under the field whose commit would grow the grid past the bed.
- "From" above "To" is valid (the axis runs downward); equal values are valid.
- Rebuild keeps the mark when the marked cell's power and speed are still in the grid (criterion 9), else clears it; the Worked line (7.2) then reads "No cell marked".

### 6.4 Edited by hand (criterion 10)

- When an item of the grid was changed by hand (entered group: a cell moved, deleted or restyled, a label retyped), a state line appears above the result: `PencilLine` 16px, "Edited by hand" 12px ink, and at the right an outline text button "Rebuild" (64 x 24px). No second sentence (M6). Rebuild replaces the items with the fields' grid, keeps or clears the mark as 6.3, and is one undo step "Rebuild test grid". It is undoable, so it does not arm.
- While the line shows, a field commit is stored (one step "Edit test grid") but the items are not rebuilt until Rebuild (criterion 10). The size line of 6.2 shows the fields' size; the result table shows the cells as built, since those are what run.
- Moving the whole grid is not a hand edit.

### 6.5 On the canvas: like a rectangle (criterion 8)

- The grid's top group shows the rectangle's 1px dashed `--accent` box and the centre move (from a box side of 48px). No resize squircles, no rotate glyphs, no skew, no knobs (hidden: a grid's size comes from its fields).
- M opens the Move chip (X, Y, Relative or Absolute, the nine-point Reference), as does a double-click on the centre move handle. S, R and K do nothing for a grid selection. Arrow keys nudge as for any object. Each move is the move step of the app.
- The Select bar shows what a group shows, minus Flip, Guide, Object to path and Linked clone for a grid selection (hidden). Align and Distribute stay.
- A double-click elsewhere on the grid enters the group (`0023`); an item changed there is a hand edit (6.4).

## 7. The result: marking the Worked cell (criteria 12 to 14)

### 7.1 The result table

- Under the fields, 8px below. `role="grid"` named "Result of Test grid 1", one Tab stop, roving focus.
- **Axis line** (16px, `aria-hidden`): "mm/min" in the 40px row-header column, "Power, %" over the cell columns, 11px `--panel-muted-fg`.
- **Header row** (16px): one `columnheader` per column, the power value ("40"), 11px tabular `--panel-muted-fg`, centred.
- **Rows** (24px each): a `rowheader` with the speed value ("3000"), 11px tabular, right-aligned in 40px; then one `gridcell` per column.
- **Cell width:** 24px up to 8 columns (40 + 8 x 24 = 232px); from 9 columns the cells share the 204px (12 columns: 17px wide, 24px high). Below the 24px target in width (M7): the keyboard grid and the canvas square are the other routes.
- **Cell look:** unmarked: a 12px square outline, 1.5px `--lane-line`, centred, no ground. Hover: `--editor-accent-hover` ground. Marked: the square outline in ink with the `Check` glyph 16px over it. The cell whose square is the canvas selection: `--value-fill` ground. Focus: 2px `--editor-accent` ring inside the cell. No colour marks a cell.
- **Press** (or Space or Enter on the focused cell) marks that cell and removes any other mark; pressing the marked cell removes the mark. One undo step "Mark cell" or "Unmark cell" (criterion 13).
- **From the canvas** (criterion 12): selecting one cell's square on the canvas (inside the entered group) highlights its table cell; the mark itself is pressed in the table (M8: no canvas gesture marks).
- Cell names: "<power> % power, <speed> mm/min", toggle button with `aria-pressed`; the marked cell's name ends with ", Worked".

### 7.2 The Worked line and Save as material

- Under the table, 24px: with a mark, `Check` 16px and "Worked: 60 % power, 3000 mm/min" 13px ink tabular; without, "No cell marked" 12px `--panel-muted-fg`.
- **Save as material:** an outline text button, 244 x 28px (the "Add slot" look of `0102`), only while a cell is marked. Without a mark it is not in the tree (M9: hidden, not disabled; no reason sentence).

### 7.3 The Worked mark on the canvas

- A chip centred on the marked cell's square: 20px circle, `--toolbar-bg` ground, 1px `--lane-line` border, `Check` 14px ink. Drawn in View: Operations and View: Drawn while the grid is shown (it is a record of the test, not an operation symbol); never output, never exported, `pointer-events: none`. It is drawn at 20px whatever the zoom, so on a small cell it covers the cell (the mark is the point).

## 8. The Settings block of a selected operation (criteria 24 to 27)

### 8.1 Layout

| Row | Content |
|---|---|
| Head, 24px | kind glyph, the operation's name 13px semibold (ellipsis, tooltip) |
| Source, 28px | the Material pick (8.2): a 244px disclosure button, chevron 12px, label "Material" 12px `--panel-muted-fg` at the left, value 13px ink at the right (the material's name, or "Own numbers"), `aria-expanded` |
| State lines, only in states (b), (c) and "Differs" | glyph 16px and text 12px ink, each wraps to two lines; at most two state lines (8.4) |
| Numbers | the fields of 8.3, own or pointing (8.4) |
| Buttons, 28px, only when one applies | "Reset to material" at the left, "Edit material" at the right (8.4) |

### 8.2 The Material pick (inline list, no popup)

- Expanding the source row shows, directly under it, a `role="radiogroup"` named "Material of <name>", rows 28px in the List row look; the current choice has the selected look (`--value-fill`, 3px bar) and `aria-checked`.
- First row "Own numbers". Then the materials of this machine in the order of the Materials section: name 13px, at the right the thickness "3.0 mm" 12px tabular muted and the kind symbols 12px (Cut red, others ink).
- A material without the set the kind needs stays in the list, muted, `aria-disabled`, the words "No Score set" 11px in place of the symbols, tooltip the same (the "Armed action" rule for an act that is not possible). Damaged and newer records are not listed.
- With more than 8 materials a 28px text field "Filter" (label inside) heads the list and narrows it by name (Proposal). The list grows in the tab's one scroll; no inner scroll area.
- A press or Enter picks: one undo step "Use material", which writes the material's numbers and clears every typed number, also when the linked material is picked again; or "Use own numbers", which ends the link and keeps the numbers that ran, typed ones included, as own numbers (criterion 25). The list closes, focus returns to the source row. Escape closes it without a change.
- **Several operations selected** (criterion 27; amends `0101` 4.4: Ctrl-press toggles a row, Shift-press and Shift+Up/Down extend, M10): the head reads "3 operations", only the source row shows (value "Mixed" when they differ, `--field-placeholder`), the numbers are not shown. A pick applies to all in one step; operations skipped for a missing set are named in an Action notice 4px under the source row, status, 5 s: "Cut 2 skipped: Birch 3 mm has no Cut set." or "2 operations skipped: no Cut set." (at most 70 characters; a partial result, so it is kept).

### 8.3 Own numbers

- Fields in the settings layout used everywhere in this slice: row 1 "Power" (118px, unit "%") and "Speed" (118px, unit "mm/min" 11px), 4px apart; row 2 "Passes" (118px, unit "×"). The implementer measures "Speed 60000 mm/min" in 118px; if it does not fit, the unit moves to a 16px heading "Speed, mm/min" as in 6.2.
- Engrave + Outline: two groups, each with a 24px sub-heading (Engrave glyph and "Hatch", Score glyph and "Outline"). Cut-out: one Cut set, no sub-heading.
- Commit as 6.3 (range texts "Power is 1 to 100 %.", "Speed is 1 to 6000 mm/min.", "Passes are 1 to 20."); one undo step "Edit settings".
- **No settings:** the state line "No settings" (12px `--panel-muted-fg`, no glyph) above the empty fields (M11: no second sentence). The first commit sets Passes to 1 if it is empty; the line goes once Power and Speed are set.

### 8.4 Pointing: states, typed numbers and Reset to material (criteria 24, 26)

| State | State line | Numbers | Buttons |
|---|---|---|---|
| (a) found | none | the fields of 8.3 with the material's current numbers | "Edit material" |
| (b) not found | `Link2Off` "Material not found, using stored numbers" | the fields with the stored numbers; the source value is the stored name | none: the pick is the route to another material or to own numbers (M13) |
| (c) found, no set | `Link2Off` "Material has no Score set, using stored numbers" | the fields with the stored numbers | "Edit material" |
| Differs, with (a), (b) or (c) | `Diff` "Differs from Birch 3 mm" (the line of (b) or (c) first, this one under it) | each typed number in its field, the material's number under it (below) | "Reset to material" added at the left |

- **The numbers are fields while pointing** (criterion 24, Frage 188 B). A typed number keeps the link: it is stored as a typed number for that field only, one undo step "Edit settings", validation and texts as 8.3. The other fields keep following the material.
- **Beside each typed number:** a 16px line under its field, 11px `--panel-muted-fg` tabular, "Material 60" (the material's number, live or stored, without unit; the unit is in the field), left-aligned to the field. It is the field's description ("Material 60 %"). No other mark on the field: the line is the sign, no colour, no border change.
- A number typed equal to the material's removes that typed number: its "Material" line goes; with the last one, the "Differs" line and Reset to material go too, in the same frame.
- **Reset to material:** outline text button, 28px high, `RotateCcw` 16px and "Reset to material" 12px ink, at the left of the buttons row; in the tree only while a number is typed (hidden, not disabled). One undo step "Reset to material", so it does not arm. It removes every typed number of this operation; the fields show the material's numbers. Focus moves to the first number field (the button leaves the tree). Polite status "Numbers reset to Birch 3 mm."
- **Own numbers** in the pick ends the link and keeps what ran (8.2); the Differs line, the "Material" lines and both buttons go.
- **A material edit** (10.3) changes the fields that follow the material in the same frame, and the "Material" line beside each typed number; the typed numbers stay. The fields show what runs; nothing about a stored copy is shown (the "Changed since saved" state is gone, ADR 0054).
- **Speed above the machine** (the profile's max feed was lowered after the test): a state line `TriangleAlert` "Speed 7000 is above this machine's 6000 mm/min." (Proposal); the field shows the number unclamped; `0029` refuses the output. Typing a valid speed clears it.
- **Edit material** expands the Materials section, selects that material, opens its fields, scrolls it into view and moves focus to its row.
- Accessible names: the state lines are part of the block's description; a field reads "Power, percent, 65, material 60 %"; the block's summary reads "Power 65 %, speed 3000 mm/min, 1 pass, from Birch 3 mm, differs".

## 9. The Save as material form (criteria 15, 16)

- A press on Save as material replaces the button with the form in place (the `0045` Add and Edit form look, 244px wide). `role="group"` named "Save as material". Focus in Name, its text selected.
- Rows (inner width 228px):
  1. **Add to**: `role="radiogroup"`, two segments 112px, "New material" (default) and "Existing". Only when this machine has at least one material; else the row is not in the tree.
  2. With New: **Name** (228px text, label inside, default "Material <N>", 1 to 60 characters, the field stops at 60), then **Thickness** (112px, "mm", empty, one decimal) and **Passes** (112px, default 1).
  3. With Existing: the inline list of 8.2 (without "Own numbers"; every material is usable, since a set of any kind can be added), then **Passes**. If the picked material has a set of the grid's kind, one sentence line 12px ink: "Replaces the Cut set (60 %, 3000 mm/min, tested 2026-11-10)."
  4. Buttons, 112px each, 4px apart: **Cancel** (outline) at the left; at the right **Save** (Primary button), or **Replace** in the `--destructive` look with the 400 ms guard of "Inline confirm" when the sentence of row 3 shows.
- Validation on Save (the first invalid field focused, one chip): "Enter a name.", "Enter a thickness.", "Thickness is 0.1 to 100 mm.", "Passes are 1 to 20.".
- Save writes the record; the form closes and Save as material returns; focus on it. No visible notice (success); the polite status says "Birch 3 mm saved." and the Materials header count changes. One undo step "Save as material" (ADR 0055 §9): Ctrl+Z in this session takes back the new record, or the added or replaced set, and the test entry it added (Frage 190 default A); the notices of 13 apply.
- Escape or Cancel drops the draft. A tab change keeps it ("Drafts"); a mark change, a Rebuild or another selected row drops it.
- Write failure: Action notice 4px under the buttons, `role="alert"`, 8 s: "Could not save the material: <reason>." The form stays with its draft.

## 10. The Materials section (criteria 19 to 23, 29)

### 10.1 Header and body

- Disclosure header as "Fixture jobs" (`0103` 3.1): 244 x 28px, chevron 12px, "Materials (N)" 14px ink, N the rows of this machine (damaged and newer included). Collapsed by default, expanded state kept for the session; expanding or collapsing disarms a trash.
- Body: `role="list"` named "Materials of <profile name>". Empty: "No materials for Generic laser." 14px `--panel-muted-fg`, not focusable.
- Order: the user's order (criterion 22); a material that is not in the order list yet (new, or from another computer) goes to the end (M14).

### 10.2 Row (40px, 48 coarse)

- **Line 1:** name 13px ink (flexible, about 150px, ellipsis, tooltip), Trash 24px, group gap 8px, Up 24px, Down 24px. No Show, no Output.
- **Line 2** (11px `--panel-muted-fg`): the thickness "3.0 mm", " · ", then the kind symbols 12px, 4px apart (Cut red, Score and Engrave ink), or "no sets".
- States as `0101` 4.2 (hover, selected with `--value-fill` and the 3px bar, focus ring). Accessible name: "<name>, 3.0 mm, Cut and Engrave sets".
- **Rename:** double-click on the name or F2: line 1 becomes the text field of `0101` 4.4 (buttons leave the tree), 1 to 60 characters, Enter or leaving commits, Escape cancels, empty reverts. One undo step "Rename material".
- **Armed trash:** as `0101` 4.5. Status on arming: "Press again to delete Birch 3 mm. 3 operations here use it." (the count from the open project, left out at 0). The second press removes the file; one undo step "Delete material", and Ctrl+Z in this session writes it back (it still arms, as every row trash of `0101`). Pointing operations go to state (b) and keep their typed numbers.
- **Order:** Up and Down buttons, drag on the name after 4px with the `0101` insertion line and ghost, Alt+Up, Alt+Down, Alt+Home, Alt+End. A library preference, not an undo step (Proposal, ADR 0055 §9).

### 10.3 The selected material's fields

Under the selected row, inside the list (the row and its fields are one window item):

| Row | Content |
|---|---|
| Thickness, 28px | 118px field, "mm" |
| Use, 16px | "Used by 3 operations here" 11px `--panel-muted-fg`; absent at 0 (Proposal) |
| Per set, in the order Cut, Score, Engrave | sub-heading 24px: kind glyph, the kind word 12px semibold, the armed trash at the right ("Delete Cut set of Birch 3 mm"); the fields of 8.3; a source line 11px muted: "Tested 2026-11-10, 5 × 5 grid, cell 60 %, 3000 mm/min" or "Edited by hand 2026-11-11" |

- Commits as 6.3; each commit writes the record (the history keeps the old set, criterion 22) and every pointing operation's numbers change in the same frame. Each commit, and each set removal, is one undo step "Edit material" of the open project (M15); Ctrl+Z takes back the record and its effect on every operation of this project together.
- The test history (criterion 18) is kept in the record and not shown in this slice (M16).
- Write failure: Action notice under the field, `role="alert"`: "Could not save Birch 3 mm: <reason>."; the field reverts.

### 10.4 Damaged and newer records (criterion 29)

- **Damaged:** line 1 `TriangleAlert` 16px ink, "Damaged material" 13px `--panel-muted-fg`, Trash, Up, Down; line 2 the reason ("Power 140 is outside 1 to 100."), ellipsis, tooltip. A press selects the row and opens nothing. Not in any pick. The trash removes the file (a user action, not a read).
- **Newer:** line 1 the name if readable, else "Material", muted; line 2 "Needs a newer Curvyo". No trash (never touched), arrows stay. Not in any pick.
- Accessible names: "Damaged material, <reason>"; "<name>, needs a newer Curvyo".

## 11. Canvas look of a grid (View: Operations)

- Cells take the look of their kind (`0101` 13.2): Cut solid red outline, Score dotted ink, Engrave the hatch. Labels take the Engrave hatch (at 3 mm they read as hatched blocks at small zoom; the values are in the table and the fields).
- Chips: one chip row on the grid's top group box instead of one per cell: the grid's kind and, with Labels on, Engrave (Proposal; per-cell chips would all collide under `0101` 13.4). The Worked chip of 7.3 is extra and outside the 500 cap.
- Show off on the row hides cells and labels; Output off draws them in the Not assigned look (`0101` 13.2).

## 12. Keyboard routes

| Action | Route |
|---|---|
| Add a test grid | Tab to the add group, Right to Add test grid, Enter or Space |
| Rows, rename, trash, order | as `0101` section 9 (Up, Down, F2, Delete, Alt+arrows); multi-select Shift+Up, Shift+Down |
| Enter the row block | Tab from the list; tab order is the visual order of 6.2, 8.1 |
| Kind, Add to | arrows inside the radiogroup |
| Fields | Enter commits, Escape reverts, Up and Down step (Shift 10) |
| Result table | one Tab stop; arrows move, Home and End to the row ends, Ctrl+Home and Ctrl+End to the corners, Space or Enter marks or unmarks |
| Material pick | Enter or Space opens; Up and Down, Enter picks, Escape closes; typing in Filter narrows, Down enters the list |
| Form | Tab through the fields; Enter in a field presses Save (never Replace, which needs its own press after the 400 ms guard); Escape cancels |
| Materials section | Enter or Space on the header; the list one Tab stop, Up and Down, F2, Delete arms, Alt+arrows; Enter on a row opens its fields, Tab enters them |
| Move the grid | M (Move chip), arrows nudge |
| Undo | Ctrl+Z and Redo for every step of section 13, material steps included (in the session that made them); the material order is not undone |

No new key: M already opens the Move chip. Digits skip Test grid rows (section 5).

## 13. Undo steps, notices, status

- **Undo steps** (names in the History list, the Undo tooltip "Undo: Edit material, You, 12:03 (Ctrl+Z)" and the Edit menu): "Add test grid", "Edit test grid", "Rebuild test grid", "Rename test grid", "Delete test grid row", "Move operation", "Mark cell", "Unmark cell", "Edit settings", "Use material", "Use own numbers", "Reset to material", "Save as material", "Edit material" (a set edit or removal, the thickness), "Rename material", "Delete material", and the Show and Output steps of `0101`. Not in undo: the material order, the remembered grid values, the save refresh (no step, no History row).
- **A material step's undo or redo** takes back the project part always and the record when it can (ADR 0055 §9). When it cannot, a notice in the canvas notice slot (where `0020` puts undo feedback), status, 5 s; the project part is done:
  - record changed since the step (another window, a synced edit): "Birch 3 mm changed since; the material was not undone." / "... was not redone."
  - the step's record text was dropped from memory (oldest first, 16 MiB): "Only the project was undone; Birch 3 mm is too old."
  - the record write failed: "Could not undo Birch 3 mm: <reason>." / "Could not redo Birch 3 mm: <reason>." (`role="alert"`, 8 s)
  - a History-list action (go to version, object undo, branch) over a material step (Proposal): "Only the project was restored; materials stay as they are."
  - Each at most 70 characters: a material name over 24 characters is cut with "…" in these texts; the reason is the host's short reason ("storage full").
- **Notices:** the refusals of section 4, the skip notice of 8.2, the write failures of 9 and 10.3, the undo notices above, the validation chips. No success notice anywhere.
- **Polite status (sr-only):** arming texts; "Test grid 1 deleted."; "Birch 3 mm saved."; "Birch 3 mm deleted."; "Numbers reset to Birch 3 mm."; "Cell marked: 60 % power, 3000 mm/min." / "Mark removed."

## 14. Accessible names

| Element | Name | State |
|---|---|---|
| Add test grid | "Add test grid" | |
| Test grid row | "<name>, <kind> test grid, <C> by <R> cells" | `aria-selected`; ", Output off" |
| Grid block | "<name>" (group) | |
| Select | "Select <name> on the canvas" | |
| Kind | "Kind" (radiogroup) | `aria-checked` |
| Fields | "Power from, percent", "Power to, percent", "Columns", "Speed from, millimetres per minute", "Speed to, ...", "Rows", "Cell size, millimetres", "Gap, millimetres", "Label power, percent", "Label speed, ...", "Label passes" | `aria-invalid` with the chip as description |
| Labels | "Labels" (switch) | `aria-checked` |
| Rebuild | "Rebuild <name>" | |
| Result table | "Result of <name>" (grid) | |
| Cell | "<p> % power, <s> mm/min", ", Worked" when marked | `aria-pressed` |
| Save as material | "Save as material" | |
| Form | "Save as material" (group); "Add to" (radiogroup); "Name", "Thickness, millimetres", "Passes"; "Save", "Replace", "Cancel" | |
| Settings block | "Settings of <name>" (group); description: the state line | |
| Material pick | "Material of <name>" (button, `aria-expanded`); list "Material of <name>" (radiogroup) | rows `aria-checked`, `aria-disabled` with "No <kind> set" |
| Edit material | "Edit material <material>" | |
| Reset to material | "Reset to material <material>" | |
| Number field, pointing | "Power, percent" (and Speed, Passes); description "Material 60 %" while typed | |
| Materials header | "Materials (N)" (button) | `aria-expanded`, `aria-controls` |
| Material row | "<name>, <thickness>, <kinds> sets" | `aria-selected` |
| Material trash | "Delete <name>"; armed "Press again to delete <name>" | |
| Set trash | "Delete <kind> set of <name>"; armed "Press again to delete <kind> set of <name>" | |
| Move up, down | "Move <name> up", "Move <name> down" | hidden at the ends |
| Rename field | "Name of <name>" | |

The canvas tick chip and the chips are `aria-hidden`; the mark is read from the table and the Worked line.

## 15. Empty, error and large states

- **No grid, no material:** the add group has its sixth button; the Materials section reads "Materials (0)" and, expanded, the empty line.
- **Grid with no mark:** "No cell marked", no Save button.
- **Operation with no settings:** "No settings" over empty fields.
- **Material missing or without the set:** states (b) and (c), with any typed numbers on top; the file still runs and saves back unchanged.
- **Damaged, newer records:** section 10.4.
- **Write failures:** sections 9 and 10.3.
- **Large:** a 12 x 12 grid: the table is 16 + 16 + 12 x 24 = 320px, rendered in full (144 cells, no window); a field commit rebuilds once, not per keystroke. 200 materials: the Materials list and the pick list are windowed against the tab's scroll (fixed 40px and 28px rows, at most 60 row elements, focused and selected rows kept); the Filter field shows. Budgets are criterion 30's, measured by the implementer.

## 16. Decisions for veto (Proposal)

- **M1 Add, then edit.** Add test grid makes the grid at once with remembered or default values; the fields edit it in place. Replaces the inline make block with "Make grid" (criteria 1 to 3): it is the rectangle model, one control set instead of two.
- **M2 Grid row:** kind glyph, no digit, no tick, line 2 "<Kind> test grid · C × R cells".
- **M3 Tab switch.** Selecting a test grid on the canvas shows the Operations tab (from Style or Document), as the grid's fields live there.
- **M4 Kind is a field.** Cut, Score, Engrave as a segmented field; a change rebuilds, clears the mark and moves the row (criterion 8 excluded Kind; with M1 there is no other place to choose it).
- **M5 No X and Y fields.** The grid moves like a rectangle: drag, arrows, M and the Move chip (criterion 8 lists X and Y; `0102` removed them for fixtures for the same reason).
- **M6 "Edited by hand" without the second sentence;** Rebuild is undoable and does not arm.
- **M7 Table cells under 24px wide** from 9 columns (17px at 12); the keyboard grid and the canvas selection are the other routes. Alternative: a horizontally scrolled table (a second scroll axis).
- **M8 Marking is done in the table only;** a canvas selection of a cell highlights its table cell.
- **M9 Save as material hidden without a mark,** no "Mark the cell that worked." (hint rule, hidden not disabled).
- **M10 Several operation rows can be selected** (Ctrl, Shift, Shift+arrows) for criterion 27; amends `0101` 4.4 "one row at a time". The block then shows only the pick.
- **M11 "No settings"** without "Pick a material or type the numbers." (hint rule).
- **M12 Place:** the row block under the list; Materials after the summary, before the View switch; collapsed by default.
- **M13 The pick is the one route** to another material or to own numbers; states (a) to (c) show "Edit material", and "Reset to material" only while a number is typed, not separate "Pick material" and "Use own numbers" buttons (criteria 24 to 26 name them; the pick does the same in one place).
- **M14 Order:** the user's order; new or unknown materials at the end, not "sorted by name" (criterion 19 against 22).
- **M15 Material edits are undo steps** (reversed after Frage 189 B, ADR 0055 §9): Save as material, Edit material, Rename material, Delete material; Ctrl+Z reverts the record and every pointing operation of this project together, in the session that made the edit. When the record cannot be rewritten, a notice says so (section 13). The order stays outside undo.
- **M17 The material's number under a typed one** ("Material 60"), no mark on the field itself; Reset to material with `RotateCcw` and text, undoable, so it does not arm.
- **M16 The test history is not shown** in this slice; it is kept in the record.

## 17. Changes asked of the product owner

- Criteria 1 to 3: one press adds the grid; the fields and their validation move to the Grid block (M1); the refusals of section 4.
- Criterion 8: Kind is a field (M4); X and Y are the Move chip (M5); the tab switch (M3).
- Criterion 10: "Edited by hand" and "Rebuild" only (M6).
- Criterion 12: marking in the table only (M8).
- Criterion 14: the button is hidden without a mark, no reason text (M9).
- Criterion 19: user order, new at the end (M14).
- Criterion 24: "No settings" only (M11); the buttons of 24 to 26 become the pick plus "Edit material" and, with typed numbers, "Reset to material" (M13); a typed number keeps the link (188 B, feature notes S1), with the material's number under it (M17).
- Criterion 26: the "stored: 60 %" note and the "Changed since saved" state go (S1b); the speed-above-machine line (8.4).
- Criteria 15, 18, 21, 22: material edits are undo steps (M15, feature notes S8, S8a); criterion 21's "not undoable" goes; the undo notices of section 13.
- Criterion 27: multi-select of rows amends `0101` criterion 3 or 4.4 (M10).
- Design system rows to add at acceptance: "Test grid row and Grid block (`0028`)", "Result table (`0028`)", "Settings block and Material pick (`0028`)", "Materials section (`0028`)", glyphs Test grid and Worked. No token.

## 18. Review checklist for the implementer

- Add group: six buttons, the 8px gap, Add test grid adds at once, focus stays, refusals under the button.
- Grid row: kind glyph, no digit, no tick, line 2, section by kind, rename, armed trash with the "become Not assigned" text, arrows and drag.
- Grid block: layout of 6.2, units in headings, no X or Y, invalid values kept with the chip and never clamped, one step per commit, Kind change moves the row, Labels switch hides its fields, "Edited by hand" and Rebuild.
- Canvas: box and centre move only, M chip, Select bar without Flip, Guide, Object to path, Linked clone; Worked chip in both views, never output.
- Result table: grid semantics, 24px rows, cell widths, mark and unmark as one step each, highlight from the canvas, names with ", Worked".
- Save form: in place, Add to only with materials, Replace with its sentence and guard, validation texts, no success notice, draft rules.
- Settings block: pick inline (no popup), disabled rows with "No <kind> set", Filter above 8, states (a), (b), (c) and "Differs from <name>" with symbol and text, editable numbers while pointing with "Material 60" under each typed one, Reset to material only with typed numbers, "Edit material" route, multi-select and the skip notice.
- Materials: collapsed header with count, rows with thickness and symbols, rename, armed trash with the use count, order, the fields of 10.3, damaged and newer rows; Save, edit, rename and delete each one undo step with its name; the undo notices of 13 when the record cannot be rewritten.
- No new colour, no popup, no tip, no success notice; contrast as section 2.

## Links

Spec: `specs/0028-material-test-library/specification.md` (criteria 1 to 31, Fragen 183 to 190), ADRs 0054 and 0055 (`adrs.md`). Models: `specs/0101-laser-operations/ux-notes.md` (sections 3, 4, 7, 9, 13), `specs/0102-laser-fixtures/ux-notes.md` (sections 0, 4.1, 4.3), `specs/0103-fixture-jobs/ux-notes.md` (sections 3, 5.1), `specs/0026-machine-profile/ux-notes.md` (library writes, damaged and newer records). Armed trash: `specs/0081-inline-delete-confirm/ux-notes.md`. Form: `docs/design-system.md`, "Document formats library". Next: `0029` reads the effective settings and owns the output gate.
